//! Rust code from the lifting analysis (stage 1 of the GPU work): the
//! memory of a formula becomes local variables.
//!
//! Every 8-byte cell of the iteration record, the variable buffer and the
//! stack that the formula touches is a local `u64` (`r3` = record bytes
//! 24..32, `v..` the variable buffer, `s..` the stack).  They are read from
//! the machine at the start and the written ones go back at the end, so the
//! machine ends in the same state as with the interpreter (checked by
//! `m3fcheck --translated`).  Accesses to parts of a cell are bit
//! operations on the local.  Local subroutines are generated once per call
//! site (the analysis follows call strings), so every return has a fixed
//! target.  The operations are those of `emit_rust`; formulas whose code
//! needs the interpreter, or that index memory with computed values, are
//! not lifted.

use super::lift::{Analysis, Loc};
use super::*;
use crate::custom::{off, IT_BASE, STACK_TOP};
use std::collections::{BTreeSet, HashMap};
use std::fmt::Write;

const STACK_LOW: u32 = BASE + 0x4000;

/// A region and byte offset in it: 'r' record (from IT_BASE), 'v' variable
/// buffer (from its start), 's' stack (from STACK_LOW).
fn region(loc: Loc) -> Option<(char, u32)> {
    match loc {
        Loc::Field(o) => Some(('r', o)),
        Loc::Var(o) => Some(('v', (o + crate::m3f::CONST_OFFSET as i32) as u32)),
        Loc::Stack(o) => Some(('s', (STACK_TOP as i32 + o) as u32 - STACK_LOW)),
        _ => None,
    }
}

/// The first cell and the index of a `FieldIdx` / `StackIdx` location.
pub(super) fn idx_region(loc: Loc) -> Option<((char, u32), u8, u8)> {
    match loc {
        Loc::FieldIdx(o, sc, mk) => Some((('r', o), sc, mk)),
        Loc::StackIdx(o, sc, mk) => Some((('s', (STACK_TOP as i32 + o) as u32 - STACK_LOW), sc, mk)),
        _ => None,
    }
}

fn mask(n: u32) -> String {
    if n >= 8 {
        "u64::MAX".into()
    } else {
        format!("{:#x}u64", (1u64 << (8 * n)) - 1)
    }
}

/// The cells and the code that uses them.
#[derive(Default)]
pub(super) struct Cells {
    used: BTreeSet<(char, u32)>,
    written: BTreeSet<(char, u32)>,
    /// the kind of the access being generated: 'd' double, 's' single,
    /// 'i' integer, 'x' SSE (for the GPU typing)
    kind: char,
    /// every access: (cell, byte offset in the cell, bytes, kind)
    pub(super) log: Vec<((char, u32), u32, u32, char)>,
    /// the op of each access in `log`
    pub(super) log_ops: Vec<usize>,
    /// the op being generated
    cur_op: usize,
}

impl Cells {
    /// The cells the formula writes.
    pub(super) fn written_cells(&self) -> impl Iterator<Item = (char, u32)> + '_ {
        self.written.iter().copied()
    }

    /// Expression (u64) of `n` bytes at `off` in region `p`.
    fn rd(&mut self, (p, off): (char, u32), n: u32) -> String {
        let (c, sh) = (off / 8, off % 8);
        self.log.push(((p, c), sh, n, self.kind));
        self.log_ops.push(self.cur_op);
        self.used.insert((p, c));
        if sh + n <= 8 {
            if sh == 0 && n == 8 {
                format!("{p}{c}")
            } else {
                format!("(({p}{c} >> {}) & {})", 8 * sh, mask(n))
            }
        } else {
            // the rest in the next cell (logged too, for the typing)
            self.log.push(((p, c + 1), 0, sh + n - 8, self.kind));
            self.log_ops.push(self.cur_op);
            self.used.insert((p, c + 1));
            format!("((({p}{c} >> {}) | ({p}{} << {})) & {})", 8 * sh, c + 1, 64 - 8 * sh, mask(n))
        }
    }

    /// Statement writing the low `n` bytes of the u64 expression `v`.
    fn wr(&mut self, (p, off): (char, u32), n: u32, v: &str) -> String {
        let (c, sh) = (off / 8, off % 8);
        self.log.push(((p, c), sh, n, self.kind));
        self.log_ops.push(self.cur_op);
        self.used.insert((p, c));
        self.written.insert((p, c));
        if sh + n <= 8 {
            if sh == 0 && n == 8 {
                format!("{p}{c} = {v};")
            } else {
                let mk = mask(n);
                format!("{p}{c} = ({p}{c} & !({mk} << {s})) | ((({v}) & {mk}) << {s});", s = 8 * sh)
            }
        } else {
            let n0 = 8 - sh;
            let n1 = n - n0;
            self.log.push(((p, c + 1), 0, n1, self.kind));
            self.log_ops.push(self.cur_op);
            self.used.insert((p, c + 1));
            self.written.insert((p, c + 1));
            format!(
                "{{ let v: u64 = {v}; {p}{c} = ({p}{c} & !({m0} << {s})) | ((v & {m0}) << {s}); {p}{c1} = ({p}{c1} & !{m1}) | ((v >> {b}) & {m1}); }}",
                m0 = mask(n0),
                s = 8 * sh,
                c1 = c + 1,
                m1 = mask(n1),
                b = 8 * n0
            )
        }
    }
}

/// x87 memory load of `kind` as an f64 expression; None for 80 bit.
fn fload_expr(c: &mut Cells, r: (char, u32), kind: FKind) -> Option<String> {
    c.kind = match kind {
        FKind::F64 => 'd',
        FKind::F32 => 's',
        _ => 'i',
    };
    Some(match kind {
        FKind::F32 => format!("(f32::from_bits({} as u32) as f64)", c.rd(r, 4)),
        FKind::F64 => format!("f64::from_bits({})", c.rd(r, 8)),
        FKind::I16 => format!("({} as u16 as i16 as f64)", c.rd(r, 2)),
        FKind::I32 => format!("({} as u32 as i32 as f64)", c.rd(r, 4)),
        FKind::I64 => format!("({} as i64 as f64)", c.rd(r, 8)),
        FKind::F80 => {
            lift_debug("an 80 bit float in memory");
            return None;
        }
    })
}

/// `LIFT_DEBUG`: says why a formula is not lifted.
pub(super) fn lift_debug(why: &str) {
    if std::env::var_os("LIFT_DEBUG").is_some() {
        eprintln!("    not lifted: {why}");
    }
}

/// x87 memory store of the f64 expression `v` (the conversions of
/// `Machine::fstore`); None for 64 / 80 bit integers.
fn fstore_stmt(c: &mut Cells, r: (char, u32), kind: FKind, v: &str) -> Option<String> {
    c.kind = match kind {
        FKind::F64 => 'd',
        FKind::F32 => 's',
        _ => 'i',
    };
    Some(match kind {
        FKind::F32 => c.wr(r, 4, &format!("(({v}) as f32).to_bits() as u64")),
        FKind::F64 => c.wr(r, 8, &format!("({v}).to_bits()")),
        FKind::I16 => {
            let w = c.wr(r, 2, "i as u64");
            format!("{{ let x = m.round_int({v}); let i = if x.is_nan() || !(-32768.0..=32767.0).contains(&x) {{ 0x8000u16 }} else {{ x as i16 as u16 }}; {w} }}")
        }
        FKind::I32 => {
            let w = c.wr(r, 4, "i as u64");
            format!(
                "{{ let x = m.round_int({v}); let i = if x.is_nan() || !(-2147483648.0..=2147483647.0).contains(&x) {{ 0x8000_0000u32 }} else {{ x as i32 as u32 }}; {w} }}"
            )
        }
        _ => {
            lift_debug(&format!("x87 store of {kind:?}"));
            return None;
        }
    })
}

fn fop(op: &FOp, a: &str, b: &str) -> String {
    match op {
        FOp::Add => format!("{a} + {b}"),
        FOp::Mul => format!("{a} * {b}"),
        FOp::Sub => format!("{a} - {b}"),
        FOp::SubR => format!("{b} - {a}"),
        FOp::Div => format!("{a} / {b}"),
        FOp::DivR => format!("{b} / {a}"),
    }
}

/// `f.rotate_right(n)` as a permutation of the locals.
fn rot(n: u8) -> String {
    let perm: Vec<String> = (0..8).map(|k| format!("f[{}]", (k + 8 - (n as usize) % 8) % 8)).collect();
    format!("f = [{}];", perm.join(", "))
}

/// Register operand of `size` bytes (u32) on the local register file.
fn reg_rd(r: u8, size: u8) -> String {
    if size == 4 {
        format!("rg[{r}]")
    } else {
        format!("reg_get(&rg, {r}, {size})")
    }
}

fn reg_wr(r: u8, size: u8, v: &str) -> String {
    if size == 4 {
        format!("rg[{r}] = {v};")
    } else {
        format!("reg_set(&mut rg, {r}, {size}, {v});")
    }
}

impl Prog {
    /// Rust code of the formula with its memory as local variables, or
    /// None when it cannot be lifted (see the module documentation).
    pub fn emit_lifted(&self, name: &str, difs: bool) -> Option<String> {
        self.emit_lifted_cells(name, difs).map(|x| x.0)
    }

    /// An x87 double access to a record field chosen by a small index
    /// (`Loc::FieldIdx`, e.g. `[esi + eax*8 - 128]` after `and eax, 3`):
    /// a match over the possible cells.  None for other ops.
    fn lifted_idx_op(&self, k: usize, cs: &[usize], an: &Analysis, c: &mut Cells) -> Option<String> {
        let (mem, kind): (&Mem, Option<FKind>) = match &self.ops[k] {
            Op::FLd64 { mem, .. } | Op::FSt64 { mem, .. } | Op::FArithM64 { mem, .. } => (mem, None),
            Op::FLd { mem, kind, .. } | Op::FSt { mem, kind, .. } | Op::FArithM { mem, kind, .. } | Op::FComM { mem, kind, .. } => (mem, Some(*kind)),
            _ => return None,
        };
        if kind.is_some_and(|k| k != FKind::F64) {
            return None;
        }
        let ((p, off), sc, mask) = idx_region(an.loc(k, cs, mem))?;
        let (ireg, _) = mem.index?;
        if sc != 8 || off % 8 != 0 {
            return None;
        }
        c.kind = 'd';
        let sel = format!("(rg[{ireg}] & {mask})");
        let fx = |i: &u8| format!("f[{}]", i & 7);
        let rd = |c: &mut Cells| -> String {
            let arms: Vec<String> = (0..=mask as u32).map(|i| format!("{i} => {}", c.rd((p, off + 8 * i), 8))).collect();
            format!("f64::from_bits(match {sel} {{ {}, _ => 0 }})", arms.join(", "))
        };
        Some(match &self.ops[k] {
            Op::FLd64 { dst, .. } | Op::FLd { dst, .. } => format!("f[{}] = {};", dst & 7, rd(c)),
            Op::FArithM64 { op, dst, .. } | Op::FArithM { op, dst, .. } => {
                let d = fx(dst);
                format!("{{ let v = {}; {d} = {}; }}", rd(c), fop(op, &d, "v"))
            }
            Op::FComM { a, .. } => format!("fsw = fcompare_cc(f[{}], {});", a & 7, rd(c)),
            Op::FSt64 { src, .. } | Op::FSt { src, .. } => {
                let v = format!("{}.to_bits()", fx(src));
                let arms: Vec<String> = (0..=mask as u32).map(|i| format!("{i} => {{ {} }}", c.wr((p, off + 8 * i), 8, &v))).collect();
                format!("match {sel} {{ {}, _ => {{}} }}", arms.join(", "))
            }
            _ => return None,
        })
    }

    /// `emit_lifted` with the cells and their access log.
    pub(super) fn emit_lifted_cells(&self, name: &str, difs: bool) -> Option<(String, Cells)> {
        let an = self.analyse(difs);
        if !an.report.problems.is_empty() {
            lift_debug(&format!("{:?}", an.report.problems));
            return None;
        }
        if let Some(a) = an.report.accesses.iter().find(|(_, l, _)| region(*l).is_none() && idx_region(*l).is_none()) {
            lift_debug(&format!("access outside the known regions: {a:?}"));
            return None;
        }
        let n = self.ops.len();
        // block starts (as in emit_rust)
        let mut start = vec![false; n + 1];
        start[0] = true;
        for (k, op) in self.ops.iter().enumerate() {
            match op {
                Op::Jcc { target, .. } | Op::Jmp(target) | Op::Call { target, .. } | Op::JccRot { target, .. } | Op::JmpRot { target, .. } | Op::CallRot { target, .. } => {
                    start[*target as usize] = true;
                    start[k + 1] = true;
                }
                Op::Ret(_) => start[k + 1] = true,
                _ => {}
            }
        }
        // blocks: (start op, call string)
        let mut keys: Vec<(usize, Vec<usize>)> = an.states.keys().filter(|(k, _)| start[*k]).cloned().collect();
        keys.sort();
        let ids: HashMap<(usize, Vec<usize>), usize> = keys.iter().enumerate().map(|(i, k)| (k.clone(), i)).collect();
        let id = |k: usize, cs: &[usize]| ids.get(&(k, cs.to_vec())).copied();
        let mut cells = Cells::default();
        let mut body = String::new();
        let jump = |t: usize| format!("steps += 1; if steps > limit {{ return Err(EmuError::StepLimit); }} b = {t}; continue;");
        for (bi, (k0, cs)) in keys.iter().enumerate() {
            let _ = writeln!(body, "            {bi} => {{");
            let mut k = *k0;
            loop {
                let (line, ended) = self.lifted_op(k, cs, &an, &mut cells, &id, &jump)?;
                if !line.is_empty() {
                    let _ = writeln!(body, "                {line}");
                }
                k += 1;
                if ended {
                    break;
                }
                if k >= n {
                    let _ = writeln!(body, "                return Err(unsup(0, \"ran past the end\".into()));");
                    break;
                }
                if start[k] {
                    let _ = writeln!(body, "                b = {};", id(k, cs)?);
                    break;
                }
            }
            let _ = writeln!(body, "            }}");
        }
        // the cells: read at the start, the written ones back at the end
        let addr = |p: char, c: u32| -> String {
            match p {
                'r' => format!("{:#x}u32", IT_BASE + 8 * c),
                'v' => format!("vb.wrapping_add({})", 8 * c),
                _ => format!("{:#x}u32", STACK_LOW + 8 * c),
            }
        };
        let mut o = String::new();
        let _ = writeln!(o, "pub(super) fn {name}(m: &mut Machine, p: &Prog, max_steps: u64) -> R<()> {{");
        let _ = writeln!(o, "    let _ = p;");
        let _ = writeln!(o, "    let mut f = [0f64; 8];");
        let _ = writeln!(o, "    let mut rg = m.regs;");
        let _ = writeln!(o, "    let mut fl = m.fl;");
        let _ = writeln!(o, "    let mut fsw = m.fsw_cc;");
        let _ = writeln!(o, "    let mut steps = m.steps;");
        let _ = writeln!(o, "    let limit = steps + max_steps;");
        if cells.used.iter().any(|c| c.0 == 'v') {
            // the variable buffer: dIFS formulas get PVar in edi, the others
            // read it from the record (as the analysis assumes)
            if difs {
                let _ = writeln!(o, "    let vb = rg[EDI].wrapping_sub({});", crate::m3f::CONST_OFFSET);
            } else {
                let _ = writeln!(o, "    let vb = m.rd32u({:#x}u32).wrapping_sub({});", IT_BASE + off::PVAR, crate::m3f::CONST_OFFSET);
            }
        }
        for (p, c) in &cells.used {
            let _ = writeln!(o, "    let mut {p}{c}: u64 = m.rd64u({});", addr(*p, *c));
        }
        let _ = writeln!(o, "    let res = (|| -> R<()> {{");
        let _ = writeln!(o, "    let mut b: usize = {};", id(0, &[])?);
        let _ = writeln!(o, "    loop {{");
        let _ = writeln!(o, "        match b {{");
        o.push_str(&body);
        let _ = writeln!(o, "            _ => return Err(unsup(0, \"bad block\".into())),");
        let _ = writeln!(o, "        }}");
        let _ = writeln!(o, "    }}");
        let _ = writeln!(o, "    }})();");
        let _ = writeln!(o, "    if res.is_ok() {{");
        for (p, c) in &cells.written {
            let _ = writeln!(o, "        m.wr64u({}, {p}{c});", addr(*p, *c));
        }
        let _ = writeln!(o, "        m.regs = rg;");
        let _ = writeln!(o, "        m.fl = fl;");
        let _ = writeln!(o, "        m.fsw_cc = fsw;");
        let _ = writeln!(o, "        m.steps = steps;");
        let _ = writeln!(o, "        m.st = f;");
        let _ = writeln!(o, "    }}");
        let _ = writeln!(o, "    res");
        let _ = writeln!(o, "}}");
        Some((o, cells))
    }

    /// How the cells of the lifted formula map to 32-bit GPU values: None
    /// if the formula cannot be lifted; else (uses SSE, the cells used both
    /// as a whole double and in parts or as integers / singles).
    pub fn gpu_typing(&self, difs: bool) -> Option<(bool, Vec<String>)> {
        let (_, cells) = self.emit_lifted_cells("x", difs)?;
        let sse = cells.log.iter().any(|e| e.3 == 'x');
        type Accesses = Vec<(u32, u32, char)>;
        let mut by_cell: std::collections::BTreeMap<(char, u32), Accesses> = Default::default();
        for (cell, off, n, k) in &cells.log {
            by_cell.entry(*cell).or_default().push((*off, *n, *k));
        }
        let mut mixed = Vec::new();
        for (cell, acc) in by_cell {
            let whole_double = acc.iter().any(|a| a.2 == 'd' && a.0 == 0 && a.1 == 8);
            let other = acc.iter().any(|a| !(a.2 == 'd' && a.0 == 0 && a.1 == 8));
            if whole_double && other {
                if std::env::var_os("GPU_REPORT_OPS").is_some() {
                    for (i, e) in cells.log.iter().enumerate() {
                        if e.0 == cell && !(e.3 == 'd' && e.1 == 0 && e.2 == 8) {
                            eprintln!("    {}{}: op {} {:?}", cell.0, cell.1, cells.log_ops[i], self.ops[cells.log_ops[i]]);
                        }
                    }
                }
                let mut kinds: Vec<String> = acc.iter().filter(|a| !(a.2 == 'd' && a.0 == 0 && a.1 == 8)).map(|a| format!("{}@{}x{}", a.2, a.0, a.1)).collect();
                kinds.sort();
                kinds.dedup();
                mixed.push(format!("{}{}: {}", cell.0, cell.1, kinds.join(",")));
            }
        }
        Some((sse, mixed))
    }

    /// One op in call string `cs`: the code and whether it ends the block;
    /// None when the op cannot be lifted.
    #[allow(clippy::too_many_arguments)]
    fn lifted_op(
        &self,
        k: usize,
        cs: &[usize],
        an: &Analysis,
        c: &mut Cells,
        id: &dyn Fn(usize, &[usize]) -> Option<usize>,
        jump: &dyn Fn(usize) -> String,
    ) -> Option<(String, bool)> {
        let loc = |mm: &Mem| region(an.loc(k, cs, mm));
        let fx = |i: &u8| format!("f[{}]", i & 7);
        c.cur_op = k;
        if let Some(line) = self.lifted_idx_op(k, cs, an, c) {
            return Some((line, false));
        }
        c.kind = match &self.ops[k] {
            Op::FLd64 { .. } | Op::FSt64 { .. } | Op::FArithM64 { .. } | Op::CallHost(_) => 'd',
            // SSE moves and arithmetic access whole doubles; the logic ops
            // and the dword shuffle need the exact bits (masks)
            Op::Gen(ins, _, _) => match ins.as_ref() {
                Ins::Sse { op: SseOp::And | SseOp::AndN | SseOp::Or | SseOp::Xor | SseOp::Pshufd, .. } => 'x',
                Ins::Sse { .. } | Ins::SseStore { .. } => 'd',
                _ => 'i',
            },
            _ => 'i',
        };
        let line = match &self.ops[k] {
            Op::MovRM { r, mem } => format!("rg[{r}] = {} as u32;", c.rd(loc(mem)?, 4)),
            Op::MovMR { mem, r } => c.wr(loc(mem)?, 4, &format!("rg[{r}] as u64")),
            Op::MovRR { d, s } => format!("rg[{d}] = rg[{s}];"),
            Op::Lea { r, mem } => format!("rg[{r}] = {};", ea_expr(mem).replace("m.regs[", "rg[")),
            Op::PushR(r) => {
                let w = c.wr(region(an.stack_loc(k, cs, true))?, 4, &format!("rg[{r}] as u64"));
                format!("{{ rg[ESP] = rg[ESP].wrapping_sub(4); {w} }}")
            }
            Op::PopR(r) => {
                let v = c.rd(region(an.stack_loc(k, cs, false))?, 4);
                format!("{{ rg[{r}] = {v} as u32; rg[ESP] = rg[ESP].wrapping_add(4); }}")
            }
            Op::FLd64 { dst, mem } => format!("f[{}] = f64::from_bits({});", dst & 7, c.rd(loc(mem)?, 8)),
            Op::FSt64 { src, mem } => c.wr(loc(mem)?, 8, &format!("f[{}].to_bits()", src & 7)),
            Op::FArithM64 { op, dst, mem } => {
                let d = fx(dst);
                format!("{{ let v = f64::from_bits({}); {d} = {}; }}", c.rd(loc(mem)?, 8), fop(op, &d, "v"))
            }
            Op::FLd { dst, kind, mem } => format!("f[{}] = {};", dst & 7, fload_expr(c, loc(mem)?, *kind)?),
            Op::FSt { src, kind, mem } => fstore_stmt(c, loc(mem)?, *kind, &fx(src))?,
            Op::FArithM { op, dst, kind, mem } => {
                let d = fx(dst);
                format!("{{ let v = {}; {d} = {}; }}", fload_expr(c, loc(mem)?, *kind)?, fop(op, &d, "v"))
            }
            Op::FMov { dst, src } => format!("f[{}] = f[{}];", dst & 7, src & 7),
            Op::FSwap { a, b } => format!("f.swap({}, {});", a & 7, b & 7),
            Op::FArith { op, dst, a, b } => format!("f[{}] = {};", dst & 7, fop(op, &fx(a), &fx(b))),
            Op::FConst { dst, v } => format!("f[{}] = f64::from_bits({:#x});", dst & 7, v.to_bits()),
            Op::FChs(i) => format!("f[{0}] = -f[{0}];", i & 7),
            Op::FAbs(i) => format!("f[{0}] = f[{0}].abs();", i & 7),
            Op::FSqrt(i) => format!("f[{0}] = f[{0}].sqrt();", i & 7),
            Op::FSin(i) => format!("f[{0}] = f[{0}].sin(); fsw &= !0x0400;", i & 7),
            Op::FCos(i) => format!("f[{0}] = f[{0}].cos(); fsw &= !0x0400;", i & 7),
            Op::FRndint(i) => format!("f[{0}] = m.round_int(f[{0}]);", i & 7),
            // fprem / fprem1 with the quotient bits in C0, C3, C1 (Machine::exec_op)
            Op::FPrem { a, b, ieee } => format!(
                "{{ let (x, y) = (f[{0}], f[{1}]); let q = if {2} {{ (x / y).round_ties_even() }} else {{ (x / y).trunc() }}; f[{0}] = if {2} {{ x - q * y }} else {{ x % y }}; \
                 let qi = q.abs() as u64; fsw = (if qi & 4 != 0 {{ 0x0100 }} else {{ 0 }}) | (if qi & 2 != 0 {{ 0x4000 }} else {{ 0 }}) | (if qi & 1 != 0 {{ 0x0200 }} else {{ 0 }}); }}",
                a & 7,
                b & 7,
                ieee
            ),
            Op::F2xm1(i) => format!("f[{0}] = f[{0}].exp2() - 1.0;", i & 7),
            Op::FSincos { src, dst } => format!("{{ let (s, c) = f[{0}].sin_cos(); f[{0}] = s; f[{1}] = c; fsw &= !0x0400; }}", src & 7, dst & 7),
            Op::FPatan { x, y } => format!("f[{0}] = f[{0}].atan2(f[{1}]);", y & 7, x & 7),
            Op::FYl2x { x, y } => format!("f[{0}] *= f[{1}].log2();", y & 7, x & 7),
            Op::FScale { a, s } => format!("f[{0}] *= f[{1}].trunc().exp2();", a & 7, s & 7),
            Op::FCom { a, b } => format!("fsw = fcompare_cc(f[{}], f[{}]);", a & 7, b & 7),
            Op::FComM { a, kind, mem } => format!("fsw = fcompare_cc(f[{}], {});", a & 7, fload_expr(c, loc(mem)?, *kind)?),
            Op::FTst(a) => format!("fsw = fcompare_cc(f[{}], 0.0);", a & 7),
            Op::Fnstsw { mem: None, depth } => {
                format!("rg[EAX] = (rg[EAX] & 0xFFFF_0000) | (fsw | {:#x}) as u32;", ((8 - *depth as u16) & 7) << 11)
            }
            Op::Fnstsw { mem: Some(mm), depth } => c.wr(loc(mm)?, 2, &format!("(fsw | {:#x}) as u64", ((8 - *depth as u16) & 7) << 11)),
            Op::Jcc { cc, target } => {
                return Some((format!("if fl.cond({cc}) {{ {} }}", jump(id(*target as usize, cs)?)), false));
            }
            Op::JccRot { cc, target, rot: r } => {
                return Some((format!("if fl.cond({cc}) {{ {} {} }}", rot(*r), jump(id(*target as usize, cs)?)), false));
            }
            Op::Jmp(t) => return Some((jump(id(*t as usize, cs)?), true)),
            Op::JmpRot { target, rot: r } => return Some((format!("{} {}", rot(*r), jump(id(*target as usize, cs)?)), true)),
            Op::Call { target, ret } | Op::CallRot { target, ret, .. } => {
                let ri = self.rets.iter().find(|e| e.0 == *ret)?.1;
                let mut c2 = cs.to_vec();
                c2.push(ri);
                // the return address of the slot the code runs in (the
                // runtime program), as the interpreter pushes it
                let w = c.wr(region(an.stack_loc(k, cs, true))?, 4, &format!("(match &p.ops[{k}] {{ Op::Call {{ ret, .. }} | Op::CallRot {{ ret, .. }} => *ret, _ => {ret:#x} }}) as u64"));
                let r = match &self.ops[k] {
                    Op::CallRot { rot: r, .. } => rot(*r),
                    _ => String::new(),
                };
                return Some((format!("{r} rg[ESP] = rg[ESP].wrapping_sub(4); {w} b = {}; continue;", id(*target as usize, &c2)?), true));
            }
            Op::Ret(nb) => {
                // the return address is popped (its cell is read: no effect);
                // the call string says where this return goes
                let esp = format!("rg[ESP] = rg[ESP].wrapping_add({});", 4 + *nb as u32);
                return Some(match cs.split_last() {
                    None => (format!("{esp} return Ok(());"), true),
                    Some((&ri, rest)) => (format!("{esp} b = {}; continue;", id(ri, rest)?), true),
                });
            }
            Op::Gen(ins, _, _) => self.lifted_gen(ins, k, cs, an, c)?,
            // a map function of the record (lift::map_call)
            Op::CallHost(Rm::Mem(m)) => {
                let sphere = super::lift::map_call_loc(&an.loc(k, cs, m))?;
                let arg = |r: usize, i: i32| Mem { base: Some(r as u8), index: None, disp: 8 * i };
                let mut v = Vec::new();
                for i in 0..3 {
                    v.push(format!("f64::from_bits({})", c.rd(loc(&arg(EAX, i))?, 8)));
                }
                let mut w = String::new();
                for i in 0..3 {
                    w.push_str(&c.wr(loc(&arg(ECX, i as i32))?, 8, &format!("r[{i}].to_bits()")));
                }
                format!("{{ let r = crate::custom::map_px({sphere}, rg[EDX] as i32, [{}]); {w} }}", v.join(", "))
            }
            op => {
                lift_debug(&format!("op {k}: {op:?}"));
                return None;
            }
        };
        Some((line, false))
    }

    fn lifted_gen(&self, ins: &Ins, k: usize, cs: &[usize], an: &Analysis, c: &mut Cells) -> Option<String> {
        let loc = |mm: &Mem| region(an.loc(k, cs, mm));
        let mut rd = |c: &mut Cells, rm: &Rm, size: u8| -> Option<String> {
            Some(match rm {
                Rm::Reg(r) => reg_rd(*r, size),
                Rm::Mem(mm) => format!("({} as u32)", c.rd(loc(mm)?, size as u32)),
            })
        };
        let wr = |c: &mut Cells, rm: &Rm, size: u8, v: &str| -> Option<String> {
            Some(match rm {
                Rm::Reg(r) => reg_wr(*r, size, v),
                Rm::Mem(mm) => c.wr(loc(mm)?, size as u32, &format!("({v}) as u64")),
            })
        };
        let src = |c: &mut Cells, s: &Src, size: u8, rd: &mut dyn FnMut(&mut Cells, &Rm, u8) -> Option<String>| -> Option<String> {
            Some(match s {
                Src::Imm(v) => format!("{:#x}u32", super::mask(*v, size)),
                Src::Rm(rm) => rd(c, rm, size)?,
            })
        };
        Some(match ins {
            Ins::Nop => String::new(),
            Ins::Sahf => "{ let ah = (rg[EAX] >> 8) as u8; fl.sf = ah & 0x80 != 0; fl.zf = ah & 0x40 != 0; fl.af = ah & 0x10 != 0; fl.pf = ah & 0x04 != 0; fl.cf = ah & 0x01 != 0; }".into(),
            Ins::Alu { op, dst, src: s, size } if !matches!(op, Alu::Adc | Alu::Sbb) => {
                let b = src(c, s, *size, &mut rd)?;
                let a = rd(c, dst, *size)?;
                let calc = format!("let bv = {b}; let av = {a}; let r = fl.alu(Alu::{op:?}, av, bv, {size});");
                match op {
                    Alu::Cmp => format!("{{ {calc} }}"),
                    _ => format!("{{ {calc} {} }}", wr(c, dst, *size, "r")?),
                }
            }
            Ins::Test { a, b, size } => {
                let bv = src(c, b, *size, &mut rd)?;
                format!("{{ let bv = {bv}; let av = {}; fl.alu(Alu::And, av, bv, {size}); }}", rd(c, a, *size)?)
            }
            Ins::Mov { dst, src: s, size } => {
                let v = src(c, s, *size, &mut rd)?;
                format!("{{ let v = {v}; {} }}", wr(c, dst, *size, "v")?)
            }
            // rcl / rcr by one (the x87 flag tricks of some formulas)
            Ins::Shift { op: Shift::Rcl, dst: Rm::Reg(r), cnt: Some(1), size: 4 } => {
                format!("{{ let x = rg[{r}]; let out = (x >> 31) & 1 == 1; rg[{r}] = (x << 1) | fl.cf as u32; fl.cf = out; }}")
            }
            Ins::Shift { op: Shift::Rcr, dst: Rm::Reg(r), cnt: Some(1), size: 4 } => {
                format!("{{ let x = rg[{r}]; let out = x & 1 == 1; rg[{r}] = (x >> 1) | ((fl.cf as u32) << 31); fl.cf = out; }}")
            }
            Ins::Shift { op: sop @ (Shift::Shl | Shift::Shr | Shift::Sar), dst: Rm::Reg(r), cnt: Some(n), size } => {
                format!("shift_reg_l(&mut rg, &mut fl, Shift::{sop:?}, {r}, {n}, {size});")
            }
            Ins::Inc(rm, 4) | Ins::Dec(rm, 4) => {
                let aop = if matches!(ins, Ins::Inc(..)) { "Add" } else { "Sub" };
                let v = rd(c, rm, 4)?;
                format!("{{ let v = {v}; let cf = fl.cf; let r = fl.alu(Alu::{aop}, v, 1, 4); fl.cf = cf; {} }}", wr(c, rm, 4, "r")?)
            }
            Ins::Sse { op, dst, src: s, imm } if sse_native(*op, s) => {
                let (s16, s8, sr) = match s {
                    Rm::Reg(r) => (format!("m.xmm[{r}]"), format!("m.xmm[{r}][0]"), Some(*r)),
                    Rm::Mem(mm) => {
                        let r = loc(mm)?;
                        let lo = c.rd(r, 8);
                        let hi = c.rd((r.0, r.1 + 8), 8);
                        (format!("[{lo}, {hi}]"), lo, None)
                    }
                };
                emit_sse_with(*op, *dst as usize, sr, &s16, &s8, *imm).replace("m.fl.", "fl.")
            }
            Ins::SseStore { op, dst, src: s } if matches!(op, SseOp::MovStore | SseOp::MovSdStore | SseOp::MovLpdStore | SseOp::MovqStore | SseOp::MovHpdStore) => {
                let r = loc(dst)?;
                match op {
                    SseOp::MovStore => {
                        let a = c.wr(r, 8, &format!("m.xmm[{s}][0]"));
                        let b = c.wr((r.0, r.1 + 8), 8, &format!("m.xmm[{s}][1]"));
                        format!("{a} {b}")
                    }
                    SseOp::MovHpdStore => c.wr(r, 8, &format!("m.xmm[{s}][1]")),
                    _ => c.wr(r, 8, &format!("m.xmm[{s}][0]")),
                }
            }
            other => {
                lift_debug(&format!("op {k}: {other:?}"));
                return None;
            }
        })
    }
}
