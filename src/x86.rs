//! A small IA-32 interpreter (integer, x87 and SSE/SSE2 subset) that runs the
//! machine code stored in Mandelbulb3D's `.m3f` formula files.
//!
//! The formulas are 32-bit Delphi `register` calling convention procedures:
//! `procedure(var x, y, z, w: Double; PIteration3D: Pointer)` with
//! eax = @x, edx = @y, ecx = @z, [esp+4] = PIteration3D, [esp+8] = @w and a
//! `ret 8` at the end.  They only touch the iteration record, their option /
//! constant buffer and the stack, so a flat emulated address space of 64 KiB
//! at a fixed base address is sufficient.
//!
//! The x87 register stack is emulated with `f64` (MB3D ran in 64-bit-mantissa
//! extended precision, so results can differ in the last bits).

use crate::iteration::Iteration;
use std::collections::HashMap;

/// Base address of the emulated memory (also used by the native test oracle).
pub const BASE: u32 = 0x1000_0000;
pub const MEM_SIZE: usize = 0x10000;
/// The allocation: 16 bytes behind the image, so that the unchecked
/// accesses of the translated formulas (address masked to the image) stay
/// inside the allocation.
const MEM_ALLOC: usize = MEM_SIZE + 16;
/// Addresses at or above this value inside the image are "magic" host
/// functions (see [`Machine::host_functions`]).
pub const MAGIC_START: u32 = BASE + 0xF000;
/// Start of the code area (pre-decoded).
pub const CODE_START: u32 = BASE + 0x8000;
/// Return address that ends an emulated call.
pub const RETURN_SENTINEL: u32 = 0xFFFF_FFF0;

#[derive(Debug, Clone, PartialEq)]
pub enum EmuError {
    /// boxed, so that the results of the emulated memory accesses stay small
    Unsupported(Box<(u32, String)>),
    MemFault(u32),
    StepLimit,
    Halt(u32),
}

impl std::fmt::Display for EmuError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            EmuError::Unsupported(e) => write!(f, "unsupported instruction at {:#x}: {}", e.0, e.1),
            EmuError::MemFault(a) => write!(f, "memory access outside the image at {a:#x}"),
            EmuError::StepLimit => write!(f, "instruction limit exceeded"),
            EmuError::Halt(a) => write!(f, "halt/int at {a:#x}"),
        }
    }
}

type R<T> = Result<T, EmuError>;

// ---------------------------------------------------------------------------
// Decoded instructions
// ---------------------------------------------------------------------------

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Mem {
    base: Option<u8>,
    index: Option<(u8, u8)>,
    disp: i32,
}

#[derive(Clone, Copy, Debug, PartialEq)]
enum Rm {
    Reg(u8),
    Mem(Mem),
}

#[derive(Clone, Copy, Debug, PartialEq)]
enum Src {
    Rm(Rm),
    Imm(u32),
}

#[derive(Clone, Copy, Debug, PartialEq)]
enum Alu {
    Add,
    Or,
    Adc,
    Sbb,
    And,
    Sub,
    Xor,
    Cmp,
}

#[derive(Clone, Copy, Debug, PartialEq)]
enum Shift {
    Rol,
    Ror,
    Rcl,
    Rcr,
    Shl,
    Shr,
    Sar,
}

#[derive(Clone, Copy, Debug, PartialEq)]
enum FKind {
    I16,
    I32,
    I64,
    F32,
    F64,
    F80,
}

#[derive(Clone, Copy, Debug, PartialEq)]
enum FOp {
    Add,
    Mul,
    Sub,
    SubR,
    Div,
    DivR,
}

#[derive(Clone, Copy, Debug, PartialEq)]
enum FUn {
    Chs,
    Abs,
    Tst,
    Xam,
    Ld1,
    Ldl2t,
    Ldl2e,
    Ldpi,
    Ldlg2,
    Ldln2,
    Ldz,
    F2xm1,
    Fyl2x,
    Fptan,
    Fpatan,
    Fprem,
    Fprem1,
    Fyl2xp1,
    Sqrt,
    Sincos,
    Rndint,
    Scale,
    Sin,
    Cos,
    Decstp,
    Incstp,
    Nop,
    Xtract,
}

#[derive(Clone, Copy, Debug, PartialEq)]
enum SseOp {
    MovU,      // movupd/movapd/movups/movaps load or reg
    MovStore,  // 128 bit store
    MovSdLoad, // movsd/movq xmm, m (zero high) or reg (low only)
    MovSdStore,
    MovSsLoad,
    MovSsStore,
    MovLpdLoad,
    MovLpdStore,
    MovHpdLoad,
    MovHpdStore,
    MovHlps,
    MovLhps,
    AddPd,
    AddSd,
    SubPd,
    SubSd,
    MulPd,
    MulSd,
    DivPd,
    DivSd,
    SqrtPd,
    SqrtSd,
    MinPd,
    MinSd,
    MaxPd,
    MaxSd,
    AddPs,
    AddSs,
    SubPs,
    SubSs,
    MulPs,
    MulSs,
    DivPs,
    DivSs,
    SqrtSs,
    MinSs,
    MaxSs,
    And,
    AndN,
    Or,
    Xor,
    ShufPd,
    ShufPs,
    UnpckLpd,
    UnpckHpd,
    UnpckLps,
    UnpckHps,
    Pshufd,
    UComiSd,
    ComiSd,
    UComiSs,
    HAddPd,
    HSubPd,
    AddSubPd,
    CvtPd2Dq,
    CvttPd2Dq,
    CvtDq2Pd,
    CvtSs2Sd,
    CvtSd2Ss,
    CvtPs2Pd,
    CvtPd2Ps,
    PAddD,
    PSubD,
    PAddQ,
    PSubQ,
    PCmpEqD,
    PCmpGtD,
    MovqLoad, // F3 0F 7E
    MovqStore,
    Psrlq(u8),
    Psllq(u8),
    Psrldq(u8),
    Pslldq(u8),
    Psrld(u8),
    Pslld(u8),
    Psrad(u8),
}

#[derive(Clone, Debug, PartialEq)]
enum Ins {
    Alu { op: Alu, dst: Rm, src: Src, size: u8 },
    Mov { dst: Rm, src: Src, size: u8 },
    Lea { reg: u8, mem: Mem },
    Push(Src),
    Pop(Rm),
    Inc(Rm, u8),
    Dec(Rm, u8),
    Not(Rm, u8),
    Neg(Rm, u8),
    Mul(Rm, u8),
    IMul1(Rm, u8),
    Div(Rm, u8),
    IDiv(Rm, u8),
    IMul3 { reg: u8, src: Rm, imm: u32 },
    IMul2 { reg: u8, src: Rm },
    Test { a: Rm, b: Src, size: u8 },
    Shift { op: Shift, dst: Rm, cnt: Option<u8>, size: u8 }, // None = CL
    Xchg { a: Rm, b: u8, size: u8 },
    Movzx { reg: u8, src: Rm, size: u8 },
    Movsx { reg: u8, src: Rm, size: u8 },
    Setcc { cc: u8, dst: Rm },
    Cmovcc { cc: u8, reg: u8, src: Rm },
    Jcc { cc: u8, target: u32 },
    Jmp(u32),
    JmpInd(Rm),
    Call(u32),
    CallInd(Rm),
    Ret(u16),
    Leave,
    Cdq,
    Cwde,
    Sahf,
    Lahf,
    Nop,
    Halt,
    // x87
    FLd(FKind, Mem),
    FSt(FKind, Mem, bool),
    FArithMem(FOp, FKind, Mem),
    /// st(dst) = st(dst) op st(src)   (dst or src is 0)
    FArith { op: FOp, dst: u8, src: u8, pop: bool },
    FLdSt(u8),
    FStSt(u8, bool),
    FXch(u8),
    FComSt { i: u8, pops: u8 },
    FComMem { kind: FKind, mem: Mem, pop: bool },
    FComi { i: u8, pop: bool },
    FUn(FUn),
    FnstswAx,
    FnstswMem(Mem),
    FnstcwMem(Mem),
    FldcwMem(Mem),
    FFree(u8),
    FCmov { cc: u8, i: u8 },
    FClex,
    FInit,
    // SSE
    Sse { op: SseOp, dst: u8, src: Rm, imm: u8 },
    SseStore { op: SseOp, dst: Mem, src: u8 },
    Cvtsi2sd { dst: u8, src: Rm },
    Cvtsd2si { reg: u8, src: Rm, trunc: bool },
    MovdToX { dst: u8, src: Rm },
    MovdFromX { dst: Rm, src: u8 },
    MovmskPd { reg: u8, src: u8 },
}

// ---------------------------------------------------------------------------
// Decoder
// ---------------------------------------------------------------------------

struct Dec<'a> {
    code: &'a [u8],
    pos: usize,
    addr0: u32,
}

impl<'a> Dec<'a> {
    fn u8(&mut self) -> R<u8> {
        let b = *self
            .code
            .get(self.pos)
            .ok_or(EmuError::MemFault(self.addr0.wrapping_add(self.pos as u32)))?;
        self.pos += 1;
        Ok(b)
    }
    fn u16(&mut self) -> R<u16> {
        Ok(self.u8()? as u16 | (self.u8()? as u16) << 8)
    }
    fn u32(&mut self) -> R<u32> {
        Ok(self.u16()? as u32 | (self.u16()? as u32) << 16)
    }
    fn i8(&mut self) -> R<i32> {
        Ok(self.u8()? as i8 as i32)
    }
    /// ModRM decoding -> (reg field, r/m operand)
    fn modrm(&mut self) -> R<(u8, Rm)> {
        let m = self.u8()?;
        let md = m >> 6;
        let reg = (m >> 3) & 7;
        let rm = m & 7;
        if md == 3 {
            return Ok((reg, Rm::Reg(rm)));
        }
        let mut mem = Mem { base: None, index: None, disp: 0 };
        if rm == 4 {
            let sib = self.u8()?;
            let scale = 1u8 << (sib >> 6);
            let idx = (sib >> 3) & 7;
            let base = sib & 7;
            if idx != 4 {
                mem.index = Some((idx, scale));
            }
            if base == 5 && md == 0 {
                mem.disp = self.u32()? as i32;
            } else {
                mem.base = Some(base);
            }
        } else if rm == 5 && md == 0 {
            mem.disp = self.u32()? as i32;
            return Ok((reg, Rm::Mem(mem)));
        } else {
            mem.base = Some(rm);
        }
        match md {
            1 => mem.disp = mem.disp.wrapping_add(self.i8()?),
            2 => mem.disp = mem.disp.wrapping_add(self.u32()? as i32),
            _ => {}
        }
        Ok((reg, Rm::Mem(mem)))
    }
    fn mem(&mut self) -> R<(u8, Mem)> {
        match self.modrm()? {
            (r, Rm::Mem(m)) => Ok((r, m)),
            _ => Err(unsup(self.addr0, "register operand where memory expected".into())),
        }
    }
    fn imm(&mut self, size: u8) -> R<u32> {
        Ok(match size {
            1 => self.u8()? as u32,
            2 => self.u16()? as u32,
            _ => self.u32()?,
        })
    }
}

#[cold]
fn unsup(addr: u32, what: String) -> EmuError {
    EmuError::Unsupported(Box::new((addr, what)))
}

/// Decode one instruction at `addr` (code slice starts at `addr`).
fn decode(code: &[u8], addr: u32) -> R<(Ins, usize)> {
    let mut d = Dec { code, pos: 0, addr0: addr };
    let mut opsize16 = false;
    let mut rep: Option<u8> = None; // F2 / F3
    let mut op;
    loop {
        op = d.u8()?;
        match op {
            0x66 => opsize16 = true,
            0xF2 | 0xF3 => rep = Some(op),
            0x26 | 0x2E | 0x36 | 0x3E | 0x64 | 0x65 | 0xF0 => {} // segment / lock: ignore
            _ => break,
        }
    }
    let osz: u8 = if opsize16 { 2 } else { 4 };
    let next = |d: &Dec| addr.wrapping_add(d.pos as u32);
    let ins = match op {
        0x00..=0x3F if (op & 7) < 6 => {
            let alu = [Alu::Add, Alu::Or, Alu::Adc, Alu::Sbb, Alu::And, Alu::Sub, Alu::Xor, Alu::Cmp]
                [(op >> 3) as usize];
            match op & 7 {
                0 => {
                    let (r, rm) = d.modrm()?;
                    Ins::Alu { op: alu, dst: rm, src: Src::Rm(Rm::Reg(r)), size: 1 }
                }
                1 => {
                    let (r, rm) = d.modrm()?;
                    Ins::Alu { op: alu, dst: rm, src: Src::Rm(Rm::Reg(r)), size: osz }
                }
                2 => {
                    let (r, rm) = d.modrm()?;
                    Ins::Alu { op: alu, dst: Rm::Reg(r), src: Src::Rm(rm), size: 1 }
                }
                3 => {
                    let (r, rm) = d.modrm()?;
                    Ins::Alu { op: alu, dst: Rm::Reg(r), src: Src::Rm(rm), size: osz }
                }
                4 => Ins::Alu { op: alu, dst: Rm::Reg(0), src: Src::Imm(d.u8()? as u32), size: 1 },
                _ => Ins::Alu { op: alu, dst: Rm::Reg(0), src: Src::Imm(d.imm(osz)?), size: osz },
            }
        }
        0x40..=0x47 => Ins::Inc(Rm::Reg(op - 0x40), osz),
        0x48..=0x4F => Ins::Dec(Rm::Reg(op - 0x48), osz),
        0x50..=0x57 => Ins::Push(Src::Rm(Rm::Reg(op - 0x50))),
        0x58..=0x5F => Ins::Pop(Rm::Reg(op - 0x58)),
        0x68 => Ins::Push(Src::Imm(d.u32()?)),
        0x6A => Ins::Push(Src::Imm(d.i8()? as u32)),
        0x69 | 0x6B => {
            let (r, rm) = d.modrm()?;
            let imm = if op == 0x69 { d.imm(osz)? } else { d.i8()? as u32 };
            Ins::IMul3 { reg: r, src: rm, imm }
        }
        0x70..=0x7F => {
            let rel = d.i8()?;
            Ins::Jcc { cc: op - 0x70, target: next(&d).wrapping_add(rel as u32) }
        }
        0x80 | 0x81 | 0x83 => {
            let (r, rm) = d.modrm()?;
            let size = if op == 0x80 { 1 } else { osz };
            let imm = match op {
                0x80 => d.u8()? as u32,
                0x81 => d.imm(osz)?,
                _ => d.i8()? as u32,
            };
            let alu = [Alu::Add, Alu::Or, Alu::Adc, Alu::Sbb, Alu::And, Alu::Sub, Alu::Xor, Alu::Cmp]
                [r as usize];
            Ins::Alu { op: alu, dst: rm, src: Src::Imm(imm), size }
        }
        0x84 | 0x85 => {
            let (r, rm) = d.modrm()?;
            Ins::Test { a: rm, b: Src::Rm(Rm::Reg(r)), size: if op == 0x84 { 1 } else { osz } }
        }
        0x86 | 0x87 => {
            let (r, rm) = d.modrm()?;
            Ins::Xchg { a: rm, b: r, size: if op == 0x86 { 1 } else { osz } }
        }
        0x88..=0x8B => {
            let (r, rm) = d.modrm()?;
            let size = if op & 1 == 0 { 1 } else { osz };
            if op < 0x8A {
                Ins::Mov { dst: rm, src: Src::Rm(Rm::Reg(r)), size }
            } else {
                Ins::Mov { dst: Rm::Reg(r), src: Src::Rm(rm), size }
            }
        }
        0x8D => {
            let (r, m) = d.mem()?;
            Ins::Lea { reg: r, mem: m }
        }
        0x8F => {
            let (_, rm) = d.modrm()?;
            Ins::Pop(rm)
        }
        0x90 => Ins::Nop,
        0x91..=0x97 => Ins::Xchg { a: Rm::Reg(op - 0x90), b: 0, size: osz },
        0x98 => Ins::Cwde,
        0x99 => Ins::Cdq,
        0x9B => Ins::Nop, // fwait
        0x9E => Ins::Sahf,
        0x9F => Ins::Lahf,
        0xA8 => Ins::Test { a: Rm::Reg(0), b: Src::Imm(d.u8()? as u32), size: 1 },
        0xA9 => Ins::Test { a: Rm::Reg(0), b: Src::Imm(d.imm(osz)?), size: osz },
        0xA1 => {
            let a = d.u32()?;
            Ins::Mov { dst: Rm::Reg(0), src: Src::Rm(Rm::Mem(Mem { base: None, index: None, disp: a as i32 })), size: osz }
        }
        0xA3 => {
            let a = d.u32()?;
            Ins::Mov { dst: Rm::Mem(Mem { base: None, index: None, disp: a as i32 }), src: Src::Rm(Rm::Reg(0)), size: osz }
        }
        0xB0..=0xB7 => Ins::Mov { dst: Rm::Reg(op - 0xB0), src: Src::Imm(d.u8()? as u32), size: 1 },
        0xB8..=0xBF => Ins::Mov { dst: Rm::Reg(op - 0xB8), src: Src::Imm(d.imm(osz)?), size: osz },
        0xC0 | 0xC1 | 0xD0..=0xD3 => {
            let (r, rm) = d.modrm()?;
            let size = if op & 1 == 0 { 1 } else { osz };
            let cnt = match op {
                0xC0 | 0xC1 => Some(d.u8()?),
                0xD0 | 0xD1 => Some(1),
                _ => None,
            };
            let sop = match r {
                0 => Shift::Rol,
                1 => Shift::Ror,
                2 => Shift::Rcl,
                3 => Shift::Rcr,
                4 | 6 => Shift::Shl,
                5 => Shift::Shr,
                _ => Shift::Sar,
            };
            Ins::Shift { op: sop, dst: rm, cnt, size }
        }
        0xC2 => Ins::Ret(d.u16()?),
        0xC3 => Ins::Ret(0),
        0xC6 | 0xC7 => {
            let (_, rm) = d.modrm()?;
            let size = if op == 0xC6 { 1 } else { osz };
            Ins::Mov { dst: rm, src: Src::Imm(d.imm(size)?), size }
        }
        0xC9 => Ins::Leave,
        0xCC | 0xCD | 0xF4 => Ins::Halt,
        0xE8 => {
            let rel = d.u32()?;
            Ins::Call(next(&d).wrapping_add(rel))
        }
        0xE9 => {
            let rel = d.u32()?;
            Ins::Jmp(next(&d).wrapping_add(rel))
        }
        0xEB => {
            let rel = d.i8()?;
            Ins::Jmp(next(&d).wrapping_add(rel as u32))
        }
        0xF6 | 0xF7 => {
            let (r, rm) = d.modrm()?;
            let size = if op == 0xF6 { 1 } else { osz };
            match r {
                0 | 1 => Ins::Test { a: rm, b: Src::Imm(d.imm(size)?), size },
                2 => Ins::Not(rm, size),
                3 => Ins::Neg(rm, size),
                4 => Ins::Mul(rm, size),
                5 => Ins::IMul1(rm, size),
                6 => Ins::Div(rm, size),
                _ => Ins::IDiv(rm, size),
            }
        }
        0xFE => {
            let (r, rm) = d.modrm()?;
            match r {
                0 => Ins::Inc(rm, 1),
                1 => Ins::Dec(rm, 1),
                _ => return Err(unsup(addr, format!("FE /{r}"))),
            }
        }
        0xFF => {
            let (r, rm) = d.modrm()?;
            match r {
                0 => Ins::Inc(rm, osz),
                1 => Ins::Dec(rm, osz),
                2 => Ins::CallInd(rm),
                4 => Ins::JmpInd(rm),
                6 => Ins::Push(Src::Rm(rm)),
                _ => return Err(unsup(addr, format!("FF /{r}"))),
            }
        }
        0xD8..=0xDF => decode_fpu(&mut d, op, addr)?,
        0x0F => decode_0f(&mut d, addr, opsize16, rep, osz)?,
        _ => return Err(unsup(addr, format!("opcode {op:02X}"))),
    };
    Ok((ins, d.pos))
}

fn decode_fpu(d: &mut Dec, op: u8, addr: u32) -> R<Ins> {
    let m = *d.code.get(d.pos).ok_or(EmuError::MemFault(addr))?;
    let arith = [FOp::Add, FOp::Mul, FOp::Add, FOp::Add, FOp::Sub, FOp::SubR, FOp::Div, FOp::DivR];
    if m < 0xC0 {
        let (r, mem) = d.mem()?;
        let r = r as usize;
        return Ok(match op {
            0xD8 | 0xDC | 0xDA | 0xDE => {
                let kind = match op {
                    0xD8 => FKind::F32,
                    0xDC => FKind::F64,
                    0xDA => FKind::I32,
                    _ => FKind::I16,
                };
                match r {
                    2 => Ins::FComMem { kind, mem, pop: false },
                    3 => Ins::FComMem { kind, mem, pop: true },
                    _ => Ins::FArithMem(arith[r], kind, mem),
                }
            }
            0xD9 => match r {
                0 => Ins::FLd(FKind::F32, mem),
                2 => Ins::FSt(FKind::F32, mem, false),
                3 => Ins::FSt(FKind::F32, mem, true),
                5 => Ins::FldcwMem(mem),
                7 => Ins::FnstcwMem(mem),
                _ => return Err(unsup(addr, format!("D9 /{r} mem"))),
            },
            0xDB => match r {
                0 => Ins::FLd(FKind::I32, mem),
                1 => Ins::FSt(FKind::I32, mem, true), // fisttp (truncating) - approx
                2 => Ins::FSt(FKind::I32, mem, false),
                3 => Ins::FSt(FKind::I32, mem, true),
                5 => Ins::FLd(FKind::F80, mem),
                7 => Ins::FSt(FKind::F80, mem, true),
                _ => return Err(unsup(addr, format!("DB /{r} mem"))),
            },
            0xDD => match r {
                0 => Ins::FLd(FKind::F64, mem),
                1 => Ins::FSt(FKind::I64, mem, true),
                2 => Ins::FSt(FKind::F64, mem, false),
                3 => Ins::FSt(FKind::F64, mem, true),
                7 => Ins::FnstswMem(mem),
                _ => return Err(unsup(addr, format!("DD /{r} mem"))),
            },
            _ => match r {
                // DF
                0 => Ins::FLd(FKind::I16, mem),
                2 => Ins::FSt(FKind::I16, mem, false),
                3 => Ins::FSt(FKind::I16, mem, true),
                5 => Ins::FLd(FKind::I64, mem),
                7 => Ins::FSt(FKind::I64, mem, true),
                _ => return Err(unsup(addr, format!("DF /{r} mem"))),
            },
        });
    }
    d.pos += 1;
    let i = m & 7;
    let row = (m >> 3) & 7; // 0..7 for C0..FF
    Ok(match op {
        0xD8 => match row {
            2 => Ins::FComSt { i, pops: 0 },
            3 => Ins::FComSt { i, pops: 1 },
            r => Ins::FArith { op: arith[r as usize], dst: 0, src: i, pop: false },
        },
        0xDC | 0xDE => {
            let pop = op == 0xDE;
            match row {
                0 => Ins::FArith { op: FOp::Add, dst: i, src: 0, pop },
                1 => Ins::FArith { op: FOp::Mul, dst: i, src: 0, pop },
                2 if !pop => Ins::FComSt { i, pops: 0 },
                3 if !pop => Ins::FComSt { i, pops: 1 },
                3 if pop && i == 1 => Ins::FComSt { i: 1, pops: 2 },
                // Intel: DC E0+i FSUBR st(i),st0 : st(i) = st0 - st(i)
                4 => Ins::FArith { op: FOp::SubR, dst: i, src: 0, pop },
                5 => Ins::FArith { op: FOp::Sub, dst: i, src: 0, pop },
                6 => Ins::FArith { op: FOp::DivR, dst: i, src: 0, pop },
                7 => Ins::FArith { op: FOp::Div, dst: i, src: 0, pop },
                _ => return Err(unsup(addr, format!("{op:02X} {m:02X}"))),
            }
        }
        0xD9 => match m {
            0xC0..=0xC7 => Ins::FLdSt(i),
            0xC8..=0xCF => Ins::FXch(i),
            0xD0 => Ins::FUn(FUn::Nop),
            0xD8..=0xDF => Ins::FStSt(i, true), // fstp1 (undocumented)
            0xE0 => Ins::FUn(FUn::Chs),
            0xE1 => Ins::FUn(FUn::Abs),
            0xE4 => Ins::FUn(FUn::Tst),
            0xE5 => Ins::FUn(FUn::Xam),
            0xE8 => Ins::FUn(FUn::Ld1),
            0xE9 => Ins::FUn(FUn::Ldl2t),
            0xEA => Ins::FUn(FUn::Ldl2e),
            0xEB => Ins::FUn(FUn::Ldpi),
            0xEC => Ins::FUn(FUn::Ldlg2),
            0xED => Ins::FUn(FUn::Ldln2),
            0xEE => Ins::FUn(FUn::Ldz),
            0xF0 => Ins::FUn(FUn::F2xm1),
            0xF1 => Ins::FUn(FUn::Fyl2x),
            0xF2 => Ins::FUn(FUn::Fptan),
            0xF3 => Ins::FUn(FUn::Fpatan),
            0xF4 => Ins::FUn(FUn::Xtract),
            0xF5 => Ins::FUn(FUn::Fprem1),
            0xF6 => Ins::FUn(FUn::Decstp),
            0xF7 => Ins::FUn(FUn::Incstp),
            0xF8 => Ins::FUn(FUn::Fprem),
            0xF9 => Ins::FUn(FUn::Fyl2xp1),
            0xFA => Ins::FUn(FUn::Sqrt),
            0xFB => Ins::FUn(FUn::Sincos),
            0xFC => Ins::FUn(FUn::Rndint),
            0xFD => Ins::FUn(FUn::Scale),
            0xFE => Ins::FUn(FUn::Sin),
            0xFF => Ins::FUn(FUn::Cos),
            _ => return Err(unsup(addr, format!("D9 {m:02X}"))),
        },
        0xDA => match m {
            0xC0..=0xDF => Ins::FCmov { cc: [2u8, 4, 6, 10][row as usize], i },
            0xE9 => Ins::FComSt { i: 1, pops: 2 },
            _ => return Err(unsup(addr, format!("DA {m:02X}"))),
        },
        0xDB => match m {
            0xC0..=0xDF => Ins::FCmov { cc: [3u8, 5, 7, 11][row as usize], i },
            0xE2 => Ins::FClex,
            0xE3 => Ins::FInit,
            0xE8..=0xF7 => Ins::FComi { i, pop: false },
            _ => return Err(unsup(addr, format!("DB {m:02X}"))),
        },
        0xDD => match row {
            0 => Ins::FFree(i),
            2 => Ins::FStSt(i, false),
            3 => Ins::FStSt(i, true),
            4 => Ins::FComSt { i, pops: 0 },
            5 => Ins::FComSt { i, pops: 1 },
            _ => return Err(unsup(addr, format!("DD {m:02X}"))),
        },
        _ => match m {
            // DF
            0xE0 => Ins::FnstswAx,
            // ffreep: no tag word is kept, so freeing st(i) and popping is fstp st(0)
            0xC0..=0xC7 => Ins::FStSt(0, true),
            0xE8..=0xF7 => Ins::FComi { i, pop: true },
            _ => return Err(unsup(addr, format!("DF {m:02X}"))),
        },
    })
}

fn decode_0f(d: &mut Dec, addr: u32, p66: bool, rep: Option<u8>, osz: u8) -> R<Ins> {
    let op = d.u8()?;
    let f2 = rep == Some(0xF2);
    let f3 = rep == Some(0xF3);
    let xop = |d: &mut Dec, sop: SseOp| -> R<Ins> {
        let (r, rm) = d.modrm()?;
        Ok(Ins::Sse { op: sop, dst: r, src: rm, imm: 0 })
    };
    let store = |d: &mut Dec, sop: SseOp, load: SseOp| -> R<Ins> {
        let (r, rm) = d.modrm()?;
        Ok(match rm {
            Rm::Mem(m) => Ins::SseStore { op: sop, dst: m, src: r },
            Rm::Reg(x) => Ins::Sse { op: load, dst: x, src: Rm::Reg(r), imm: 0 },
        })
    };
    Ok(match op {
        0x1F => {
            d.modrm()?;
            Ins::Nop
        }
        0x10 => xop(
            d,
            if f2 {
                SseOp::MovSdLoad
            } else if f3 {
                SseOp::MovSsLoad
            } else {
                SseOp::MovU
            },
        )?,
        0x11 => {
            if f2 {
                store(d, SseOp::MovSdStore, SseOp::MovSdLoad)?
            } else if f3 {
                store(d, SseOp::MovSsStore, SseOp::MovSsLoad)?
            } else {
                store(d, SseOp::MovStore, SseOp::MovU)?
            }
        }
        0x28 => xop(d, SseOp::MovU)?,
        0x29 => store(d, SseOp::MovStore, SseOp::MovU)?,
        0x12 => {
            let (r, rm) = d.modrm()?;
            match rm {
                Rm::Reg(x) if !p66 => Ins::Sse { op: SseOp::MovHlps, dst: r, src: Rm::Reg(x), imm: 0 },
                _ => Ins::Sse { op: SseOp::MovLpdLoad, dst: r, src: rm, imm: 0 },
            }
        }
        0x13 => store(d, SseOp::MovLpdStore, SseOp::MovLpdLoad)?,
        0x16 => {
            let (r, rm) = d.modrm()?;
            match rm {
                Rm::Reg(x) if !p66 => Ins::Sse { op: SseOp::MovLhps, dst: r, src: Rm::Reg(x), imm: 0 },
                _ => Ins::Sse { op: SseOp::MovHpdLoad, dst: r, src: rm, imm: 0 },
            }
        }
        0x17 => store(d, SseOp::MovHpdStore, SseOp::MovHpdLoad)?,
        0x14 => xop(d, if p66 { SseOp::UnpckLpd } else { SseOp::UnpckLps })?,
        0x15 => xop(d, if p66 { SseOp::UnpckHpd } else { SseOp::UnpckHps })?,
        0x2A if f2 => {
            let (r, rm) = d.modrm()?;
            Ins::Cvtsi2sd { dst: r, src: rm }
        }
        0x2C | 0x2D if f2 => {
            let (r, rm) = d.modrm()?;
            Ins::Cvtsd2si { reg: r, src: rm, trunc: op == 0x2C }
        }
        0x2E => xop(d, if p66 { SseOp::UComiSd } else { SseOp::UComiSs })?,
        0x2F => xop(d, if p66 { SseOp::ComiSd } else { SseOp::UComiSs })?,
        0x40..=0x4F => {
            let (r, rm) = d.modrm()?;
            Ins::Cmovcc { cc: op - 0x40, reg: r, src: rm }
        }
        0x50 if p66 => {
            let (r, rm) = d.modrm()?;
            match rm {
                Rm::Reg(x) => Ins::MovmskPd { reg: r, src: x },
                _ => return Err(unsup(addr, "movmskpd mem".into())),
            }
        }
        0x51 if !f2 && !f3 && !p66 => return Err(unsup(addr, "sqrtps".into())),
        0x51 => xop(d, if f2 { SseOp::SqrtSd } else if f3 { SseOp::SqrtSs } else { SseOp::SqrtPd })?,
        0x5D | 0x5F if !f2 && !f3 && !p66 => return Err(unsup(addr, "minps/maxps".into())),
        0x54 => xop(d, SseOp::And)?,
        0x55 => xop(d, SseOp::AndN)?,
        0x56 => xop(d, SseOp::Or)?,
        0x57 => xop(d, SseOp::Xor)?,
        0x58 | 0x59 | 0x5C | 0x5D | 0x5E | 0x5F => {
            let idx = match op {
                0x58 => 0,
                0x59 => 1,
                0x5C => 2,
                0x5D => 3,
                0x5E => 4,
                _ => 5,
            };
            let ops = if f2 {
                [SseOp::AddSd, SseOp::MulSd, SseOp::SubSd, SseOp::MinSd, SseOp::DivSd, SseOp::MaxSd]
            } else if p66 {
                [SseOp::AddPd, SseOp::MulPd, SseOp::SubPd, SseOp::MinPd, SseOp::DivPd, SseOp::MaxPd]
            } else if f3 {
                [SseOp::AddSs, SseOp::MulSs, SseOp::SubSs, SseOp::MinSs, SseOp::DivSs, SseOp::MaxSs]
            } else {
                // minps/maxps (indices 3, 5) are rejected above
                [SseOp::AddPs, SseOp::MulPs, SseOp::SubPs, SseOp::AddPs, SseOp::DivPs, SseOp::AddPs]
            };
            xop(d, ops[idx])?
        }
        0x5A => xop(
            d,
            if f2 {
                SseOp::CvtSd2Ss
            } else if f3 {
                SseOp::CvtSs2Sd
            } else if p66 {
                SseOp::CvtPd2Ps
            } else {
                SseOp::CvtPs2Pd
            },
        )?,
        0x6E if p66 => {
            let (r, rm) = d.modrm()?;
            Ins::MovdToX { dst: r, src: rm }
        }
        0x7E if p66 => {
            let (r, rm) = d.modrm()?;
            Ins::MovdFromX { dst: rm, src: r }
        }
        0x7E if f3 => xop(d, SseOp::MovqLoad)?,
        0xD6 if p66 => store(d, SseOp::MovqStore, SseOp::MovqLoad)?,
        0x6F if p66 || f3 => xop(d, SseOp::MovU)?,
        0x7F if p66 || f3 => store(d, SseOp::MovStore, SseOp::MovU)?,
        0x70 if p66 => {
            let (r, rm) = d.modrm()?;
            let imm = d.u8()?;
            Ins::Sse { op: SseOp::Pshufd, dst: r, src: rm, imm }
        }
        0x72 | 0x73 if p66 => {
            let (r, rm) = d.modrm()?;
            let imm = d.u8()?;
            let x = match rm {
                Rm::Reg(x) => x,
                _ => return Err(unsup(addr, "shift mem".into())),
            };
            let sop = match (op, r) {
                (0x73, 2) => SseOp::Psrlq(imm),
                (0x73, 6) => SseOp::Psllq(imm),
                (0x73, 3) => SseOp::Psrldq(imm),
                (0x73, 7) => SseOp::Pslldq(imm),
                (0x72, 2) => SseOp::Psrld(imm),
                (0x72, 6) => SseOp::Pslld(imm),
                (0x72, 4) => SseOp::Psrad(imm),
                _ => return Err(unsup(addr, format!("0F {op:02X} /{r}"))),
            };
            Ins::Sse { op: sop, dst: x, src: Rm::Reg(x), imm }
        }
        0x7C if p66 => xop(d, SseOp::HAddPd)?,
        0x7D if p66 => xop(d, SseOp::HSubPd)?,
        0xD0 if p66 => xop(d, SseOp::AddSubPd)?,
        0x80..=0x8F => {
            let rel = d.u32()?;
            Ins::Jcc { cc: op - 0x80, target: addr.wrapping_add(d.pos as u32).wrapping_add(rel) }
        }
        0x90..=0x9F => {
            let (_, rm) = d.modrm()?;
            Ins::Setcc { cc: op - 0x90, dst: rm }
        }
        0xAF => {
            let (r, rm) = d.modrm()?;
            Ins::IMul2 { reg: r, src: rm }
        }
        0xB6 | 0xB7 => {
            let (r, rm) = d.modrm()?;
            Ins::Movzx { reg: r, src: rm, size: if op == 0xB6 { 1 } else { 2 } }
        }
        0xBE | 0xBF => {
            let (r, rm) = d.modrm()?;
            Ins::Movsx { reg: r, src: rm, size: if op == 0xBE { 1 } else { 2 } }
        }
        0xC6 => {
            let (r, rm) = d.modrm()?;
            let imm = d.u8()?;
            Ins::Sse { op: if p66 { SseOp::ShufPd } else { SseOp::ShufPs }, dst: r, src: rm, imm }
        }
        0xE6 if f2 => xop(d, SseOp::CvtPd2Dq)?,
        0xE6 if f3 => xop(d, SseOp::CvtDq2Pd)?,
        0xE6 if p66 => xop(d, SseOp::CvttPd2Dq)?,
        0xFE if p66 => xop(d, SseOp::PAddD)?,
        0xFA if p66 => xop(d, SseOp::PSubD)?,
        0xD4 if p66 => xop(d, SseOp::PAddQ)?,
        0xFB if p66 => xop(d, SseOp::PSubQ)?,
        0x76 if p66 => xop(d, SseOp::PCmpEqD)?,
        0x66 if p66 => xop(d, SseOp::PCmpGtD)?,
        0xDB if p66 => xop(d, SseOp::And)?,
        0xDF if p66 => xop(d, SseOp::AndN)?,
        0xEB if p66 => xop(d, SseOp::Or)?,
        0xEF if p66 => xop(d, SseOp::Xor)?,
        _ => {
            let _ = osz;
            return Err(unsup(addr, format!("0F {op:02X}{}", if p66 { " (66)" } else { "" })));
        }
    })
}

// ---------------------------------------------------------------------------
// Machine
// ---------------------------------------------------------------------------

/// Host functions callable from formula code (`call` to a magic address).
pub type HostFn = fn(&mut Machine) -> R<()>;

/// The arithmetic flags of the emulated CPU.  The interpreter keeps them in
/// the machine; the translated formulas (`native/`) keep a local copy, so
/// that the compiler can drop flags nobody reads.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Flags {
    pub cf: bool,
    pub zf: bool,
    pub sf: bool,
    pub of: bool,
    pub pf: bool,
    pub af: bool,
}

impl Flags {
    #[inline(always)]
    pub fn set_szp(&mut self, r: u32, size: u8) {
        let bits = size as u32 * 8;
        self.zf = mask(r, size) == 0;
        self.sf = (r >> (bits - 1)) & 1 == 1;
        self.pf = (r as u8).count_ones() % 2 == 0;
    }
    #[inline(always)]
    pub fn cond(&self, cc: u8) -> bool {
        let r = match cc >> 1 {
            0 => self.of,
            1 => self.cf,
            2 => self.zf,
            3 => self.cf || self.zf,
            4 => self.sf,
            5 => self.pf,
            6 => self.sf != self.of,
            _ => self.zf || (self.sf != self.of),
        };
        if cc & 1 == 1 {
            !r
        } else {
            r
        }
    }
    #[inline(always)]
    fn alu(&mut self, op: Alu, a: u32, b: u32, size: u8) -> u32 {
        let bits = size as u32 * 8;
        let m = if size == 4 { u32::MAX } else { (1u32 << bits) - 1 };
        let sign = 1u32 << (bits - 1);
        let (a, b) = (a & m, b & m);
        let r = match op {
            Alu::Add | Alu::Adc => {
                let c = if op == Alu::Adc && self.cf { 1u64 } else { 0 };
                let full = a as u64 + b as u64 + c;
                let r = (full as u32) & m;
                self.cf = full > m as u64;
                self.of = ((a ^ r) & (b ^ r) & sign) != 0;
                self.af = ((a ^ b ^ r) & 0x10) != 0;
                r
            }
            Alu::Sub | Alu::Sbb | Alu::Cmp => {
                let c = if op == Alu::Sbb && self.cf { 1u64 } else { 0 };
                let r = (a as u64).wrapping_sub(b as u64).wrapping_sub(c) as u32 & m;
                self.cf = (b as u64 + c) > a as u64;
                self.of = ((a ^ b) & (a ^ r) & sign) != 0;
                self.af = ((a ^ b ^ r) & 0x10) != 0;
                r
            }
            Alu::And | Alu::Or | Alu::Xor => {
                let r = match op {
                    Alu::And => a & b,
                    Alu::Or => a | b,
                    _ => a ^ b,
                };
                self.cf = false;
                self.of = false;
                r
            }
        };
        self.set_szp(r, size);
        r
    }
}

#[inline(always)]
fn reg_get(regs: &[u32; 8], r: u8, size: u8) -> u32 {
    match size {
        1 => {
            if r < 4 {
                regs[r as usize] & 0xFF
            } else {
                (regs[(r - 4) as usize] >> 8) & 0xFF
            }
        }
        2 => regs[r as usize] & 0xFFFF,
        _ => regs[r as usize],
    }
}

#[inline(always)]
fn reg_set(regs: &mut [u32; 8], r: u8, size: u8, v: u32) {
    match size {
        1 => {
            if r < 4 {
                let x = &mut regs[r as usize];
                *x = (*x & !0xFF) | (v & 0xFF);
            } else {
                let x = &mut regs[(r - 4) as usize];
                *x = (*x & !0xFF00) | ((v & 0xFF) << 8);
            }
        }
        2 => {
            let x = &mut regs[r as usize];
            *x = (*x & !0xFFFF) | (v & 0xFFFF);
        }
        _ => regs[r as usize] = v,
    }
}

#[inline(always)]
fn shift_reg_l(regs: &mut [u32; 8], fl: &mut Flags, op: Shift, reg: u8, c: u32, size: u8) {
    let c = c & 31;
    if c == 0 {
        return;
    }
    let bits = size as u32 * 8;
    let v = reg_get(regs, reg, size);
    let m = mask(u32::MAX, size);
    let r = match op {
        Shift::Shl => {
            fl.cf = c <= bits && (v >> (bits - c)) & 1 == 1;
            let r = if c >= 32 { 0 } else { (v << c) & m };
            fl.of = ((r >> (bits - 1)) & 1 == 1) != fl.cf;
            r
        }
        Shift::Shr => {
            fl.cf = (v >> (c - 1)) & 1 == 1;
            fl.of = (v >> (bits - 1)) & 1 == 1;
            if c >= 32 { 0 } else { v >> c }
        }
        _ => {
            let sv = sign_extend(v, size);
            fl.cf = (sv >> (c - 1).min(31)) & 1 == 1;
            fl.of = false;
            (sv >> c.min(31)) as u32 & m
        }
    };
    fl.set_szp(r, size);
    reg_set(regs, reg, size, r);
}

/// `fcom`: the condition bits C3 C2 C0 of the FPU status word.
#[inline(always)]
fn fcompare_cc(a: f64, b: f64) -> u16 {
    // branch free: unordered 0x4500, a < b 0x0100, a == b 0x4000, a > b 0
    let (lt, eq, gt) = (a < b, a == b, a > b);
    let un = !(lt | eq | gt);
    ((lt | un) as u16) << 8 | ((eq | un) as u16) << 14 | (un as u16) << 10
}

/// `push` on a local register file (translated formulas).
#[allow(dead_code)] // used by generated code when a formula needs it
#[inline(always)]
fn push_l(m: &mut Machine, r: &mut [u32; 8], v: u32) -> R<()> {
    r[ESP] = r[ESP].wrapping_sub(4);
    m.wr32(r[ESP], v)
}

/// `push` / `pop` on a local register file with an unchecked stack access.
#[inline(always)]
fn push_lu(m: &mut Machine, r: &mut [u32; 8], v: u32) {
    r[ESP] = r[ESP].wrapping_sub(4);
    m.wr32u(r[ESP], v)
}

#[inline(always)]
fn pop_lu(m: &mut Machine, r: &mut [u32; 8]) -> u32 {
    let v = m.rd32u(r[ESP]);
    r[ESP] = r[ESP].wrapping_add(4);
    v
}

/// `pop` on a local register file (translated formulas).
#[allow(dead_code)] // used by generated code when a formula needs it
#[inline(always)]
fn pop_l(m: &mut Machine, r: &mut [u32; 8]) -> R<u32> {
    let v = m.rd32(r[ESP])?;
    r[ESP] = r[ESP].wrapping_add(4);
    Ok(v)
}

#[derive(Clone)]
pub struct Machine {
    /// fixed size, so that the bounds checks compare with a constant
    pub mem: Box<[u8; MEM_ALLOC]>,
    pub regs: [u32; 8],
    /// the arithmetic flags
    fl: Flags,
    st: [f64; 8],
    top: usize,
    /// condition bits C0..C3 of the FPU status word (bit 8, 9, 10, 14)
    fsw_cc: u16,
    fcw: u16,
    pub xmm: [[u64; 2]; 8],
    cache: HashMap<u32, (Ins, u8)>,
    /// Pre-decoded instructions of the code area (see [`Machine::predecode`]).
    table: std::sync::Arc<Vec<Option<(Ins, u8)>>>,
    /// Compiled programs by entry address (None only while one runs, see
    /// [`Machine::with_prog`]).
    progs: Option<std::sync::Arc<Vec<(u32, Prog)>>>,
    pub host: Vec<(u32, HostFn)>,
    pub steps: u64,
}

const TRIG_SLOTS: usize = 256;

/// The half width of [`atan2_bracket`]'s interval: the approximation
/// (Abramowitz & Stegun 4.4.49) is within 1e-5 of the true angle, the
/// library's `atan2` within an ulp of it.
const ATAN2_EPS: f64 = 4e-5;

/// An interval that contains `y.atan2(x)`, from a fast approximation (one
/// division and a polynomial). The translated formulas use it where the
/// angle is only scaled and rounded to an integer (a symmetry's sector): if
/// both ends round to the same integer, so does the exact angle, as the
/// scaling steps are monotonic; else they call `atan2` (see
/// `lift_emit::atan_round_chain`). Zero, infinite and NaN inputs give the
/// exact value at both ends.
#[allow(dead_code)] // used by generated code
#[inline]
fn atan2_bracket(y: f64, x: f64) -> (f64, f64) {
    let (ax, ay) = (x.abs(), y.abs());
    if !(ax > 0.0 && ay > 0.0 && ax < f64::INFINITY && ay < f64::INFINITY) {
        let a = y.atan2(x);
        return (a, a);
    }
    let (mn, mx) = if ay > ax { (ax, ay) } else { (ay, ax) };
    let z = mn / mx;
    let z2 = z * z;
    let mut a = z * (0.999_977_26 + z2 * (-0.332_623_47 + z2 * (0.193_543_46 + z2 * (-0.116_432_87 + z2 * (0.052_653_32 + z2 * -0.011_721_20)))));
    if ay > ax {
        a = std::f64::consts::FRAC_PI_2 - a;
    }
    if x < 0.0 {
        a = std::f64::consts::PI - a;
    }
    if y < 0.0 {
        a = -a;
    }
    (a - ATAN2_EPS, a + ATAN2_EPS)
}

const EAX: usize = 0;
const ECX: usize = 1;
const EDX: usize = 2;
const ESP: usize = 4;
const EBP: usize = 5;
const EBX: usize = 3;
const ESI: usize = 6;
const EDI: usize = 7;

impl Machine {
    pub fn new() -> Machine {
        Machine {
            mem: vec![0u8; MEM_ALLOC].into_boxed_slice().try_into().expect("memory size"),
            regs: [0; 8],
            fl: Flags::default(),
            st: [0.0; 8],
            top: 0,
            fsw_cc: 0,
            fcw: 0x037F,
            xmm: [[0; 2]; 8],
            cache: HashMap::new(),
            table: std::sync::Arc::new(Vec::new()),
            progs: Some(std::sync::Arc::new(Vec::new())),
            host: Vec::new(),
            steps: 0,
        }
    }

    /// `x.sin_cos()`, remembered by the exact bits of `x`: the same result,
    /// fast when a formula takes the sine of a few angles over and over (a
    /// rounded sector number times a constant: Apollo2D-IFS' symmetry). The
    /// translator uses it only where the argument comes from such a value.
    /// The results are kept per thread.
    #[inline]
    pub fn sin_cos_cached(&mut self, x: f64) -> (f64, f64) {
        thread_local! {
            // every slot holds a true entry: sin_cos(0.0) = (0.0, 1.0)
            static TRIG: std::cell::RefCell<[(u64, f64, f64); TRIG_SLOTS]> = const { std::cell::RefCell::new([(0, 0.0, 1.0); TRIG_SLOTS]) };
        }
        let b = x.to_bits();
        TRIG.with_borrow_mut(|t| {
            let e = &mut t[(b.wrapping_mul(0x9E37_79B9_7F4A_7C15) >> 56) as usize];
            if e.0 != b {
                let (s, c) = x.sin_cos();
                *e = (b, s, c);
            }
            (e.1, e.2)
        })
    }

    /// Forget decoded instructions (after code was changed).
    pub fn clear_cache(&mut self) {
        self.cache.clear();
        self.table = std::sync::Arc::new(Vec::new());
    }

    /// Decodes all instructions reachable from `entries` in the code area
    /// starting at `CODE_START` into a shared table (cheap to clone).
    pub fn predecode(&mut self, entries: &[u32]) {
        let len = (MAGIC_START - CODE_START) as usize;
        let mut t: Vec<Option<(Ins, u8)>> = vec![None; len];
        let mut work: Vec<u32> = entries.to_vec();
        while let Some(mut a) = work.pop() {
            loop {
                let o = a.wrapping_sub(CODE_START) as usize;
                if o >= len || t[o].is_some() {
                    break;
                }
                let Ok(mo) = self.idx(a, 1) else { break };
                let end = (mo + 16).min(self.mem.len());
                let Ok((ins, l)) = decode(&self.mem[mo..end], a) else { break };
                let stop = match &ins {
                    Ins::Jcc { target, .. } => {
                        work.push(*target);
                        false
                    }
                    Ins::Call(tg) => {
                        work.push(*tg);
                        false
                    }
                    Ins::Jmp(tg) => {
                        work.push(*tg);
                        true
                    }
                    Ins::Ret(_) | Ins::JmpInd(_) | Ins::Halt => true,
                    _ => false,
                };
                t[o] = Some((ins, l as u8));
                if stop {
                    break;
                }
                a = a.wrapping_add(l as u32);
            }
        }
        self.table = std::sync::Arc::new(t);
        let mut progs = Vec::new();
        if std::env::var_os("MB3D_NO_COMPILE").is_none() {
            for &e in entries {
                match self.compile(e) {
                    Ok(p) => progs.push((e, p)),
                    Err(r) => {
                        if std::env::var_os("MB3D_DEBUG_COMPILE").is_some() {
                            eprintln!("not compiled ({e:#x}): {r}");
                        }
                    }
                }
            }
        }
        self.progs = Some(std::sync::Arc::new(progs));
    }

    /// The compiled programs.
    fn progs(&self) -> &[(u32, Prog)] {
        self.progs.as_deref().map_or(&[], |v| v)
    }

    /// Runs `f` on the machine with program number `i`: the programs are
    /// taken out of the machine for the call, so that the program can be
    /// borrowed while the machine changes (a program never starts another
    /// one).
    #[inline(always)]
    fn with_prog<T>(&mut self, i: usize, f: impl FnOnce(&mut Self, &Prog) -> T) -> T {
        let progs = self.progs.take().expect("a compiled program started another one");
        let r = f(self, &progs[i].1);
        self.progs = Some(progs);
        r
    }

    /// The compiled program for `entry`, if the code could be compiled.
    pub fn prog(&self, entry: u32) -> Option<&Prog> {
        self.progs().iter().find(|(e, _)| *e == entry).map(|(_, p)| p)
    }

    /// Runs the translated formula at `entry` directly on `it` (see
    /// [`DirectFn`]), `pvar` = its `PVar`; None if it has no direct version
    /// (or translated formulas are switched off).
    pub fn call_direct(&mut self, entry: u32, it: &mut Iteration, pvar: u32, max_steps: u64) -> Option<R<()>> {
        if !NATIVE_ENABLED.load(std::sync::atomic::Ordering::Relaxed) {
            return None;
        }
        let i = self.progs().iter().position(|(e, _)| *e == entry)?;
        let f = self.progs()[i].1.direct?;
        Some(self.with_prog(i, |m, p| f(m, p, it, pvar, max_steps)))
    }

    /// The number of the compiled program at `entry` if it has a direct
    /// version ([`Progs::direct_fn`] calls it); None as for
    /// [`Machine::call_direct`].
    pub fn direct_index(&self, entry: u32) -> Option<usize> {
        if !NATIVE_ENABLED.load(std::sync::atomic::Ordering::Relaxed) {
            return None;
        }
        let i = self.progs().iter().position(|(e, _)| *e == entry)?;
        self.progs()[i].1.direct.map(|_| i)
    }

    /// The compiled programs, taken out of the machine (so that a loop can
    /// call [`DifsFn`]s with them); [`Machine::restore_progs`] puts them
    /// back.
    pub fn take_progs(&mut self) -> Progs {
        Progs(self.progs.take())
    }

    pub fn restore_progs(&mut self, p: Progs) {
        self.progs = p.0;
    }



    /// The registers of `call_difs` for a [`DifsFn`] (the return address
    /// pushed as `call_difs` does).
    #[inline]
    pub fn difs_regs(&mut self, esi: u32, edi: u32, ebx: u32, ecx: u32, stack_top: u32) -> [u32; 8] {
        let mut r = [0u32; 8];
        r[ESI] = esi;
        r[EDI] = edi;
        r[EBX] = ebx;
        r[ECX] = ecx;
        r[EDX] = esi + 128;
        r[ESP] = stack_top - 4;
        self.wr32u(stack_top - 4, RETURN_SENTINEL);
        self.fsw_cc = 0;
        r
    }

    /// Whether the code at `entry` runs compiled.
    pub fn is_compiled(&self, entry: u32) -> bool {
        self.progs().iter().any(|(e, _)| *e == entry)
    }

    // ---- memory ----
    #[inline]
    fn idx(&self, a: u32, n: usize) -> R<usize> {
        let o = a.wrapping_sub(BASE) as usize;
        if o + n <= MEM_SIZE {
            Ok(o)
        } else {
            Err(EmuError::MemFault(a))
        }
    }
    #[inline]
    pub fn rd8(&self, a: u32) -> R<u8> {
        Ok(self.mem[self.idx(a, 1)?])
    }
    #[inline]
    pub fn rd16(&self, a: u32) -> R<u16> {
        let i = self.idx(a, 2)?;
        Ok(u16::from_le_bytes([self.mem[i], self.mem[i + 1]]))
    }
    #[inline]
    pub fn rd32(&self, a: u32) -> R<u32> {
        let i = self.idx(a, 4)?;
        Ok(u32::from_le_bytes(self.mem[i..i + 4].try_into().unwrap()))
    }
    #[inline]
    pub fn rd64(&self, a: u32) -> R<u64> {
        let i = self.idx(a, 8)?;
        Ok(u64::from_le_bytes(self.mem[i..i + 8].try_into().unwrap()))
    }
    #[inline]
    pub fn wr8(&mut self, a: u32, v: u8) -> R<()> {
        let i = self.idx(a, 1)?;
        self.mem[i] = v;
        Ok(())
    }
    #[inline]
    pub fn wr16(&mut self, a: u32, v: u16) -> R<()> {
        let i = self.idx(a, 2)?;
        self.mem[i..i + 2].copy_from_slice(&v.to_le_bytes());
        Ok(())
    }
    #[inline]
    pub fn wr32(&mut self, a: u32, v: u32) -> R<()> {
        let i = self.idx(a, 4)?;
        self.mem[i..i + 4].copy_from_slice(&v.to_le_bytes());
        Ok(())
    }
    #[inline]
    pub fn wr64(&mut self, a: u32, v: u64) -> R<()> {
        let i = self.idx(a, 8)?;
        self.mem[i..i + 8].copy_from_slice(&v.to_le_bytes());
        Ok(())
    }
    /// f64 access for fixed addresses inside the image (the
    /// iteration record), without an error result; panics if out of range.
    #[inline(always)]
    pub fn put_f64(&mut self, a: u32, v: f64) {
        let i = a.wrapping_sub(BASE) as usize;
        assert!(i + 8 <= MEM_SIZE);
        self.mem[i..i + 8].copy_from_slice(&v.to_bits().to_le_bytes());
    }
    #[inline(always)]
    pub fn get_f64(&self, a: u32) -> f64 {
        let i = a.wrapping_sub(BASE) as usize;
        assert!(i + 8 <= MEM_SIZE);
        f64::from_bits(u64::from_le_bytes(self.mem[i..i + 8].try_into().unwrap()))
    }
    #[inline(always)]
    pub fn put_u32(&mut self, a: u32, v: u32) {
        let i = a.wrapping_sub(BASE) as usize;
        assert!(i + 4 <= MEM_SIZE);
        self.mem[i..i + 4].copy_from_slice(&v.to_le_bytes());
    }
    #[inline(always)]
    pub fn get_u32(&self, a: u32) -> u32 {
        let i = a.wrapping_sub(BASE) as usize;
        assert!(i + 4 <= MEM_SIZE);
        u32::from_le_bytes(self.mem[i..i + 4].try_into().unwrap())
    }

    #[inline]
    pub fn rdf64(&self, a: u32) -> R<f64> {
        Ok(f64::from_bits(self.rd64(a)?))
    }
    #[inline]
    pub fn wrf64(&mut self, a: u32, v: f64) -> R<()> {
        self.wr64(a, v.to_bits())
    }
    // ---- unchecked accesses for the translated formulas, at addresses the
    // lifting analysis resolved to the iteration record, the variable
    // buffer or the stack (masked into the image: never out of bounds)
    #[inline(always)]
    fn ui(a: u32) -> usize {
        (a.wrapping_sub(BASE) as usize) & (MEM_SIZE - 1)
    }
    /// The `N` bytes at `a` with one check (the variable buffer cells a
    /// translated formula reads, see [`cell`]); a fault outside the image.
    #[inline(always)]
    pub fn bytes_at<const N: usize>(&self, a: u32) -> R<&[u8; N]> {
        let i = a.wrapping_sub(BASE) as usize;
        match self.mem[..MEM_SIZE].get(i..i + N) {
            Some(s) => Ok(s.try_into().unwrap()),
            None => Err(EmuError::MemFault(a)),
        }
    }
    #[inline(always)]
    pub fn rd8u(&self, a: u32) -> u8 {
        self.mem[Self::ui(a)]
    }
    #[inline(always)]
    pub fn rd16u(&self, a: u32) -> u16 {
        let i = Self::ui(a);
        u16::from_le_bytes([self.mem[i], self.mem[i + 1]])
    }
    #[inline(always)]
    pub fn rd32u(&self, a: u32) -> u32 {
        let i = Self::ui(a);
        u32::from_le_bytes(self.mem[i..i + 4].try_into().unwrap())
    }
    #[inline(always)]
    pub fn rd64u(&self, a: u32) -> u64 {
        let i = Self::ui(a);
        u64::from_le_bytes(self.mem[i..i + 8].try_into().unwrap())
    }
    #[inline(always)]
    pub fn rdf64u(&self, a: u32) -> f64 {
        f64::from_bits(self.rd64u(a))
    }
    #[inline(always)]
    pub fn wr8u(&mut self, a: u32, v: u8) {
        self.mem[Self::ui(a)] = v;
    }
    #[inline(always)]
    pub fn wr16u(&mut self, a: u32, v: u16) {
        let i = Self::ui(a);
        self.mem[i..i + 2].copy_from_slice(&v.to_le_bytes());
    }
    #[inline(always)]
    pub fn wr32u(&mut self, a: u32, v: u32) {
        let i = Self::ui(a);
        self.mem[i..i + 4].copy_from_slice(&v.to_le_bytes());
    }
    #[inline(always)]
    pub fn wr64u(&mut self, a: u32, v: u64) {
        let i = Self::ui(a);
        self.mem[i..i + 8].copy_from_slice(&v.to_le_bytes());
    }
    #[inline(always)]
    pub fn wrf64u(&mut self, a: u32, v: f64) {
        self.wr64u(a, v.to_bits())
    }
    #[inline(always)]
    fn floadu(&self, kind: FKind, a: u32) -> f64 {
        match kind {
            FKind::F32 => f32::from_bits(self.rd32u(a)) as f64,
            FKind::F64 => self.rdf64u(a),
            FKind::I16 => self.rd16u(a) as i16 as f64,
            FKind::I32 => self.rd32u(a) as i32 as f64,
            FKind::I64 => self.rd64u(a) as i64 as f64,
            FKind::F80 => f80_to_f64(self.rd64u(a), self.rd16u(a.wrapping_add(8))),
        }
    }
    #[allow(dead_code)] // used by generated code when a formula needs it
    #[inline]
    fn fstoreu(&mut self, kind: FKind, a: u32, v: f64) {
        // the same conversions as `fstore`, without the bounds checks
        let mut tmp = [0u8; 10];
        let n = match kind {
            FKind::F32 => {
                tmp[..4].copy_from_slice(&(v as f32).to_bits().to_le_bytes());
                4
            }
            FKind::F64 => {
                tmp[..8].copy_from_slice(&v.to_bits().to_le_bytes());
                8
            }
            _ => {
                // integer and 80 bit stores: through the checked path on a
                // scratch machine would be overkill; they are rare
                let r = self.fstore(kind, a, v);
                debug_assert!(r.is_ok());
                return;
            }
        };
        let i = Self::ui(a);
        self.mem[i..i + n].copy_from_slice(&tmp[..n]);
    }

    pub fn rdf32(&self, a: u32) -> R<f32> {
        Ok(f32::from_bits(self.rd32(a)?))
    }
    pub fn wrf32(&mut self, a: u32, v: f32) -> R<()> {
        self.wr32(a, v.to_bits())
    }
    pub fn write_bytes(&mut self, a: u32, b: &[u8]) -> R<()> {
        let i = self.idx(a, b.len())?;
        self.mem[i..i + b.len()].copy_from_slice(b);
        Ok(())
    }

    fn ea(&self, m: &Mem) -> u32 {
        let mut a = m.disp as u32;
        if let Some(b) = m.base {
            a = a.wrapping_add(self.regs[b as usize]);
        }
        if let Some((i, s)) = m.index {
            a = a.wrapping_add(self.regs[i as usize].wrapping_mul(s as u32));
        }
        a
    }

    // ---- registers of various sizes ----

    #[inline(always)]
    fn get_reg(&self, r: u8, size: u8) -> u32 {
        reg_get(&self.regs, r, size)
    }
    #[inline(always)]
    fn set_reg(&mut self, r: u8, size: u8, v: u32) {
        reg_set(&mut self.regs, r, size, v);
    }
    fn get_rm(&self, rm: &Rm, size: u8) -> R<u32> {
        match rm {
            Rm::Reg(r) => Ok(self.get_reg(*r, size)),
            Rm::Mem(m) => {
                let a = self.ea(m);
                match size {
                    1 => Ok(self.rd8(a)? as u32),
                    2 => Ok(self.rd16(a)? as u32),
                    _ => self.rd32(a),
                }
            }
        }
    }
    fn set_rm(&mut self, rm: &Rm, size: u8, v: u32) -> R<()> {
        match rm {
            Rm::Reg(r) => {
                self.set_reg(*r, size, v);
                Ok(())
            }
            Rm::Mem(m) => {
                let a = self.ea(m);
                match size {
                    1 => self.wr8(a, v as u8),
                    2 => self.wr16(a, v as u16),
                    _ => self.wr32(a, v),
                }
            }
        }
    }
    fn get_src(&self, s: &Src, size: u8) -> R<u32> {
        match s {
            Src::Imm(v) => Ok(mask(*v, size)),
            Src::Rm(rm) => self.get_rm(rm, size),
        }
    }

    #[inline]
    pub fn push(&mut self, v: u32) -> R<()> {
        self.regs[ESP] = self.regs[ESP].wrapping_sub(4);
        self.wr32(self.regs[ESP], v)
    }
    #[inline]
    pub fn pop(&mut self) -> R<u32> {
        let v = self.rd32(self.regs[ESP])?;
        self.regs[ESP] = self.regs[ESP].wrapping_add(4);
        Ok(v)
    }

    // ---- flags ----
    #[inline]
    fn set_szp(&mut self, r: u32, size: u8) {
        self.fl.set_szp(r, size);
    }
    #[inline]
    fn cond(&self, cc: u8) -> bool {
        self.fl.cond(cc)
    }
    fn eflags_ah(&self) -> u8 {
        (self.fl.sf as u8) << 7 | (self.fl.zf as u8) << 6 | (self.fl.af as u8) << 4 | (self.fl.pf as u8) << 2 | 2 | self.fl.cf as u8
    }

    #[inline]
    fn alu(&mut self, op: Alu, a: u32, b: u32, size: u8) -> u32 {
        self.fl.alu(op, a, b, size)
    }

    // ---- x87 ----
    #[inline]
    fn st(&self, i: u8) -> f64 {
        self.st[(self.top + i as usize) & 7]
    }
    #[inline]
    fn st_mut(&mut self, i: u8) -> &mut f64 {
        &mut self.st[(self.top + i as usize) & 7]
    }
    #[inline]
    fn fpush(&mut self, v: f64) {
        self.top = (self.top + 7) & 7;
        self.st[self.top] = v;
    }
    #[inline]
    fn fpop(&mut self) -> f64 {
        let v = self.st[self.top];
        self.top = (self.top + 1) & 7;
        v
    }
    fn fsw(&self) -> u16 {
        self.fsw_cc | ((self.top as u16) << 11)
    }
    #[inline]
    fn fcompare(&mut self, a: f64, b: f64) {
        self.fsw_cc = fcompare_cc(a, b);
    }
    #[inline]
    fn round_int(&self, v: f64) -> f64 {
        match (self.fcw >> 10) & 3 {
            0 => round_even(v),
            1 => v.floor(),
            2 => v.ceil(),
            _ => v.trunc(),
        }
    }
    #[inline]
    fn fload(&self, kind: FKind, a: u32) -> R<f64> {
        Ok(match kind {
            FKind::F32 => self.rdf32(a)? as f64,
            FKind::F64 => self.rdf64(a)?,
            FKind::I16 => self.rd16(a)? as i16 as f64,
            FKind::I32 => self.rd32(a)? as i32 as f64,
            FKind::I64 => self.rd64(a)? as i64 as f64,
            FKind::F80 => {
                let mant = self.rd64(a)?;
                let se = self.rd16(a + 8)?;
                f80_to_f64(mant, se)
            }
        })
    }
    #[inline]
    fn fstore(&mut self, kind: FKind, a: u32, v: f64) -> R<()> {
        match kind {
            FKind::F32 => self.wrf32(a, v as f32),
            FKind::F64 => self.wrf64(a, v),
            FKind::I16 => {
                let r = self.round_int(v);
                let i = if r.is_nan() || !(-32768.0..=32767.0).contains(&r) { 0x8000u16 } else { r as i16 as u16 };
                self.wr16(a, i)
            }
            FKind::I32 => {
                let r = self.round_int(v);
                let i = if r.is_nan() || !(-2147483648.0..=2147483647.0).contains(&r) {
                    0x8000_0000u32
                } else {
                    r as i32 as u32
                };
                self.wr32(a, i)
            }
            FKind::I64 => {
                let r = self.round_int(v);
                let i = if r.is_nan() || r.abs() >= 9.2233720368547758e18 { 1u64 << 63 } else { r as i64 as u64 };
                self.wr64(a, i)
            }
            FKind::F80 => {
                let (m, se) = f64_to_f80(v);
                self.wr64(a, m)?;
                self.wr16(a + 8, se)
            }
        }
    }

    fn farith(op: FOp, a: f64, b: f64) -> f64 {
        match op {
            FOp::Add => a + b,
            FOp::Mul => a * b,
            FOp::Sub => a - b,
            FOp::SubR => b - a,
            FOp::Div => a / b,
            FOp::DivR => b / a,
        }
    }

    // ---- xmm helpers ----
    fn xmm_src(&self, rm: &Rm, bytes: usize) -> R<[u64; 2]> {
        match rm {
            Rm::Reg(r) => Ok(self.xmm[*r as usize]),
            Rm::Mem(m) => {
                let a = self.ea(m);
                Ok(match bytes {
                    4 => [self.rd32(a)? as u64, 0],
                    8 => [self.rd64(a)?, 0],
                    _ => [self.rd64(a)?, self.rd64(a + 8)?],
                })
            }
        }
    }

    /// Run until the sentinel return address is reached.
    pub fn run(&mut self, entry: u32, max_steps: u64) -> R<()> {
        let mut eip = entry;
        let limit = self.steps + max_steps;
        let table = self.table.clone();
        loop {
            if eip == RETURN_SENTINEL {
                return Ok(());
            }
            if eip >= MAGIC_START && eip < BASE + MEM_SIZE as u32 {
                // host function: behaves like a called procedure
                let f = self
                    .host
                    .iter()
                    .find(|(a, _)| *a == eip)
                    .map(|(_, f)| *f)
                    .ok_or(EmuError::Halt(eip))?;
                f(self)?;
                eip = self.pop()?; // return address; host fn adjusted stack args
                continue;
            }
            self.steps += 1;
            if self.steps > limit {
                return Err(EmuError::StepLimit);
            }
            let o = eip.wrapping_sub(CODE_START) as usize;
            if let Some(Some((ins, len))) = table.get(o) {
                let next = eip.wrapping_add(*len as u32);
                eip = self.exec(ins, next, eip)?;
                continue;
            }
            let (ins, len) = match self.cache.get(&eip) {
                Some(x) => x.clone(),
                None => {
                    let o = self.idx(eip, 1)?;
                    let end = (o + 16).min(self.mem.len());
                    let (ins, len) = decode(&self.mem[o..end], eip)?;
                    self.cache.insert(eip, (ins.clone(), len as u8));
                    (ins, len as u8)
                }
            };
            let next = eip.wrapping_add(len as u32);
            eip = self.exec(&ins, next, eip)?;
        }
    }

    fn exec(&mut self, ins: &Ins, next: u32, at: u32) -> R<u32> {
        match ins {
            Ins::Alu { op, dst, src, size } => {
                let a = self.get_rm(dst, *size)?;
                let b = self.get_src(src, *size)?;
                let r = self.alu(*op, a, b, *size);
                if *op != Alu::Cmp {
                    self.set_rm(dst, *size, r)?;
                }
            }
            Ins::Mov { dst, src, size } => {
                let v = self.get_src(src, *size)?;
                self.set_rm(dst, *size, v)?;
            }
            Ins::Lea { reg, mem } => self.regs[*reg as usize] = self.ea(mem),
            Ins::Push(s) => {
                let v = self.get_src(s, 4)?;
                self.push(v)?;
            }
            Ins::Pop(rm) => {
                let v = self.pop()?;
                self.set_rm(rm, 4, v)?;
            }
            Ins::Inc(rm, size) | Ins::Dec(rm, size) => {
                let a = self.get_rm(rm, *size)?;
                let cf = self.fl.cf;
                let op = if matches!(ins, Ins::Inc(..)) { Alu::Add } else { Alu::Sub };
                let r = self.alu(op, a, 1, *size);
                self.fl.cf = cf;
                self.set_rm(rm, *size, r)?;
            }
            Ins::Not(rm, size) => {
                let a = self.get_rm(rm, *size)?;
                self.set_rm(rm, *size, !a)?;
            }
            Ins::Neg(rm, size) => {
                let a = self.get_rm(rm, *size)?;
                let r = self.alu(Alu::Sub, 0, a, *size);
                self.fl.cf = mask(a, *size) != 0;
                self.set_rm(rm, *size, r)?;
            }
            Ins::Mul(rm, size) | Ins::IMul1(rm, size) if *size != 4 => {
                let b = self.get_rm(rm, *size)?;
                let signed = matches!(ins, Ins::IMul1(..));
                if *size == 1 {
                    let a = self.regs[EAX] & 0xFF;
                    let r = if signed { (a as u8 as i8 as i32 * b as u8 as i8 as i32) as u32 } else { a * b };
                    self.set_reg(0, 2, r);
                    self.fl.cf = if signed { (r as u16 as i16) != (r as u8 as i8 as i16) } else { (r >> 8) & 0xFF != 0 };
                } else {
                    let a = self.regs[EAX] & 0xFFFF;
                    let r = if signed { (a as u16 as i16 as i32 * b as u16 as i16 as i32) as u32 } else { a * b };
                    self.set_reg(0, 2, r);
                    self.set_reg(2, 2, r >> 16);
                    self.fl.cf = if signed { (r as i32) != (r as u16 as i16 as i32) } else { (r >> 16) != 0 };
                }
                self.fl.of = self.fl.cf;
            }
            Ins::Mul(rm, size) | Ins::IMul1(rm, size) => {
                let _ = size;
                let b = self.get_rm(rm, 4)?;
                let a = self.regs[EAX];
                let r: u64 = if matches!(ins, Ins::Mul(..)) {
                    a as u64 * b as u64
                } else {
                    (a as i32 as i64 * b as i32 as i64) as u64
                };
                self.regs[EAX] = r as u32;
                self.regs[EDX] = (r >> 32) as u32;
                let hi_used = if matches!(ins, Ins::Mul(..)) {
                    (r >> 32) != 0
                } else {
                    (r as i64) != (r as u32 as i32 as i64)
                };
                self.fl.cf = hi_used;
                self.fl.of = hi_used;
            }
            Ins::Div(rm, size) | Ins::IDiv(rm, size) if *size != 4 => {
                let b = self.get_rm(rm, *size)?;
                if b == 0 {
                    return Err(EmuError::Halt(at));
                }
                let signed = matches!(ins, Ins::IDiv(..));
                if *size == 1 {
                    let n = self.regs[EAX] & 0xFFFF;
                    let (q, r) = if signed {
                        let (n, d) = (n as u16 as i16 as i32, b as u8 as i8 as i32);
                        let q = n / d;
                        if q != q as i8 as i32 {
                            return Err(EmuError::Halt(at));
                        }
                        (q as u32, (n % d) as u32)
                    } else {
                        if n / b > 0xFF {
                            return Err(EmuError::Halt(at));
                        }
                        (n / b, n % b)
                    };
                    self.set_reg(0, 1, q);
                    self.set_reg(4, 1, r); // AH
                } else {
                    let n = (self.regs[EDX] & 0xFFFF) << 16 | (self.regs[EAX] & 0xFFFF);
                    let (q, r) = if signed {
                        // i32::MIN / -1 overflows; like a quotient out of
                        // range it is a divide error on the CPU
                        let (n, d) = (n as i32, b as u16 as i16 as i32);
                        match n.checked_div(d) {
                            Some(q) if q == q as i16 as i32 => (q as u32, (n % d) as u32),
                            _ => return Err(EmuError::Halt(at)),
                        }
                    } else {
                        if n / b > 0xFFFF {
                            return Err(EmuError::Halt(at));
                        }
                        (n / b, n % b)
                    };
                    self.set_reg(0, 2, q);
                    self.set_reg(2, 2, r);
                }
            }
            Ins::Div(rm, size) | Ins::IDiv(rm, size) => {
                let _ = size;
                let b = self.get_rm(rm, 4)?;
                if b == 0 {
                    return Err(EmuError::Halt(at));
                }
                let n = (self.regs[EDX] as u64) << 32 | self.regs[EAX] as u64;
                if matches!(ins, Ins::Div(..)) {
                    let q = n / b as u64;
                    if q > u32::MAX as u64 {
                        return Err(EmuError::Halt(at));
                    }
                    self.regs[EAX] = q as u32;
                    self.regs[EDX] = (n % b as u64) as u32;
                } else {
                    let n = n as i64;
                    let b = b as i32 as i64;
                    let Some(q) = n.checked_div(b).filter(|q| *q == *q as i32 as i64) else {
                        return Err(EmuError::Halt(at));
                    };
                    self.regs[EAX] = q as u32;
                    self.regs[EDX] = (n % b) as u32;
                }
            }
            Ins::IMul3 { reg, src, imm } => {
                let a = self.get_rm(src, 4)? as i32 as i64;
                let r = a * (*imm as i32 as i64);
                self.regs[*reg as usize] = r as u32;
                self.fl.cf = r != (r as i32 as i64);
                self.fl.of = self.fl.cf;
            }
            Ins::IMul2 { reg, src } => {
                let a = self.regs[*reg as usize] as i32 as i64;
                let r = a * (self.get_rm(src, 4)? as i32 as i64);
                self.regs[*reg as usize] = r as u32;
                self.fl.cf = r != (r as i32 as i64);
                self.fl.of = self.fl.cf;
            }
            Ins::Test { a, b, size } => {
                let x = self.get_rm(a, *size)?;
                let y = self.get_src(b, *size)?;
                self.alu(Alu::And, x, y, *size);
            }
            Ins::Shift { op, dst, cnt, size } => {
                let c = match cnt {
                    Some(c) => *c as u32,
                    None => self.regs[ECX] & 0xFF,
                } & 31;
                if c != 0 {
                    let bits = *size as u32 * 8;
                    let v = self.get_rm(dst, *size)?;
                    let m = mask(u32::MAX, *size);
                    let r = match op {
                        Shift::Shl => {
                            self.fl.cf = c <= bits && (v >> (bits - c)) & 1 == 1;
                            let r = if c >= 32 { 0 } else { (v << c) & m };
                            self.fl.of = ((r >> (bits - 1)) & 1 == 1) != self.fl.cf;
                            self.set_szp(r, *size);
                            r
                        }
                        Shift::Shr => {
                            self.fl.cf = (v >> (c - 1)) & 1 == 1;
                            self.fl.of = (v >> (bits - 1)) & 1 == 1;
                            let r = if c >= 32 { 0 } else { v >> c };
                            self.set_szp(r, *size);
                            r
                        }
                        Shift::Sar => {
                            let sv = sign_extend(v, *size);
                            self.fl.cf = (sv >> (c - 1).min(31)) & 1 == 1;
                            self.fl.of = false;
                            let r = (sv >> c.min(31)) as u32 & m;
                            self.set_szp(r, *size);
                            r
                        }
                        Shift::Rol => {
                            let c = c % bits;
                            let r = ((v << c) | (v >> ((bits - c) % bits))) & m;
                            self.fl.cf = r & 1 == 1;
                            r
                        }
                        Shift::Ror => {
                            let c = c % bits;
                            let r = ((v >> c) | (v << ((bits - c) % bits))) & m;
                            self.fl.cf = (r >> (bits - 1)) & 1 == 1;
                            r
                        }
                        Shift::Rcl | Shift::Rcr => {
                            let mut r = v;
                            for _ in 0..c {
                                if *op == Shift::Rcl {
                                    let out = (r >> (bits - 1)) & 1 == 1;
                                    r = ((r << 1) | self.fl.cf as u32) & m;
                                    self.fl.cf = out;
                                } else {
                                    let out = r & 1 == 1;
                                    r = (r >> 1) | ((self.fl.cf as u32) << (bits - 1));
                                    self.fl.cf = out;
                                }
                            }
                            r
                        }
                    };
                    self.set_rm(dst, *size, r)?;
                }
            }
            Ins::Xchg { a, b, size } => {
                let x = self.get_rm(a, *size)?;
                let y = self.get_reg(*b, *size);
                self.set_rm(a, *size, y)?;
                self.set_reg(*b, *size, x);
            }
            Ins::Movzx { reg, src, size } => {
                let v = self.get_rm(src, *size)?;
                self.regs[*reg as usize] = v;
            }
            Ins::Movsx { reg, src, size } => {
                let v = self.get_rm(src, *size)?;
                self.regs[*reg as usize] = sign_extend(v, *size) as u32;
            }
            Ins::Setcc { cc, dst } => {
                let v = self.cond(*cc) as u32;
                self.set_rm(dst, 1, v)?;
            }
            Ins::Cmovcc { cc, reg, src } => {
                let v = self.get_rm(src, 4)?;
                if self.cond(*cc) {
                    self.regs[*reg as usize] = v;
                }
            }
            Ins::Jcc { cc, target } => {
                if self.cond(*cc) {
                    return Ok(*target);
                }
            }
            Ins::Jmp(t) => return Ok(*t),
            Ins::JmpInd(rm) => return self.get_rm(rm, 4),
            Ins::Call(t) => {
                self.push(next)?;
                return Ok(*t);
            }
            Ins::CallInd(rm) => {
                let t = self.get_rm(rm, 4)?;
                self.push(next)?;
                return Ok(t);
            }
            Ins::Ret(n) => {
                let r = self.pop()?;
                self.regs[ESP] = self.regs[ESP].wrapping_add(*n as u32);
                return Ok(r);
            }
            Ins::Leave => {
                self.regs[ESP] = self.regs[EBP];
                self.regs[EBP] = self.pop()?;
            }
            Ins::Cdq => self.regs[EDX] = if (self.regs[EAX] as i32) < 0 { u32::MAX } else { 0 },
            Ins::Cwde => self.regs[EAX] = self.regs[EAX] as u16 as i16 as i32 as u32,
            Ins::Sahf => {
                let ah = (self.regs[EAX] >> 8) as u8;
                self.fl.sf = ah & 0x80 != 0;
                self.fl.zf = ah & 0x40 != 0;
                self.fl.af = ah & 0x10 != 0;
                self.fl.pf = ah & 0x04 != 0;
                self.fl.cf = ah & 0x01 != 0;
            }
            Ins::Lahf => {
                let ah = self.eflags_ah() as u32;
                self.regs[EAX] = (self.regs[EAX] & !0xFF00) | (ah << 8);
            }
            Ins::Nop => {}
            Ins::Halt => return Err(EmuError::Halt(at)),

            // ---------------- x87 ----------------
            Ins::FLd(k, m) => {
                let v = self.fload(*k, self.ea(m))?;
                self.fpush(v);
            }
            Ins::FSt(k, m, pop) => {
                let a = self.ea(m);
                self.fstore(*k, a, self.st(0))?;
                if *pop {
                    self.fpop();
                }
            }
            Ins::FArithMem(op, k, m) => {
                let b = self.fload(*k, self.ea(m))?;
                let a = self.st(0);
                *self.st_mut(0) = Self::farith(*op, a, b);
            }
            Ins::FArith { op, dst, src, pop } => {
                let a = self.st(*dst);
                let b = self.st(*src);
                *self.st_mut(*dst) = Self::farith(*op, a, b);
                if *pop {
                    self.fpop();
                }
            }
            Ins::FLdSt(i) => {
                let v = self.st(*i);
                self.fpush(v);
            }
            Ins::FStSt(i, pop) => {
                let v = self.st(0);
                *self.st_mut(*i) = v;
                if *pop {
                    self.fpop();
                }
            }
            Ins::FXch(i) => {
                let a = self.st(0);
                let b = self.st(*i);
                *self.st_mut(0) = b;
                *self.st_mut(*i) = a;
            }
            Ins::FComSt { i, pops } => {
                self.fcompare(self.st(0), self.st(*i));
                for _ in 0..*pops {
                    self.fpop();
                }
            }
            Ins::FComMem { kind, mem, pop } => {
                let b = self.fload(*kind, self.ea(mem))?;
                self.fcompare(self.st(0), b);
                if *pop {
                    self.fpop();
                }
            }
            Ins::FComi { i, pop } => {
                let (a, b) = (self.st(0), self.st(*i));
                self.fl.of = false;
                self.fl.sf = false;
                self.fl.af = false;
                if a.is_nan() || b.is_nan() {
                    self.fl.zf = true;
                    self.fl.pf = true;
                    self.fl.cf = true;
                } else {
                    self.fl.zf = a == b;
                    self.fl.pf = false;
                    self.fl.cf = a < b;
                }
                if *pop {
                    self.fpop();
                }
            }
            Ins::FCmov { cc, i } => {
                if self.cond(*cc) {
                    let v = self.st(*i);
                    *self.st_mut(0) = v;
                }
            }
            Ins::FUn(u) => self.funary(*u),
            Ins::FnstswAx => {
                let w = self.fsw() as u32;
                self.regs[EAX] = (self.regs[EAX] & 0xFFFF_0000) | w;
            }
            Ins::FnstswMem(m) => {
                let a = self.ea(m);
                self.wr16(a, self.fsw())?;
            }
            Ins::FnstcwMem(m) => {
                let a = self.ea(m);
                self.wr16(a, self.fcw)?;
            }
            Ins::FldcwMem(m) => self.fcw = self.rd16(self.ea(m))?,
            Ins::FFree(_) | Ins::FClex => {}
            Ins::FInit => {
                self.top = 0;
                self.fsw_cc = 0;
                self.fcw = 0x037F;
            }

            // ---------------- SSE ----------------
            Ins::Sse { op, dst, src, imm } => self.sse(*op, *dst as usize, src, *imm)?,
            Ins::SseStore { op, dst, src } => {
                let a = self.ea(dst);
                let x = self.xmm[*src as usize];
                match op {
                    SseOp::MovStore => {
                        self.wr64(a, x[0])?;
                        self.wr64(a + 8, x[1])?;
                    }
                    SseOp::MovSdStore | SseOp::MovLpdStore | SseOp::MovqStore => self.wr64(a, x[0])?,
                    SseOp::MovHpdStore => self.wr64(a, x[1])?,
                    SseOp::MovSsStore => self.wr32(a, x[0] as u32)?,
                    _ => return Err(unsup(at, format!("store {op:?}"))),
                }
            }
            Ins::Cvtsi2sd { dst, src } => {
                let v = self.get_rm(src, 4)? as i32 as f64;
                self.xmm[*dst as usize][0] = v.to_bits();
            }
            Ins::Cvtsd2si { reg, src, trunc } => {
                let v = f64::from_bits(self.xmm_src(src, 8)?[0]);
                let r = if *trunc { v.trunc() } else { v.round_ties_even() };
                self.regs[*reg as usize] =
                    if r.is_nan() || !(-2147483648.0..=2147483647.0).contains(&r) { 0x8000_0000 } else { r as i32 as u32 };
            }
            Ins::MovdToX { dst, src } => {
                let v = self.get_rm(src, 4)?;
                self.xmm[*dst as usize] = [v as u64, 0];
            }
            Ins::MovdFromX { dst, src } => {
                let v = self.xmm[*src as usize][0] as u32;
                self.set_rm(dst, 4, v)?;
            }
            Ins::MovmskPd { reg, src } => {
                let x = self.xmm[*src as usize];
                self.regs[*reg as usize] = ((x[0] >> 63) | ((x[1] >> 63) << 1)) as u32;
            }
        }
        Ok(next)
    }

    fn funary(&mut self, u: FUn) {
        use std::f64::consts::*;
        match u {
            FUn::Chs => *self.st_mut(0) = -self.st(0),
            FUn::Abs => *self.st_mut(0) = self.st(0).abs(),
            FUn::Tst => self.fcompare(self.st(0), 0.0),
            FUn::Xam => {
                let v = self.st(0);
                let c1 = if v.is_sign_negative() { 0x0200 } else { 0 };
                let cls = if v.is_nan() {
                    0x0100
                } else if v.is_infinite() {
                    0x0500
                } else if v == 0.0 {
                    0x4000
                } else if v.is_subnormal() {
                    0x4400
                } else {
                    0x0400
                };
                self.fsw_cc = cls | c1;
            }
            FUn::Ld1 => self.fpush(1.0),
            FUn::Ldl2t => self.fpush(LOG2_10),
            FUn::Ldl2e => self.fpush(LOG2_E),
            FUn::Ldpi => self.fpush(PI),
            FUn::Ldlg2 => self.fpush(LOG10_2),
            FUn::Ldln2 => self.fpush(LN_2),
            FUn::Ldz => self.fpush(0.0),
            FUn::F2xm1 => *self.st_mut(0) = self.st(0).exp2() - 1.0,
            FUn::Fyl2x => {
                let x = self.st(0);
                let y = self.st(1);
                *self.st_mut(1) = y * x.log2();
                self.fpop();
            }
            FUn::Fyl2xp1 => {
                let x = self.st(0);
                let y = self.st(1);
                *self.st_mut(1) = y * x.ln_1p() * LOG2_E;
                self.fpop();
            }
            FUn::Fptan => {
                *self.st_mut(0) = self.st(0).tan();
                self.fpush(1.0);
                self.fsw_cc &= !0x0400;
            }
            FUn::Fpatan => {
                let x = self.st(0);
                let y = self.st(1);
                *self.st_mut(1) = y.atan2(x);
                self.fpop();
            }
            FUn::Fprem | FUn::Fprem1 => {
                let a = self.st(0);
                let b = self.st(1);
                let q = if u == FUn::Fprem { (a / b).trunc() } else { (a / b).round_ties_even() };
                let r = if u == FUn::Fprem { a % b } else { a - q * b };
                *self.st_mut(0) = r;
                // C2 = 0 (complete), C0, C3, C1 = low bits of quotient
                let qi = q.abs() as u64;
                self.fsw_cc = (if qi & 4 != 0 { 0x0100 } else { 0 })
                    | (if qi & 2 != 0 { 0x4000 } else { 0 })
                    | (if qi & 1 != 0 { 0x0200 } else { 0 });
            }
            FUn::Sqrt => *self.st_mut(0) = self.st(0).sqrt(),
            FUn::Sincos => {
                let v = self.st(0);
                *self.st_mut(0) = v.sin();
                self.fpush(v.cos());
                self.fsw_cc &= !0x0400;
            }
            FUn::Rndint => *self.st_mut(0) = self.round_int(self.st(0)),
            FUn::Scale => {
                let s = self.st(1).trunc();
                *self.st_mut(0) = self.st(0) * s.exp2();
            }
            FUn::Sin => {
                *self.st_mut(0) = self.st(0).sin();
                self.fsw_cc &= !0x0400;
            }
            FUn::Cos => {
                *self.st_mut(0) = self.st(0).cos();
                self.fsw_cc &= !0x0400;
            }
            FUn::Decstp => self.top = (self.top + 7) & 7,
            FUn::Incstp => self.top = (self.top + 1) & 7,
            FUn::Xtract => {
                let v = self.st(0);
                if v == 0.0 || !v.is_finite() {
                    *self.st_mut(0) = f64::NEG_INFINITY;
                    self.fpush(v);
                } else {
                    let e = v.abs().log2().floor();
                    *self.st_mut(0) = e;
                    self.fpush(v / e.exp2());
                }
            }
            FUn::Nop => {}
        }
    }

    fn sse(&mut self, op: SseOp, d: usize, src: &Rm, imm: u8) -> R<()> {
        let fd = |x: u64| f64::from_bits(x);
        let bd = |x: f64| x.to_bits();
        let fs = |x: u64| f32::from_bits(x as u32);
        macro_rules! pd {
            ($f:expr) => {{
                let s = self.xmm_src(src, 16)?;
                let a = self.xmm[d];
                self.xmm[d] = [bd($f(fd(a[0]), fd(s[0]))), bd($f(fd(a[1]), fd(s[1])))];
            }};
        }
        macro_rules! sd {
            ($f:expr) => {{
                let s = self.xmm_src(src, 8)?;
                let a = self.xmm[d];
                self.xmm[d][0] = bd($f(fd(a[0]), fd(s[0])));
            }};
        }
        macro_rules! ps {
            ($f:expr) => {{
                let s = self.xmm_src(src, 16)?;
                let a = self.xmm[d];
                let mut r = [0u64; 2];
                for q in 0..2 {
                    let lo = $f(fs(a[q]), fs(s[q])).to_bits() as u64;
                    let hi = $f(fs(a[q] >> 32), fs(s[q] >> 32)).to_bits() as u64;
                    r[q] = lo | (hi << 32);
                }
                self.xmm[d] = r;
            }};
        }
        macro_rules! ss {
            ($f:expr) => {{
                let s = self.xmm_src(src, 4)?;
                let a = self.xmm[d];
                let r = $f(fs(a[0]), fs(s[0])).to_bits() as u64;
                self.xmm[d][0] = (a[0] & 0xFFFF_FFFF_0000_0000) | r;
            }};
        }
        let minf = |a: f64, b: f64| if a < b { a } else { b };
        let maxf = |a: f64, b: f64| if a > b { a } else { b };
        let minf32 = |a: f32, b: f32| if a < b { a } else { b };
        let maxf32 = |a: f32, b: f32| if a > b { a } else { b };
        match op {
            SseOp::MovU => self.xmm[d] = self.xmm_src(src, 16)?,
            SseOp::MovSdLoad | SseOp::MovqLoad => {
                let s = self.xmm_src(src, 8)?;
                match src {
                    Rm::Reg(_) if op == SseOp::MovSdLoad => self.xmm[d][0] = s[0],
                    _ => self.xmm[d] = [s[0], 0],
                }
            }
            SseOp::MovSsLoad => {
                let s = self.xmm_src(src, 4)?;
                match src {
                    Rm::Reg(_) => self.xmm[d][0] = (self.xmm[d][0] & !0xFFFF_FFFF) | (s[0] & 0xFFFF_FFFF),
                    _ => self.xmm[d] = [s[0] & 0xFFFF_FFFF, 0],
                }
            }
            SseOp::MovLpdLoad => self.xmm[d][0] = self.xmm_src(src, 8)?[0],
            SseOp::MovHpdLoad => self.xmm[d][1] = self.xmm_src(src, 8)?[0],
            SseOp::MovHlps => self.xmm[d][0] = self.xmm_src(src, 16)?[1],
            SseOp::MovLhps => self.xmm[d][1] = self.xmm_src(src, 16)?[0],
            SseOp::AddPd => pd!(|a: f64, b: f64| a + b),
            SseOp::SubPd => pd!(|a: f64, b: f64| a - b),
            SseOp::MulPd => pd!(|a: f64, b: f64| a * b),
            SseOp::DivPd => pd!(|a: f64, b: f64| a / b),
            SseOp::MinPd => pd!(minf),
            SseOp::MaxPd => pd!(maxf),
            SseOp::SqrtPd => {
                let s = self.xmm_src(src, 16)?;
                self.xmm[d] = [bd(fd(s[0]).sqrt()), bd(fd(s[1]).sqrt())];
            }
            SseOp::AddSd => sd!(|a: f64, b: f64| a + b),
            SseOp::SubSd => sd!(|a: f64, b: f64| a - b),
            SseOp::MulSd => sd!(|a: f64, b: f64| a * b),
            SseOp::DivSd => sd!(|a: f64, b: f64| a / b),
            SseOp::MinSd => sd!(minf),
            SseOp::MaxSd => sd!(maxf),
            SseOp::SqrtSd => sd!(|_a: f64, b: f64| b.sqrt()),
            SseOp::AddPs => ps!(|a: f32, b: f32| a + b),
            SseOp::SubPs => ps!(|a: f32, b: f32| a - b),
            SseOp::MulPs => ps!(|a: f32, b: f32| a * b),
            SseOp::DivPs => ps!(|a: f32, b: f32| a / b),
            SseOp::AddSs => ss!(|a: f32, b: f32| a + b),
            SseOp::SubSs => ss!(|a: f32, b: f32| a - b),
            SseOp::MulSs => ss!(|a: f32, b: f32| a * b),
            SseOp::DivSs => ss!(|a: f32, b: f32| a / b),
            SseOp::MinSs => ss!(minf32),
            SseOp::MaxSs => ss!(maxf32),
            SseOp::SqrtSs => ss!(|_a: f32, b: f32| b.sqrt()),
            SseOp::And | SseOp::AndN | SseOp::Or | SseOp::Xor => {
                let s = self.xmm_src(src, 16)?;
                let a = self.xmm[d];
                let f = |x: u64, y: u64| match op {
                    SseOp::And => x & y,
                    SseOp::AndN => !x & y,
                    SseOp::Or => x | y,
                    _ => x ^ y,
                };
                self.xmm[d] = [f(a[0], s[0]), f(a[1], s[1])];
            }
            SseOp::ShufPd => {
                let s = self.xmm_src(src, 16)?;
                let a = self.xmm[d];
                self.xmm[d] = [a[(imm & 1) as usize], s[((imm >> 1) & 1) as usize]];
            }
            SseOp::ShufPs | SseOp::Pshufd => {
                let s = self.xmm_src(src, 16)?;
                let a = self.xmm[d];
                let dw = |v: [u64; 2], i: u8| -> u32 { (v[(i >> 1) as usize] >> ((i & 1) * 32)) as u32 };
                let r: [u32; 4] = if op == SseOp::Pshufd {
                    [dw(s, imm & 3), dw(s, (imm >> 2) & 3), dw(s, (imm >> 4) & 3), dw(s, (imm >> 6) & 3)]
                } else {
                    [dw(a, imm & 3), dw(a, (imm >> 2) & 3), dw(s, (imm >> 4) & 3), dw(s, (imm >> 6) & 3)]
                };
                self.xmm[d] = [r[0] as u64 | (r[1] as u64) << 32, r[2] as u64 | (r[3] as u64) << 32];
            }
            SseOp::UnpckLpd => {
                let s = self.xmm_src(src, 16)?;
                self.xmm[d][1] = s[0];
            }
            SseOp::UnpckHpd => {
                let s = self.xmm_src(src, 16)?;
                let a = self.xmm[d];
                self.xmm[d] = [a[1], s[1]];
            }
            SseOp::UnpckLps | SseOp::UnpckHps => {
                let s = self.xmm_src(src, 16)?;
                let a = self.xmm[d];
                let q = if op == SseOp::UnpckLps { 0 } else { 1 };
                let (a0, a1) = (a[q] & 0xFFFF_FFFF, a[q] >> 32);
                let (s0, s1) = (s[q] & 0xFFFF_FFFF, s[q] >> 32);
                self.xmm[d] = [a0 | (s0 << 32), a1 | (s1 << 32)];
            }
            SseOp::UComiSd | SseOp::ComiSd | SseOp::UComiSs => {
                let (a, b) = if op == SseOp::UComiSs {
                    (fs(self.xmm[d][0]) as f64, fs(self.xmm_src(src, 4)?[0]) as f64)
                } else {
                    (fd(self.xmm[d][0]), fd(self.xmm_src(src, 8)?[0]))
                };
                self.fl.of = false;
                self.fl.sf = false;
                self.fl.af = false;
                if a.is_nan() || b.is_nan() {
                    self.fl.zf = true;
                    self.fl.pf = true;
                    self.fl.cf = true;
                } else {
                    self.fl.zf = a == b;
                    self.fl.pf = false;
                    self.fl.cf = a < b;
                }
            }
            SseOp::HAddPd | SseOp::HSubPd => {
                let s = self.xmm_src(src, 16)?;
                let a = self.xmm[d];
                self.xmm[d] = if op == SseOp::HAddPd {
                    [bd(fd(a[0]) + fd(a[1])), bd(fd(s[0]) + fd(s[1]))]
                } else {
                    [bd(fd(a[0]) - fd(a[1])), bd(fd(s[0]) - fd(s[1]))]
                };
            }
            SseOp::AddSubPd => {
                let s = self.xmm_src(src, 16)?;
                let a = self.xmm[d];
                self.xmm[d] = [bd(fd(a[0]) - fd(s[0])), bd(fd(a[1]) + fd(s[1]))];
            }
            SseOp::CvtPd2Dq | SseOp::CvttPd2Dq => {
                let s = self.xmm_src(src, 16)?;
                let c = |v: f64| -> u64 {
                    let r = if op == SseOp::CvttPd2Dq { v.trunc() } else { v.round_ties_even() };
                    (if r.is_nan() || !(-2147483648.0..=2147483647.0).contains(&r) { 0x8000_0000u32 } else { r as i32 as u32 }) as u64
                };
                self.xmm[d] = [c(fd(s[0])) | (c(fd(s[1])) << 32), 0];
            }
            SseOp::CvtDq2Pd => {
                let s = self.xmm_src(src, 8)?;
                self.xmm[d] = [bd(s[0] as u32 as i32 as f64), bd((s[0] >> 32) as u32 as i32 as f64)];
            }
            SseOp::CvtSs2Sd => {
                let s = self.xmm_src(src, 4)?;
                self.xmm[d][0] = bd(fs(s[0]) as f64);
            }
            SseOp::CvtSd2Ss => {
                let s = self.xmm_src(src, 8)?;
                let v = (fd(s[0]) as f32).to_bits() as u64;
                self.xmm[d][0] = (self.xmm[d][0] & 0xFFFF_FFFF_0000_0000) | v;
            }
            SseOp::CvtPs2Pd => {
                let s = self.xmm_src(src, 8)?;
                self.xmm[d] = [bd(fs(s[0]) as f64), bd(fs(s[0] >> 32) as f64)];
            }
            SseOp::CvtPd2Ps => {
                let s = self.xmm_src(src, 16)?;
                let lo = (fd(s[0]) as f32).to_bits() as u64;
                let hi = (fd(s[1]) as f32).to_bits() as u64;
                self.xmm[d] = [lo | (hi << 32), 0];
            }
            SseOp::PAddD | SseOp::PSubD | SseOp::PCmpEqD | SseOp::PCmpGtD => {
                let s = self.xmm_src(src, 16)?;
                let a = self.xmm[d];
                let mut r = [0u64; 2];
                for q in 0..2 {
                    for h in 0..2 {
                        let x = (a[q] >> (h * 32)) as u32;
                        let y = (s[q] >> (h * 32)) as u32;
                        let v = match op {
                            SseOp::PAddD => x.wrapping_add(y),
                            SseOp::PSubD => x.wrapping_sub(y),
                            SseOp::PCmpEqD => if x == y { u32::MAX } else { 0 },
                            _ => if (x as i32) > (y as i32) { u32::MAX } else { 0 },
                        };
                        r[q] |= (v as u64) << (h * 32);
                    }
                }
                self.xmm[d] = r;
            }
            SseOp::PAddQ | SseOp::PSubQ => {
                let s = self.xmm_src(src, 16)?;
                let a = self.xmm[d];
                self.xmm[d] = if op == SseOp::PAddQ {
                    [a[0].wrapping_add(s[0]), a[1].wrapping_add(s[1])]
                } else {
                    [a[0].wrapping_sub(s[0]), a[1].wrapping_sub(s[1])]
                };
            }
            SseOp::Psrlq(n) | SseOp::Psllq(n) => {
                let a = self.xmm[d];
                let f = |x: u64| {
                    if n >= 64 {
                        0
                    } else if matches!(op, SseOp::Psrlq(_)) {
                        x >> n
                    } else {
                        x << n
                    }
                };
                self.xmm[d] = [f(a[0]), f(a[1])];
            }
            SseOp::Psrld(n) | SseOp::Pslld(n) | SseOp::Psrad(n) => {
                let a = self.xmm[d];
                let f = |x: u32| -> u32 {
                    match op {
                        SseOp::Psrld(_) => if n >= 32 { 0 } else { x >> n },
                        SseOp::Pslld(_) => if n >= 32 { 0 } else { x << n },
                        _ => ((x as i32) >> n.min(31)) as u32,
                    }
                };
                let g = |q: u64| f(q as u32) as u64 | (f((q >> 32) as u32) as u64) << 32;
                self.xmm[d] = [g(a[0]), g(a[1])];
            }
            SseOp::Psrldq(n) | SseOp::Pslldq(n) => {
                let a = self.xmm[d];
                let v = (a[0] as u128) | ((a[1] as u128) << 64);
                let sh = (n.min(16) as u32) * 8;
                let r = if sh >= 128 {
                    0
                } else if matches!(op, SseOp::Psrldq(_)) {
                    v >> sh
                } else {
                    v << sh
                };
                self.xmm[d] = [r as u64, (r >> 64) as u64];
            }
            _ => return Err(unsup(0, format!("sse {op:?}"))),
        }
        Ok(())
    }

    /// Set up a formula call: eax=@x, edx=@y, ecx=@z, push @w, push PIt,
    /// push the sentinel return address.
    pub fn call_formula(&mut self, entry: u32, it_c1: u32, stack_top: u32, max_steps: u64) -> R<()> {
        if let Some(i) = self.progs().iter().position(|(e, _)| *e == entry) {
            return self.with_prog(i, |m, p| m.call_compiled(p, it_c1, stack_top, max_steps));
        }
        let x = it_c1 - 32;
        self.regs = [0; 8];
        self.regs[EAX] = x;
        self.regs[EDX] = x + 8;
        self.regs[ECX] = x + 16;
        self.regs[ESP] = stack_top;
        self.push(x + 24)?; // @w
        self.push(it_c1)?; // PIteration3D
        self.push(RETURN_SENTINEL)?;
        self.top = 0;
        self.fsw_cc = 0;
        self.run(entry, max_steps)
    }

    /// Calls a dIFS formula (`doHybridIFS3D` convention): esi points to
    /// `TIteration3Dext + 144`, edi to the variable buffer, ebx is the hybrid
    /// slot and ecx the remaining iteration count; no stack arguments.
    pub fn call_difs(&mut self, entry: u32, esi: u32, edi: u32, ebx: u32, ecx: u32, stack_top: u32, max_steps: u64) -> R<()> {
        self.regs = [0; 8];
        self.regs[ESI] = esi;
        self.regs[EDI] = edi;
        self.regs[EBX] = ebx;
        self.regs[ECX] = ecx;
        self.regs[EDX] = esi + 128;
        self.regs[ESP] = stack_top;
        self.push(RETURN_SENTINEL)?;
        self.fsw_cc = 0;
        if let Some(i) = self.progs().iter().position(|(e, _)| *e == entry) {
            return self.with_prog(i, |m, p| m.run_prog(p, max_steps));
        }
        self.top = 0;
        self.run(entry, max_steps)
    }

    /// [`Machine::call_difs`] while the programs are out of the machine
    /// ([`Machine::take_progs`]).
    #[allow(clippy::too_many_arguments)]
    pub fn call_difs_taken(&mut self, progs: &Progs, entry: u32, esi: u32, edi: u32, ebx: u32, ecx: u32, stack_top: u32, max_steps: u64) -> R<()> {
        let p = progs.0.as_ref().and_then(|v| v.iter().find(|(e, _)| *e == entry)).map(|(_, p)| p);
        match p {
            Some(p) => {
                self.regs = [0; 8];
                self.regs[ESI] = esi;
                self.regs[EDI] = edi;
                self.regs[EBX] = ebx;
                self.regs[ECX] = ecx;
                self.regs[EDX] = esi + 128;
                self.regs[ESP] = stack_top;
                self.push(RETURN_SENTINEL)?;
                self.fsw_cc = 0;
                self.run_prog(p, max_steps)
            }
            // not compiled: the interpreter does not need the programs
            None => self.call_difs(entry, esi, edi, ebx, ecx, stack_top, max_steps),
        }
    }

    /// For host functions: read the stack argument `n` (0 = first pushed
    /// last, i.e. [esp+4]).
    pub fn stack_arg(&self, n: u32) -> R<u32> {
        self.rd32(self.regs[ESP] + 4 + 4 * n)
    }
    /// For host functions that pop `n` bytes of arguments (stdcall/`ret n`):
    /// keeps the return address on top.
    pub fn drop_args(&mut self, n: u32) -> R<()> {
        let ret = self.rd32(self.regs[ESP])?;
        self.regs[ESP] += n;
        self.wr32(self.regs[ESP], ret)
    }
    pub fn reg(&self, r: usize) -> u32 {
        self.regs[r]
    }
}

impl std::fmt::Debug for Machine {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Machine {{ steps: {} }}", self.steps)
    }
}

impl Default for Machine {
    fn default() -> Self {
        Self::new()
    }
}

#[inline]
fn mask(v: u32, size: u8) -> u32 {
    match size {
        1 => v & 0xFF,
        2 => v & 0xFFFF,
        _ => v,
    }
}

#[inline]
fn sign_extend(v: u32, size: u8) -> i32 {
    match size {
        1 => v as u8 as i8 as i32,
        2 => v as u16 as i16 as i32,
        _ => v as i32,
    }
}

/// The 8-byte cell at offset `o` of `b` (from [`Machine::bytes_at`]).
#[allow(dead_code)] // used by generated code
#[inline(always)]
fn cell<const N: usize>(b: &[u8; N], o: usize) -> u64 {
    u64::from_le_bytes(b[o..o + 8].try_into().unwrap())
}

/// `v.round_ties_even()` without the call into libm (`rint`) that it is on
/// CPUs without SSE4.1: adding and removing 2^52 rounds to an integer in
/// the default rounding mode; larger values are integers already (NaN is
/// made quiet as by `rint`).
#[inline(always)]
fn round_even(v: f64) -> f64 {
    const BIG: f64 = 4503599627370496.0;
    let a = v.abs();
    if a < BIG {
        ((a + BIG) - BIG).copysign(v)
    } else {
        v + 0.0
    }
}

fn f80_to_f64(mant: u64, se: u16) -> f64 {
    let sign = if se & 0x8000 != 0 { -1.0 } else { 1.0 };
    let exp = (se & 0x7FFF) as i32;
    if exp == 0 && mant == 0 {
        return 0.0 * sign;
    }
    if exp == 0x7FFF {
        return if mant << 1 == 0 { sign * f64::INFINITY } else { f64::NAN };
    }
    // scale in steps: 2^(e-63) alone can be out of the f64 range (1e-300 is about 2^-997)
    let mut x = (mant as f64) * 2f64.powi(-63);
    let mut e = exp - 16383;
    while e > 1023 {
        x *= 2f64.powi(1023);
        e -= 1023;
    }
    while e < -1022 {
        x *= 2f64.powi(-1022);
        e += 1022;
    }
    sign * x * 2f64.powi(e)
}

fn f64_to_f80(v: f64) -> (u64, u16) {
    let sign: u16 = if v.is_sign_negative() { 0x8000 } else { 0 };
    if v == 0.0 {
        return (0, sign);
    }
    if v.is_nan() {
        return (0xC000_0000_0000_0000, 0x7FFF | sign);
    }
    if v.is_infinite() {
        return (0x8000_0000_0000_0000, 0x7FFF | sign);
    }
    let bits = v.abs().to_bits();
    let e = ((bits >> 52) & 0x7FF) as i32;
    let f = bits & ((1u64 << 52) - 1);
    let (mant, exp) = if e == 0 {
        // subnormal
        let lz = f.leading_zeros() as i32 - 11;
        (f << (11 + lz), -1022 - lz)
    } else {
        ((f << 11) | (1u64 << 63), e - 1023)
    };
    (mant, (exp + 16383) as u16 | sign)
}

/// Debug helper: disassemble-ish listing of decoded instructions.
pub fn decode_listing(code: &[u8], addr: u32) -> Vec<String> {
    let mut out = Vec::new();
    let mut pos = 0;
    while pos < code.len() {
        match decode(&code[pos..], addr + pos as u32) {
            Ok((ins, len)) => {
                out.push(format!("{:#x}: {:?}", addr + pos as u32, ins));
                pos += len;
            }
            Err(e) => {
                out.push(format!("{e}"));
                break;
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_even_is_round_ties_even() {
        let mut vals = vec![0.0, -0.0, 0.5, -0.5, 1.5, 2.5, -2.5, 0.49999999999999994, 4503599627370495.5, -4503599627370495.5, 4503599627370496.0, 1e300, f64::INFINITY, -f64::INFINITY, f64::MIN_POSITIVE, -1e-310];
        let mut x = 0x1234_5678_9abc_def0u64;
        for _ in 0..100_000 {
            x = x.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
            vals.push(f64::from_bits(x));
            vals.push((x >> 11) as f64 / (1u64 << (x % 60)) as f64 - 1e6);
        }
        for v in vals {
            let (a, b) = (round_even(v), v.round_ties_even());
            assert!(a.to_bits() == b.to_bits() || (a.is_nan() && b.is_nan()), "{v:e}: {a:e} vs {b:e}");
        }
    }

    #[test]
    fn f80_round_trip() {
        for v in [1.0, -2.5, 1e-300, 3.14159e200, 0.1, f64::MIN_POSITIVE, 1e-310, f64::from_bits(1), f64::MAX] {
            let (m, e) = f64_to_f80(v);
            assert_eq!(f80_to_f64(m, e), v);
        }
    }

    #[test]
    fn runs_cosine_pow2() {
        // CosinePow2.m3f code
        let hex = "558BEC5657DD028B75088B7E30DC0ADD00DC08D9C0D8C2DD01D9C0D8C8D8E2DC\
                   4FF0DC4628DD19D8C0D9C9D9FADEF9D9C0DC08DC0AD8C0DC4620DD1AD9C9DEE2\
                   DEC9DC4618DD185F5E5DC20800";
        let code: Vec<u8> = (0..hex.len() / 2).map(|i| u8::from_str_radix(&hex[2 * i..2 * i + 2], 16).unwrap()).collect();
        let mut m = Machine::new();
        let code_at = BASE + 0x8000;
        m.write_bytes(code_at, &code).unwrap();
        let it = BASE + 0x1000 + 56; // C1
        let pvar = BASE + 0x2100;
        m.wr32(it + 48, pvar).unwrap();
        m.wrf64(pvar - 16, 1.0).unwrap(); // Z multiplier
        let (x, y, z) = (0.3, -0.2, 0.5);
        m.wrf64(it - 32, x).unwrap();
        m.wrf64(it - 24, y).unwrap();
        m.wrf64(it - 16, z).unwrap();
        for (k, j) in [0.1, 0.2, 0.3].iter().enumerate() {
            m.wrf64(it + 24 + 8 * k as u32, *j).unwrap();
        }
        m.call_formula(code_at, it, BASE + 0x7F00, 10_000).unwrap();
        let nx = m.rdf64(it - 32).unwrap();
        let ny = m.rdf64(it - 24).unwrap();
        let nz = m.rdf64(it - 16).unwrap();
        assert!(nx.is_finite() && ny.is_finite() && nz.is_finite());
        assert!(nx != x || ny != y || nz != z);
    }

    fn run_code(code: &[u8]) -> R<()> {
        let mut m = Machine::new();
        m.write_bytes(CODE_START, code).unwrap();
        m.predecode(&[CODE_START]);
        m.regs[ESP] = BASE + 0x7F00;
        m.push(RETURN_SENTINEL).unwrap();
        m.run(CODE_START, 1000)
    }

    #[test]
    fn idiv_overflow_halts() {
        // mov edx, 0x8000; xor eax, eax; mov cx, -1; idiv cx; ret
        let r = run_code(&[0xBA, 0, 0x80, 0, 0, 0x31, 0xC0, 0x66, 0xB9, 0xFF, 0xFF, 0x66, 0xF7, 0xF9, 0xC3]);
        assert!(matches!(r, Err(EmuError::Halt(a)) if a == CODE_START + 11), "{r:?}");
        // mov edx, 0x80000000; xor eax, eax; mov ecx, -1; idiv ecx; ret
        let r = run_code(&[0xBA, 0, 0, 0, 0x80, 0x31, 0xC0, 0xB9, 0xFF, 0xFF, 0xFF, 0xFF, 0xF7, 0xF9, 0xC3]);
        assert!(matches!(r, Err(EmuError::Halt(a)) if a == CODE_START + 12), "{r:?}");
        // mov ax, 0x8000; mov cl, -1; idiv cl; ret (quotient out of range)
        let r = run_code(&[0x66, 0xB8, 0, 0x80, 0xB1, 0xFF, 0xF6, 0xF9, 0xC3]);
        assert!(matches!(r, Err(EmuError::Halt(a)) if a == CODE_START + 6), "{r:?}");
        // mov edx, -1; mov eax, -7; mov ecx, 2; idiv ecx; ret
        assert!(run_code(&[0xBA, 0xFF, 0xFF, 0xFF, 0xFF, 0xB8, 0xF9, 0xFF, 0xFF, 0xFF, 0xB9, 2, 0, 0, 0, 0xF7, 0xF9, 0xC3]).is_ok());
    }

    /// Runs `code` (ending in a ret) interpreted and compiled; returns the
    /// f64 at `BASE + 0x2000` of both runs.
    fn run_both(code: &[u8]) -> (f64, f64) {
        let mut out = [0.0; 2];
        for (k, o) in out.iter_mut().enumerate() {
            let mut m = Machine::new();
            m.write_bytes(CODE_START, code).unwrap();
            m.predecode(&[CODE_START]);
            m.regs[ESP] = BASE + 0x7F00;
            m.push(RETURN_SENTINEL).unwrap();
            if k == 0 {
                m.run(CODE_START, 1000).unwrap();
            } else {
                let p = m.compile(CODE_START).expect("compiles");
                m.run_prog(&p, 1000).unwrap();
            }
            *o = m.rdf64(BASE + 0x2000).unwrap();
        }
        (out[0], out[1])
    }

    #[test]
    fn ffreep_pops() {
        // fld1; fldz; ffreep st(1); fstp qword [BASE+0x2000]; ret
        let mut code = vec![0xD9, 0xE8, 0xD9, 0xEE, 0xDF, 0xC1, 0xDD, 0x1D];
        code.extend_from_slice(&(BASE + 0x2000).to_le_bytes());
        code.push(0xC3);
        assert_eq!(run_both(&code), (1.0, 1.0));
    }

    #[test]
    fn compiled_call_return_into_forced_join_rotates() {
        // xor eax, eax; jz L; fld1; fld1; jmp J; L: fldpi; call S;
        // J: fstp qword [BASE+0x2000]; ret; S: ret
        // (J is forced to depth 2, the return from S arrives with depth 1)
        let mut code = vec![0x31, 0xC0, 0x74, 0x06, 0xD9, 0xE8, 0xD9, 0xE8, 0xEB, 0x07, 0xD9, 0xEB, 0xE8, 7, 0, 0, 0, 0xDD, 0x1D];
        code.extend_from_slice(&(BASE + 0x2000).to_le_bytes());
        code.extend_from_slice(&[0xC3, 0xC3]);
        let pi = std::f64::consts::PI;
        assert_eq!(run_both(&code), (pi, pi));
    }

    #[test]
    fn packed_single_sqrt_min_max_unsupported() {
        for op in [0x51, 0x5D, 0x5F] {
            assert!(decode(&[0x0F, op, 0xC1], CODE_START).is_err(), "0F {op:02X}");
            assert!(decode(&[0x66, 0x0F, op, 0xC1], CODE_START).is_ok(), "66 0F {op:02X}");
            assert!(decode(&[0xF3, 0x0F, op, 0xC1], CODE_START).is_ok(), "F3 0F {op:02X}");
        }
    }

    #[test]
    fn compiled_ret_loop_hits_step_limit() {
        // L: push L; ret
        let mut code = vec![0x68];
        code.extend_from_slice(&CODE_START.to_le_bytes());
        code.push(0xC3);
        let mut m = Machine::new();
        m.write_bytes(CODE_START, &code).unwrap();
        m.predecode(&[CODE_START]);
        let p = m.compile(CODE_START).expect("compiles");
        m.regs[ESP] = BASE + 0x7F00;
        m.push(RETURN_SENTINEL).unwrap();
        assert!(matches!(m.run_prog(&p, 1000), Err(EmuError::StepLimit)));
    }
}

// ===========================================================================
// Compiled execution: x87 stack resolved at compile time
// ===========================================================================
//
// Formula code keeps a statically known x87 stack depth at every
// instruction (it is hand written straight line code with simple branches).
// `compile` resolves every st(i) reference to an absolute slot, so the hot
// floating point instructions become simple array operations ("micro ops")
// without any stack bookkeeping.  Integer and SSE instructions are executed
// through the normal interpreter path.  Code that does not fit this model
// (calls, fincstp, inconsistent stack depth) is not compiled and runs in the
// plain interpreter.

#[derive(Clone, Debug)]
enum Op {
    /// any non-x87 instruction, executed by `Machine::exec`
    Gen(Box<Ins>, u32, u32), // ins, next address, own address
    /// `call [mem]` to a host function (`Machine::host`, e.g. the record's
    /// map functions); it returns to the next op
    CallHost(Rm),
    MovRM { r: u8, mem: Mem },
    FLd64 { dst: u8, mem: Mem },
    FSt64 { src: u8, mem: Mem },
    FArithM64 { op: FOp, dst: u8, mem: Mem },
    MovMR { mem: Mem, r: u8 },
    MovRR { d: u8, s: u8 },
    PushR(u8),
    PopR(u8),
    Lea { r: u8, mem: Mem },
    Jcc { cc: u8, target: u32 },
    Jmp(u32),
    /// jumps into a join point that was reached with another x87 depth
    /// (the x87 register file is rotated like a different TOP would)
    JccRot { cc: u8, target: u32, rot: u8 },
    JmpRot { target: u32, rot: u8 },
    CallRot { target: u32, ret: u32, rot: u8 },
    Ret(u16),
    /// local subroutine call (target = op index after resolving)
    Call { target: u32, ret: u32 },
    FLd { dst: u8, kind: FKind, mem: Mem },
    FSt { src: u8, kind: FKind, mem: Mem },
    FMov { dst: u8, src: u8 },
    FSwap { a: u8, b: u8 },
    FArith { op: FOp, dst: u8, a: u8, b: u8 },
    FArithM { op: FOp, dst: u8, kind: FKind, mem: Mem },
    FConst { dst: u8, v: f64 },
    FChs(u8),
    FAbs(u8),
    FSqrt(u8),
    FSin(u8),
    FCos(u8),
    FRndint(u8),
    F2xm1(u8),
    FSincos { src: u8, dst: u8 }, // src := sin, dst := cos (dst = src + 1)
    FPtan { src: u8, dst: u8 },
    FPatan { x: u8, y: u8 },   // y := atan2(y, x)
    FYl2x { x: u8, y: u8 },    // y := y * log2(x)
    FYl2xp1 { x: u8, y: u8 },
    FScale { a: u8, s: u8 },
    FPrem { a: u8, b: u8, ieee: bool },
    FXtract { src: u8, dst: u8 },
    FCom { a: u8, b: u8 },
    FComM { a: u8, kind: FKind, mem: Mem },
    FTst(u8),
    FXam(u8),
    FComi { a: u8, b: u8 },
    FCmov { cc: u8, dst: u8, src: u8 },
    Fnstsw { mem: Option<Mem>, depth: u8 },
}

/// Formula code translated to Rust ahead of time (see `native/`).
pub type NativeFn = fn(&mut Machine, &Prog, u64) -> R<()>;

/// A translated formula called directly with the iteration state (no call
/// set up, no copying of the iteration record through the emulated memory):
/// (machine, program, iteration, `PVar`, step limit).  The machine still
/// holds the stack and the variable buffer.
pub type DirectFn = fn(&mut Machine, &Prog, &mut Iteration, u32, u64) -> R<()>;

/// A translated dIFS formula called without the set up of
/// `Machine::call_difs`: (machine, program, the registers call_difs sets up
/// and the formula leaves, step limit).  The stack slot of the return
/// address must hold `RETURN_SENTINEL` (`Machine::difs_regs`).
pub type DifsFn = fn(&mut Machine, &Prog, &mut [u32; 8], u64) -> R<()>;

/// The [`NativeFn`] of a formula with a [`DifsFn`]: runs it on the
/// registers `Machine::call_difs` set up.  The flags, the x87 registers and
/// `fsw_cc` are not kept (as for a direct formula).
fn difs_via_machine(m: &mut Machine, p: &Prog, max_steps: u64) -> R<()> {
    let mut regs = m.regs;
    let r = (p.difs.expect("dIFS formula"))(m, p, &mut regs, max_steps);
    if r.is_ok() {
        m.regs = regs;
    }
    r
}

/// The registers and stack arguments `Machine::call_compiled` sets up for a
/// formula call (the stack in the machine's memory).
#[inline(always)]
fn direct_regs(m: &mut Machine) -> [u32; 8] {
    use crate::custom::{IT_C1, STACK_TOP};
    let x = IT_C1 - 32;
    let mut rg = [0u32; 8];
    rg[EAX] = x;
    rg[EDX] = x + 8;
    rg[ECX] = x + 16;
    rg[ESP] = STACK_TOP - 12;
    m.wr32u(STACK_TOP - 4, x + 24);
    m.wr32u(STACK_TOP - 8, IT_C1);
    m.wr32u(STACK_TOP - 12, RETURN_SENTINEL);
    rg
}

/// The registers of [`direct_regs`] without the call's stack in the
/// machine (for formulas that keep their stack in locals).
#[inline(always)]
fn direct_regs_local() -> [u32; 8] {
    use crate::custom::{IT_C1, STACK_TOP};
    let x = IT_C1 - 32;
    let mut rg = [0u32; 8];
    rg[EAX] = x;
    rg[EDX] = x + 8;
    rg[ECX] = x + 16;
    rg[ESP] = STACK_TOP - 12;
    rg
}

/// Cell `c` (the 8 bytes at `J4 + 8c`) of the iteration record for a direct
/// formula: the fields `Iteration::run_custom` copies into the record come
/// from `it`, the others from the record in memory.
#[inline(always)]
fn rec_get(m: &Machine, it: &Iteration, pvar: u32, c: u32) -> u64 {
    let mem = || m.rd64u(crate::custom::IT_BASE + 8 * c);
    let low = |v: u32| (mem() & !0xFFFF_FFFF) | v as u64;
    match c {
        0 => it.j[3].to_bits(),
        1 => it.rold.to_bits(),
        3..=6 => it.v[c as usize - 3].to_bits(),
        7..=9 => it.c[c as usize - 7].to_bits(),
        10..=12 => it.j[c as usize - 10].to_bits(),
        13 => low(pvar),
        14 => it.rout.to_bits(),
        15 => it.it_result as u32 as u64 | (it.max_it as u32 as u64) << 32,
        16 => low((it.rstop as f32).to_bits()),
        31 => it.otrap.to_bits(),
        32 => it.vary_scale.to_bits(),
        33 => low(it.first_it as u32),
        34 => it.dfree[0].to_bits(),
        35 => it.dfree[1].to_bits(),
        36 => it.deriv1.to_bits(),
        37 => it.deriv2.to_bits(),
        38 => it.deriv3.to_bits(),
        _ => mem(),
    }
}

/// Stores cell `c` written by a direct formula: the fields `run_custom`
/// reads back go to `it`, everything else to the record in memory (the
/// other fields `run_custom` copies in are overwritten at the next call).
#[inline(always)]
fn rec_set(m: &mut Machine, it: &mut Iteration, c: u32, v: u64) {
    let f = f64::from_bits(v);
    match c {
        0 => it.j[3] = f,
        3..=6 => it.v[c as usize - 3] = f,
        10..=12 => it.j[c as usize - 10] = f,
        14 => it.rout = f,
        31 => it.otrap = f,
        32 => it.vary_scale = f,
        34 => it.dfree[0] = f,
        35 => it.dfree[1] = f,
        36 => it.deriv1 = f,
        37 => it.deriv2 = f,
        38 => it.deriv3 = f,
        _ => {
            if c == 33 {
                it.first_it = v as u32 as i32;
            }
            m.wr64u(crate::custom::IT_BASE + 8 * c, v)
        }
    }
}

/// The fields of the iteration record a direct formula works on
/// (`rec_get`), read from the record in the emulated memory.
fn load_record(m: &Machine, it: &mut Iteration) {
    use crate::custom::{off, IT_BASE};
    let f = |o: u32| m.rdf64u(IT_BASE + o);
    let u = |o: u32| m.rd32u(IT_BASE + o);
    for k in 0..4 {
        it.v[k] = f(off::X + 8 * k as u32);
    }
    for k in 0..3 {
        it.c[k] = f(off::C1 + 8 * k as u32);
        it.j[k] = f(off::J1 + 8 * k as u32);
    }
    it.j[3] = f(off::J4);
    it.rold = f(off::ROLD);
    it.rout = f(off::ROUT);
    it.it_result = u(off::IT_RESULT) as i32;
    it.max_it = u(off::MAX_IT) as i32;
    it.rstop = f32::from_bits(u(off::RSTOP)) as f64;
    it.otrap = f(off::OTRAP);
    it.vary_scale = f(off::VARY_SCALE);
    it.first_it = u(off::FIRST_IT) as i32;
    it.dfree = [f(off::DFREE1), f(off::DFREE2)];
    it.deriv1 = f(off::DERIV1);
    it.deriv2 = f(off::DERIV1 + 8);
    it.deriv3 = f(off::DERIV1 + 16);
}

/// The fields a direct formula may change (`rec_set`), written to the
/// record in the emulated memory.
fn store_record(m: &mut Machine, it: &Iteration) {
    use crate::custom::{off, IT_BASE};
    let mut put = |o: u32, v: f64| m.wrf64u(IT_BASE + o, v);
    for k in 0..4 {
        put(off::X + 8 * k as u32, it.v[k]);
    }
    for k in 0..3 {
        put(off::J1 + 8 * k as u32, it.j[k]);
    }
    put(off::J4, it.j[3]);
    put(off::ROUT, it.rout);
    put(off::OTRAP, it.otrap);
    put(off::VARY_SCALE, it.vary_scale);
    put(off::DFREE1, it.dfree[0]);
    put(off::DFREE2, it.dfree[1]);
    put(off::DERIV1, it.deriv1);
    put(off::DERIV1 + 8, it.deriv2);
    put(off::DERIV1 + 16, it.deriv3);
    m.wr32u(IT_BASE + off::FIRST_IT, it.first_it as u32);
}

/// The [`NativeFn`] of a formula with a direct version: runs it on the
/// iteration record in the emulated memory (for callers of
/// `Machine::call_formula`).  The registers other than `esp` are not kept.
fn direct_via_memory(m: &mut Machine, p: &Prog, max_steps: u64) -> R<()> {
    use crate::custom::{off, IT_BASE, STACK_TOP};
    let mut it = Iteration::default();
    load_record(m, &mut it);
    let pvar = m.rd32u(IT_BASE + off::PVAR);
    // the return pops the stack arguments like the emulated call
    let direct = p.direct.expect("direct formula");
    let r = direct(m, p, &mut it, pvar, max_steps);
    if r.is_ok() {
        store_record(m, &it);
        m.regs[ESP] = STACK_TOP;
    }
    r
}

/// Whether translated native formulas are used (`MB3D_NO_NATIVE` turns them off).
pub static NATIVE_ENABLED: std::sync::LazyLock<std::sync::atomic::AtomicBool> =
    std::sync::LazyLock::new(|| std::sync::atomic::AtomicBool::new(std::env::var_os("MB3D_NO_NATIVE").is_none()));

mod native;

pub mod handwritten;

pub mod lift;

mod lift_emit;

pub mod lift_wgsl;

/// The compiled programs of a machine while they are out of it (see
/// [`Machine::take_progs`]).
pub struct Progs(Option<std::sync::Arc<Vec<(u32, Prog)>>>);

impl Progs {
    /// The direct version of the translated dIFS formula at `entry` (see
    /// [`DifsFn`]) and its program; None if there is none (or translated
    /// formulas are switched off).
    pub fn difs_fn(&self, entry: u32) -> Option<(DifsFn, &Prog)> {
        if !NATIVE_ENABLED.load(std::sync::atomic::Ordering::Relaxed) {
            return None;
        }
        let (_, p) = self.0.as_ref()?.iter().find(|(e, _)| *e == entry)?;
        Some((p.difs?, p))
    }

    /// The direct version of program number `i` (from
    /// [`Machine::direct_index`]) and the program, if that program is the
    /// one at `entry`.
    #[inline]
    pub fn direct_fn(&self, i: usize, entry: u32) -> Option<(DirectFn, &Prog)> {
        let (e, p) = self.0.as_ref()?.get(i)?;
        if *e != entry {
            return None;
        }
        Some((p.direct?, p))
    }
}

/// A compiled formula.
pub struct Prog {
    ops: Vec<Op>,
    /// hash of the code bytes (position independent), to find the
    /// translated native version (`native/`)
    pub hash: u64,
    native: Option<NativeFn>,
    /// the direct version of the translated formula, if any (`native`
    /// then runs it on the emulated memory)
    direct: Option<DirectFn>,
    /// the direct version of a translated dIFS formula, if any (`native`
    /// then runs it on the machine's registers)
    difs: Option<DifsFn>,
    /// return addresses of local calls and the op index they return to
    rets: Vec<(u32, usize)>,
    /// op index for instruction addresses (jump targets)
    index: HashMap<u32, usize>,
}

fn fpu_effect(ins: &Ins) -> Option<(i32, i32)> {
    // (minimum depth required, depth change); None = not an x87 op
    Some(match ins {
        Ins::FLd(..) => (0, 1),
        Ins::FSt(_, _, pop) => (1, if *pop { -1 } else { 0 }),
        Ins::FArithMem(..) => (1, 0),
        Ins::FArith { dst, src, pop, .. } => ((*dst.max(src) as i32) + 1, if *pop { -1 } else { 0 }),
        Ins::FLdSt(i) => (*i as i32 + 1, 1),
        Ins::FStSt(i, pop) => ((*i as i32 + 1).max(1), if *pop { -1 } else { 0 }),
        Ins::FXch(i) => (*i as i32 + 1, 0),
        Ins::FComSt { i, pops } => (*i as i32 + 1, -(*pops as i32)),
        Ins::FComMem { pop, .. } => (1, if *pop { -1 } else { 0 }),
        Ins::FComi { i, pop } => (*i as i32 + 1, if *pop { -1 } else { 0 }),
        Ins::FCmov { i, .. } => (*i as i32 + 1, 0),
        Ins::FUn(u) => match u {
            FUn::Ld1 | FUn::Ldl2t | FUn::Ldl2e | FUn::Ldpi | FUn::Ldlg2 | FUn::Ldln2 | FUn::Ldz => (0, 1),
            FUn::Fyl2x | FUn::Fpatan | FUn::Fyl2xp1 => (2, -1),
            FUn::Fptan | FUn::Sincos | FUn::Xtract => (1, 1),
            FUn::Fprem | FUn::Fprem1 | FUn::Scale => (2, 0),
            FUn::Nop => (0, 0),
            FUn::Decstp => (0, 1),
            FUn::Incstp => (0, -1),
            _ => (1, 0),
        },
        Ins::FnstswAx | Ins::FnstswMem(_) => (0, 0),
        Ins::FFree(_) | Ins::FClex => (0, 0),
        Ins::FInit => (100, 0),
        _ => return None,
    })
}

/// Static x87 depth analysis from `a0` with depth `d0`; returns the depth at
/// `ret` (None if the code never returns).
/// State of the x87 depth analysis.  Join points reached with different
/// depths get the depth most incoming edges agree on (`forced`); the other
/// edges become run time failures (`bad`).  In MB3D formulas these are
/// paths for invalid option values that leave garbage on the x87 stack.
#[derive(Default)]
struct DepthAnalysis {
    depth: HashMap<u32, i32>,
    exits: HashMap<u32, Option<i32>>,
    forced: HashMap<u32, i32>,
    /// edges into a join point with another depth: x87 rotation to apply
    bad: HashMap<(u32, u32), i32>,
    votes: HashMap<u32, HashMap<i32, u32>>,
    conflicts: Vec<u32>,
    /// calls of subroutines analysed at another x87 depth: rotation
    call_rot: HashMap<u32, i32>,
}

fn analyze<'t>(
    get: &dyn Fn(u32) -> Option<&'t (Ins, u8)>,
    a0: u32,
    d0: i32,
    st: &mut DepthAnalysis,
    level: u32,
) -> Result<Option<i32>, String> {
    if level > 8 {
        return Err("call nesting too deep".into());
    }
    let mut exit: Option<i32> = None;
    let mut work = vec![(a0, d0, u32::MAX)];
    while let Some((a, d, from)) = work.pop() {
        *st.votes.entry(a).or_default().entry(d).or_default() += 1;
        if let Some(&f) = st.forced.get(&a) {
            if f != d {
                st.bad.insert((from, a), f - d);
                continue;
            }
        }
        if let Some(&old) = st.depth.get(&a) {
            if old != d {
                st.conflicts.push(a);
            }
            continue;
        }
        let (ins, len) = get(a).ok_or(format!("{a:#x}: not decoded"))?;
        st.depth.insert(a, d);
        let mut nd = d;
        if let Some((need, delta)) = fpu_effect(ins) {
            if need >= 100 {
                return Err(format!("{a:#x}: {ins:?}"));
            }
            nd = d + delta;
            if !(-8..=16).contains(&nd) {
                return Err(format!("{a:#x}: x87 depth {nd}"));
            }
        }
        let next = a.wrapping_add(*len as u32);
        match ins {
            Ins::Jcc { target, .. } => {
                work.push((*target, nd, a));
                work.push((next, nd, a));
            }
            Ins::Jmp(t) => work.push((*t, nd, a)),
            Ins::Ret(_) => {
                if let Some(e) = exit {
                    if e != nd {
                        return Err(format!("{a:#x}: inconsistent x87 depth at return"));
                    }
                }
                exit = Some(nd);
            }
            Ins::Call(t) => {
                let e = match st.exits.get(t) {
                    Some(e) => *e,
                    None => {
                        st.exits.insert(*t, None);
                        let e = analyze(get, *t, nd, st, level + 1)?;
                        st.exits.insert(*t, e);
                        e
                    }
                };
                let entry_depth = *st.depth.get(t).ok_or(format!("{a:#x}: subroutine not analysed"))?;
                let rot = entry_depth - nd;
                if rot != 0 {
                    st.call_rot.insert(a, rot);
                }
                if let Some(e) = e {
                    work.push((next, e - rot, a));
                }
            }
            // an indirect call is a host function (the map functions of
            // the record); they keep the x87 stack
            Ins::JmpInd(_) | Ins::Halt => return Err(format!("{a:#x}: {ins:?}")),
            _ => work.push((next, nd, a)),
        }
    }
    Ok(exit)
}

impl Machine {
    /// Compiles the code at `entry` (must be pre-decoded).  Returns None if
    /// the code does not fit the static x87 stack model.
    pub fn compile(&self, entry: u32) -> Result<Prog, String> {
        let table = self.table.clone();
        let get = |a: u32| -> Option<&(Ins, u8)> { table.get(a.wrapping_sub(CODE_START) as usize)?.as_ref() };
        // depth analysis (recursive over local subroutine calls)
        let mut forced: HashMap<u32, i32> = HashMap::new();
        let mut st;
        let mut round = 0;
        loop {
            st = DepthAnalysis { forced: forced.clone(), ..Default::default() };
            analyze(&get, entry, 0, &mut st, 0)?;
            if st.conflicts.is_empty() {
                break;
            }
            round += 1;
            if round > 6 {
                return Err(format!("{:#x}: inconsistent x87 depth", st.conflicts[0]));
            }
            for a in &st.conflicts {
                // the depth most edges arrive with
                let v = &st.votes[a];
                let best = v.iter().max_by_key(|(d, n)| (**n, **d)).map(|(d, _)| *d).unwrap();
                forced.insert(*a, best);
            }
        }
        let depth = st.depth;
        let bad = st.bad;
        let call_rot = st.call_rot;
        let mut addrs: Vec<u32> = depth.keys().copied().collect();
        addrs.sort_unstable();
        let mut index = HashMap::new();
        let mut ops = Vec::with_capacity(addrs.len());
        for (ai, &a) in addrs.iter().enumerate() {
            index.insert(a, ops.len());
            let (ins, len) = get(a).ok_or("decode")?;
            let d = depth[&a];
            let next = a.wrapping_add(*len as u32);
            // fallthrough must be the next op
            let falls = !matches!(ins, Ins::Jmp(_) | Ins::Ret(_) | Ins::Call(_));
            if falls && addrs.get(ai + 1) != Some(&next) {
                return Err(format!("{a:#x}: fallthrough not contiguous"));
            }
            let slot = |x: i32| -> u8 { x.rem_euclid(8) as u8 };
            let st = |i: u8| -> u8 { slot(d - 1 - i as i32) };
            let top = slot(d - 1);
            let below = slot(d - 2);
            let op = match ins {
                Ins::Jcc { cc, target } if bad.contains_key(&(a, *target)) => {
                    Op::JccRot { cc: *cc, target: *target, rot: bad[&(a, *target)].rem_euclid(8) as u8 }
                }
                Ins::Jcc { cc, target } => Op::Jcc { cc: *cc, target: *target },
                Ins::Jmp(t) if bad.contains_key(&(a, *t)) => Op::JmpRot { target: *t, rot: bad[&(a, *t)].rem_euclid(8) as u8 },
                Ins::Jmp(t) => Op::Jmp(*t),
                Ins::Ret(n) => Op::Ret(*n),
                Ins::Call(t) => Op::Call { target: *t, ret: next },
                Ins::CallInd(rm) => Op::CallHost(*rm),
                Ins::FLd(FKind::F64, mem) => Op::FLd64 { dst: slot(d), mem: *mem },
                Ins::FLd(kind, mem) => Op::FLd { dst: slot(d), kind: *kind, mem: *mem },
                Ins::FSt(FKind::F64, mem, _) => Op::FSt64 { src: top, mem: *mem },
                Ins::FArithMem(op, FKind::F64, mem) => Op::FArithM64 { op: *op, dst: top, mem: *mem },
                Ins::FSt(kind, mem, _) => Op::FSt { src: top, kind: *kind, mem: *mem },
                Ins::FArithMem(op, kind, mem) => Op::FArithM { op: *op, dst: top, kind: *kind, mem: *mem },
                Ins::FArith { op, dst, src, .. } => Op::FArith { op: *op, dst: st(*dst), a: st(*dst), b: st(*src) },
                Ins::FLdSt(i) => Op::FMov { dst: slot(d), src: st(*i) },
                Ins::FStSt(i, _) => Op::FMov { dst: st(*i), src: top },
                Ins::FXch(i) => Op::FSwap { a: top, b: st(*i) },
                Ins::FComSt { i, .. } => Op::FCom { a: top, b: st(*i) },
                Ins::FComMem { kind, mem, .. } => Op::FComM { a: top, kind: *kind, mem: *mem },
                Ins::FComi { i, .. } => Op::FComi { a: top, b: st(*i) },
                Ins::FCmov { cc, i } => Op::FCmov { cc: *cc, dst: top, src: st(*i) },
                Ins::FnstswAx => Op::Fnstsw { mem: None, depth: slot(d) },
                Ins::FnstswMem(m) => Op::Fnstsw { mem: Some(*m), depth: slot(d) },
                Ins::FFree(_) | Ins::FClex => Op::Gen(Box::new(Ins::Nop), next, a),
                Ins::FUn(u) => {
                    use std::f64::consts::*;
                    let push = slot(d);
                    match u {
                        FUn::Ld1 => Op::FConst { dst: push, v: 1.0 },
                        FUn::Ldl2t => Op::FConst { dst: push, v: LOG2_10 },
                        FUn::Ldl2e => Op::FConst { dst: push, v: LOG2_E },
                        FUn::Ldpi => Op::FConst { dst: push, v: PI },
                        FUn::Ldlg2 => Op::FConst { dst: push, v: LOG10_2 },
                        FUn::Ldln2 => Op::FConst { dst: push, v: LN_2 },
                        FUn::Ldz => Op::FConst { dst: push, v: 0.0 },
                        FUn::Chs => Op::FChs(top),
                        FUn::Abs => Op::FAbs(top),
                        FUn::Sqrt => Op::FSqrt(top),
                        FUn::Sin => Op::FSin(top),
                        FUn::Cos => Op::FCos(top),
                        FUn::Rndint => Op::FRndint(top),
                        FUn::F2xm1 => Op::F2xm1(top),
                        FUn::Tst => Op::FTst(top),
                        FUn::Xam => Op::FXam(top),
                        FUn::Sincos => Op::FSincos { src: top, dst: push },
                        FUn::Fptan => Op::FPtan { src: top, dst: push },
                        FUn::Xtract => Op::FXtract { src: top, dst: push },
                        FUn::Fpatan => Op::FPatan { x: top, y: below },
                        FUn::Fyl2x => Op::FYl2x { x: top, y: below },
                        FUn::Fyl2xp1 => Op::FYl2xp1 { x: top, y: below },
                        FUn::Scale => Op::FScale { a: top, s: below },
                        FUn::Fprem => Op::FPrem { a: top, b: below, ieee: false },
                        FUn::Fprem1 => Op::FPrem { a: top, b: below, ieee: true },
                        FUn::Nop => Op::Gen(Box::new(Ins::Nop), next, a),
                        FUn::Decstp | FUn::Incstp => Op::Gen(Box::new(Ins::Nop), next, a),
                    }
                }
                Ins::FInit => return Err("finit".into()),
                Ins::Mov { dst: Rm::Reg(r), src: Src::Rm(Rm::Mem(m)), size: 4 } => Op::MovRM { r: *r, mem: *m },
                Ins::Mov { dst: Rm::Mem(m), src: Src::Rm(Rm::Reg(r)), size: 4 } => Op::MovMR { mem: *m, r: *r },
                Ins::Mov { dst: Rm::Reg(r), src: Src::Rm(Rm::Reg(q)), size: 4 } => Op::MovRR { d: *r, s: *q },
                Ins::Push(Src::Rm(Rm::Reg(r))) => Op::PushR(*r),
                Ins::Pop(Rm::Reg(r)) => Op::PopR(*r),
                Ins::Lea { reg, mem } => Op::Lea { r: *reg, mem: *mem },
                other => Op::Gen(Box::new(other.clone()), next, a),
            };
            if let Op::Call { target, .. } = &op {
                // the subroutine was compiled for another x87 depth: rotate the
                // register file around the call; it returns to a landing op,
                // which also rotates into a join point forced to another depth
                let rot = call_rot.get(&a).copied().unwrap_or(0);
                let back = bad.get(&(a, next)).copied().unwrap_or(0) - rot;
                if rot != 0 || back != 0 {
                    let fake = 0xFFFE_0000 | ops.len() as u32;
                    ops.push(if rot != 0 {
                        Op::CallRot { target: *target, ret: fake, rot: rot.rem_euclid(8) as u8 }
                    } else {
                        Op::Call { target: *target, ret: fake }
                    });
                    index.insert(fake, ops.len());
                    ops.push(Op::JmpRot { target: next, rot: back.rem_euclid(8) as u8 });
                    continue;
                }
            }
            ops.push(op);
            if falls {
                if let Some(&rot) = bad.get(&(a, next)) {
                    ops.push(Op::JmpRot { target: next, rot: rot.rem_euclid(8) as u8 });
                }
            }
        }
        // resolve jump targets
        for op in ops.iter_mut() {
            match op {
                Op::Jcc { target, .. }
                | Op::Jmp(target)
                | Op::Call { target, .. }
                | Op::JccRot { target, .. }
                | Op::JmpRot { target, .. }
                | Op::CallRot { target, .. } => {
                    let i = *index.get(target).ok_or("jump target")?;
                    *target = i as u32;
                }
                _ => {}
            }
        }
        // FNV-1a over the instruction bytes and their offsets from the entry
        let mut hash: u64 = 0xcbf29ce484222325;
        for &a in &addrs {
            let (_, len) = get(a).ok_or("decode")?;
            let o = self.idx(a, *len as usize).map_err(|e| e.to_string())?;
            for &byte in a.wrapping_sub(entry).to_le_bytes().iter().chain(&self.mem[o..o + *len as usize]) {
                hash ^= byte as u64;
                hash = hash.wrapping_mul(0x100000001b3);
            }
        }
        let native = native::lookup(hash, ops.len());
        let rets = ops
            .iter()
            .enumerate()
            .filter_map(|(k, o)| match o {
                Op::Call { ret, .. } | Op::CallRot { ret, .. } => Some((*ret, k + 1)),
                _ => None,
            })
            .collect();
        let direct = native::lookup_direct(hash, ops.len());
        let difs = handwritten::lookup_difs(hash, ops.len()).or_else(|| native::lookup_difs(hash, ops.len()));
        let native = if direct.is_some() {
            Some(direct_via_memory as NativeFn)
        } else if difs.is_some() {
            Some(difs_via_machine as NativeFn)
        } else {
            native
        };
        Ok(Prog { ops, index, hash, native, direct, difs, rets })
    }

    fn fload_m(&self, kind: FKind, mem: &Mem) -> R<f64> {
        self.fload(kind, self.ea(mem))
    }

    /// Run a compiled formula (registers / stack already set up).
    pub fn run_prog(&mut self, p: &Prog, max_steps: u64) -> R<()> {
        if let Some(nf) = p.native {
            if NATIVE_ENABLED.load(std::sync::atomic::Ordering::Relaxed) {
                return nf(self, p, max_steps);
            }
        }
        let mut f = [0f64; 8];
        let mut pc = 0usize;
        let limit = self.steps + max_steps;
        let n = p.ops.len();
        while pc < n {
            let Some(op) = p.ops.get(pc) else { break };
            match op {
                Op::JccRot { cc, target, rot } => {
                    if self.cond(*cc) {
                        f.rotate_right(*rot as usize);
                        self.steps += 1;
                        if self.steps > limit {
                            return Err(EmuError::StepLimit);
                        }
                        pc = *target as usize;
                        continue;
                    }
                }
                Op::JmpRot { target, rot } => {
                    f.rotate_right(*rot as usize);
                    self.steps += 1;
                    if self.steps > limit {
                        return Err(EmuError::StepLimit);
                    }
                    pc = *target as usize;
                    continue;
                }
                Op::Jcc { cc, target } => {
                    if self.cond(*cc) {
                        self.steps += 1;
                        if self.steps > limit {
                            return Err(EmuError::StepLimit);
                        }
                        pc = *target as usize;
                        continue;
                    }
                }
                Op::Jmp(t) => {
                    self.steps += 1;
                    if self.steps > limit {
                        return Err(EmuError::StepLimit);
                    }
                    pc = *t as usize;
                    continue;
                }
                Op::Call { target, ret } => {
                    self.steps += 1;
                    if self.steps > limit {
                        return Err(EmuError::StepLimit);
                    }
                    self.push(*ret)?;
                    pc = *target as usize;
                    continue;
                }
                Op::CallRot { target, ret, rot } => {
                    f.rotate_right(*rot as usize);
                    self.steps += 1;
                    if self.steps > limit {
                        return Err(EmuError::StepLimit);
                    }
                    self.push(*ret)?;
                    pc = *target as usize;
                    continue;
                }
                Op::Ret(k) => {
                    let r = self.pop()?;
                    self.regs[ESP] = self.regs[ESP].wrapping_add(*k as u32);
                    if r != RETURN_SENTINEL {
                        // a loop through ret (push L; ret) must run out of steps too
                        self.steps += 1;
                        if self.steps > limit {
                            return Err(EmuError::StepLimit);
                        }
                        match p.index.get(&r) {
                            Some(&i) => {
                                pc = i;
                                continue;
                            }
                            None => return Err(unsup(r, "compiled code returned to an unknown address".into())),
                        }
                    }
                    // leave the stack contents for compatibility
                    for (i, v) in f.iter().enumerate() {
                        self.st[i] = *v;
                    }
                    return Ok(());
                }
                other => self.exec_op(other, &mut f)?,
            }
            pc += 1;
        }
        Err(unsup(0, "compiled code ran past its end".into()))
    }

    /// One non-control-flow micro op on the slot registers `f`.
    #[inline]
    fn exec_op(&mut self, op: &Op, f: &mut [f64; 8]) -> R<()> {
        match op {
            Op::CallHost(rm) => {
                // as Machine::run: the host function finds its return
                // address on the stack and drops its stack arguments
                let t = self.get_rm(rm, 4)?;
                let hf = self.host.iter().find(|(a, _)| *a == t).map(|(_, f)| *f).ok_or(EmuError::Halt(t))?;
                self.push(RETURN_SENTINEL)?;
                hf(self)?;
                self.pop()?;
            }
                Op::Gen(ins, next, at) => {
                    let r = self.exec(ins, *next, *at)?;
                    if r != *next {
                        // only control flow changes eip; Gen never does
                        return Err(unsup(*at, "unexpected control flow".into()));
                    }
                }
                Op::MovRM { r, mem } => self.regs[*r as usize] = self.rd32(self.ea(mem))?,
                Op::FLd64 { dst, mem } => f[(*dst & 7) as usize] = self.rdf64(self.ea(mem))?,
                Op::FSt64 { src, mem } => {
                    let a = self.ea(mem);
                    self.wrf64(a, f[(*src & 7) as usize])?;
                }
                Op::FArithM64 { op, dst, mem } => {
                    let b = self.rdf64(self.ea(mem))?;
                    let a = f[(*dst & 7) as usize];
                    f[(*dst & 7) as usize] = match op {
                        FOp::Add => a + b,
                        FOp::Mul => a * b,
                        FOp::Sub => a - b,
                        FOp::SubR => b - a,
                        FOp::Div => a / b,
                        FOp::DivR => b / a,
                    };
                }
                Op::MovMR { mem, r } => {
                    let a = self.ea(mem);
                    self.wr32(a, self.regs[*r as usize])?;
                }
                Op::MovRR { d, s } => self.regs[*d as usize] = self.regs[*s as usize],
                Op::PushR(r) => {
                    let v = self.regs[*r as usize];
                    self.push(v)?;
                }
                Op::PopR(r) => {
                    let v = self.pop()?;
                    self.regs[*r as usize] = v;
                }
                Op::Lea { r, mem } => self.regs[*r as usize] = self.ea(mem),
                Op::FLd { dst, kind, mem } => f[(*dst & 7) as usize] = self.fload_m(*kind, mem)?,
                Op::FSt { src, kind, mem } => {
                    let a = self.ea(mem);
                    self.fstore(*kind, a, f[(*src & 7) as usize])?;
                }
                Op::FMov { dst, src } => f[(*dst & 7) as usize] = f[(*src & 7) as usize],
                Op::FSwap { a, b } => f.swap((*a & 7) as usize, (*b & 7) as usize),
                Op::FArith { op, dst, a, b } => {
                    f[(*dst & 7) as usize] = Self::farith(*op, f[(*a & 7) as usize], f[(*b & 7) as usize]);
                }
                Op::FArithM { op, dst, kind, mem } => {
                    let b = self.fload_m(*kind, mem)?;
                    f[(*dst & 7) as usize] = Self::farith(*op, f[(*dst & 7) as usize], b);
                }
                Op::FConst { dst, v } => f[(*dst & 7) as usize] = *v,
                Op::FChs(i) => f[(*i & 7) as usize] = -f[(*i & 7) as usize],
                Op::FAbs(i) => f[(*i & 7) as usize] = f[(*i & 7) as usize].abs(),
                Op::FSqrt(i) => f[(*i & 7) as usize] = f[(*i & 7) as usize].sqrt(),
                Op::FSin(i) => {
                    f[(*i & 7) as usize] = f[(*i & 7) as usize].sin();
                    self.fsw_cc &= !0x0400;
                }
                Op::FCos(i) => {
                    f[(*i & 7) as usize] = f[(*i & 7) as usize].cos();
                    self.fsw_cc &= !0x0400;
                }
                Op::FRndint(i) => f[(*i & 7) as usize] = self.round_int(f[(*i & 7) as usize]),
                Op::F2xm1(i) => f[(*i & 7) as usize] = f[(*i & 7) as usize].exp2() - 1.0,
                Op::FSincos { src, dst } => {
                    let (s, c) = f[(*src & 7) as usize].sin_cos();
                    f[(*src & 7) as usize] = s;
                    f[(*dst & 7) as usize] = c;
                    self.fsw_cc &= !0x0400;
                }
                Op::FPtan { src, dst } => {
                    f[(*src & 7) as usize] = f[(*src & 7) as usize].tan();
                    f[(*dst & 7) as usize] = 1.0;
                    self.fsw_cc &= !0x0400;
                }
                Op::FXtract { src, dst } => {
                    let v = f[(*src & 7) as usize];
                    if v == 0.0 || !v.is_finite() {
                        f[(*src & 7) as usize] = f64::NEG_INFINITY;
                        f[(*dst & 7) as usize] = v;
                    } else {
                        let e = v.abs().log2().floor();
                        f[(*src & 7) as usize] = e;
                        f[(*dst & 7) as usize] = v / e.exp2();
                    }
                }
                Op::FPatan { x, y } => f[(*y & 7) as usize] = f[(*y & 7) as usize].atan2(f[(*x & 7) as usize]),
                Op::FYl2x { x, y } => f[(*y & 7) as usize] *= f[(*x & 7) as usize].log2(),
                Op::FYl2xp1 { x, y } => f[(*y & 7) as usize] *= f[(*x & 7) as usize].ln_1p() * std::f64::consts::LOG2_E,
                Op::FScale { a, s } => f[(*a & 7) as usize] *= f[(*s & 7) as usize].trunc().exp2(),
                Op::FPrem { a, b, ieee } => {
                    let (x, y) = (f[(*a & 7) as usize], f[(*b & 7) as usize]);
                    let q = if *ieee { (x / y).round_ties_even() } else { (x / y).trunc() };
                    f[(*a & 7) as usize] = if *ieee { x - q * y } else { x % y };
                    let qi = q.abs() as u64;
                    self.fsw_cc = (if qi & 4 != 0 { 0x0100 } else { 0 })
                        | (if qi & 2 != 0 { 0x4000 } else { 0 })
                        | (if qi & 1 != 0 { 0x0200 } else { 0 });
                }
                Op::FCom { a, b } => self.fcompare(f[(*a & 7) as usize], f[(*b & 7) as usize]),
                Op::FComM { a, kind, mem } => {
                    let b = self.fload_m(*kind, mem)?;
                    self.fcompare(f[(*a & 7) as usize], b);
                }
                Op::FTst(a) => self.fcompare(f[(*a & 7) as usize], 0.0),
                Op::FXam(a) => {
                    // reuse the interpreter implementation on a temporary stack
                    let save = (self.st, self.top);
                    self.top = 0;
                    self.st[0] = f[(*a & 7) as usize];
                    self.funary(FUn::Xam);
                    (self.st, self.top) = save;
                }
                Op::FComi { a, b } => {
                    let (x, y) = (f[(*a & 7) as usize], f[(*b & 7) as usize]);
                    self.fl.of = false;
                    self.fl.sf = false;
                    self.fl.af = false;
                    if x.is_nan() || y.is_nan() {
                        self.fl.zf = true;
                        self.fl.pf = true;
                        self.fl.cf = true;
                    } else {
                        self.fl.zf = x == y;
                        self.fl.pf = false;
                        self.fl.cf = x < y;
                    }
                }
                Op::FCmov { cc, dst, src } => {
                    if self.cond(*cc) {
                        f[(*dst & 7) as usize] = f[(*src & 7) as usize];
                    }
                }
                Op::Fnstsw { mem, depth } => {
                    let top = ((8 - *depth as u16) & 7) << 11;
                    let w = self.fsw_cc | top;
                    match mem {
                        None => self.regs[EAX] = (self.regs[EAX] & 0xFFFF_0000) | w as u32,
                        Some(m) => {
                            let a = self.ea(m);
                            self.wr16(a, w)?;
                        }
                    }
                }
                Op::Jcc { .. } | Op::Jmp(_) | Op::Call { .. } | Op::Ret(_) | Op::JccRot { .. } | Op::JmpRot { .. } | Op::CallRot { .. } => {
                    return Err(unsup(0, "control flow op in exec_op".into()))
                }
        }
        Ok(())
    }

    /// Like `call_formula`, but uses a compiled program if one exists.
    pub fn call_compiled(&mut self, p: &Prog, it_c1: u32, stack_top: u32, max_steps: u64) -> R<()> {
        let x = it_c1 - 32;
        self.regs = [0; 8];
        self.regs[EAX] = x;
        self.regs[EDX] = x + 8;
        self.regs[ECX] = x + 16;
        self.regs[ESP] = stack_top;
        self.push(x + 24)?;
        self.push(it_c1)?;
        self.push(RETURN_SENTINEL)?;
        self.fsw_cc = 0;
        self.run_prog(p, max_steps)
    }
}

impl std::fmt::Debug for Prog {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Prog({} ops, hash {:016x}, native {})", self.ops.len(), self.hash, self.native.is_some())
    }
}

/// Rewrites the memory accesses of a line to the unchecked variants (no
/// `Result`): for ops whose addresses the lifting analysis resolved.
fn uncheck(line: &str) -> String {
    let names = [
        ("m.rd8(", "m.rd8u("),
        ("m.rd16(", "m.rd16u("),
        ("m.rd32(", "m.rd32u("),
        ("m.rd64(", "m.rd64u("),
        ("m.rdf64(", "m.rdf64u("),
        ("m.fload(", "m.floadu("),
        ("m.wr8(", "m.wr8u("),
        ("m.wr16(", "m.wr16u("),
        ("m.wr32(", "m.wr32u("),
        ("m.wr64(", "m.wr64u("),
        ("m.wrf64(", "m.wrf64u("),
        ("m.fstore(", "m.fstoreu("),
        ("push_l(", "push_lu("),
        ("pop_l(", "pop_lu("),
    ];
    let mut l = line.to_string();
    for (a, b) in names {
        let mut from = 0;
        while let Some(i) = l[from..].find(a).map(|i| i + from) {
            // the matching parenthesis, then drop a following `?`
            let open = i + a.len() - 1;
            let mut depth = 0;
            let mut close = None;
            for (j, c) in l[open..].char_indices() {
                match c {
                    '(' => depth += 1,
                    ')' => {
                        depth -= 1;
                        if depth == 0 {
                            close = Some(open + j);
                            break;
                        }
                    }
                    _ => {}
                }
            }
            let Some(c) = close else { break };
            let q = l[c + 1..].starts_with('?');
            l = format!("{}{}{}{}", &l[..i], b, &l[i + a.len()..=c], &l[c + 1 + q as usize..]);
            from = i + b.len();
        }
    }
    l
}

/// Rewrites a line of generated code from the machine's registers, flags
/// and counters to the locals of the translated function (`rg`, `fl`,
/// `fsw`, `steps`); interpreter calls get the machine state written back
/// before and read again after.
fn localize(line: &str, permute_rotations: bool) -> String {
    const SYNC_OUT: &str = "m.regs = rg; m.fl = fl; m.fsw_cc = fsw; m.steps = steps;";
    const SYNC_IN: &str = "rg = m.regs; fl = m.fl; fsw = m.fsw_cc; steps = m.steps;";
    let mut l = line.to_string();
    for call in ["m.exec(ins, *nx, *at)?;", "m.exec_op("] {
        if let Some(i) = l.find(call) {
            // the whole statement up to its `?;`
            let end = l[i..].find("?;").map(|e| i + e + 2).unwrap_or(l.len());
            let stmt = &l[i..end - 2];
            let new = format!("{{ {SYNC_OUT} let res = {stmt}; {SYNC_IN} res?; }}");
            l = format!("{}{}{}", &l[..i], new, &l[end..]);
            return l;
        }
    }
    let rep = [
        ("m.regs[", "rg["),
        ("m.alu(", "fl.alu("),
        ("m.cond(", "fl.cond("),
        ("m.fl.", "fl."),
        ("m.fsw_cc", "fsw"),
        ("m.steps", "steps"),
        ("m.shift_reg_const(", "shift_reg_l(&mut rg, &mut fl, "),
        ("m.get_reg(", "reg_get(&rg, "),
        ("m.set_reg(", "reg_set(&mut rg, "),
        ("m.push(", "push_l(m, &mut rg, "),
        ("m.pop()", "pop_l(m, &mut rg)"),
    ];
    for (a, b) in rep {
        l = l.replace(a, b);
    }
    // f.rotate_right(n) as a fixed permutation (keeps f in registers;
    // rotate_right is a memory move).  Not in code with local subroutines:
    // their returns are dynamic jumps, and the permutations become register
    // shuffles at every call site (slower than the memory move there)
    #[allow(clippy::while_immutable_condition)] // the body breaks out
    while permute_rotations {
        let Some(i) = l.find("f.rotate_right(") else { break };
        let end = l[i..].find(");").map(|e| i + e + 2).unwrap_or(l.len());
        let n: usize = l[i + "f.rotate_right(".len()..end - 2].trim().parse().unwrap_or(0);
        let perm: Vec<String> = (0..8).map(|k| format!("f[{}]", (k + 8 - n % 8) % 8)).collect();
        l = format!("{}f = [{}];{}", &l[..i], perm.join(", "), &l[end..]);
    }
    // m.fcompare(a, b);  ->  fsw = fcompare_cc(a, b);
    while let Some(i) = l.find("m.fcompare(") {
        let end = l[i..].find(");").map(|e| i + e + 2).unwrap_or(l.len());
        let args = &l[i + "m.fcompare(".len()..end - 2];
        l = format!("{}fsw = fcompare_cc({args});{}", &l[..i], &l[end..]);
    }
    l
}

/// Rust expression reading the operand `rm` of `size` bytes (as u32).
fn rm_read(rm: &Rm, size: u8) -> String {
    match (rm, size) {
        (Rm::Reg(r), 4) => format!("m.regs[{r}]"),
        (Rm::Reg(r), _) => format!("m.get_reg({r}, {size})"),
        (Rm::Mem(mm), 1) => format!("(m.rd8({})? as u32)", ea_expr(mm)),
        (Rm::Mem(mm), 2) => format!("(m.rd16({})? as u32)", ea_expr(mm)),
        (Rm::Mem(mm), _) => format!("m.rd32({})?", ea_expr(mm)),
    }
}

/// Rust statement writing the u32 value `v` to the operand `rm` of `size` bytes.
fn rm_write(rm: &Rm, size: u8, v: &str) -> String {
    match (rm, size) {
        (Rm::Reg(r), 4) => format!("m.regs[{r}] = {v};"),
        (Rm::Reg(r), _) => format!("m.set_reg({r}, {size}, {v});"),
        (Rm::Mem(mm), 1) => format!("{{ let a = {}; m.wr8(a, {v} as u8)?; }}", ea_expr(mm)),
        (Rm::Mem(mm), 2) => format!("{{ let a = {}; m.wr16(a, {v} as u16)?; }}", ea_expr(mm)),
        (Rm::Mem(mm), _) => format!("{{ let a = {}; m.wr32(a, {v})?; }}", ea_expr(mm)),
    }
}

/// Rust expression for the source operand of `size` bytes.
fn src_expr(src: &Src, size: u8) -> String {
    match src {
        Src::Imm(v) => format!("{:#x}u32", mask(*v, size)),
        Src::Rm(rm) => rm_read(rm, size),
    }
}

/// Rust expression for the effective address of `m`.
fn ea_expr(m: &Mem) -> String {
    let mut e = format!("{:#x}u32", m.disp as u32);
    if let Some(b) = m.base {
        e = format!("m.regs[{b}].wrapping_add({e})");
    }
    if let Some((i, sc)) = m.index {
        e = format!("({e}).wrapping_add(m.regs[{i}].wrapping_mul({sc}))");
    }
    e
}

/// SSE2 double precision ops that `emit_rust` translates (the others run
/// through the interpreter).
fn sse_native(op: SseOp, src: &Rm) -> bool {
    use SseOp::*;
    match op {
        MovU | MovLpdLoad | MovHpdLoad | AddPd | SubPd | MulPd | DivPd | MinPd | MaxPd | SqrtPd | AddSd | SubSd | MulSd | DivSd
        | MinSd | MaxSd | SqrtSd | And | AndN | Or | Xor | UnpckLpd | UnpckHpd | ShufPd | HAddPd | UComiSd | ComiSd | Pshufd => true,
        MovSdLoad | MovqLoad | CvtPd2Dq | CvtDq2Pd => true,
        MovHlps | MovLhps => matches!(src, Rm::Reg(_)),
        _ => false,
    }
}

/// Rust code for one SSE op, with the semantics of `Machine::sse`.
fn emit_sse(op: SseOp, d: usize, src: &Rm, imm: u8) -> String {
    let s16 = match src {
        Rm::Reg(r) => format!("m.xmm[{r}]"),
        Rm::Mem(mm) => format!("{{ let a = {}; [m.rd64(a)?, m.rd64(a.wrapping_add(8))?] }}", ea_expr(mm)),
    };
    let s8 = match src {
        Rm::Reg(r) => format!("m.xmm[{r}][0]"),
        Rm::Mem(mm) => format!("m.rd64({})?", ea_expr(mm)),
    };
    let src_reg = match src {
        Rm::Reg(r) => Some(*r),
        Rm::Mem(_) => None,
    };
    emit_sse_with(op, d, src_reg, &s16, &s8, imm)
}

/// `emit_sse` with the source given as expressions: `s16` the 16 bytes
/// (`[u64; 2]`), `s8` the low 8 bytes (`u64`); `src_reg` when the source is
/// a register.
fn emit_sse_with(op: SseOp, d: usize, src_reg: Option<u8>, s16: &str, s8: &str, imm: u8) -> String {
    use SseOp::*;
    let fd = |x: &str| format!("f64::from_bits({x})");
    let bin = |o: &str, a: &str, b: &str| -> String {
        match o {
            "min" => format!("{{ let (p, q) = ({}, {}); if p < q {{ p }} else {{ q }} }}", fd(a), fd(b)),
            "max" => format!("{{ let (p, q) = ({}, {}); if p > q {{ p }} else {{ q }} }}", fd(a), fd(b)),
            _ => format!("({} {o} {})", fd(a), fd(b)),
        }
    };
    let pd = |o: &str| format!("{{ let s: [u64; 2] = {s16}; let a = m.xmm[{d}]; m.xmm[{d}] = [{}.to_bits(), {}.to_bits()]; }}", bin(o, "a[0]", "s[0]"), bin(o, "a[1]", "s[1]"));
    let sd = |o: &str| format!("{{ let s: u64 = {s8}; let a = m.xmm[{d}][0]; m.xmm[{d}][0] = {}.to_bits(); }}", bin(o, "a", "s"));
    let bits = |o: &str| format!("{{ let s: [u64; 2] = {s16}; let a = m.xmm[{d}]; m.xmm[{d}] = [{}, {}]; }}",
        o.replace("X", "a[0]").replace("Y", "s[0]"), o.replace("X", "a[1]").replace("Y", "s[1]"));
    match op {
        // the flags as in `Machine::sse`
        UComiSd | ComiSd => format!(
            "{{ let b = f64::from_bits({s8}); let a = f64::from_bits(m.xmm[{d}][0]); m.fl.of = false; m.fl.sf = false; m.fl.af = false; \
             let un = a.is_nan() || b.is_nan(); m.fl.zf = un || a == b; m.fl.pf = un; m.fl.cf = un || a < b; }}"
        ),
        Pshufd => format!(
            "{{ let s: [u64; 2] = {s16}; let dw = |i: u8| -> u64 {{ (s[(i >> 1) as usize] >> ((i & 1) * 32)) & 0xFFFF_FFFF }}; \
             m.xmm[{d}] = [dw({}) | dw({}) << 32, dw({}) | dw({}) << 32]; }}",
            imm & 3,
            (imm >> 2) & 3,
            (imm >> 4) & 3,
            (imm >> 6) & 3
        ),
        MovU => format!("m.xmm[{d}] = {s16};"),
        MovSdLoad => match src_reg {
            Some(r) => format!("m.xmm[{d}][0] = m.xmm[{r}][0];"),
            None => format!("{{ let s: u64 = {s8}; m.xmm[{d}] = [s, 0]; }}"),
        },
        MovqLoad => format!("{{ let s: u64 = {s8}; m.xmm[{d}] = [s, 0]; }}"),
        MovLpdLoad => format!("{{ let s: u64 = {s8}; m.xmm[{d}][0] = s; }}"),
        MovHpdLoad => format!("{{ let s: u64 = {s8}; m.xmm[{d}][1] = s; }}"),
        MovHlps => format!("{{ let s: [u64; 2] = {s16}; m.xmm[{d}][0] = s[1]; }}"),
        MovLhps => format!("{{ let s: [u64; 2] = {s16}; m.xmm[{d}][1] = s[0]; }}"),
        AddPd => pd("+"),
        SubPd => pd("-"),
        MulPd => pd("*"),
        DivPd => pd("/"),
        MinPd => pd("min"),
        MaxPd => pd("max"),
        SqrtPd => format!("{{ let s: [u64; 2] = {s16}; m.xmm[{d}] = [{}.sqrt().to_bits(), {}.sqrt().to_bits()]; }}", fd("s[0]"), fd("s[1]")),
        AddSd => sd("+"),
        SubSd => sd("-"),
        MulSd => sd("*"),
        DivSd => sd("/"),
        MinSd => sd("min"),
        MaxSd => sd("max"),
        SqrtSd => format!("{{ let s: u64 = {s8}; m.xmm[{d}][0] = {}.sqrt().to_bits(); }}", fd("s")),
        And => bits("X & Y"),
        AndN => bits("!X & Y"),
        Or => bits("X | Y"),
        Xor => bits("X ^ Y"),
        UnpckLpd => format!("{{ let s: [u64; 2] = {s16}; m.xmm[{d}][1] = s[0]; }}"),
        UnpckHpd => format!("{{ let s: [u64; 2] = {s16}; let a = m.xmm[{d}]; m.xmm[{d}] = [a[1], s[1]]; }}"),
        ShufPd => format!("{{ let s: [u64; 2] = {s16}; let a = m.xmm[{d}]; m.xmm[{d}] = [a[{}], s[{}]]; }}", imm & 1, (imm >> 1) & 1),
        // as Machine::sse: rounded to nearest even, the integer indefinite
        // outside the int32 range
        CvtPd2Dq => format!(
            "{{ let s: [u64; 2] = {s16}; let c = |v: f64| -> u64 {{ let r = v.round_ties_even(); (if r.is_nan() || !(-2147483648.0..=2147483647.0).contains(&r) {{ 0x8000_0000u32 }} else {{ r as i32 as u32 }}) as u64 }}; m.xmm[{d}] = [c(f64::from_bits(s[0])) | (c(f64::from_bits(s[1])) << 32), 0]; }}"
        ),
        CvtDq2Pd => format!("{{ let s: u64 = {s8}; m.xmm[{d}] = [(s as u32 as i32 as f64).to_bits(), ((s >> 32) as u32 as i32 as f64).to_bits()]; }}"),
        HAddPd => format!("{{ let s: [u64; 2] = {s16}; let a = m.xmm[{d}]; m.xmm[{d}] = [{}.to_bits(), {}.to_bits()]; }}", bin("+", "a[0]", "a[1]"), bin("+", "s[0]", "s[1]")),
        _ => unreachable!("sse_native"),
    }
}

fn f64_lit(v: f64) -> String {
    format!("f64::from_bits({:#x})", v.to_bits())
}

impl Prog {
    /// Whether a translated native version is attached.
    pub fn is_native(&self) -> bool {
        self.native.is_some()
    }

    /// Whether the translated version is called directly (see [`DirectFn`]).
    pub fn is_direct(&self) -> bool {
        self.direct.is_some()
    }

    /// Translates the program into a Rust function `name` with the
    /// signature of [`NativeFn`].  Hot micro ops become inline code with
    /// constant slot numbers; all others call `Machine::exec_op` on the
    /// same op of the runtime `Prog` (which must have the same `hash`).
    /// `difs`: the formula uses the dIFS calling convention (for the lifting
    /// analysis, which decides which accesses need no bounds check).
    pub fn emit_rust(&self, name: &str, difs: bool) -> String {
        let safe = self.lift_report(difs).safe_ops().unwrap_or_else(|| vec![false; self.ops.len()]);
        use std::fmt::Write;
        let n = self.ops.len();
        // basic block starts
        let mut start = vec![false; n + 1];
        start[0] = true;
        for (k, op) in self.ops.iter().enumerate() {
            match op {
                Op::Jcc { target, .. }
                | Op::Jmp(target)
                | Op::Call { target, .. }
                | Op::JccRot { target, .. }
                | Op::JmpRot { target, .. }
                | Op::CallRot { target, .. } => {
                    start[*target as usize] = true;
                    start[k + 1] = true;
                }
                Op::Ret(_) => start[k + 1] = true,
                _ => {}
            }
        }
        let mut block_of = vec![0usize; n + 1];
        let mut nb = 0;
        for k in 0..=n {
            if start[k] {
                if k > 0 {
                    nb += 1;
                }
            }
            block_of[k] = nb;
        }
        // possible return targets: ops after calls
        let ret_targets: Vec<usize> =
            self.ops.iter().enumerate().filter(|(_, o)| matches!(o, Op::Call { .. } | Op::CallRot { .. })).map(|(k, _)| k + 1).collect();
        let fop = |op: &FOp, a: &str, b: &str| -> String {
            match op {
                FOp::Add => format!("{a} + {b}"),
                FOp::Mul => format!("{a} * {b}"),
                FOp::Sub => format!("{a} - {b}"),
                FOp::SubR => format!("{b} - {a}"),
                FOp::Div => format!("{a} / {b}"),
                FOp::DivR => format!("{b} / {a}"),
            }
        };
        let permute_rotations = !self.ops.iter().any(|op| matches!(op, Op::Call { .. } | Op::CallRot { .. }));
        let mut o = String::new();
        // registers, flags, FPU condition bits and the step counter are
        // locals (the compiler keeps them in registers and drops unused
        // flags); they go back to the machine at the end and around
        // interpreter calls
        let _ = writeln!(o, "pub(super) fn {name}(m: &mut Machine, p: &Prog, max_steps: u64) -> R<()> {{");
        let _ = writeln!(o, "    let mut f = [0f64; 8];
    let mut rg = m.regs;
    let mut fl = m.fl;
    let mut fsw = m.fsw_cc;
    let mut steps = m.steps;
    let limit = steps + max_steps;
    let res = (|| -> R<()> {{
    let mut b: u32 = 0;
    loop {{
        match b {{");
        let mut k = 0;
        while k < n {
            let _ = writeln!(o, "            {} => {{", block_of[k]);
            let mut ended = false;
            loop {
                let op = &self.ops[k];
                let line = match op {
                    Op::MovRM { r, mem } => format!("m.regs[{r}] = m.rd32({})?;", ea_expr(mem)),
                    Op::FLd64 { dst, mem } => format!("f[{}] = m.rdf64({})?;", dst & 7, ea_expr(mem)),
                    Op::FSt64 { src, mem } => format!("{{ let a = {}; m.wrf64(a, f[{}])?; }}", ea_expr(mem), src & 7),
                    Op::FArithM64 { op, dst, mem } => {
                        let d = format!("f[{}]", dst & 7);
                        format!("{{ let v = m.rdf64({})?; {d} = {}; }}", ea_expr(mem), fop(op, &d, "v"))
                    }
                    Op::MovMR { mem, r } => format!("{{ let a = {}; m.wr32(a, m.regs[{r}])?; }}", ea_expr(mem)),
                    Op::MovRR { d, s } => format!("m.regs[{d}] = m.regs[{s}];"),
                    Op::PushR(r) => format!("{{ let v = m.regs[{r}]; m.push(v)?; }}"),
                    Op::PopR(r) => format!("{{ let v = m.pop()?; m.regs[{r}] = v; }}"),
                    Op::Lea { r, mem } => format!("m.regs[{r}] = {};", ea_expr(mem)),
                    Op::FMov { dst, src } => format!("f[{}] = f[{}];", dst & 7, src & 7),
                    Op::FSwap { a, b } => format!("f.swap({}, {});", a & 7, b & 7),
                    Op::FArith { op, dst, a, b } => {
                        format!("f[{}] = {};", dst & 7, fop(op, &format!("f[{}]", a & 7), &format!("f[{}]", b & 7)))
                    }
                    Op::FConst { dst, v } => format!("f[{}] = {};", dst & 7, f64_lit(*v)),
                    Op::FChs(i) => format!("f[{0}] = -f[{0}];", i & 7),
                    Op::FAbs(i) => format!("f[{0}] = f[{0}].abs();", i & 7),
                    Op::FSqrt(i) => format!("f[{0}] = f[{0}].sqrt();", i & 7),
                    Op::JccRot { cc, target, rot } => format!(
                        "if m.cond({cc}) {{ f.rotate_right({rot}); m.steps += 1; if m.steps > limit {{ return Err(EmuError::StepLimit); }} b = {}; continue; }}",
                        block_of[*target as usize]
                    ),
                    Op::JmpRot { target, rot } => {
                        ended = true;
                        format!(
                            "f.rotate_right({rot}); m.steps += 1; if m.steps > limit {{ return Err(EmuError::StepLimit); }} b = {}; continue;",
                            block_of[*target as usize]
                        )
                    }
                    Op::Jcc { cc, target } => format!(
                        "if m.cond({cc}) {{ m.steps += 1; if m.steps > limit {{ return Err(EmuError::StepLimit); }} b = {}; continue; }}",
                        block_of[*target as usize]
                    ),
                    Op::Jmp(t) => {
                        ended = true;
                        format!(
                            "m.steps += 1; if m.steps > limit {{ return Err(EmuError::StepLimit); }} b = {}; continue;",
                            block_of[*t as usize]
                        )
                    }
                    Op::CallRot { target, rot, .. } => {
                        ended = true;
                        format!(
                            "f.rotate_right({rot}); if let Op::CallRot {{ ret, .. }} = &p.ops[{k}] {{ m.push(*ret)?; }} b = {}; continue;",
                            block_of[*target as usize]
                        )
                    }
                    Op::Call { target, .. } => {
                        ended = true;
                        format!(
                            "if let Op::Call {{ ret, .. }} = &p.ops[{k}] {{ m.push(*ret)?; }} b = {}; continue;",
                            block_of[*target as usize]
                        )
                    }
                    Op::Ret(n2) => {
                        ended = true;
                        let mut arms = String::new();
                        for &t in &ret_targets {
                            let _ = write!(arms, "Some({t}) => {{ b = {}; continue; }} ", block_of[t]);
                        }
                        format!(
                            "let r = m.pop()?; m.regs[ESP] = m.regs[ESP].wrapping_add({n2});                              if r != RETURN_SENTINEL {{ match p.rets.iter().find(|e| e.0 == r).map(|e| e.1) {{ {arms}_ => return Err(unsup(r, \"bad return\".into())) }} }}                              m.st = f; return Ok(());"
                        )
                    }
                    Op::FLd { dst, kind, mem } => format!("f[{}] = m.fload(FKind::{kind:?}, {})?;", dst & 7, ea_expr(mem)),
                    Op::FArithM { op, dst, kind, mem } => {
                        let d = format!("f[{}]", dst & 7);
                        format!("{{ let v = m.fload(FKind::{kind:?}, {})?; {d} = {}; }}", ea_expr(mem), fop(op, &d, "v"))
                    }
                    Op::FCom { a, b } => format!("m.fcompare(f[{}], f[{}]);", a & 7, b & 7),
                    Op::FComM { a, kind, mem } => {
                        format!("{{ let v = m.fload(FKind::{kind:?}, {})?; m.fcompare(f[{}], v); }}", ea_expr(mem), a & 7)
                    }
                    Op::FTst(a) => format!("m.fcompare(f[{}], 0.0);", a & 7),
                    Op::Fnstsw { mem: None, depth } => format!(
                        "m.regs[EAX] = (m.regs[EAX] & 0xFFFF_0000) | (m.fsw_cc | {:#x}) as u32;",
                        ((8 - *depth as u16) & 7) << 11
                    ),
                    Op::Gen(ins, _, _) => match ins.as_ref() {
                        Ins::Nop => String::new(),
                        Ins::Sahf => "{ let ah = (m.regs[EAX] >> 8) as u8; m.fl.sf = ah & 0x80 != 0; m.fl.zf = ah & 0x40 != 0; \
                                      m.fl.af = ah & 0x10 != 0; m.fl.pf = ah & 0x04 != 0; m.fl.cf = ah & 0x01 != 0; }"
                            .to_string(),
                        Ins::Alu { op, dst, src, size } if !matches!(op, Alu::Adc | Alu::Sbb) => {
                            let b = src_expr(src, *size);
                            let calc = format!("{{ let bv = {b}; let av = {}; let r = m.alu(Alu::{op:?}, av, bv, {size});", rm_read(dst, *size));
                            match op {
                                Alu::Cmp => format!("{calc} }}"),
                                _ => format!("{calc} {} }}", rm_write(dst, *size, "r")),
                            }
                        }
                        Ins::Test { a, b, size } => {
                            format!("{{ let bv = {}; let av = {}; m.alu(Alu::And, av, bv, {size}); }}", src_expr(b, *size), rm_read(a, *size))
                        }
                        Ins::Shift { op: sop @ (Shift::Shl | Shift::Shr | Shift::Sar), dst: Rm::Reg(r), cnt: Some(c), size } => {
                            format!("m.shift_reg_const(Shift::{sop:?}, {r}, {c}, {size});")
                        }
                        Ins::Inc(rm, 4) | Ins::Dec(rm, 4) => {
                            let aop = if matches!(ins.as_ref(), Ins::Inc(..)) { "Add" } else { "Sub" };
                            match rm {
                                Rm::Reg(r) => format!(
                                    "{{ let cf = m.fl.cf; let r = m.alu(Alu::{aop}, m.regs[{r}], 1, 4); m.fl.cf = cf; m.regs[{r}] = r; }}"
                                ),
                                Rm::Mem(mm) => format!(
                                    "{{ let a = {}; let v = m.rd32(a)?; let cf = m.fl.cf; let r = m.alu(Alu::{aop}, v, 1, 4); m.fl.cf = cf; m.wr32(a, r)?; }}",
                                    ea_expr(mm)
                                ),
                            }
                        }
                        Ins::Mov { dst, src, size } => {
                            format!("{{ let v = {}; {} }}", src_expr(src, *size), rm_write(dst, *size, "v"))
                        }
                        Ins::Sse { op, dst, src, imm } if sse_native(*op, src) => emit_sse(*op, *dst as usize, src, *imm),
                        Ins::SseStore { op, dst, src } if matches!(op, SseOp::MovStore | SseOp::MovSdStore | SseOp::MovLpdStore | SseOp::MovqStore | SseOp::MovHpdStore) => {
                            let a = ea_expr(dst);
                            match op {
                                SseOp::MovStore => format!("{{ let a = {a}; let x = m.xmm[{src}]; m.wr64(a, x[0])?; m.wr64(a.wrapping_add(8), x[1])?; }}"),
                                SseOp::MovHpdStore => format!("{{ let a = {a}; let x = m.xmm[{src}][1]; m.wr64(a, x)?; }}"),
                                _ => format!("{{ let a = {a}; let x = m.xmm[{src}][0]; m.wr64(a, x)?; }}"),
                            }
                        }
                        _ => format!(
                            "if let Op::Gen(ins, nx, at) = &p.ops[{k}] {{ m.exec(ins, *nx, *at)?; }} // {}",
                            format!("{ins:?}").replace('\n', " ")
                        ),
                    },
                    Op::FSt { src, kind, mem } => format!("{{ let a = {}; m.fstore(FKind::{kind:?}, a, f[{}])?; }}", ea_expr(mem), src & 7),
                    Op::FSin(i) => format!("f[{0}] = f[{0}].sin(); m.fsw_cc &= !0x0400;", i & 7),
                    Op::FCos(i) => format!("f[{0}] = f[{0}].cos(); m.fsw_cc &= !0x0400;", i & 7),
                    Op::FRndint(i) => format!("f[{0}] = m.round_int(f[{0}]);", i & 7),
                    Op::F2xm1(i) => format!("f[{0}] = f[{0}].exp2() - 1.0;", i & 7),
                    Op::FSincos { src, dst } => format!(
                        "{{ let (s, c) = f[{0}].sin_cos(); f[{0}] = s; f[{1}] = c; m.fsw_cc &= !0x0400; }}",
                        src & 7,
                        dst & 7
                    ),
                    Op::FPatan { x, y } => format!("f[{0}] = f[{0}].atan2(f[{1}]);", y & 7, x & 7),
                    Op::FYl2x { x, y } => format!("f[{0}] *= f[{1}].log2();", y & 7, x & 7),
                    Op::FScale { a, s } => format!("f[{0}] *= f[{1}].trunc().exp2();", a & 7, s & 7),
                    _ => format!("m.exec_op(&p.ops[{k}], &mut f)?; // {}", format!("{op:?}").replace('\n', " ")),
                };
                let mut l = localize(&line, permute_rotations);
                if safe.get(k).copied().unwrap_or(false) {
                    l = uncheck(&l);
                }
                let _ = writeln!(o, "                {l}");
                k += 1;
                if ended || k >= n || start[k] {
                    break;
                }
            }
            if !ended {
                if k < n {
                    let _ = writeln!(o, "                b = {};", block_of[k]);
                } else {
                    let _ = writeln!(o, "                return Err(unsup(0, \"ran past the end\".into()));");
                }
            }
            let _ = writeln!(o, "            }}");
        }
        let _ = writeln!(o, "            _ => return Err(unsup(0, \"bad block\".into())),
        }}
    }}
    }})();
    // after an error the machine state does not matter (the caller drops
    // the result, the next call starts with fresh registers); writing it
    // back only on success keeps the locals dead on the error paths
    if res.is_ok() {{
        m.regs = rg;
        m.fl = fl;
        m.fsw_cc = fsw;
        m.steps = steps;
    }}
    res
}}");
        o
    }

    pub fn len(&self) -> usize {
        self.ops.len()
    }
    pub fn is_empty(&self) -> bool {
        self.ops.is_empty()
    }
    pub fn has(&self, addr: u32) -> bool {
        self.index.contains_key(&addr)
    }
}

#[cfg(test)]
mod atan2_bracket_tests {
    use super::{atan2_bracket, ATAN2_EPS};

    #[test]
    fn contains_atan2() {
        let mut seed = 0x2545_F491_4F6C_DD1Du64;
        let mut next = || {
            seed ^= seed << 13;
            seed ^= seed >> 7;
            seed ^= seed << 17;
            seed
        };
        let mut worst = 0.0f64;
        for k in 0..2_000_000 {
            // mixed scales, signs and the octant borders
            let (y, x) = match k % 4 {
                0 => ((next() as i64 as f64) / 9.2e18, (next() as i64 as f64) / 9.2e18),
                1 => (f64::from_bits(next() >> 2) * if next() & 1 == 0 { 1.0 } else { -1.0 }, f64::from_bits(next() >> 2) * if next() & 1 == 0 { 1.0 } else { -1.0 }),
                2 => {
                    let v = (next() as i64 as f64) / 9.2e18;
                    (v, v * (1.0 + (next() as i64 as f64) / 9.2e30))
                }
                _ => ((next() % 2001) as f64 - 1000.0, (next() % 2001) as f64 - 1000.0),
            };
            let e = y.atan2(x);
            let (lo, hi) = atan2_bracket(y, x);
            assert!(lo <= e && e <= hi || (e.is_nan() && lo.is_nan()), "y {y} x {x}: {e} not in [{lo}, {hi}]");
            if lo < hi {
                worst = worst.max((e - (lo + hi) * 0.5).abs());
            }
        }
        assert!(worst < ATAN2_EPS * 0.5, "approximation error {worst}");
        for (y, x) in [(0.0, 1.0), (-0.0, -1.0), (1.0, 0.0), (0.0, 0.0), (f64::INFINITY, 1.0), (f64::NAN, 1.0), (1.0, f64::NEG_INFINITY)] {
            let (lo, hi) = atan2_bracket(y, x);
            let e: f64 = y.atan2(x);
            assert!(lo.to_bits() == e.to_bits() && hi.to_bits() == e.to_bits() || e.is_nan() && lo.is_nan());
        }
    }
}
