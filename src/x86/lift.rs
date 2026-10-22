//! Lifting analysis of compiled formula programs (stage 1 of the GPU work):
//! which memory every access of a formula touches.
//!
//! The formula code is hand written x86 with a fixed calling convention
//! (eax, edx, ecx point to x, y, z; [esp+4] = PIteration3D, [esp+8] = @w;
//! the constants lie below `PVar`, read from the iteration record).  An
//! abstract interpretation over the micro-ops follows the integer
//! registers and the 32-bit stack cells as known addresses (`K`), `PVar`
//! relative addresses (`Pv`), plain data (`D`) or unknown values (`Top`),
//! and classifies every memory operand.  A formula whose accesses all
//! resolve can be lifted: its memory becomes named values (iteration
//! fields, constants, stack temporaries) instead of the emulated memory.

use super::*;
use crate::custom::{off, IT_BASE, IT_C1, STACK_TOP};
use std::collections::BTreeMap;

/// Abstract value of a 32-bit register or stack cell.
#[derive(Clone, Copy, Debug, PartialEq)]
enum AVal {
    /// a known constant (addresses of the iteration record and the stack,
    /// return addresses, immediate values)
    K(u32),
    /// one of a few known constants (the return addresses of a subroutine
    /// called from several places); sorted, unused entries 0
    Ks([u32; 4]),
    /// `PVar + offset` (the variable buffer of the formula's slot)
    Pv(i32),
    /// data that is not used as an address
    D,
    /// unknown (a join of different values)
    Top,
    /// a known base (`K` or `Pv` as the bool says: true = Pv) plus a
    /// computed index: the address of an element of an array
    Dyn(bool, i32),
    /// data in 0..=mask (after `and reg, mask`): a small index
    Bnd(u32),
    /// data i << shift with i in 0..=mask (a small index scaled by a shift);
    /// bit 7 of the shift byte: negated, -(i << shift)
    BndS(u32, u8),
    /// a known address plus a small index times a scale: one of a few
    /// record fields chosen at run time (base, scale, mask)
    Idx(u32, u8, u8),
}

impl AVal {
    fn consts(self) -> Option<Vec<u32>> {
        match self {
            AVal::K(x) => Some(vec![x]),
            AVal::Ks(v) => Some(v.iter().copied().filter(|x| *x != 0).collect()),
            _ => None,
        }
    }

    fn join(self, o: AVal) -> AVal {
        if self == o {
            return self;
        }
        if let (Some(mut a), Some(b)) = (self.consts(), o.consts()) {
            a.extend(b);
            a.sort_unstable();
            a.dedup();
            if a.len() <= 4 && !a.contains(&0) {
                let mut v = [0u32; 4];
                v[..a.len()].copy_from_slice(&a);
                return AVal::Ks(v);
            }
            return AVal::D;
        }
        match (self, o) {
            // a pointer that walks over the variable buffer (a loop over the
            // formula's options): somewhere in it, the address at run time
            (AVal::Pv(a), AVal::Pv(b)) | (AVal::Pv(a), AVal::Dyn(true, b)) | (AVal::Dyn(true, a), AVal::Pv(b)) | (AVal::Dyn(true, a), AVal::Dyn(true, b)) => AVal::Dyn(true, a.min(b)),
            (AVal::Bnd(a), AVal::Bnd(b)) => AVal::Bnd(a.max(b)),
            (AVal::BndS(a, s), AVal::BndS(b, t)) if s == t => AVal::BndS(a.max(b), s),
            (AVal::BndS(..), _) | (_, AVal::BndS(..)) => AVal::D,
            (AVal::Bnd(m), AVal::K(k)) | (AVal::K(k), AVal::Bnd(m)) if k <= m => AVal::Bnd(m),
            (AVal::Bnd(_), AVal::D | AVal::K(_) | AVal::Ks(_)) | (AVal::D | AVal::K(_) | AVal::Ks(_), AVal::Bnd(_)) => AVal::D,
            // constants used as data (masks, immediates) on one path
            (AVal::D, AVal::K(_) | AVal::Ks(_)) | (AVal::K(_) | AVal::Ks(_), AVal::D) => AVal::D,
            _ => AVal::Top,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub(super) struct State {
    regs: [AVal; 8],
    /// 32-bit stack cells by address
    stack: BTreeMap<u32, AVal>,
}

impl State {
    fn join(&self, o: &State) -> State {
        let mut regs = self.regs;
        for (r, x) in regs.iter_mut().zip(o.regs) {
            *r = r.join(x);
        }
        let mut stack = BTreeMap::new();
        for (a, v) in &self.stack {
            if let Some(w) = o.stack.get(a) {
                stack.insert(*a, v.join(*w));
            }
        }
        State { regs, stack }
    }
}

/// The map function a host call goes to (`true`: the sphere map), from the
/// address the target is read from.
fn map_call(a: AVal) -> Option<bool> {
    match a {
        AVal::K(x) if x == IT_BASE + off::PMAPFUNC => Some(true),
        AVal::K(x) if x == IT_BASE + off::PMAPFUNC + 4 => Some(false),
        _ => None,
    }
}

/// [`map_call`] by the location of the target.
pub(super) fn map_call_loc(l: &Loc) -> Option<bool> {
    match l {
        Loc::Field(o) if *o == off::PMAPFUNC => Some(true),
        Loc::Field(o) if *o == off::PMAPFUNC + 4 => Some(false),
        _ => None,
    }
}

/// Where a memory operand points.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Loc {
    /// the iteration record (`TIteration3Dext`), offset from J4
    Field(u32),
    /// the variable buffer of the slot, offset from `PVar`
    Var(i32),
    /// the stack, offset from the stack top at the call
    Stack(i32),
    /// an element of the record, the variable buffer or the stack chosen by
    /// a computed index (base offset as for `Field`, `Var`, `Stack`); the
    /// region becomes an indexable array
    DynField(u32),
    DynVar(i32),
    DynStack(i32),
    /// one of a few record fields chosen by a small index at run time:
    /// (base offset, scale, mask), the field at base + index * scale
    FieldIdx(u32, u8, u8),
    /// the same on the stack (offset as for `Stack`), e.g. a component of a
    /// local vector chosen by an option
    StackIdx(i32, u8, u8),
    /// anything else (not liftable)
    Unresolved,
}

/// Result of the analysis of one formula.
#[derive(Debug, Default)]
pub struct LiftReport {
    /// memory accesses by location: (op index, location, write)
    pub accesses: Vec<(usize, Loc, bool)>,
    /// reasons the formula cannot be lifted (empty: liftable)
    pub problems: Vec<String>,
    /// ops reached by the analysis, and (op, call string) states
    pub reached: usize,
    pub contexts: usize,
    pub ops: usize,
}

const STACK_LOW: u32 = BASE + 0x4000;

/// Abstract address of a memory operand.
fn addr_of(s: &State, m: &Mem) -> AVal {
    let base = match m.base {
        Some(b) => s.regs[b as usize],
        None => AVal::K(0),
    };
    if let Some((i, sc)) = m.index {
        return match (base, s.regs[i as usize]) {
            (AVal::K(b), AVal::K(x)) => AVal::K(b.wrapping_add(x.wrapping_mul(sc as u32)).wrapping_add(m.disp as u32)),
            (AVal::K(b), AVal::Bnd(x)) if x <= 15 && sc <= 8 => AVal::Idx(b.wrapping_add(m.disp as u32), sc, x as u8),
            // the small index as the base register, the address as the index
            (AVal::Bnd(x), AVal::K(b)) if x <= 15 && sc == 1 => AVal::Idx(b.wrapping_add(m.disp as u32), 1, x as u8),
            (AVal::BndS(x, shn), AVal::K(b)) if x <= 15 && sc == 1 && shn & 0x80 == 0 && (1u32 << shn) <= 128 => AVal::Idx(b.wrapping_add(m.disp as u32), 1 << shn, x as u8),
            (AVal::K(b), AVal::BndS(x, shn)) if x <= 15 && ((sc as u32) << (shn & 0x7F)) <= 128 => {
                let step = sc << (shn & 0x7F);
                if shn & 0x80 != 0 {
                    // a negated index: the elements below the address,
                    // counted from the lowest (bit 7 of the mask: reversed)
                    AVal::Idx(b.wrapping_add(m.disp as u32).wrapping_sub(x * step as u32), step, x as u8 | 0x80)
                } else {
                    AVal::Idx(b.wrapping_add(m.disp as u32), step, x as u8)
                }
            }
            (AVal::K(b), AVal::D | AVal::Bnd(_)) => AVal::Dyn(false, b.wrapping_add(m.disp as u32) as i32),
            (AVal::Pv(o), AVal::D | AVal::Bnd(_)) => AVal::Dyn(true, o.wrapping_add(m.disp)),
            _ => AVal::Top,
        };
    }
    match base {
        AVal::K(b) => AVal::K(b.wrapping_add(m.disp as u32)),
        AVal::Pv(o) => AVal::Pv(o.wrapping_add(m.disp)),
        AVal::Dyn(true, o) => AVal::Dyn(true, o.wrapping_add(m.disp)),
        _ => AVal::Top,
    }
}


fn classify(a: AVal) -> Loc {
    match a {
        AVal::K(x) if (IT_BASE..IT_BASE + 0x200).contains(&x) => Loc::Field(x - IT_BASE),
        AVal::K(x) if (STACK_LOW..=STACK_TOP).contains(&x) => Loc::Stack(x as i32 - STACK_TOP as i32),
        AVal::Pv(o) if (-(crate::m3f::CONST_OFFSET as i32)..0x400 - crate::m3f::CONST_OFFSET as i32).contains(&o) => Loc::Var(o),
        AVal::Dyn(false, b) if (IT_BASE..IT_BASE + 0x200).contains(&(b as u32)) => Loc::DynField(b as u32 - IT_BASE),
        AVal::Dyn(false, b) if (STACK_LOW..=STACK_TOP).contains(&(b as u32)) => Loc::DynStack(b - STACK_TOP as i32),
        AVal::Dyn(true, o) => Loc::DynVar(o),
        AVal::Idx(b, sc, mk) if (IT_BASE..IT_BASE + 0x200).contains(&b) && b + (mk & 0x7F) as u32 * sc as u32 + 8 <= IT_BASE + 0x200 => Loc::FieldIdx(b - IT_BASE, sc, mk),
        // (as for Stack: the last element may start up to the stack top)
        AVal::Idx(b, sc, mk) if (STACK_LOW..=STACK_TOP).contains(&b) && b + (mk & 0x7F) as u32 * sc as u32 <= STACK_TOP => Loc::StackIdx(b as i32 - STACK_TOP as i32, sc, mk),
        _ => Loc::Unresolved,
    }
}

struct Ctx<'a> {
    p: &'a Prog,
    rep: LiftReport,
    /// the op being analysed (for the accesses of push and pop)
    cur: usize,
}

impl Ctx<'_> {
    fn addr(&self, s: &State, m: &Mem) -> AVal {
        addr_of(s, m)
    }

    fn access(&mut self, k: usize, s: &State, m: &Mem, write: bool) -> AVal {
        let a = self.addr(s, m);
        let loc = classify(a);
        if loc == Loc::Unresolved {
            self.rep.problems.push(format!("op {k}: address {a:?} ({m:?})"));
        }
        self.rep.accesses.push((k, loc, write));
        a
    }

    /// Value of a 32-bit load from `a`.
    fn load(&self, s: &State, a: AVal) -> AVal {
        match a {
            AVal::K(x) if x == IT_BASE + off::PVAR => AVal::Pv(0),
            AVal::K(x) if (STACK_LOW..=STACK_TOP).contains(&x) => s.stack.get(&x).copied().unwrap_or(AVal::Top),
            AVal::K(_) | AVal::Pv(_) | AVal::Idx(..) | AVal::Dyn(true, _) => AVal::D,
            _ => AVal::Top,
        }
    }

    fn store(&self, s: &mut State, a: AVal, v: AVal, bytes: u32) {
        if let AVal::K(x) = a {
            if (STACK_LOW..=STACK_TOP).contains(&x) {
                // overlapping cells become data
                let touched: Vec<u32> = (x.saturating_sub(3)..x + bytes).filter(|c| s.stack.contains_key(c)).collect();
                for c in touched {
                    s.stack.insert(c, AVal::D);
                }
                if bytes == 4 {
                    s.stack.insert(x, v);
                } else {
                    for c in (x..x + bytes).step_by(4) {
                        s.stack.insert(c, AVal::D);
                    }
                }
            }
        }
    }

    fn push(&mut self, s: &mut State, v: AVal) {
        if let AVal::K(sp) = s.regs[ESP] {
            let sp = sp.wrapping_sub(4);
            s.regs[ESP] = AVal::K(sp);
            s.stack.insert(sp, v);
            self.rep.accesses.push((self.cur, classify(AVal::K(sp)), true));
        } else {
            s.regs[ESP] = AVal::Top;
            self.rep.accesses.push((self.cur, Loc::Unresolved, true));
        }
    }

    fn pop(&mut self, s: &mut State) -> AVal {
        if let AVal::K(sp) = s.regs[ESP] {
            let v = s.stack.get(&sp).copied().unwrap_or(AVal::Top);
            s.regs[ESP] = AVal::K(sp.wrapping_add(4));
            self.rep.accesses.push((self.cur, classify(AVal::K(sp)), false));
            v
        } else {
            self.rep.accesses.push((self.cur, Loc::Unresolved, false));
            AVal::Top
        }
    }

    /// Value of a register or memory operand (32 bit); records the access.
    fn read_rm(&mut self, k: usize, s: &State, rm: &Rm, size: u8) -> AVal {
        match rm {
            Rm::Reg(r) if size == 4 => s.regs[*r as usize],
            Rm::Reg(_) => AVal::D,
            Rm::Mem(m) => {
                let a = self.access(k, s, m, false);
                if size == 4 {
                    self.load(s, a)
                } else {
                    AVal::D
                }
            }
        }
    }

    fn write_rm(&mut self, k: usize, s: &mut State, rm: &Rm, size: u8, v: AVal) {
        match rm {
            Rm::Reg(r) if size == 4 => s.regs[*r as usize] = v,
            // a part of a register: data (r = 4..7 are ah..bh for bytes)
            Rm::Reg(r) if size == 1 => s.regs[(*r & 3) as usize] = AVal::D,
            Rm::Reg(r) => s.regs[*r as usize] = AVal::D,
            Rm::Mem(m) => {
                let a = self.access(k, s, m, true);
                self.store(s, a, if size == 4 { v } else { AVal::D }, size as u32);
            }
        }
    }

    /// Transfer function of one op; returns the successor op indices.
    fn step(&mut self, k: usize, s: &mut State) -> Vec<usize> {
        let p = self.p;
        self.cur = k;
        let next = vec![k + 1];
        match &p.ops[k] {
            Op::MovRM { r, mem } => {
                let a = self.access(k, s, mem, false);
                s.regs[*r as usize] = self.load(s, a);
            }
            Op::MovMR { mem, r } => {
                let a = self.access(k, s, mem, true);
                let v = s.regs[*r as usize];
                self.store(s, a, v, 4);
            }
            Op::MovRR { d, s: src } => s.regs[*d as usize] = s.regs[*src as usize],
            Op::PushR(r) => {
                let v = s.regs[*r as usize];
                self.push(s, v);
            }
            Op::PopR(r) => {
                let v = self.pop(s);
                s.regs[*r as usize] = v;
            }
            Op::Lea { r, mem } => s.regs[*r as usize] = self.addr(s, mem),
            Op::FLd64 { mem, .. } | Op::FArithM64 { mem, .. } | Op::FLd { mem, .. } | Op::FArithM { mem, .. } | Op::FComM { mem, .. } => {
                self.access(k, s, mem, false);
            }
            Op::FSt64 { mem, .. } => {
                let a = self.access(k, s, mem, true);
                self.store(s, a, AVal::D, 8);
            }
            Op::FSt { kind, mem, .. } => {
                let a = self.access(k, s, mem, true);
                let n = match kind {
                    FKind::I16 => 2,
                    FKind::F32 | FKind::I32 => 4,
                    FKind::F80 => 10,
                    _ => 8,
                };
                self.store(s, a, AVal::D, n);
            }
            Op::Fnstsw { mem, .. } => match mem {
                Some(m) => {
                    let a = self.access(k, s, m, true);
                    self.store(s, a, AVal::D, 2);
                }
                None => s.regs[EAX] = AVal::D,
            },
            Op::Jcc { target, .. } | Op::JccRot { target, .. } => return vec![k + 1, *target as usize],
            Op::Jmp(t) | Op::JmpRot { target: t, .. } => return vec![*t as usize],
            Op::Call { target, ret } | Op::CallRot { target, ret, .. } => {
                self.push(s, AVal::K(*ret));
                return vec![*target as usize];
            }
            Op::Ret(n) => {
                let r = self.pop(s);
                if let AVal::K(sp) = s.regs[ESP] {
                    s.regs[ESP] = AVal::K(sp.wrapping_add(*n as u32));
                }
                let Some(addrs) = r.consts() else {
                    self.rep.problems.push(format!("op {k}: return address {r:?}"));
                    return vec![];
                };
                let mut succ = Vec::new();
                for a in addrs {
                    if a == RETURN_SENTINEL {
                        continue;
                    }
                    match p.rets.iter().find(|e| e.0 == a) {
                        Some(e) => succ.push(e.1),
                        None => self.rep.problems.push(format!("op {k}: return to {a:#x}")),
                    }
                }
                return succ;
            }
            Op::Gen(ins, _, _) => self.gen(k, s, ins),
            // the record's map functions (PMapFunc, PMapFunc2): eax = @vec,
            // edx = map number, ecx = @result (3 doubles each); eax, ecx and
            // edx are not kept
            Op::CallHost(rm) => match rm {
                Rm::Mem(m) if map_call(self.addr(s, m)).is_some() => {
                    for i in 0..3 {
                        self.access(k, s, &Mem { base: Some(EAX as u8), index: None, disp: 8 * i }, false);
                    }
                    for i in 0..3 {
                        let m = Mem { base: Some(ECX as u8), index: None, disp: 8 * i };
                        let a = self.access(k, s, &m, true);
                        self.store(s, a, AVal::D, 8);
                    }
                    for r in [EAX, ECX, EDX] {
                        s.regs[r] = AVal::Top;
                    }
                }
                _ => self.rep.problems.push(format!("op {k}: host call {rm:?}")),
            },
            _ => {}
        }
        next
    }

    fn gen(&mut self, k: usize, s: &mut State, ins: &Ins) {
        match ins {
            Ins::Alu { op, dst, src, size } => {
                let b = match src {
                    Src::Imm(v) => AVal::K(*v),
                    Src::Rm(rm) => self.read_rm(k, s, rm, *size),
                };
                let a = self.read_rm(k, s, dst, *size);
                if *op == Alu::Cmp {
                    return;
                }
                let r = match (op, a, b, size) {
                    (Alu::Add, AVal::K(x), AVal::K(y), 4) => AVal::K(x.wrapping_add(y)),
                    (Alu::Sub, AVal::K(x), AVal::K(y), 4) => AVal::K(x.wrapping_sub(y)),
                    (Alu::Add, AVal::Pv(o), AVal::K(y), 4) => AVal::Pv(o.wrapping_add(y as i32)),
                    (Alu::Sub, AVal::Pv(o), AVal::K(y), 4) => AVal::Pv(o.wrapping_sub(y as i32)),
                    // (the base of a walking pointer stays: the loop ends)
                    (Alu::Add | Alu::Sub, AVal::Dyn(true, o), AVal::K(_), 4) => AVal::Dyn(true, o),
                    (Alu::And, AVal::K(x), AVal::K(y), 4) => AVal::K(x & y),
                    (Alu::Or, AVal::K(x), AVal::K(y), 4) => AVal::K(x | y),
                    (Alu::Xor, AVal::K(x), AVal::K(y), 4) => AVal::K(x ^ y),
                    // a small mask: an index into a few fields
                    (Alu::And, _, AVal::K(y), 4) if y <= 15 => AVal::Bnd(y),
                    // c - (i << s) with c = mask << s: the same set of values
                    (Alu::Sub, AVal::K(c), AVal::BndS(m, sh), 4) if sh & 0x80 == 0 && c == m << sh => AVal::BndS(m, sh),
                    (Alu::Sub, AVal::K(c), AVal::Bnd(m), 4) if c == m => AVal::Bnd(m),
                    // pointer arithmetic with data: unknown address
                    (_, AVal::Pv(_), _, 4) | (_, AVal::K(_), AVal::D | AVal::Top, 4) if matches!(op, Alu::Add | Alu::Sub) => AVal::Top,
                    _ => AVal::D,
                };
                self.write_rm(k, s, dst, *size, r);
            }
            Ins::Mov { dst, src, size } => {
                let v = match src {
                    Src::Imm(v) => AVal::K(*v),
                    Src::Rm(rm) => self.read_rm(k, s, rm, *size),
                };
                self.write_rm(k, s, dst, *size, v);
            }
            Ins::Lea { reg, mem } => s.regs[*reg as usize] = self.addr(s, mem),
            Ins::Push(src) => {
                let v = match src {
                    Src::Imm(v) => AVal::K(*v),
                    Src::Rm(rm) => self.read_rm(k, s, rm, 4),
                };
                self.push(s, v);
            }
            Ins::Pop(rm) => {
                let v = self.pop(s);
                self.write_rm(k, s, rm, 4, v);
            }
            Ins::Test { a, b, size } => {
                self.read_rm(k, s, a, *size);
                if let Src::Rm(rm) = b {
                    self.read_rm(k, s, rm, *size);
                }
            }
            Ins::Inc(rm, size) | Ins::Dec(rm, size) | Ins::Not(rm, size) | Ins::Neg(rm, size) => {
                let v = self.read_rm(k, s, rm, *size);
                let r = match (ins, v, size) {
                    (Ins::Inc(..), AVal::K(x), 4) => AVal::K(x.wrapping_add(1)),
                    (Ins::Dec(..), AVal::K(x), 4) => AVal::K(x.wrapping_sub(1)),
                    (Ins::Neg(..), AVal::BndS(m, sh), 4) => AVal::BndS(m, sh ^ 0x80),
                    (Ins::Neg(..), AVal::Bnd(m), 4) => AVal::BndS(m, 0x80),
                    _ => AVal::D,
                };
                self.write_rm(k, s, rm, *size, r);
            }
            Ins::Shift { op, dst, cnt, size } => {
                let v = self.read_rm(k, s, dst, *size);
                // a small index shifted left: a few offsets (element size)
                let r = match (op, v, cnt, size) {
                    (Shift::Shl, AVal::Bnd(m), Some(n), 4) if *n <= 4 => AVal::BndS(m, *n),
                    (Shift::Shl, AVal::BndS(m, sh), Some(n), 4) if sh & 0x80 == 0 && sh + *n <= 6 => AVal::BndS(m, sh + *n),
                    _ => AVal::D,
                };
                self.write_rm(k, s, dst, *size, r);
            }
            Ins::Mul(rm, size) | Ins::IMul1(rm, size) | Ins::Div(rm, size) | Ins::IDiv(rm, size) => {
                self.read_rm(k, s, rm, *size);
                s.regs[EAX] = AVal::D;
                s.regs[EDX] = AVal::D;
            }
            Ins::IMul3 { reg, src, .. } | Ins::IMul2 { reg, src } => {
                self.read_rm(k, s, src, 4);
                s.regs[*reg as usize] = AVal::D;
            }
            Ins::Xchg { a, b, size } => {
                let va = self.read_rm(k, s, a, *size);
                let vb = if *size == 4 { s.regs[*b as usize] } else { AVal::D };
                self.write_rm(k, s, a, *size, vb);
                self.write_rm(k, s, &Rm::Reg(*b), *size, va);
            }
            Ins::Movzx { reg, src, size } | Ins::Movsx { reg, src, size } => {
                self.read_rm(k, s, src, *size);
                s.regs[*reg as usize] = AVal::D;
            }
            Ins::Setcc { dst, .. } => self.write_rm(k, s, dst, 1, AVal::D),
            Ins::Cmovcc { reg, src, .. } => {
                let v = self.read_rm(k, s, src, 4);
                s.regs[*reg as usize] = s.regs[*reg as usize].join(v);
            }
            Ins::Leave => {
                s.regs[ESP] = s.regs[EBP];
                let v = self.pop(s);
                s.regs[EBP] = v;
            }
            Ins::Cdq => s.regs[EDX] = AVal::D,
            Ins::Cwde | Ins::Lahf => s.regs[EAX] = AVal::D,
            Ins::Sahf | Ins::Nop | Ins::FClex => {}
            Ins::FnstcwMem(m) | Ins::FnstswMem(m) => {
                let a = self.access(k, s, m, true);
                self.store(s, a, AVal::D, 2);
            }
            Ins::FldcwMem(m) => {
                self.access(k, s, m, false);
            }
            Ins::FLd(_, m) | Ins::FArithMem(_, _, m) | Ins::FComMem { mem: m, .. } => {
                self.access(k, s, m, false);
            }
            Ins::FSt(_, m, _) => {
                let a = self.access(k, s, m, true);
                self.store(s, a, AVal::D, 8);
            }
            Ins::Sse { src, .. } | Ins::Cvtsi2sd { src, .. } | Ins::MovdToX { src, .. } => {
                if let Rm::Mem(m) = src {
                    self.access(k, s, m, false);
                }
            }
            Ins::SseStore { op, dst, .. } => {
                let a = self.access(k, s, dst, true);
                let n = if *op == SseOp::MovStore { 16 } else { 8 };
                self.store(s, a, AVal::D, n);
            }
            Ins::Cvtsd2si { reg, src, .. } => {
                if let Rm::Mem(m) = src {
                    self.access(k, s, m, false);
                }
                s.regs[*reg as usize] = AVal::D;
            }
            Ins::MovdFromX { dst, .. } => self.write_rm(k, s, dst, 4, AVal::D),
            Ins::MovmskPd { reg, .. } => s.regs[*reg as usize] = AVal::D,
            Ins::FnstswAx => s.regs[EAX] = AVal::D,
            other if fpu_effect(other).is_some() => {}
            other => self.rep.problems.push(format!("op {k}: {other:?}")),
        }
    }
}

impl LiftReport {
    /// The ops whose memory accesses all resolve to the iteration record,
    /// the variable buffer or the stack in every call context (their
    /// accesses need no bounds check).  None when the formula writes through
    /// a computed stack index (not modelled by the analysis).
    pub fn safe_ops(&self) -> Option<Vec<bool>> {
        if self.accesses.iter().any(|(_, l, w)| *w && matches!(l, Loc::DynStack(_))) {
            return None;
        }
        let mut safe = vec![false; self.ops];
        let mut bad = vec![false; self.ops];
        for (k, l, _) in &self.accesses {
            match l {
                Loc::Field(_) | Loc::Var(_) | Loc::Stack(_) => safe[*k] = true,
                _ => bad[*k] = true,
            }
        }
        Some(safe.iter().zip(&bad).map(|(s, b)| *s && !b).collect())
    }
}

/// The analysis with its states, for the code generators.
pub(super) struct Analysis {
    /// the abstract state before each op, per call string
    pub states: std::collections::HashMap<(usize, Vec<usize>), State>,
    pub report: LiftReport,
}

impl Analysis {
    /// Where the memory operand `m` of op `k` in call string `cs` points.
    pub fn loc(&self, k: usize, cs: &[usize], m: &Mem) -> Loc {
        match self.states.get(&(k, cs.to_vec())) {
            Some(s) => classify(addr_of(s, m)),
            None => Loc::Unresolved,
        }
    }
    /// The register of an indexed memory operand that holds the small
    /// index (`Bnd` / `BndS`) and its scale: the index register, or the
    /// base when the address is in the index register.
    pub fn idx_reg(&self, k: usize, cs: &[usize], m: &Mem) -> Option<(u8, u8)> {
        let s = self.states.get(&(k, cs.to_vec()))?;
        let small = |r: u8| matches!(s.regs[r as usize], AVal::Bnd(_) | AVal::BndS(..));
        match (m.base, m.index) {
            (_, Some((i, sc))) if small(i) => Some((i, sc)),
            (Some(b), Some((_, 1))) if small(b) => Some((b, 1)),
            _ => None,
        }
    }
    /// The stack location of a push (`push`: the cell below ESP) or pop.
    pub fn stack_loc(&self, k: usize, cs: &[usize], push: bool) -> Loc {
        match self.states.get(&(k, cs.to_vec())).map(|s| s.regs[ESP]) {
            Some(AVal::K(sp)) => classify(AVal::K(if push { sp.wrapping_sub(4) } else { sp })),
            _ => Loc::Unresolved,
        }
    }
}

impl Prog {
    /// The lifting analysis of this formula (see the module documentation);
    /// `difs`: the dIFS calling convention (`doHybridIFS3D`).
    pub fn lift_report(&self, difs: bool) -> LiftReport {
        self.analyse(difs).report
    }

    /// The analysis with its states (see `lift_report`).
    pub(super) fn analyse(&self, difs: bool) -> Analysis {
        let n = self.ops.len();
        let mut init = State { regs: [AVal::D; 8], stack: BTreeMap::new() };
        if difs {
            // esi = the record + 144, edx = esi + 128, edi = PVar,
            // ebx = slot, ecx = remaining count (Machine::call_difs)
            init.regs[ESI] = AVal::K(IT_BASE + 144);
            init.regs[EDX] = AVal::K(IT_BASE + 272);
            init.regs[EDI] = AVal::Pv(0);
            let sp = STACK_TOP - 4;
            init.regs[ESP] = AVal::K(sp);
            init.stack.insert(sp, AVal::K(RETURN_SENTINEL));
        } else {
            let x = IT_C1 - 32;
            init.regs[EAX] = AVal::K(x);
            init.regs[EDX] = AVal::K(x + 8);
            init.regs[ECX] = AVal::K(x + 16);
            // the call: push @w, push PIteration3D, push the return address
            let sp = STACK_TOP - 12;
            init.regs[ESP] = AVal::K(sp);
            init.stack.insert(sp + 8, AVal::K(x + 24));
            init.stack.insert(sp + 4, AVal::K(IT_C1));
            init.stack.insert(sp, AVal::K(RETURN_SENTINEL));
        }
        // context sensitive: a state per (op, call string), so that a local
        // subroutine is analysed separately for every call site (the lifted
        // code inlines it there)
        type Key = (usize, Vec<usize>);
        let mut states: std::collections::HashMap<Key, State> = std::collections::HashMap::new();
        states.insert((0, Vec::new()), init);
        let mut work: Vec<Key> = vec![(0, Vec::new())];
        let mut ctx = Ctx { p: self, rep: LiftReport { ops: n, ..Default::default() }, cur: 0 };
        let mut visits = 0usize;
        // successors of op k in call string cs, after the transfer function
        let succ_of = |ctx: &mut Ctx, k: usize, cs: &Vec<usize>, s: &mut State| -> Vec<Key> {
            match &self.ops[k] {
                Op::Call { ret, .. } | Op::CallRot { ret, .. } => {
                    let succ = ctx.step(k, s);
                    let Some(ri) = self.rets.iter().find(|e| e.0 == *ret).map(|e| e.1) else {
                        ctx.rep.problems.push(format!("op {k}: unknown return point"));
                        return vec![];
                    };
                    if cs.len() >= 6 {
                        ctx.rep.problems.push(format!("op {k}: calls nested too deep"));
                        return vec![];
                    }
                    let mut c2 = cs.clone();
                    c2.push(ri);
                    succ.into_iter().map(|t| (t, c2.clone())).collect()
                }
                Op::Ret(_) => {
                    // the transfer function pops the address; the call string
                    // says where this return goes
                    let mut s2 = s.clone();
                    ctx.step(k, &mut s2);
                    *s = s2;
                    match cs.split_last() {
                        Some((&ri, rest)) => vec![(ri, rest.to_vec())],
                        None => vec![],
                    }
                }
                _ => ctx.step(k, s).into_iter().map(|t| (t, cs.clone())).collect(),
            }
        };
        while let Some((k, cs)) = work.pop() {
            visits += 1;
            if visits > 400_000 {
                ctx.rep.problems.push("no fixed point".into());
                break;
            }
            if k >= n {
                ctx.rep.problems.push(format!("runs past the end at op {k}"));
                continue;
            }
            let mut s = states[&(k, cs.clone())].clone();
            let succ = succ_of(&mut ctx, k, &cs, &mut s);
            for t in succ {
                let new = match states.get(&t) {
                    None => Some(s.clone()),
                    Some(old) => {
                        let j = old.join(&s);
                        if &j == old {
                            None
                        } else {
                            Some(j)
                        }
                    }
                };
                if let Some(ns) = new {
                    states.insert(t.clone(), ns);
                    work.push(t);
                }
            }
        }
        // one final pass over the fixed point for the accesses and problems
        ctx.rep.accesses.clear();
        ctx.rep.problems.retain(|p| p == "no fixed point" || p.starts_with("runs past") || p.contains("nested") || p.contains("unknown return point"));
        let trace = std::env::var_os("LIFT_TRACE").is_some();
        let mut keys: Vec<Key> = states.keys().cloned().collect();
        keys.sort();
        for key in &keys {
            let (k, cs) = key;
            let mut s = states[key].clone();
            if trace {
                let st: Vec<String> = s.stack.iter().map(|(a, v)| format!("{:+}={v:?}", *a as i64 - STACK_TOP as i64)).collect();
                eprintln!("{k:4} {cs:?} {:?} regs {:?} stack [{}]", self.ops[*k], s.regs, st.join(" "));
            }
            succ_of(&mut ctx, *k, cs, &mut s);
        }
        let mut reached: Vec<usize> = keys.iter().map(|k| k.0).collect();
        reached.dedup();
        ctx.rep.reached = reached.len();
        ctx.rep.contexts = keys.len();
        ctx.rep.problems.sort();
        ctx.rep.problems.dedup();
        Analysis { states, report: ctx.rep }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn report(code: &[u8]) -> LiftReport {
        let mut m = Machine::new();
        m.write_bytes(CODE_START, code).unwrap();
        m.predecode(&[CODE_START]);
        m.compile(CODE_START).expect("compiles").lift_report(false)
    }

    #[test]
    fn word_write_clobbers_its_register() {
        // mov esi, IT_BASE; mov si, ax; fld qword [esi]; fstp st0; ret
        let mut code = vec![0xBE];
        code.extend_from_slice(&IT_BASE.to_le_bytes());
        code.extend_from_slice(&[0x66, 0x8B, 0xF0, 0xDD, 0x06, 0xDD, 0xD8, 0xC3]);
        let r = report(&code);
        assert!(r.accesses.iter().any(|(_, l, _)| *l == Loc::Unresolved), "{r:?}");
    }
}
