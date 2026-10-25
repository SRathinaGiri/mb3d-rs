//! Rust code from the lifting analysis (stage 1 of the GPU work): the
//! memory of a formula becomes local variables.
//!
//! Every 8-byte cell of the iteration record, the variable buffer and the
//! stack that the formula touches is a local `u64` (`r3` = record bytes
//! 24..32, `v..` the variable buffer, `s..` the stack).  They are read from
//! the machine at the start and the written ones go back at the end, so the
//! machine ends in the same state as with the interpreter (checked by
//! `m3fcheck --translated`).  The direct and dIFS versions keep the stack
//! in locals only, as the GPU code does: it starts with what the call
//! pushes and 0 below, and is dead after the return (`m3fcheck` puts
//! garbage there, so a formula reading stack it did not write would show).
//! Accesses to parts of a cell are bit
//! operations on the local.  Local subroutines are generated once per call
//! site (the analysis follows call strings), so every return has a fixed
//! target.  The operations are those of `emit_rust`; formulas whose code
//! needs the interpreter, or that index memory with computed values, are
//! not lifted.
//!
//! The blocks are laid out as structured code (labelled blocks and loops,
//! see `structure`), so that the compiler sees direct branches; only flow
//! that does not nest that way gets the state machine of `emit_rust`
//! (`dispatch`; `M3F2RS_DISPATCH` forces it for comparisons).

use super::lift::{Analysis, Loc};
use super::*;
use crate::custom::{off, IT_BASE, STACK_TOP};
use std::collections::{BTreeSet, HashMap};
use std::fmt::Write;

const STACK_LOW: u32 = BASE + 0x4000;

/// A region and byte offset in it: 'r' record (from IT_BASE), 'v' variable
/// buffer (from its start), 's' stack (from STACK_LOW).
pub(super) fn region(loc: Loc) -> Option<(char, u32)> {
    match loc {
        Loc::Field(o) => Some(('r', o)),
        Loc::Var(o) => Some(('v', (o + crate::m3f::CONST_OFFSET as i32) as u32)),
        Loc::Stack(o) => Some(('s', (STACK_TOP as i32 + o) as u32 - STACK_LOW)),
        _ => None,
    }
}

/// The first cell and the index of a `FieldIdx` / `StackIdx` location:
/// (region, offset), element size, mask, reversed (a negated index
/// register: bit 7 of the mask).
pub(super) fn idx_region(loc: Loc) -> Option<((char, u32), u8, u8)> {
    match loc {
        Loc::FieldIdx(o, sc, mk) => Some((('r', o), sc, mk)),
        Loc::StackIdx(o, sc, mk) => Some((('s', (STACK_TOP as i32 + o) as u32 - STACK_LOW), sc, mk)),
        _ => None,
    }
}

thread_local! {
    /// the element of an indexed location (`FieldIdx` / `StackIdx`) the
    /// op being generated stands for (see `idx_switch`)
    static IDX_AT: std::cell::Cell<Option<(char, u32)>> = const { std::cell::Cell::new(None) };
}

/// [`region`], with an indexed location as the element [`IDX_AT`] says.
pub(super) fn region_at(loc: Loc) -> Option<(char, u32)> {
    match (loc, IDX_AT.with(|c| c.get())) {
        (Loc::FieldIdx(..) | Loc::StackIdx(..), Some(r)) => Some(r),
        _ => region(loc),
    }
}

/// The memory operand of an op that can be generated element by element.
pub(super) fn op_mem(op: &Op) -> Option<Mem> {
    match op {
        Op::MovRM { mem, .. } | Op::MovMR { mem, .. } | Op::FLd64 { mem, .. } | Op::FSt64 { mem, .. } | Op::FArithM64 { mem, .. } => Some(*mem),
        Op::FLd { mem, .. } | Op::FSt { mem, .. } | Op::FArithM { mem, .. } | Op::FComM { mem, .. } => Some(*mem),
        Op::Gen(ins, _, _) => match ins.as_ref() {
            Ins::Sse { src: Rm::Mem(m), .. } | Ins::SseStore { dst: m, .. } => Some(*m),
            Ins::Mov { dst: Rm::Mem(m), .. } | Ins::Mov { src: Src::Rm(Rm::Mem(m)), .. } => Some(*m),
            Ins::Alu { dst: Rm::Mem(m), .. } | Ins::Alu { src: Src::Rm(Rm::Mem(m)), .. } => Some(*m),
            _ => None,
        },
        _ => None,
    }
}

/// An op whose memory operand is an indexed location, as a switch over
/// the elements: `gen` generates the op for the element IDX_AT; `sel` the
/// selector expression of the index register; `case` the arm syntax.
pub(super) fn idx_switch(op: &Op, an: &Analysis, k: usize, cs: &[usize], wgsl: bool, gen: &mut dyn FnMut() -> Option<String>) -> Option<Option<String>> {
    if IDX_AT.with(|c| c.get()).is_some() {
        return None;
    }
    let mem = op_mem(op)?;
    let ((p, off), sc, mk) = idx_region(an.loc(k, cs, &mem))?;
    let (ireg, isc) = an.idx_reg(k, cs, &mem)?;
    let (mask, rev) = (mk & 0x7F, mk & 0x80 != 0);
    if sc < isc || !(sc / isc).is_power_of_two() {
        return Some(None);
    }
    let ish = (sc / isc).trailing_zeros();
    let sel = match (wgsl, rev) {
        (true, false) => format!("((rg[{ireg}] >> {ish}u) & {mask}u)"),
        (true, true) => format!("({mask}u - (((0u - rg[{ireg}]) >> {ish}u) & {mask}u))"),
        (false, false) => format!("((rg[{ireg}] >> {ish}) & {mask})"),
        (false, true) => format!("({mask} - ((0u32.wrapping_sub(rg[{ireg}]) >> {ish}) & {mask}))"),
    };
    let mut arms = String::new();
    for i in 0..=mask as u32 {
        IDX_AT.with(|c| c.set(Some((p, off + sc as u32 * i))));
        let line = gen();
        IDX_AT.with(|c| c.set(None));
        let Some(line) = line else { return Some(None) };
        if wgsl {
            arms.push_str(&format!("case {i}u: {{ {line} }} "));
        } else {
            arms.push_str(&format!("{i} => {{ {line} }} "));
        }
    }
    Some(Some(if wgsl { format!("switch {sel} {{ {arms}default: {{}} }}") } else { format!("match {sel} {{ {arms}_ => {{}} }}") }))
}

fn mask(n: u32) -> String {
    if n >= 8 {
        "u64::MAX".into()
    } else {
        format!("{:#x}u64", (1u64 << (8 * n)) - 1)
    }
}

/// The jumps in the generated code of a block are placeholders until the
/// control flow is laid out: `\x01J7\x02` a jump of the formula to block 7
/// (counted by the interpreter), `\x01G7\x02` a call or return going to block
/// 7, `\x01R\x02` the return of the formula.
pub(super) struct Jump;

impl Jump {
    pub(super) const RETURN: &'static str = "\x01R\x02";

    pub(super) fn mark(t: usize, counted: bool) -> String {
        format!("\x01{}{t}\x02", if counted { 'J' } else { 'G' })
    }

    /// The placeholders in `s`: (kind, target block; 0 for a return).
    pub(super) fn find(s: &str) -> Vec<(char, usize)> {
        s.split('\x01').skip(1).filter_map(|p| p.split_once('\x02')).map(|(m, _)| (m.as_bytes()[0] as char, m[1..].parse().unwrap_or(0))).collect()
    }

    /// `s` with every placeholder replaced by `f(kind, target)`.
    pub(super) fn replace(s: &str, f: &dyn Fn(char, usize) -> String) -> String {
        let mut parts = s.split('\x01');
        let mut o = parts.next().unwrap_or("").to_string();
        for p in parts {
            let (m, rest) = p.split_once('\x02').unwrap_or((p, ""));
            o.push_str(&f(m.as_bytes()[0] as char, m[1..].parse().unwrap_or(0)));
            o.push_str(rest);
        }
        o
    }
}

/// The generated code of one block: its lines and the block it falls
/// through to.
#[derive(Default)]
pub(super) struct Block {
    pub(super) lines: Vec<String>,
    pub(super) fall: Option<usize>,
}

impl Block {
    /// The blocks this one goes to (jumps first, the fall through last).
    fn succs(&self) -> Vec<usize> {
        let mut v: Vec<usize> = self.lines.iter().flat_map(|l| Jump::find(l)).filter(|j| j.0 != 'R').map(|j| j.1).collect();
        v.extend(self.fall);
        v
    }
}

/// The blocks as a state machine: `loop { match b { ... } }`, every jump of
/// the formula counted against the step limit.
fn dispatch(blocks: &[Block]) -> String {
    let resolve = |kind: char, t: usize| match kind {
        'J' => format!("steps += 1; if steps > limit {{ return Err(EmuError::StepLimit); }} b = {t}; continue;"),
        'G' => format!("b = {t}; continue;"),
        _ => "return Ok(());".into(),
    };
    let mut body = String::new();
    for (bi, bl) in blocks.iter().enumerate() {
        let _ = writeln!(body, "            {bi} => {{");
        for l in &bl.lines {
            let _ = writeln!(body, "                {}", Jump::replace(l, &resolve));
        }
        if let Some(t) = bl.fall {
            let _ = writeln!(body, "                b = {t};");
        }
        let _ = writeln!(body, "            }}");
    }
    body
}

/// [`dispatch`] in WGSL: `for (guard) { switch (b) { ... } }`, a jump
/// sets `b` and continues the loop.
pub(super) fn dispatch_wgsl(blocks: &[Block]) -> String {
    let resolve = |kind: char, t: usize| match kind {
        'R' => "return;".to_string(),
        _ => format!("b = {t}u; continue;"),
    };
    let mut body = String::new();
    for (bi, bl) in blocks.iter().enumerate() {
        let _ = writeln!(body, "            case {bi}u: {{");
        for l in &bl.lines {
            let _ = writeln!(body, "                {}", Jump::replace(l, &resolve));
        }
        if let Some(t) = bl.fall {
            let _ = writeln!(body, "                b = {t}u;");
        }
        let _ = writeln!(body, "            }}");
    }
    body
}

/// The unset value of the `brk` and `cont` flags of [`structure_wgsl`].
pub(super) const NO_FLAG: &str = "0xffffffffu";

/// [`structure`] in WGSL (the same layout; kept apart so that the Rust
/// code stays as it is).  WGSL has no labels: a label scope is a one-pass
/// `loop { ... break; }`, a jump out of a scope that is not the innermost
/// one sets `brk` (the target scope) or `cont` (the target loop) and
/// breaks; every scope and loop end checks the flags and breaks on until
/// the target is reached.  The flags hold block ids, `NO_FLAG` when unset
/// (the entry block 0 can be a loop header).  Loops are guarded by `guard`
/// like the dispatcher.  None when the flow does not nest (the dispatcher
/// then).
pub(super) fn structure_wgsl(blocks: &[Block], entry: usize) -> Option<String> {
    let nb = blocks.len();
    let mut order = Vec::with_capacity(nb);
    let mut seen = vec![false; nb];
    let mut stack: Vec<(usize, usize)> = vec![(entry, 0)];
    seen[entry] = true;
    while let Some(&mut (b, ref mut i)) = stack.last_mut() {
        let succs = blocks[b].succs();
        if *i < succs.len() {
            let s = succs[*i];
            *i += 1;
            if s < nb && !seen[s] {
                seen[s] = true;
                stack.push((s, 0));
            }
        } else {
            order.push(b);
            stack.pop();
        }
    }
    order.reverse();
    let mut pos = vec![usize::MAX; nb];
    for (p, &b) in order.iter().enumerate() {
        pos[b] = p;
    }
    let mut lend: Vec<Option<usize>> = vec![None; order.len()];
    for (p, &b) in order.iter().enumerate() {
        for t in blocks[b].succs() {
            let q = *pos.get(t)?;
            if q <= p {
                lend[q] = Some(lend[q].map_or(p, |e: usize| e.max(p)));
            }
        }
    }
    let loops: Vec<(usize, usize)> = lend.iter().enumerate().filter_map(|(h, e)| e.map(|e| (h, e))).collect();
    let ctx = |p: usize| loops.iter().filter(|&&(h, e)| h < p && p <= e).max_by_key(|l| l.0).copied();
    let plain = |p: usize, q: usize, last: bool| last && q == p + 1 && !loops.iter().any(|&(h, e)| e == p && h <= p);
    let last_jump = |b: usize| blocks[b].fall.is_none() && !blocks[b].lines.last().is_some_and(|l| l.starts_with("if "));
    let mut labels = vec![false; order.len()];
    for (p, &b) in order.iter().enumerate() {
        let exits = blocks[b].lines.iter().flat_map(|l| Jump::find(l)).filter(|j| j.0 != 'R').map(|j| (j.1, last_jump(b))).chain(blocks[b].fall.map(|t| (t, true)));
        for (t, last) in exits {
            let q = pos[t];
            if q > p && !plain(p, q, last) {
                if ctx(q).is_some_and(|(h, _)| p < h) {
                    return None;
                }
                labels[q] = true;
            }
        }
    }
    #[derive(PartialEq, Clone, Copy)]
    enum Open {
        Scope(usize),
        Loop(usize),
    }
    let scopes_in = |c: Option<(usize, usize)>| -> Vec<usize> {
        let mut v: Vec<usize> = (0..order.len()).filter(|&q| labels[q] && ctx(q) == c).collect();
        v.sort_by(|a, b| b.cmp(a));
        v
    };
    let step = "guard = guard + 1u; if (guard > 200000u) { return; }";
    // the label scopes nest deeply with many forward jumps; naga's parser
    // has a brace nesting limit (127), the lines nest a few more
    if labels.iter().filter(|l| **l).count() + loops.len() > 48 {
        return None;
    }
    let mut open: Vec<Open> = Vec::new();
    let mut code = String::new();
    let ind = |open: &[Open]| "    ".repeat(1 + open.len());
    // the flag check after a construct closed: `arrive` = the label of a
    // closed scope (the jumps to it end here)
    let check = |open: &[Open], arrive: Option<usize>| -> String {
        let mut c = String::new();
        if let Some(q) = arrive {
            let _ = write!(c, "if (brk == {q}u) {{ brk = {NO_FLAG}; }} else ");
        }
        match open.last() {
            Some(Open::Loop(h)) => {
                let h = order[*h];
                let _ = write!(c, "if (cont == {h}u) {{ cont = {NO_FLAG}; {step} continue; }} else if (brk != {NO_FLAG} || cont != {NO_FLAG}) {{ break; }}");
            }
            Some(Open::Scope(_)) => {
                let _ = write!(c, "if (brk != {NO_FLAG} || cont != {NO_FLAG}) {{ break; }}");
            }
            None => {
                let _ = write!(c, "{{}}");
            }
        }
        c
    };
    for q in scopes_in(None) {
        let _ = writeln!(code, "{}loop {{ // 'b{}", ind(&open), order[q]);
        open.push(Open::Scope(q));
    }
    for (p, &b) in order.iter().enumerate() {
        loop {
            match open.last() {
                Some(Open::Scope(q)) if *q == p => {}
                Some(Open::Loop(h)) if p > 0 && lend[*h] == Some(p - 1) => {}
                _ => break,
            }
            let top = open.pop();
            match top {
                Some(Open::Scope(q)) => {
                    let _ = writeln!(code, "{}break; }} // 'b{}", ind(&open), order[q]);
                    let _ = writeln!(code, "{}{}", ind(&open), check(&open, Some(order[q])));
                }
                _ => {
                    let _ = writeln!(code, "{}}}", ind(&open));
                    let _ = writeln!(code, "{}{}", ind(&open), check(&open, None));
                }
            }
        }
        if open.contains(&Open::Scope(p)) || open.iter().any(|o| matches!(o, Open::Loop(h) if lend[*h].is_some_and(|e| e < p))) {
            return None;
        }
        if let Some(e) = lend[p] {
            let _ = writeln!(code, "{}loop {{ // 'l{b}", ind(&open));
            open.push(Open::Loop(p));
            for q in scopes_in(Some((p, e))) {
                let _ = writeln!(code, "{}loop {{ // 'b{}", ind(&open), order[q]);
                open.push(Open::Scope(q));
            }
        }
        // a jump: plain break / continue when the target construct is the
        // innermost open one, else through the flags
        let open_now = open.clone();
        let resolve = |kind: char, t: usize| -> String {
            if kind == 'R' {
                return "return;".into();
            }
            let q = pos[t];
            if q <= p {
                if open_now.last() == Some(&Open::Loop(q)) {
                    format!("{step} continue;")
                } else {
                    format!("cont = {t}u; break;")
                }
            } else if plain(p, q, kind == 'F' || last_jump(b)) {
                String::new()
            } else if open_now.last() == Some(&Open::Scope(q)) {
                "break;".into()
            } else {
                format!("brk = {t}u; break;")
            }
        };
        let i = ind(&open);
        let _ = writeln!(code, "{i}// block {b}");
        for l in &blocks[b].lines {
            let _ = writeln!(code, "{i}{}", Jump::replace(l, &resolve));
        }
        if let Some(t) = blocks[b].fall {
            let r = resolve('F', t);
            if !r.is_empty() {
                let _ = writeln!(code, "{i}{r}");
            }
        }
    }
    while let Some(Open::Loop(h)) = open.last() {
        if lend[*h] != Some(order.len() - 1) {
            return None;
        }
        open.pop();
        let _ = writeln!(code, "{}}}", ind(&open));
    }
    if !open.is_empty() {
        return None;
    }
    Some(code)
}

/// The calling convention of a lifted function.
#[derive(Clone, Copy, PartialEq)]
enum Conv {
    /// a [`NativeFn`]: everything in the machine
    Machine,
    /// a [`DirectFn`] (normal formulas)
    Direct,
    /// a [`DifsFn`] (dIFS formulas)
    Difs,
}

/// Structured control flow of the blocks (`structure`).
struct Structured {
    code: String,
    /// whether there are loops (guarded by the step counter)
    loops: bool,
}

/// The blocks as structured Rust: laid out in reverse post order, a jump
/// forward leaves a labelled block that ends just before its target
/// (`break 'b7`), a jump backward continues a labelled loop (`continue
/// 'l7`, counted against the step limit); a block that falls through to the
/// next one needs neither.  None when the flow does not nest that way (the
/// dispatcher is used then).
fn structure(blocks: &[Block], entry: usize) -> Option<Structured> {
    let nb = blocks.len();
    // reverse post order; the successor visited last comes right after a
    // block, so the fall through is visited last
    let mut order = Vec::with_capacity(nb);
    let mut seen = vec![false; nb];
    let mut stack: Vec<(usize, usize)> = vec![(entry, 0)];
    seen[entry] = true;
    while let Some(&mut (b, ref mut i)) = stack.last_mut() {
        let succs = blocks[b].succs();
        if *i < succs.len() {
            let s = succs[*i];
            *i += 1;
            if s < nb && !seen[s] {
                seen[s] = true;
                stack.push((s, 0));
            }
        } else {
            order.push(b);
            stack.pop();
        }
    }
    order.reverse();
    let mut pos = vec![usize::MAX; nb];
    for (p, &b) in order.iter().enumerate() {
        pos[b] = p;
    }
    // loops: header position -> last position of the body
    let mut lend: Vec<Option<usize>> = vec![None; order.len()];
    for (p, &b) in order.iter().enumerate() {
        for t in blocks[b].succs() {
            let q = *pos.get(t)?;
            if q <= p {
                lend[q] = Some(lend[q].map_or(p, |e: usize| e.max(p)));
            }
        }
    }
    let loops: Vec<(usize, usize)> = lend.iter().enumerate().filter_map(|(h, e)| e.map(|e| (h, e))).collect();
    // the innermost loop around position p (not the one p is the header of)
    let ctx = |p: usize| loops.iter().filter(|&&(h, e)| h < p && p <= e).max_by_key(|l| l.0).copied();
    // whether the exit of the block at p to the one at q needs no code: the
    // last exit of the block (not a conditional jump followed by the fall
    // through) to the next block in the same loop
    let plain = |p: usize, q: usize, last: bool| last && q == p + 1 && !loops.iter().any(|&(h, e)| e == p && h <= p);
    // the jumps in the lines are conditional if the block falls through (or
    // ends with a conditional jump: one ending the code)
    let last_jump = |b: usize| blocks[b].fall.is_none() && !blocks[b].lines.last().is_some_and(|l| l.starts_with("if "));
    // the targets of forward jumps that need a label, checked to lie in the
    // same loop as the jump (no jumps into a loop)
    let mut labels = vec![false; order.len()];
    for (p, &b) in order.iter().enumerate() {
        let exits = blocks[b].lines.iter().flat_map(|l| Jump::find(l)).filter(|j| j.0 != 'R').map(|j| (j.1, last_jump(b))).chain(blocks[b].fall.map(|t| (t, true)));
        for (t, last) in exits {
            let q = pos[t];
            if q > p && !plain(p, q, last) {
                if ctx(q).is_some_and(|(h, _)| p < h) {
                    return None;
                }
                labels[q] = true;
            }
        }
    }
    // a construct: a label scope ends before position q, a loop after it
    #[derive(PartialEq)]
    enum Open {
        Scope(usize),
        Loop(usize),
    }
    let scopes_in = |c: Option<(usize, usize)>| -> Vec<usize> {
        let mut v: Vec<usize> = (0..order.len()).filter(|&q| labels[q] && ctx(q) == c).collect();
        v.sort_by(|a, b| b.cmp(a));
        v
    };
    let mut open: Vec<Open> = Vec::new();
    let mut code = String::new();
    // indented by the loops only (the label scopes nest deeply)
    let ind = |open: &[Open]| "    ".repeat(2 + open.iter().filter(|o| matches!(o, Open::Loop(_))).count());
    for q in scopes_in(None) {
        let _ = writeln!(code, "{}'b{}: {{", ind(&open), order[q]);
        open.push(Open::Scope(q));
    }
    for (p, &b) in order.iter().enumerate() {
        // close the constructs that end here (they must be innermost)
        loop {
            match open.last() {
                Some(Open::Scope(q)) if *q == p => {}
                Some(Open::Loop(h)) if p > 0 && lend[*h] == Some(p - 1) => {}
                _ => break,
            }
            if let Some(Open::Scope(q)) = open.pop() {
                let _ = writeln!(code, "{}}} // 'b{}", ind(&open), order[q]);
            } else {
                let _ = writeln!(code, "{}}}", ind(&open));
            }
        }
        if open.contains(&Open::Scope(p)) || open.iter().any(|o| matches!(o, Open::Loop(h) if lend[*h].is_some_and(|e| e < p))) {
            return None;
        }
        if let Some(e) = lend[p] {
            let _ = writeln!(code, "{}'l{b}: loop {{", ind(&open));
            open.push(Open::Loop(p));
            for q in scopes_in(Some((p, e))) {
                let _ = writeln!(code, "{}'b{}: {{", ind(&open), order[q]);
                open.push(Open::Scope(q));
            }
        }
        // 'F' the fall through
        let resolve = |kind: char, t: usize| -> String {
            if kind == 'R' {
                return "break 'body;".into();
            }
            let q = pos[t];
            if q <= p {
                format!("steps += 1; if steps > limit {{ return Err(EmuError::StepLimit); }} continue 'l{t};")
            } else if plain(p, q, kind == 'F' || last_jump(b)) {
                String::new()
            } else {
                format!("break 'b{t};")
            }
        };
        let i = ind(&open);
        let _ = writeln!(code, "{i}// block {b}");
        for l in &blocks[b].lines {
            let _ = writeln!(code, "{i}{}", Jump::replace(l, &resolve));
        }
        if let Some(t) = blocks[b].fall {
            let r = resolve('F', t);
            if !r.is_empty() {
                let _ = writeln!(code, "{i}{r}");
            }
        }
    }
    // only loops ending with the last block are left
    while let Some(Open::Loop(h)) = open.last() {
        if lend[*h] != Some(order.len() - 1) {
            return None;
        }
        open.pop();
        let _ = writeln!(code, "{}}}", ind(&open));
    }
    if !open.is_empty() {
        return None;
    }
    Some(Structured { code, loops: !loops.is_empty() })
}

/// Machine accesses of generated code at computed addresses (see
/// `local_stack` in `emit_lifted_mode`).
const DYNAMIC_ACCESS: [&str; 9] = ["m.rd32(", "m.wr32(", "m.rd32u(", "m.wr32u(", "m.rdf64(", "m.rdf64u(", "m.wrf64u(", "m.floadu(", "m.exec_op("];

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

    /// The cells the formula reads or writes.
    #[cfg(feature = "jit")]
    pub(super) fn used_cells(&self) -> impl Iterator<Item = (char, u32)> + '_ {
        self.used.iter().copied()
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

/// A step of an angle's way from `fpatan` to `frndint` (see `atan_round_chain`).
enum ChainStep {
    Neg,
    /// with another register: the tracked value is `fop`'s first operand or not
    Reg { other: u8, op: FOp, first: bool },
    /// with a memory operand (the tracked value is the first operand)
    Mem { op: FOp },
}

/// `fpatan` whose angle is only scaled (negated, multiplied, divided by,
/// added to other values) and then rounded by `frndint` in the same block,
/// with nothing else reading it: the angle matters only through the rounded
/// integer (a symmetry's sector: Apollo2D-IFS, the PolyFold IFS). Returns
/// the `frndint`'s op index and the steps by op index. Every step is
/// monotonic in the angle, so if both ends of `atan2_bracket`'s interval
/// round to the same integer, so does the exact angle.
fn atan_round_chain(ops: &[Op], k0: usize, start: &[bool]) -> Option<(usize, Vec<(usize, ChainStep)>)> {
    let Op::FPatan { y, .. } = ops[k0] else { return None };
    let mut p = y & 7;
    let mut steps = Vec::new();
    let mut k = k0 + 1;
    while k < ops.len() && !start[k] {
        let r = |v: u8| v & 7;
        match &ops[k] {
            Op::FRndint(i) if r(*i) == p => return Some((k, steps)),
            Op::FRndint(_) => {}
            Op::FChs(i) if r(*i) == p => steps.push((k, ChainStep::Neg)),
            Op::FChs(_) | Op::FAbs(_) => {
                if matches!(&ops[k], Op::FAbs(i) if r(*i) == p) {
                    return None;
                }
            }
            Op::FSwap { a, b } => {
                if r(*a) == p {
                    p = r(*b);
                } else if r(*b) == p {
                    p = r(*a);
                }
            }
            Op::FLd { dst, .. } | Op::FLd64 { dst, .. } | Op::FConst { dst, .. } => {
                if r(*dst) == p {
                    return None;
                }
            }
            Op::FMov { dst, src } => {
                if r(*dst) == p || r(*src) == p {
                    return None;
                }
            }
            Op::FArithM64 { op, dst, .. } | Op::FArithM { op, dst, .. } => {
                if r(*dst) == p {
                    // v / angle is not monotonic across 0
                    if *op == FOp::DivR {
                        return None;
                    }
                    steps.push((k, ChainStep::Mem { op: *op }));
                }
            }
            Op::FArith { op, dst, a, b } => {
                let (ta, tb) = (r(*a) == p, r(*b) == p);
                if r(*dst) != p {
                    if ta || tb {
                        return None;
                    }
                } else {
                    if ta == tb {
                        return None; // overwritten, or angle with itself
                    }
                    // the angle must not be the divisor
                    if (*op == FOp::Div && tb) || (*op == FOp::DivR && ta) {
                        return None;
                    }
                    steps.push((k, ChainStep::Reg { other: if ta { r(*b) } else { r(*a) }, op: *op, first: ta }));
                }
            }
            _ => return None,
        }
        k += 1;
    }
    None
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

    /// The direct version of [`Prog::emit_lifted`] (a [`DirectFn`]): the
    /// iteration record cells come from and go to the `Iteration` (see
    /// `rec_get` / `rec_set`), the call needs no set up in the emulated
    /// machine.  Not for dIFS formulas.
    pub fn emit_lifted_direct(&self, name: &str) -> Option<String> {
        self.emit_lifted_mode(name, false, Conv::Direct).map(|x| x.0)
    }

    /// The direct version of [`Prog::emit_lifted`] for a dIFS formula (a
    /// [`DifsFn`]): the registers `Machine::call_difs` sets up come in and go
    /// back in `regs`, the iteration record stays in the machine.
    pub fn emit_lifted_difs(&self, name: &str) -> Option<String> {
        self.emit_lifted_mode(name, true, Conv::Difs).map(|x| x.0)
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
        let ((p, off), sc, mk) = idx_region(an.loc(k, cs, mem))?;
        let (mask, rev) = (mk & 0x7F, mk & 0x80 != 0);
        let (ireg, isc) = an.idx_reg(k, cs, &mem)?;
        // the index register holds i * sc / isc (the cells take any offset)
        if sc < isc || !(sc / isc).is_power_of_two() {
            return None;
        }
        let ish = (sc / isc).trailing_zeros();
        c.kind = 'd';
        let sel = if rev { format!("({mask} - ((0u32.wrapping_sub(rg[{ireg}]) >> {ish}) & {mask}))") } else { format!("((rg[{ireg}] >> {ish}) & {mask})") };
        let fx = |i: &u8| format!("f[{}]", i & 7);
        let rd = |c: &mut Cells| -> String {
            let arms: Vec<String> = (0..=mask as u32).map(|i| format!("{i} => {}", c.rd((p, off + sc as u32 * i), 8))).collect();
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
                let arms: Vec<String> = (0..=mask as u32).map(|i| format!("{i} => {{ {} }}", c.wr((p, off + sc as u32 * i), 8, &v))).collect();
                format!("match {sel} {{ {}, _ => {{}} }}", arms.join(", "))
            }
            _ => return None,
        })
    }

    /// Tracks which x87 registers hold a quantity with few distinct values: a
/// rounded value (`frndint`), an integer load or a constant, combined only
/// with such values or with memory operands (the formula's parameters).
/// `fsincos` of such a register uses `Machine::sin_cos_cached` (exact: keyed
/// by the argument's bits; a wrong guess here costs speed, not results).
fn quantized(op: &Op, q: &mut [bool; 8]) {
    let i = |r: &u8| *r as usize & 7;
    match op {
        Op::FRndint(r) => q[i(r)] = true,
        Op::FConst { dst, .. } => q[i(dst)] = true,
        Op::FLd { dst, kind, .. } => q[i(dst)] = matches!(kind, FKind::I16 | FKind::I32 | FKind::I64),
        Op::FLd64 { dst, .. } => q[i(dst)] = false,
        Op::FMov { dst, src } => q[i(dst)] = q[i(src)],
        Op::FSwap { a, b } => q.swap(i(a), i(b)),
        Op::FArith { dst, a, b, .. } => q[i(dst)] = q[i(a)] && q[i(b)],
        Op::FArithM64 { .. } | Op::FArithM { .. } | Op::FChs(_) | Op::FAbs(_) => {}
        Op::FSqrt(r) | Op::FSin(r) | Op::FCos(r) | Op::F2xm1(r) => q[i(r)] = false,
        Op::FPrem { a, .. } | Op::FScale { a, .. } => q[i(a)] = false,
        Op::FPtan { src, dst } | Op::FSincos { src, dst } => {
            q[i(src)] = false;
            q[i(dst)] = false;
        }
        Op::FPatan { y, .. } | Op::FYl2x { y, .. } => q[i(y)] = false,
        _ => {}
    }
}

/// `emit_lifted` with the cells and their access log.
    pub(super) fn emit_lifted_cells(&self, name: &str, difs: bool) -> Option<(String, Cells)> {
        self.emit_lifted_mode(name, difs, Conv::Machine)
    }

    fn emit_lifted_mode(&self, name: &str, difs: bool, conv: Conv) -> Option<(String, Cells)> {
        let direct = conv == Conv::Direct;
        let an = self.analyse(difs);
        if !an.report.problems.is_empty() {
            lift_debug(&format!("{:?}", an.report.problems));
            return None;
        }
        if let Some(a) = an.report.accesses.iter().find(|(_, l, w)| region(*l).is_none() && idx_region(*l).is_none() && !matches!((*l, *w), (Loc::DynVar(_), false))) {
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
        // the code of every block, its jumps as placeholders (see `Jump`)
        let mut blocks: Vec<Block> = Vec::new();
        let jump = |t: usize, counted: bool| Jump::mark(t, counted);
        for (k0, cs) in keys.iter() {
            let mut bl = Block::default();
            let mut k = *k0;
            // the x87 registers holding a "few values" quantity (see `quantized`)
            let mut q = [false; 8];
            // an fpatan angle on its way to frndint: (fpatan's index, frndint's
            // index, the steps' code, the steps by op index)
            let mut chain: Option<(usize, usize, Vec<String>, Vec<(usize, ChainStep)>)> = None;
            loop {
                let (mut line, ended) = self.lifted_op(k, cs, &an, &mut cells, &id, &jump)?;
                if let Op::FPatan { x, y } = &self.ops[k] {
                    if std::env::var_os("M3F2RS_EXACT_ATAN").is_none() {
                        if let Some((end, steps)) = atan_round_chain(&self.ops, k, &start) {
                            line = format!("let (pa{k}y, pa{k}x) = (f[{0}], f[{1}]); let (pa{k}lo, pa{k}hi) = atan2_bracket(pa{k}y, pa{k}x); f[{0}] = pa{k}lo;", y & 7, x & 7);
                            chain = Some((k, end, Vec::new(), steps));
                        }
                    }
                } else if let Some((k0, end, code, steps)) = &mut chain {
                    if let Some((_, st)) = steps.iter().find(|(i, _)| *i == k) {
                        let c = format!("c{k0}_{k}");
                        match st {
                            ChainStep::Neg => code.push("v = -v;".into()),
                            ChainStep::Reg { other, op, first } => {
                                line = format!("let {c} = f[{other}]; {line}");
                                code.push(format!("v = {};", if *first { fop(op, "v", &c) } else { fop(op, &c, "v") }));
                            }
                            ChainStep::Mem { op } => {
                                // `{ let v = <operand>; f[d] = ...; }` keeps the operand as well
                                let body = line.strip_suffix('}')?;
                                line = format!("let {c} = {body}v }};");
                                code.push(format!("v = {};", fop(op, "v", &c)));
                            }
                        }
                    } else if k == *end {
                        let Op::FRndint(i) = &self.ops[k] else { return None };
                        let ch = code.join(" ");
                        line = format!(
                            "{{ let ch = |mut v: f64| -> f64 {{ {ch} v }}; let r1 = m.round_int(ch(pa{k0}lo)); let r2 = m.round_int(ch(pa{k0}hi)); \
                             f[{0}] = if r1 == r2 && r1.to_bits() == r2.to_bits() {{ r1 }} else {{ m.round_int(ch(pa{k0}y.atan2(pa{k0}x))) }}; }}",
                            i & 7
                        );
                        chain = None;
                    }
                }
                if let Op::FSincos { src, .. } = &self.ops[k] {
                    if q[*src as usize & 7] {
                        let a = format!("f[{}].sin_cos()", src & 7);
                        line = line.replace(&a, &format!("m.sin_cos_cached(f[{}])", src & 7));
                    }
                }
                Self::quantized(&self.ops[k], &mut q);
                if !line.is_empty() {
                    bl.lines.push(line);
                }
                k += 1;
                if ended {
                    break;
                }
                if k >= n {
                    bl.lines.push("return Err(unsup(0, \"ran past the end\".into()));".into());
                    break;
                }
                if start[k] {
                    bl.fall = Some(id(k, cs)?);
                    break;
                }
            }
            blocks.push(bl);
        }
        let entry = id(0, &[])?;
        // structured control flow where it can be found, else the dispatcher
        let structured = if std::env::var_os("M3F2RS_DISPATCH").is_some() { None } else { structure(&blocks, entry) };
        let body = match &structured {
            Some(s) => s.code.clone(),
            None => dispatch(&blocks),
        };
        // the cells: read at the start, the written ones back at the end
        let addr = |p: char, c: u32| -> String {
            match p {
                'r' => format!("{:#x}u32", IT_BASE + 8 * c),
                'v' => format!("vb.wrapping_add({})", 8 * c),
                _ => format!("{:#x}u32", STACK_LOW + 8 * c),
            }
        };
        // the step counter guards the loops (the dispatcher counts every jump)
        let steps = structured.as_ref().is_none_or(|s| s.loops);
        // the variable buffer cells are read from one checked slice (after
        // `vb` is set)
        let vcells: Vec<u32> = cells.used.iter().filter(|c| c.0 == 'v').map(|c| c.1).collect();
        let vslice = |o: &mut String| {
            if let (Some(lo), Some(hi)) = (vcells.first(), vcells.last()) {
                let _ = writeln!(o, "    let vm = m.bytes_at::<{}>(vb.wrapping_add({}))?;", 8 * (hi - lo + 1), 8 * lo);
            }
        };
        let load = |p: char, c: u32| -> String {
            match p {
                'v' => format!("cell(vm, {})", 8 * (c - vcells[0])),
                _ => format!("m.rd64u({})", addr(p, c)),
            }
        };
        // the stack of a direct or dIFS formula in locals only (as on the
        // GPU): the cells start with what the call pushes, the others with
        // 0, and nothing goes back to the machine (callers read nothing
        // below the stack pointer).  Not when the code addresses memory
        // through registers (the stack could be among it).
        let local_stack = conv != Conv::Machine && !DYNAMIC_ACCESS.iter().any(|f| body.contains(f));
        let stack_init = |c: u32| -> String {
            let call_cells: Vec<(u32, u32)> = if difs {
                vec![(STACK_TOP - 4, RETURN_SENTINEL)]
            } else {
                let x = crate::custom::IT_C1 - 32;
                vec![(STACK_TOP - 12, RETURN_SENTINEL), (STACK_TOP - 8, crate::custom::IT_C1), (STACK_TOP - 4, x + 24)]
            };
            let half = |a: u32| call_cells.iter().find(|(ca, _)| *ca == a).map_or(0, |(_, v)| *v) as u64;
            let a = STACK_LOW + 8 * c;
            format!("{:#x}u64", half(a) | half(a + 4) << 32)
        };
        let load = |p: char, c: u32| -> String { if p == 's' && local_stack { stack_init(c) } else { load(p, c) } };
        let mut o = String::new();
        if direct {
            // the state `Machine::call_compiled` sets up, without the machine
            let _ = writeln!(o, "pub(super) fn {name}(m: &mut Machine, p: &Prog, it: &mut Iteration, pvar: u32, max_steps: u64) -> R<()> {{");
            let _ = writeln!(o, "    let _ = p;");
            let _ = writeln!(o, "    let mut f = [0f64; 8];");
            if local_stack {
                let _ = writeln!(o, "    let mut rg = direct_regs_local();");
            } else {
                let _ = writeln!(o, "    let mut rg = direct_regs(m);");
            }
            let _ = writeln!(o, "    let mut fl = Flags::default();");
            let _ = writeln!(o, "    let mut fsw = 0u16;");
            if steps {
                let _ = writeln!(o, "    let mut steps = 0u64;");
                let _ = writeln!(o, "    let limit = max_steps;");
            }
            if cells.used.iter().any(|c| c.0 == 'v') {
                let _ = writeln!(o, "    let vb = pvar.wrapping_sub({});", crate::m3f::CONST_OFFSET);
            }
            vslice(&mut o);
            for (p, c) in &cells.used {
                if *p == 'r' {
                    let _ = writeln!(o, "    let mut r{c}: u64 = rec_get(m, it, pvar, {c});");
                } else {
                    let _ = writeln!(o, "    let mut {p}{c}: u64 = {};", load(*p, *c));
                }
            }
        } else if conv == Conv::Difs {
            // the registers of `Machine::call_difs` (fsw_cc = 0); the flags
            // start cleared as for a direct formula
            let _ = writeln!(o, "pub(super) fn {name}(m: &mut Machine, p: &Prog, regs: &mut [u32; 8], max_steps: u64) -> R<()> {{");
            let _ = writeln!(o, "    let _ = p;");
            let _ = writeln!(o, "    let mut f = [0f64; 8];");
            let _ = writeln!(o, "    let mut rg = *regs;");
            let _ = writeln!(o, "    let mut fl = Flags::default();");
            let _ = writeln!(o, "    let mut fsw = 0u16;");
            if steps {
                let _ = writeln!(o, "    let mut steps = 0u64;");
                let _ = writeln!(o, "    let limit = max_steps;");
            }
            if cells.used.iter().any(|c| c.0 == 'v') {
                let _ = writeln!(o, "    let vb = rg[EDI].wrapping_sub({});", crate::m3f::CONST_OFFSET);
            }
            vslice(&mut o);
            for (p, c) in &cells.used {
                let _ = writeln!(o, "    let mut {p}{c}: u64 = {};", load(*p, *c));
            }
        } else {
            let _ = writeln!(o, "pub(super) fn {name}(m: &mut Machine, p: &Prog, max_steps: u64) -> R<()> {{");
            let _ = writeln!(o, "    let _ = p;");
            let _ = writeln!(o, "    let mut f = [0f64; 8];");
            let _ = writeln!(o, "    let mut rg = m.regs;");
            let _ = writeln!(o, "    let mut fl = m.fl;");
            let _ = writeln!(o, "    let mut fsw = m.fsw_cc;");
            if steps {
                let _ = writeln!(o, "    let mut steps = m.steps;");
                let _ = writeln!(o, "    let limit = steps + max_steps;");
            }
            if cells.used.iter().any(|c| c.0 == 'v') {
                // the variable buffer: dIFS formulas get PVar in edi, the others
                // read it from the record (as the analysis assumes)
                if difs {
                    let _ = writeln!(o, "    let vb = rg[EDI].wrapping_sub({});", crate::m3f::CONST_OFFSET);
                } else {
                    let _ = writeln!(o, "    let vb = m.rd32u({:#x}u32).wrapping_sub({});", IT_BASE + off::PVAR, crate::m3f::CONST_OFFSET);
                }
            }
            vslice(&mut o);
            for (p, c) in &cells.used {
                let _ = writeln!(o, "    let mut {p}{c}: u64 = {};", load(*p, *c));
            }
        }
        // the state goes back to the machine when the formula returns (not
        // after an error)
        let write_back = |o: &mut String, ind: &str| {
            for (p, c) in &cells.written {
                if *p == 's' && local_stack {
                    continue;
                }
                if direct && *p == 'r' {
                    let _ = writeln!(o, "{ind}rec_set(m, it, {c}, r{c});");
                } else {
                    let _ = writeln!(o, "{ind}m.wr64u({}, {p}{c});", addr(*p, *c));
                }
            }
            if conv == Conv::Difs {
                // the caller reads the registers
                let _ = writeln!(o, "{ind}*regs = rg;");
            } else if !direct {
                let _ = writeln!(o, "{ind}m.regs = rg;");
                let _ = writeln!(o, "{ind}m.fl = fl;");
                let _ = writeln!(o, "{ind}m.fsw_cc = fsw;");
                if steps {
                    let _ = writeln!(o, "{ind}m.steps = steps;");
                }
                let _ = writeln!(o, "{ind}m.st = f;");
            }
        };
        if structured.is_some() {
            // errors return at once; a return of the formula leaves 'body
            let _ = writeln!(o, "    'body: {{");
            o.push_str(&body);
            let _ = writeln!(o, "    }}");
            write_back(&mut o, "    ");
            let _ = writeln!(o, "    Ok(())");
            let _ = writeln!(o, "}}");
            return Some((o, cells));
        }
        let _ = writeln!(o, "    let res = (|| -> R<()> {{");
        let _ = writeln!(o, "    let mut b: usize = {entry};");
        let _ = writeln!(o, "    loop {{");
        let _ = writeln!(o, "        match b {{");
        o.push_str(&body);
        let _ = writeln!(o, "            _ => return Err(unsup(0, \"bad block\".into())),");
        let _ = writeln!(o, "        }}");
        let _ = writeln!(o, "    }}");
        let _ = writeln!(o, "    }})();");
        let _ = writeln!(o, "    if res.is_ok() {{");
        write_back(&mut o, "        ");
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
        jump: &dyn Fn(usize, bool) -> String,
    ) -> Option<(String, bool)> {
        let loc = |mm: &Mem| region_at(an.loc(k, cs, mm));
        let fx = |i: &u8| format!("f[{}]", i & 7);
        c.cur_op = k;
        // an indexed memory operand: the op once per element
        if let Some(r) = idx_switch(&self.ops[k], an, k, cs, false, &mut || self.lifted_op(k, cs, an, c, id, jump).map(|x| x.0)) {
            return r.map(|line| (line, false));
        }
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
        // a read through a pointer that walks over the variable buffer
        // (Loc::DynVar): the address at run time
        let dyn_rd = |mm: &Mem, n: u32| -> Option<String> {
            if !matches!(an.loc(k, cs, mm), Loc::DynVar(_)) {
                return None;
            }
            let ea = super::ea_expr(mm).replace("m.regs[", "rg[");
            Some(match n {
                4 => format!("m.rd32({ea})?"),
                _ => format!("m.rdf64({ea})?.to_bits()"),
            })
        };
        if let Some(line) = match &self.ops[k] {
            Op::MovRM { r, mem } => dyn_rd(mem, 4).map(|e| format!("rg[{r}] = {e};")),
            Op::FLd64 { dst, mem } => dyn_rd(mem, 8).map(|e| format!("f[{}] = f64::from_bits({e});", dst & 7)),
            Op::FArithM64 { op, dst, mem } => dyn_rd(mem, 8).map(|e| {
                let d = fx(dst);
                format!("{{ let v = f64::from_bits({e}); {d} = {}; }}", fop(op, &d, "v"))
            }),
            _ => None,
        } {
            return Some((line, false));
        }
        let line = match &self.ops[k] {
            Op::MovRM { r, mem } => format!("rg[{r}] = {} as u32;", c.rd(loc(mem)?, 4)),
            Op::MovMR { mem, r } => c.wr(loc(mem)?, 4, &format!("rg[{r}] as u64")),
            Op::MovRR { d, s } => format!("rg[{d}] = rg[{s}];"),
            Op::Lea { r, mem } => format!("rg[{r}] = {};", ea_expr(mem).replace("m.regs[", "rg[")),
            // push esp stores the old value, pop esp sets the popped one
            Op::PushR(r) if *r as usize == ESP => {
                let w = c.wr(region(an.stack_loc(k, cs, true))?, 4, "v as u64");
                format!("{{ let v = rg[ESP]; rg[ESP] = rg[ESP].wrapping_sub(4); {w} }}")
            }
            Op::PushR(r) => {
                let w = c.wr(region(an.stack_loc(k, cs, true))?, 4, &format!("rg[{r}] as u64"));
                format!("{{ rg[ESP] = rg[ESP].wrapping_sub(4); {w} }}")
            }
            Op::PopR(r) if *r as usize == ESP => format!("rg[ESP] = {} as u32;", c.rd(region(an.stack_loc(k, cs, false))?, 4)),
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
            Op::FPtan { src, dst } => format!("f[{0}] = f[{0}].tan(); f[{1}] = 1.0; fsw &= !0x0400;", src & 7, dst & 7),
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
                return Some((format!("if fl.cond({cc}) {{ {} }}", jump(id(*target as usize, cs)?, true)), false));
            }
            Op::JccRot { cc, target, rot: r } => {
                return Some((format!("if fl.cond({cc}) {{ {} {} }}", rot(*r), jump(id(*target as usize, cs)?, true)), false));
            }
            Op::Jmp(t) => return Some((jump(id(*t as usize, cs)?, true), true)),
            Op::JmpRot { target, rot: r } => return Some((format!("{} {}", rot(*r), jump(id(*target as usize, cs)?, true)), true)),
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
                return Some((format!("{r} rg[ESP] = rg[ESP].wrapping_sub(4); {w} {}", jump(id(*target as usize, &c2)?, false)), true));
            }
            Op::Ret(nb) => {
                // the return address is popped (its cell is read: no effect);
                // the call string says where this return goes
                let esp = format!("rg[ESP] = rg[ESP].wrapping_add({});", 4 + *nb as u32);
                return Some(match cs.split_last() {
                    None => (format!("{esp} {}", Jump::RETURN), true),
                    Some((&ri, rest)) => (format!("{esp} {}", jump(id(ri, rest)?, false)), true),
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
        let loc = |mm: &Mem| region_at(an.loc(k, cs, mm));
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
            Ins::Not(rm, size) => {
                let v = rd(c, rm, *size)?;
                wr(c, rm, *size, &format!("!({v})"))?
            }
            Ins::Neg(rm, size) => {
                let v = rd(c, rm, *size)?;
                let m = super::mask(u32::MAX, *size);
                format!("{{ let v = {v}; let r = fl.alu(Alu::Sub, 0, v, {size}); fl.cf = v & {m:#x} != 0; {} }}", wr(c, rm, *size, "r")?)
            }
            // one-operand mul / imul / div / idiv (Machine::exec; a quotient
            // out of range halts)
            Ins::Mul(rm, size) | Ins::IMul1(rm, size) => {
                let sg = matches!(ins, Ins::IMul1(..));
                let b = rd(c, rm, *size)?;
                match size {
                    1 => format!("{{ let b = {b}; let a = rg[EAX] & 0xFF; let r = if {sg} {{ (a as u8 as i8 as i32 * b as u8 as i8 as i32) as u32 }} else {{ a * b }}; reg_set(&mut rg, 0, 2, r); fl.cf = if {sg} {{ (r as u16 as i16) != (r as u8 as i8 as i16) }} else {{ (r >> 8) & 0xFF != 0 }}; fl.of = fl.cf; }}"),
                    2 => format!("{{ let b = {b}; let a = rg[EAX] & 0xFFFF; let r = if {sg} {{ (a as u16 as i16 as i32 * b as u16 as i16 as i32) as u32 }} else {{ a * b }}; reg_set(&mut rg, 0, 2, r); reg_set(&mut rg, 2, 2, r >> 16); fl.cf = if {sg} {{ (r as i32) != (r as u16 as i16 as i32) }} else {{ (r >> 16) != 0 }}; fl.of = fl.cf; }}"),
                    _ => format!("{{ let b = {b}; let a = rg[EAX]; let r: u64 = if {sg} {{ (a as i32 as i64 * b as i32 as i64) as u64 }} else {{ a as u64 * b as u64 }}; rg[EAX] = r as u32; rg[EDX] = (r >> 32) as u32; fl.cf = if {sg} {{ (r as i64) != (r as u32 as i32 as i64) }} else {{ (r >> 32) != 0 }}; fl.of = fl.cf; }}"),
                }
            }
            Ins::Div(rm, size) | Ins::IDiv(rm, size) => {
                let sg = matches!(ins, Ins::IDiv(..));
                let b = rd(c, rm, *size)?;
                match size {
                    1 => format!("{{ let b = {b}; if b == 0 {{ return Err(EmuError::Halt(0)); }} let n = rg[EAX] & 0xFFFF; let (q, r) = if {sg} {{ let (n, d) = (n as u16 as i16 as i32, b as u8 as i8 as i32); let q = n / d; if q != q as i8 as i32 {{ return Err(EmuError::Halt(0)); }} (q as u32, (n % d) as u32) }} else {{ if n / b > 0xFF {{ return Err(EmuError::Halt(0)); }} (n / b, n % b) }}; reg_set(&mut rg, 0, 1, q); reg_set(&mut rg, 4, 1, r); }}"),
                    2 => format!("{{ let b = {b}; if b == 0 {{ return Err(EmuError::Halt(0)); }} let n = (rg[EDX] & 0xFFFF) << 16 | (rg[EAX] & 0xFFFF); let (q, r) = if {sg} {{ let (n, d) = (n as i32, b as u16 as i16 as i32); match n.checked_div(d) {{ Some(q) if q == q as i16 as i32 => (q as u32, (n % d) as u32), _ => return Err(EmuError::Halt(0)) }} }} else {{ if n / b > 0xFFFF {{ return Err(EmuError::Halt(0)); }} (n / b, n % b) }}; reg_set(&mut rg, 0, 2, q); reg_set(&mut rg, 2, 2, r); }}"),
                    _ => format!("{{ let b = {b}; if b == 0 {{ return Err(EmuError::Halt(0)); }} let n = (rg[EDX] as u64) << 32 | rg[EAX] as u64; let (q, r) = if {sg} {{ let (n, d) = (n as i64, b as i32 as i64); match n.checked_div(d) {{ Some(q) if q == q as i32 as i64 => (q as u32, (n % d) as u32), _ => return Err(EmuError::Halt(0)) }} }} else {{ if n / b as u64 > u32::MAX as u64 {{ return Err(EmuError::Halt(0)); }} ((n / b as u64) as u32, (n % b as u64) as u32) }}; rg[EAX] = q; rg[EDX] = r; }}"),
                }
            }
            Ins::Inc(rm, size) | Ins::Dec(rm, size) => {
                let aop = if matches!(ins, Ins::Inc(..)) { "Add" } else { "Sub" };
                let v = rd(c, rm, *size)?;
                format!("{{ let v = {v}; let cf = fl.cf; let r = fl.alu(Alu::{aop}, v, 1, {size}); fl.cf = cf; {} }}", wr(c, rm, *size, "r")?)
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

#[cfg(test)]
mod tests {
    use super::*;

    fn block(lines: &[String], fall: Option<usize>) -> Block {
        Block { lines: lines.to_vec(), fall }
    }

    #[test]
    fn structure_conditional_jump_to_the_next_block() {
        // 0: if c goto 2, else 1; 1: if c goto 3, else 2; 2, 3: return.
        // The reverse post order is 0 1 3 2: the conditional jump of 1 to
        // the next block must still leave the block
        let cond = |t| format!("if fl.cond(4) {{ {} }}", Jump::mark(t, true));
        let blocks = vec![block(&[cond(2)], Some(1)), block(&[cond(3)], Some(2)), block(&[Jump::RETURN.to_string()], None), block(&[Jump::RETURN.to_string()], None)];
        let s = structure(&blocks, 0).expect("structured");
        assert!(!s.loops);
        assert!(s.code.contains("// block 1\n        if fl.cond(4) { break 'b3; }\n        break 'b2;\n        } // 'b3\n        // block 3"), "{}", s.code);
    }

    #[test]
    fn structure_loop() {
        // 0 -> 1; 1: if c { goto 1 } fall to 2; 2: return
        let blocks = vec![
            block(&[], Some(1)),
            block(&[format!("if fl.cond(4) {{ {} }}", Jump::mark(1, true))], Some(2)),
            block(&[Jump::RETURN.to_string()], None),
        ];
        let s = structure(&blocks, 0).expect("structured");
        assert!(s.loops);
        assert!(s.code.contains("'l1: loop {"), "{}", s.code);
        assert!(s.code.contains("continue 'l1;"), "{}", s.code);
        assert!(s.code.contains("break 'b2;"), "{}", s.code);
    }

    #[test]
    fn structure_wgsl_loop_flags() {
        let cond = |t| format!("if (cond(4u)) {{ {} }}", Jump::mark(t, true));
        // 0: if c goto 2, else 1; 1: if c goto 0, else 2; 2: if c goto 0,
        // else 3; 3: return.  The jump of 1 back to the entry block (the
        // loop header) leaves the scope 'b2 through `cont`
        let blocks = vec![block(&[cond(2)], Some(1)), block(&[cond(0)], Some(2)), block(&[cond(0)], Some(3)), block(&[Jump::RETURN.to_string()], None)];
        let w = structure_wgsl(&blocks, 0).expect("structured");
        assert!(w.contains("if (cond(4u)) { cont = 0u; break; }"), "{w}");
        assert!(w.contains("else if (cont == 0u) { cont = 0xffffffffu;"), "{w}");
        assert!(w.contains("else if (brk != 0xffffffffu || cont != 0xffffffffu) { break; }"), "{w}");
        // 0 -> 3; 3: if c goto 1, else 2; 1: if c goto 2, else 3; 2: return.
        // The loop header is block 3 at position 1: the flag check tests
        // the block id
        let blocks = vec![block(&[], Some(3)), block(&[cond(2)], Some(3)), block(&[Jump::RETURN.to_string()], None), block(&[cond(1)], Some(2))];
        let w = structure_wgsl(&blocks, 0).expect("structured");
        assert!(w.contains("else if (cont == 3u) {"), "{w}");
    }

    fn prog_of(code: &[u8]) -> Prog {
        let mut m = Machine::new();
        m.write_bytes(CODE_START, code).unwrap();
        m.predecode(&[CODE_START]);
        m.compile(CODE_START).expect("compiles")
    }

    #[test]
    fn push_pop_esp() {
        // push esp; pop eax; push eax; pop esp; ret
        let p = prog_of(&[0x54, 0x58, 0x50, 0x5C, 0xC3]);
        let r = p.emit_lifted("f", false).expect("lifts");
        assert!(r.contains("{ let v = rg[ESP]; rg[ESP] = rg[ESP].wrapping_sub(4); s2014 = (s2014 & !(0xffffffffu64 << 0)) | (((v as u64)"), "{r}");
        assert!(r.contains("\n        rg[ESP] = ((s2014 >> 0) & 0xffffffffu64) as u32;\n"), "{r}");
        let w = p.emit_wgsl("f", false).expect("lifts").code;
        assert!(w.contains("{ let v = rg[4]; rg[4] = rg[4] - 4u; s2014l = (v); }"), "{w}");
        assert!(w.contains("\n    rg[4] = s2014l;\n"), "{w}");
    }

    #[test]
    fn div_quotient_out_of_range_halts() {
        // div ecx; idiv ecx; div cx; idiv cl; ret
        let p = prog_of(&[0xF7, 0xF1, 0xF7, 0xF9, 0x66, 0xF7, 0xF1, 0xF6, 0xF9, 0xC3]);
        let r = p.emit_lifted("f", false).expect("lifts");
        assert!(r.contains("if n / b as u64 > u32::MAX as u64 { return Err(EmuError::Halt(0)); }"), "{r}");
        assert!(r.contains("Some(q) if q == q as i32 as i64 =>"), "{r}");
        assert!(r.contains("if n / b > 0xFFFF { return Err(EmuError::Halt(0)); }"), "{r}");
        assert!(r.contains("if q != q as i8 as i32 { return Err(EmuError::Halt(0)); }"), "{r}");
    }
}
