//! Two more views of the datapath, both drawn from one description of its parts and the
//! connections between them:
//!
//! - the model: every part, laid out to be read rather than to match the book, with the value in
//!   each part (registers can be edited in place) and the IR drawn as its 16 bits, with each
//!   field sliced out of it by a bracket and a wire;
//! - the focus view: only the parts and connections the current state uses, laid out in the
//!   order the data flows, with room to say what each part does with its inputs.

use super::figure::{
    alu_op, arrow_head, combinational, flow_dashes, segment_distance, sext, wire_value_long,
    wire_value_text,
};
use super::{ir_bits_used, ir_fields};
use crate::emulator::trace::{Activity, Dp, MicroRecord, Sig, Snapshot};
use crate::emulator::{disasm, Emulator};
use crate::panes::glossary::Explain;
use crate::panes::ui_kit;
use crate::theme::ThemeSettings;
use egui::{vec2, Align2, Color32, CornerRadius, FontId, Pos2, Rect, Shape, Stroke, StrokeKind};
use std::collections::HashMap;

/// A part of the datapath.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, PartialOrd, Ord)]
pub enum N {
    Ir,
    Control,
    Pc,
    PcInc,
    PcDec,
    PcMux,
    Zext,
    Sext11,
    Sext9,
    Sext6,
    Sext5,
    Zero,
    Addr1Mux,
    Addr2Mux,
    Adder,
    MarMux,
    RegFile,
    Sr2Mux,
    Alu,
    Logic,
    Psr,
    SetPriv,
    KbdPriority,
    IntCmp,
    Bus,
    Mar,
    Mdr,
    Memory,
    SpInc,
    SpDec,
    SavedUsp,
    SavedSsp,
    SpMux,
    TabX00,
    TabX01,
    TableMux,
    Table,
    VecIntv,
    VecX00,
    VecX01,
    VecX02,
    VectorMux,
    VectorSel,
    Vector,
}

/// When a connection is in use.
#[derive(Clone, Copy)]
enum On {
    Dp(Dp),
    /// The bits of IR this state reads (a register number)
    Bits(u8, u8),
    /// A wire, only while a signal is asserted
    DpSig(Dp, Sig),
    Sig(Sig),
}

/// What a connection carries.
#[derive(Clone, Copy)]
enum V {
    /// The traced wire's value (plus an offset, for the wire into an incrementer)
    Wire(i16),
    /// Bits of IR
    Bits(u8, u8),
    Priority,
    Nzp,
    /// An 8-bit register: Table or Vector
    Table,
    Vector,
}

struct E {
    on: On,
    from: N,
    to: N,
    val: V,
    /// The control signal that opens this connection, shown beside it
    sig: &'static str,
}

const fn w(dp: Dp, from: N, to: N, sig: &'static str) -> E {
    E {
        on: On::Dp(dp),
        from,
        to,
        val: V::Wire(0),
        sig,
    }
}
const fn slice(on: On, hi: u8, lo: u8, to: N, sig: &'static str) -> E {
    E {
        on,
        from: N::Ir,
        to,
        val: V::Bits(hi, lo),
        sig,
    }
}

const EDGES: &[E] = &[
    // IR fields
    slice(On::Dp(Dp::IrToControl), 15, 12, N::Control, "opcode"),
    slice(On::Sig(Sig::LdBen), 11, 9, N::Control, "nzp → BEN"),
    slice(On::Bits(11, 9), 11, 9, N::RegFile, "DR / SR"),
    slice(On::Bits(8, 6), 8, 6, N::RegFile, "SR1 / BaseR"),
    slice(On::Bits(5, 5), 5, 5, N::Sr2Mux, "IR[5]"),
    slice(On::Bits(2, 0), 2, 0, N::RegFile, "SR2"),
    slice(On::Dp(Dp::Sext11), 10, 0, N::Sext11, "PCoffset11"),
    slice(On::Dp(Dp::Sext9), 8, 0, N::Sext9, "PCoffset9"),
    slice(On::Dp(Dp::Zext), 7, 0, N::Zext, "trapvect8"),
    slice(On::Dp(Dp::IrToVector), 7, 0, N::VectorSel, "trapvect8"),
    slice(On::Dp(Dp::Sext6), 5, 0, N::Sext6, "offset6"),
    slice(On::Dp(Dp::Sext5), 4, 0, N::Sext5, "imm5"),
    // PC
    w(Dp::GatePc, N::Pc, N::Bus, "GatePC"),
    E {
        on: On::Dp(Dp::PcInc),
        from: N::Pc,
        to: N::PcInc,
        val: V::Wire(-1),
        sig: "",
    },
    w(Dp::PcInc, N::PcInc, N::PcMux, ""),
    w(Dp::PcMux, N::PcMux, N::Pc, "LD.PC"),
    w(Dp::BusToPcMux, N::Bus, N::PcMux, ""),
    w(Dp::AdderToPcMux, N::Adder, N::PcMux, ""),
    w(Dp::PcDec, N::Pc, N::PcDec, ""),
    w(Dp::GatePcDec, N::PcDec, N::Bus, "GatePC-1"),
    // address adder
    w(Dp::PcToAddr1, N::Pc, N::Addr1Mux, ""),
    w(Dp::RegToAddr1, N::RegFile, N::Addr1Mux, "SR1"),
    w(Dp::Addr1Mux, N::Addr1Mux, N::Adder, ""),
    w(Dp::Sext11, N::Sext11, N::Addr2Mux, ""),
    w(Dp::Sext9, N::Sext9, N::Addr2Mux, ""),
    w(Dp::Sext6, N::Sext6, N::Addr2Mux, ""),
    w(Dp::ZeroToAddr2, N::Zero, N::Addr2Mux, ""),
    w(Dp::Addr2Mux, N::Addr2Mux, N::Adder, ""),
    w(Dp::AdderToMarMux, N::Adder, N::MarMux, ""),
    w(Dp::ZextToMarMux, N::Zext, N::MarMux, ""),
    w(Dp::GateMarMux, N::MarMux, N::Bus, "GateMARMUX"),
    // registers and ALU
    w(Dp::BusToReg, N::Bus, N::RegFile, "LD.REG"),
    w(Dp::Sr1Out, N::RegFile, N::Alu, "SR1"),
    w(Dp::Sr2Out, N::RegFile, N::Sr2Mux, "SR2"),
    w(Dp::Sext5, N::Sext5, N::Sr2Mux, ""),
    w(Dp::Sr2Mux, N::Sr2Mux, N::Alu, ""),
    w(Dp::GateAlu, N::Alu, N::Bus, "GateALU"),
    // IR and condition codes
    w(Dp::BusToIr, N::Bus, N::Ir, "LD.IR"),
    w(Dp::BusToLogic, N::Bus, N::Logic, ""),
    E {
        on: On::Dp(Dp::Logic),
        from: N::Logic,
        to: N::Psr,
        val: V::Nzp,
        sig: "LD.CC",
    },
    E {
        on: On::Sig(Sig::LdBen),
        from: N::Psr,
        to: N::Control,
        val: V::Nzp,
        sig: "BEN",
    },
    // PSR
    w(Dp::GatePsr, N::Psr, N::Bus, "GatePSR"),
    w(Dp::BusToPsr, N::Bus, N::Psr, "LD.PSR"),
    w(Dp::SetPriv, N::SetPriv, N::Psr, "LD.Priv"),
    E {
        on: On::DpSig(Dp::IntPriorityIn, Sig::LdPriority),
        from: N::KbdPriority,
        to: N::Psr,
        val: V::Wire(0),
        sig: "LD.Priority",
    },
    w(Dp::IntPriorityIn, N::KbdPriority, N::IntCmp, ""),
    E {
        on: On::Dp(Dp::IntSignal),
        from: N::Psr,
        to: N::IntCmp,
        val: V::Priority,
        sig: "",
    },
    w(Dp::IntSignal, N::IntCmp, N::Control, "INT"),
    // memory
    w(Dp::BusToMar, N::Bus, N::Mar, "LD.MAR"),
    w(Dp::MarToMem, N::Mar, N::Memory, ""),
    w(Dp::BusToMdr, N::Bus, N::Mdr, "LD.MDR"),
    w(Dp::MemToMdr, N::Memory, N::Mdr, "LD.MDR"),
    w(Dp::MdrToMem, N::Mdr, N::Memory, "R.W = WR"),
    w(Dp::GateMdr, N::Mdr, N::Bus, "GateMDR"),
    // the stack pointer
    E {
        on: On::Dp(Dp::SpIncPath),
        from: N::RegFile,
        to: N::SpInc,
        val: V::Wire(-1),
        sig: "R6",
    },
    w(Dp::SpIncPath, N::SpInc, N::SpMux, ""),
    E {
        on: On::Dp(Dp::SpDecPath),
        from: N::RegFile,
        to: N::SpDec,
        val: V::Wire(1),
        sig: "R6",
    },
    w(Dp::SpDecPath, N::SpDec, N::SpMux, ""),
    w(Dp::UspIn, N::RegFile, N::SavedUsp, "LD.SavedUSP"),
    w(Dp::SspIn, N::RegFile, N::SavedSsp, "LD.SavedSSP"),
    w(Dp::UspOut, N::SavedUsp, N::SpMux, ""),
    w(Dp::SspOut, N::SavedSsp, N::SpMux, ""),
    w(Dp::GateSp, N::SpMux, N::Bus, "GateSP"),
    // vector tables
    w(Dp::TabX00, N::TabX00, N::TableMux, ""),
    w(Dp::TabX01, N::TabX01, N::TableMux, ""),
    E {
        on: On::Dp(Dp::TableMux),
        from: N::TableMux,
        to: N::Table,
        val: V::Table,
        sig: "LD.Table",
    },
    w(Dp::VecIntv, N::VecIntv, N::VectorMux, ""),
    w(Dp::VecX00, N::VecX00, N::VectorMux, ""),
    w(Dp::VecX01, N::VecX01, N::VectorMux, ""),
    w(Dp::VecX02, N::VecX02, N::VectorMux, ""),
    w(Dp::VectorMux, N::VectorMux, N::VectorSel, ""),
    E {
        on: On::Dp(Dp::VectorSel),
        from: N::VectorSel,
        to: N::Vector,
        val: V::Vector,
        sig: "LD.Vector",
    },
    E {
        on: On::Dp(Dp::GateVector),
        from: N::Table,
        to: N::Bus,
        val: V::Table,
        sig: "GateVector",
    },
    E {
        on: On::Dp(Dp::GateVector),
        from: N::Vector,
        to: N::Bus,
        val: V::Vector,
        sig: "",
    },
];

/// Registers: in the focus view, one read and written in the same state is shown twice, as it
/// was and as it becomes.
fn is_register(n: N) -> bool {
    matches!(
        n,
        N::Pc
            | N::Ir
            | N::Mar
            | N::Mdr
            | N::RegFile
            | N::Psr
            | N::SavedUsp
            | N::SavedSsp
            | N::Table
            | N::Vector
    )
}

#[derive(Clone, Copy, PartialEq)]
enum Lit {
    Idle,
    /// Live up to a gate that is off (the true state)
    ToGate,
    Trail,
    /// Carrying a value this state doesn't use (the true state)
    Live,
    Active,
}

/// Everything the views draw from.
pub struct Ctx<'a> {
    pub rec: Option<&'a MicroRecord>,
    pub trail: Activity,
    /// The machine now
    pub snap: Snapshot,
    /// The machine before this state
    pub prev: Option<Snapshot>,
    /// (Saved_SSP, Saved_USP, PSR, Table, Vector)
    pub extra: [u16; 5],
    pub flow_time: Option<f64>,
    pub hover: Option<Pos2>,
    /// The true state: every live wire lit, with its value (not in the focus view)
    pub live: Option<&'a super::live::Live>,
    /// The instruction's address, to disassemble IR with its real targets
    pub ir_addr: usize,
}

impl Ctx<'_> {
    fn act(&self) -> Activity {
        self.rec.map(|r| r.activity).unwrap_or_default()
    }
    fn used_bits(&self) -> u16 {
        self.rec.map_or(0, |r| ir_bits_used(self.snap.ir, r))
    }
    fn lit(&self, e: &E) -> Lit {
        let (act, trail) = (self.act(), self.trail);
        let has = |a: Activity, on: On| match on {
            On::Dp(d) => a.has(d),
            On::DpSig(d, s) => a.has(d) && a.signal(s),
            On::Sig(s) => a.signal(s),
            On::Bits(..) => false,
        };
        match e.on {
            On::Bits(hi, lo) => {
                if self
                    .rec
                    .is_some_and(|r| field_used(hi, lo, self.snap.ir, r))
                {
                    Lit::Active
                } else if self.live.is_some() {
                    Lit::Live
                } else {
                    Lit::Idle
                }
            }
            on if has(act, on) => Lit::Active,
            on => {
                // the true state: anything carrying a value is lit, wires into closed gates up to them
                if let Some(l) = self.live {
                    match on {
                        On::Dp(d) | On::DpSig(d, _) if l.has(d) => return Lit::Live,
                        On::Dp(d) | On::DpSig(d, _) if l.stops_at_gate(d) => return Lit::ToGate,
                        On::Sig(_) | On::Bits(..) => return Lit::Live,
                        _ => {}
                    }
                }
                if has(trail, on) {
                    Lit::Trail
                } else {
                    Lit::Idle
                }
            }
        }
    }
    fn psr(&self) -> u16 {
        self.extra[2]
    }
    /// What a connection carries, if known (its value in use, or what is on it anyway).
    fn value(&self, e: &E, lit: Lit) -> Option<(String, String)> {
        let active = lit == Lit::Active;
        if !active && self.live.is_none() {
            return None;
        }
        let bits = |hi: u8, lo: u8| -> String {
            (lo..=hi)
                .rev()
                .map(|b| if self.snap.ir >> b & 1 == 1 { '1' } else { '0' })
                .collect()
        };
        Some(match e.val {
            // BEN tests IR[11:9] against N, Z, P
            V::Bits(11, 9) if e.to == N::Control => {
                let b = bits(11, 9);
                let f = |bit: u8, ch: char| {
                    if self.snap.ir >> bit & 1 == 1 {
                        ch
                    } else {
                        '-'
                    }
                };
                let nzp = format!("{}{}{}", f(11, 'n'), f(10, 'z'), f(9, 'p'));
                (
                    format!("{b} {nzp}"),
                    format!("IR[11:9] = {b}: branch if {nzp} (BEN)"),
                )
            }
            V::Bits(hi, lo) => {
                let b = bits(hi, lo);
                let v = (self.snap.ir >> lo) & ((1u16 << (hi - lo + 1)) - 1);
                let long = match (hi, lo) {
                    (11, 9) | (8, 6) | (2, 0) => format!("IR[{hi}:{lo}] = {b}: R{v}"),
                    (15, 12) => format!("IR[15:12] = {b}: the opcode"),
                    (5, 5) => format!("IR[5] = {b}: {}", if v == 1 { "SEXT(imm5)" } else { "SR2" }),
                    _ => format!("IR[{hi}:{lo}] = {b}"),
                };
                let short = match (hi, lo) {
                    (11, 9) | (8, 6) | (2, 0) => format!("{b} R{v}"),
                    _ => b,
                };
                (short, long)
            }
            V::Priority => {
                let s = format!("PL{}", self.psr() >> 8 & 7);
                (s.clone(), format!("{s}: the program's priority, PSR[10:8]"))
            }
            V::Nzp => {
                let s = nzp_text(self.snap.nzp as u16);
                (s.clone(), s)
            }
            V::Table => {
                let s = format!("x{:02X}", self.extra[3] & 0xFF);
                (s.clone(), s)
            }
            V::Vector => {
                let s = format!("x{:02X}", self.extra[4] & 0xFF);
                (s.clone(), s)
            }
            V::Wire(pre) => {
                let dp = match e.on {
                    On::Dp(d) | On::DpSig(d, _) => d,
                    _ => return None,
                };
                let v = active
                    .then(|| self.rec.and_then(|r| r.wire(dp)))
                    .flatten()
                    .or_else(|| self.live.and_then(|l| l.value(dp)))
                    .or_else(|| combinational(dp, &self.snap, &self.extra))?
                    .wrapping_add(pre as u16);
                let dp = if pre != 0 { Dp::Bus } else { dp };
                (wire_value_text(dp, v), wire_value_long(dp, v))
            }
        })
    }
}

/// Whether a state uses an IR field as a register number (or IR[5] as SR2MUX's select).
fn field_used(hi: u8, lo: u8, ir: u16, r: &MicroRecord) -> bool {
    let a = r.activity;
    let op = ir >> 12;
    let stack =
        a.has(Dp::SpMux) || a.has(Dp::SavedSsp) || a.has(Dp::SavedUsp) || a.has(Dp::Sr1ToSp);
    match (hi, lo) {
        // DR (DRMUX = IR[11:9], not R7 for JSR and TRAP) or the register a store writes out (SR1MUX)
        (11, 9) => {
            (a.signal(Sig::LdReg) && op != 0x4 && op != 0xF && !stack)
                || (r.state == 23 && a.has(Dp::Sr1Out))
        }
        // SR1 or BaseR (SR1MUX = IR[8:6])
        (8, 6) => (a.has(Dp::Sr1Out) || a.has(Dp::RegToAddr1)) && r.state != 23 && !stack,
        (5, 5) => a.has(Dp::Sr2Mux),
        (2, 0) => a.has(Dp::Sr2Out),
        _ => false,
    }
}

fn nzp_text(v: u16) -> String {
    let f = |b: u16, c: char| {
        if v & b != 0 {
            c.to_ascii_uppercase()
        } else {
            c
        }
    };
    format!("{}{}{}", f(4, 'n'), f(2, 'z'), f(1, 'p'))
}

fn name(n: N) -> &'static str {
    match n {
        N::Ir => "IR",
        N::Control => "CONTROL",
        N::Pc => "PC",
        N::PcInc => "+1",
        N::PcDec => "−1",
        N::PcMux => "PCMUX",
        N::Zext => "ZEXT",
        N::Sext11 | N::Sext9 | N::Sext6 | N::Sext5 => "SEXT",
        N::Zero => "0",
        N::Addr1Mux => "ADDR1MUX",
        N::Addr2Mux => "ADDR2MUX",
        N::Adder => "ADDER",
        N::MarMux => "MARMUX",
        N::RegFile => "REG FILE",
        N::Sr2Mux => "SR2MUX",
        N::Alu => "ALU",
        N::Logic => "LOGIC",
        N::Psr => "PSR",
        N::SetPriv => "Set.Priv",
        N::KbdPriority => "KEYBOARD",
        N::IntCmp => "A > B",
        N::Bus => "BUS",
        N::Mar => "MAR",
        N::Mdr => "MDR",
        N::Memory => "MEMORY",
        N::SpInc => "+1",
        N::SpDec => "−1",
        N::SavedUsp => "Saved.USP",
        N::SavedSsp => "Saved.SSP",
        N::SpMux => "SPMUX",
        N::TabX00 | N::VecX00 => "x00",
        N::TabX01 | N::VecX01 => "x01",
        N::VecX02 => "x02",
        N::VecIntv => "INTV",
        N::TableMux => "TableMUX",
        N::Table => "Table",
        N::VectorMux => "VectorMUX",
        N::VectorSel => "Vector MUX",
        N::Vector => "Vector",
    }
}

fn help(n: N) -> &'static str {
    match n {
        N::Ir => "IR: the instruction being executed. Its fields are sliced out by wires: the opcode to control, register numbers to the register file, offsets to the sign extenders",
        N::Control => "Control: the state machine. It reads the opcode (and BEN, INT) to pick the next state and asserts the control signals",
        N::Pc => "PC: the address of the next instruction",
        N::PcInc => "Incrementer: PC + 1",
        N::PcDec => "Decrementer: PC − 1, the return address an interrupt pushes",
        N::PcMux => "PCMUX: the PC's next value is PC + 1, the bus, or the address adder",
        N::Zext => "ZEXT: zero-extends IR[7:0], the trap vector",
        N::Sext11 => "SEXT: sign-extends IR[10:0] (PCoffset11, JSR)",
        N::Sext9 => "SEXT: sign-extends IR[8:0] (PCoffset9: LD, ST, LDI, STI, LEA, BR)",
        N::Sext6 => "SEXT: sign-extends IR[5:0] (offset6: LDR, STR)",
        N::Sext5 => "SEXT: sign-extends IR[4:0] (imm5: ADD, AND)",
        N::Zero => "The constant 0 (JMP, JSRR: no offset)",
        N::Addr1Mux => "ADDR1MUX: the adder's base, PC or a register (BaseR)",
        N::Addr2Mux => "ADDR2MUX: the adder's offset",
        N::Adder => "The address adder: base + offset",
        N::MarMux => "MARMUX: the adder's result or ZEXT(trapvect8) onto the bus",
        N::RegFile => "The register file: reads SR1 and SR2, writes DR (the register numbers come from IR)",
        N::Sr2Mux => "SR2MUX: the ALU's B input is SR2 or SEXT(imm5), chosen by IR[5]",
        N::Alu => "The ALU: ADD, AND, NOT, or pass A through",
        N::Logic => "LOGIC: N, Z or P from the value on the bus",
        N::Psr => "PSR: privilege PSR[15], priority PSR[10:8], condition codes PSR[2:0]",
        N::SetPriv => "Set.Priv: 0, for Supervisor mode",
        N::KbdPriority => "The keyboard's interrupt priority, PL4",
        N::IntCmp => "Compares the interrupt's priority with the program's: INT",
        N::Bus => "The bus: one 16-bit value per clock cycle, driven by exactly one gate",
        N::Mar => "MAR: the address memory reads or writes",
        N::Mdr => "MDR: the word read from or written to memory",
        N::Memory => "Memory (and the device registers): reads into MDR, or writes MDR, at the address in MAR",
        N::SpInc => "R6 + 1: pop",
        N::SpDec => "R6 − 1: push",
        N::SavedUsp => "Saved.USP: the user stack pointer while in Supervisor mode",
        N::SavedSsp => "Saved.SSP: the supervisor stack pointer while in User mode",
        N::SpMux => "SPMUX: picks the new stack pointer",
        N::TabX00 | N::TabX01 => "A constant for Table: x00 traps, x01 interrupts and exceptions",
        N::VecIntv | N::VecX00 | N::VecX01 | N::VecX02 => "Vector constants: INTV from the device, x00 privilege, x01 illegal opcode, x02 ACV",
        N::TableMux => "TableMUX: picks the table",
        N::Table => "Table: which vector table, the high byte of the entry's address",
        N::VectorMux => "VectorMUX: an interrupt or exception vector",
        N::VectorSel => "Picks IR[7:0] (TRAP) or VectorMUX",
        N::Vector => "Vector: the entry in the table, the low byte of its address",
    }
}

/// A part's value line (and a second line of detail), from what it holds or what it does now.
fn content(n: N, c: &Ctx<'_>) -> (String, String) {
    let s = &c.snap;
    let a = c.act();
    let rec = c.rec;
    let wire = |d: Dp| {
        rec.and_then(|r| r.wire(d))
            .filter(|_| a.has(d))
            .or_else(|| c.live.and_then(|l| l.value(d)))
    };
    let was = |old: Option<u16>, new: u16| match old {
        Some(o) if o != new => format!("was x{o:04X}"),
        _ => String::new(),
    };
    let h = |v: u16| format!("x{v:04X}");
    match n {
        N::Pc => (h(s.pc), was(c.prev.map(|p| p.pc), s.pc)),
        N::Mar => (h(s.mar), was(c.prev.map(|p| p.mar), s.mar)),
        N::Mdr => (h(s.mdr), was(c.prev.map(|p| p.mdr), s.mdr)),
        N::Ir => (
            h(s.ir),
            disasm::disassemble(s.ir, c.ir_addr, &HashMap::new()),
        ),
        N::Psr => {
            let psr = c.psr();
            (
                format!("x{psr:04X}"),
                format!(
                    "{} · PL{} · {}",
                    if psr & 0x8000 != 0 {
                        "User"
                    } else {
                        "Supervisor"
                    },
                    psr >> 8 & 7,
                    nzp_text(s.nzp as u16)
                ),
            )
        }
        N::SavedSsp => (h(c.extra[0]), String::new()),
        N::SavedUsp => (h(c.extra[1]), String::new()),
        N::Table => (format!("x{:02X}", c.extra[3] & 0xFF), String::new()),
        N::Vector => (format!("x{:02X}", c.extra[4] & 0xFF), String::new()),
        N::PcInc => (wire(Dp::PcInc).map(h).unwrap_or_default(), String::new()),
        N::PcDec => (
            wire(Dp::GatePcDec).map(h).unwrap_or_default(),
            String::new(),
        ),
        N::PcMux => (
            if a.signal(Sig::PcMuxInc) {
                "PC + 1"
            } else if a.signal(Sig::PcMuxBus) {
                "bus"
            } else if a.signal(Sig::PcMuxAdder) {
                "adder"
            } else {
                ""
            }
            .into(),
            String::new(),
        ),
        N::MarMux => (
            if a.signal(Sig::MarMuxZext) {
                "ZEXT"
            } else if a.signal(Sig::MarMuxAdder) {
                "adder"
            } else {
                ""
            }
            .into(),
            String::new(),
        ),
        N::Addr1Mux => (
            if a.signal(Sig::Addr1Pc) {
                "PC"
            } else if a.signal(Sig::Addr1Base) {
                "BaseR"
            } else {
                ""
            }
            .into(),
            String::new(),
        ),
        N::Addr2Mux => (
            if a.signal(Sig::Addr2Off11) {
                "offset11"
            } else if a.signal(Sig::Addr2Off9) {
                "offset9"
            } else if a.signal(Sig::Addr2Off6) {
                "offset6"
            } else if a.signal(Sig::Addr2Zero) {
                "0"
            } else {
                ""
            }
            .into(),
            String::new(),
        ),
        N::Adder => match (wire(Dp::Addr1Mux), wire(Dp::Addr2Mux)) {
            (Some(b), Some(o)) => {
                let r = b.wrapping_add(o);
                (h(r), format!("x{b:04X} + {}", o as i16))
            }
            _ => (String::new(), String::new()),
        },
        N::Sext11 | N::Sext9 | N::Sext6 | N::Sext5 | N::Zext => {
            let (bits, dp) = match n {
                N::Sext11 => (11, Dp::Sext11),
                N::Sext9 => (9, Dp::Sext9),
                N::Sext6 => (6, Dp::Sext6),
                N::Sext5 => (5, Dp::Sext5),
                _ => (8, Dp::Zext),
            };
            let raw = s.ir & ((1 << bits) - 1);
            let v = if n == N::Zext { raw } else { sext(raw, bits) };
            if a.has(dp) || c.live.is_some() {
                (
                    h(v),
                    if n == N::Zext {
                        format!("{raw}")
                    } else {
                        format!("{}", v as i16)
                    },
                )
            } else {
                (String::new(), String::new())
            }
        }
        N::Sr2Mux => (
            if a.signal(Sig::Sr2MuxImm) {
                "imm5".into()
            } else if a.has(Dp::Sr2Out) {
                "SR2".into()
            } else {
                String::new()
            },
            String::new(),
        ),
        N::Alu => {
            let op = alu_op(&a);
            let out =
                wire(Dp::GateAlu).or_else(|| rec.filter(|_| a.has(Dp::Alu)).and_then(|r| r.value));
            match (wire(Dp::Sr1Out), wire(Dp::Sr2Mux), out) {
                (Some(x), b, Some(r)) if a.signal(Sig::AlukNot) || b.is_none() => {
                    (h(r), format!("{op} x{x:04X}"))
                }
                (Some(x), Some(y), Some(r)) => (h(r), format!("x{x:04X} {op} x{y:04X}")),
                (_, _, Some(r)) => (h(r), op.to_string()),
                _ => (String::new(), op.to_string()),
            }
        }
        N::Logic => match wire(Dp::BusToLogic) {
            Some(v) => (
                nzp_text(s.nzp as u16),
                format!(
                    "x{v:04X} is {}",
                    if v & 0x8000 != 0 {
                        "negative"
                    } else if v == 0 {
                        "zero"
                    } else {
                        "positive"
                    }
                ),
            ),
            None => (String::new(), String::new()),
        },
        N::Control => match rec {
            Some(r) => (format!("state {}", r.state), String::new()),
            None => (String::new(), String::new()),
        },
        N::Memory => match rec.filter(|r| r.activity.has(Dp::Memory)) {
            Some(r) => {
                let v = r.wire(Dp::MemToMdr).or(r.wire(Dp::MdrToMem)).unwrap_or(0);
                let at = r.wire(Dp::MarToMem).unwrap_or(s.mar);
                if a.signal(Sig::RwWrite) {
                    (format!("M[x{at:04X}] ← x{v:04X}"), "write".into())
                } else {
                    (format!("M[x{at:04X}] = x{v:04X}"), "read".into())
                }
            }
            None => (String::new(), String::new()),
        },
        N::Bus => match rec
            .filter(|r| r.activity.has(Dp::Bus))
            .and_then(|r| r.value)
        {
            Some(v) => (h(v), String::new()),
            None => (String::new(), String::new()),
        },
        N::SpInc => (
            wire(Dp::SpIncPath).map(h).unwrap_or_default(),
            String::new(),
        ),
        N::SpDec => (
            wire(Dp::SpDecPath).map(h).unwrap_or_default(),
            String::new(),
        ),
        N::SpMux => (
            if a.signal(Sig::SpMuxInc) {
                "R6 + 1"
            } else if a.signal(Sig::SpMuxDec) {
                "R6 − 1"
            } else if a.signal(Sig::SpMuxSsp) {
                "Saved.SSP"
            } else if a.signal(Sig::SpMuxUsp) {
                "Saved.USP"
            } else {
                ""
            }
            .into(),
            String::new(),
        ),
        N::SetPriv => ("0".into(), String::new()),
        N::KbdPriority => ("PL4".into(), String::new()),
        N::IntCmp => (
            if a.has(Dp::IntSignal) {
                "INT = 1".into()
            } else {
                String::new()
            },
            String::new(),
        ),
        _ => (String::new(), String::new()),
    }
}

// ───────────────────────── drawing ─────────────────────────

/// Canvas units to screen.
#[derive(Clone, Copy)]
struct Tf {
    origin: Pos2,
    k: f32,
}
impl Tf {
    fn p(&self, x: f32, y: f32) -> Pos2 {
        self.origin + vec2(x, y) * self.k
    }
    fn r(&self, r: Rect) -> Rect {
        Rect::from_min_max(self.p(r.min.x, r.min.y), self.p(r.max.x, r.max.y))
    }
    fn fit(area: Rect, w: f32, h: f32, max: f32) -> Tf {
        let k = (area.width() / w).min(area.height() / h).min(max).max(0.05);
        Tf {
            origin: Pos2::new(area.center().x - w * k / 2.0, area.min.y + 4.0),
            k,
        }
    }
}

struct Painter<'a> {
    p: egui::Painter,
    theme: &'a ThemeSettings,
    tf: Tf,
    hits: Vec<(Rect, String)>,
    wires: Vec<(Vec<Pos2>, String)>,
    chips: Vec<Rect>,
    /// The parts' boxes, which labels on wires keep clear of
    blocked: Vec<Rect>,
}

impl Painter<'_> {
    fn fs(&self, size: f32) -> f32 {
        (size * self.tf.k).max(5.5)
    }
    fn colors(&self, lit: Lit) -> (Color32, Stroke) {
        let t = self.theme;
        let accent = t.accent_color_primary;
        match lit {
            Lit::Active | Lit::Live => (
                t.panel_background.blend(accent.gamma_multiply(0.14)),
                Stroke::new(1.8_f32, accent),
            ),
            Lit::Trail => (
                t.panel_background,
                Stroke::new(1.2_f32, accent.gamma_multiply(0.45)),
            ),
            Lit::Idle | Lit::ToGate => (
                t.panel_background,
                Stroke::new(1.0_f32, t.secondary_text_color.gamma_multiply(0.45)),
            ),
        }
    }

    /// A part: name, value and a detail line.
    fn node(
        &mut self,
        n: N,
        r: Rect,
        lit: Lit,
        value: &str,
        detail: &str,
        title_override: Option<&str>,
    ) {
        let t = self.theme;
        let (fill, stroke) = self.colors(lit);
        let corner = if matches!(n, N::Bus) { 10 } else { 4 };
        self.p.rect(
            r,
            CornerRadius::same(corner),
            fill,
            stroke,
            StrokeKind::Inside,
        );
        let title = title_override.unwrap_or(name(n));
        let title_col = if lit == Lit::Active {
            t.accent_color_primary
        } else {
            t.secondary_text_color
        };
        let fit = |size: f32, text: &str, width: f32| super::figure::fit(size, text, width);
        let title_size = fit(self.fs(9.5), title, r.width() - 6.0);
        let value_size = fit(self.fs(11.5), value, r.width() - 6.0);
        let detail_size = fit(self.fs(8.5), detail, r.width() - 6.0);
        // what fits: the detail goes first, then the title (which the tooltip still gives)
        let mut show_title = true;
        let mut show_value = !value.trim().is_empty() || value == " ";
        let mut show_detail = !detail.is_empty();
        let need = |t: bool, v: bool, d: bool| {
            t as u8 as f32 * title_size * 1.25
                + v as u8 as f32 * value_size * 1.3
                + d as u8 as f32 * detail_size * 1.25
                + 4.0
        };
        if need(show_title, show_value, show_detail) > r.height() {
            show_detail = false;
        }
        if need(show_title, show_value, show_detail) > r.height() && show_value {
            show_title = false;
        }
        if !show_title && !show_value {
            show_title = true;
        }
        let _ = &mut show_value;
        let lines: Vec<(&str, FontId, Color32)> = [
            (
                show_title,
                title,
                FontId::proportional(title_size),
                title_col,
            ),
            (
                show_value,
                value,
                FontId::monospace(value_size),
                if lit == Lit::Active {
                    t.strong_text_color
                } else {
                    t.primary_text_color
                },
            ),
            (
                show_detail,
                detail,
                FontId::proportional(detail_size),
                t.secondary_text_color,
            ),
        ]
        .into_iter()
        .filter(|(on, ..)| *on)
        .map(|(_, text, font, col)| (text, font, col))
        .collect();
        let line_h = r.height() / lines.len() as f32;
        for (i, (text, font, col)) in lines.into_iter().enumerate() {
            let y = r.min.y + line_h * (i as f32 + 0.5);
            self.p.text(
                Pos2::new(r.center().x, y),
                Align2::CENTER_CENTER,
                text,
                font,
                col,
            );
        }
        self.hits.push((r, help(n).to_string()));
    }

    /// A connection along `pts`, lit, with its value on it.
    fn wire(
        &mut self,
        pts: Vec<Pos2>,
        lit: Lit,
        value: Option<(String, String)>,
        sig: &str,
        tip: String,
        flow: Option<f64>,
    ) {
        let t = self.theme;
        let accent = t.accent_color_primary;
        let k = self.tf.k.max(0.6);
        match lit {
            Lit::Idle => {
                let col = t.secondary_text_color.gamma_multiply(0.35);
                self.p
                    .add(Shape::line(pts.clone(), Stroke::new(1.0_f32 * k, col)));
                arrow_head(&self.p, &pts, 5.0 * k, col);
            }
            Lit::Trail => {
                let col = accent.gamma_multiply(0.3);
                self.p
                    .add(Shape::line(pts.clone(), Stroke::new(1.3_f32 * k, col)));
                arrow_head(&self.p, &pts, 5.5 * k, col);
            }
            Lit::Active => {
                self.p.add(Shape::line(
                    pts.clone(),
                    Stroke::new(6.0 * k, accent.gamma_multiply(0.15)),
                ));
                self.p
                    .add(Shape::line(pts.clone(), Stroke::new(2.2 * k, accent)));
                if let Some(time) = flow {
                    flow_dashes(
                        &self.p,
                        &pts,
                        5.0 * k,
                        9.0 * k,
                        (time * 30.0) as f32 * k,
                        Stroke::new(2.2 * k, Color32::WHITE.gamma_multiply(0.75)),
                    );
                }
                arrow_head(&self.p, &pts, 8.0 * k, accent);
            }
            // carrying a value, though this state doesn't use it: lit, but still
            Lit::Live => {
                self.p.add(Shape::line(
                    pts.clone(),
                    Stroke::new(1.8 * k, accent.gamma_multiply(0.85)),
                ));
                arrow_head(&self.p, &pts, 7.0 * k, accent.gamma_multiply(0.85));
            }
            // live up to its gate, which is off: lit to a bar across the wire, then dead
            Lit::ToGate => {
                let idle = t.secondary_text_color.gamma_multiply(0.35);
                self.p
                    .add(Shape::line(pts.clone(), Stroke::new(1.0_f32 * k, idle)));
                arrow_head(&self.p, &pts, 5.0 * k, idle);
                let total: f32 = pts.windows(2).map(|w| w[0].distance(w[1])).sum();
                let stop = (total * 0.5).min(total - 12.0 * k).max(0.0);
                let lit_part = super::figure::path_span(&pts, 0.0, stop);
                if let [.., a, b] = lit_part.as_slice() {
                    let dir = (*b - *a).normalized();
                    let across = vec2(-dir.y, dir.x) * 5.0 * k;
                    self.p.add(Shape::line(
                        lit_part.clone(),
                        Stroke::new(1.8 * k, accent.gamma_multiply(0.85)),
                    ));
                    self.p.line_segment(
                        [*b - across, *b + across],
                        Stroke::new(2.0 * k, accent.gamma_multiply(0.85)),
                    );
                }
            }
        }
        let mut tip = tip;
        if let Some((short, long)) = value {
            tip.push_str(&format!("\n{long}"));
            if lit != Lit::ToGate {
                self.value_chip(&pts, &short, lit == Lit::Active);
            }
        }
        if lit == Lit::Active && !sig.is_empty() {
            self.label_beside(&pts, sig);
        }
        self.wires.push((pts, tip));
    }

    /// A control signal's name beside a wire, near its end, where it hits no part, value or
    /// other label (left out if there is no such place: the signals are listed above anyway).
    fn label_beside(&mut self, pts: &[Pos2], text: &str) {
        let t = self.theme;
        let galley = self.p.layout_no_wrap(
            text.to_string(),
            FontId::proportional(self.fs(7.5)),
            t.accent_color_primary,
        );
        let size = galley.size();
        let gap = 3.0 + self.tf.k * 2.0;
        let mut spots = Vec::new();
        for (a, b) in pts.windows(2).map(|w| (w[0], w[1])).rev() {
            let dir = (b - a).normalized();
            let across = if dir.x.abs() > 0.5 {
                vec2(0.0, size.y / 2.0 + gap)
            } else {
                vec2(size.x / 2.0 + gap, 0.0)
            };
            for f in [0.8_f32, 0.5, 0.2] {
                let at = a.lerp(b, f);
                spots.push(at - across);
                spots.push(at + across);
            }
        }
        // clear of chips, parts, and the wire itself (and the wires drawn so far)
        let on_wire = |r: Rect, pts: &[Pos2]| {
            pts.windows(2)
                .any(|w| Rect::from_two_pos(w[0], w[1]).expand(1.0).intersects(r))
        };
        let free = |c: &Pos2| {
            let r = Rect::from_center_size(*c, size);
            !self.chips.iter().any(|q| q.expand(1.0).intersects(r))
                && !self.blocked.iter().any(|q| q.expand(1.0).intersects(r))
                && !on_wire(r, pts)
                && !self.wires.iter().any(|(w, _)| on_wire(r, w))
        };
        if let Some(at) = spots.into_iter().find(|c| free(c)) {
            let r = Rect::from_center_size(at, size);
            self.p.galley(r.min, galley, t.accent_color_primary);
            self.chips.push(r);
        }
    }

    /// A value on a wire: on its own last segment if there is room, else further back.
    fn value_chip(&mut self, pts: &[Pos2], text: &str, hot: bool) {
        let t = self.theme;
        let font = FontId::monospace(self.fs(if hot { 8.5 } else { 7.5 }));
        let fg = if hot {
            t.window_background
        } else {
            t.secondary_text_color
        };
        let galley = self.p.layout_no_wrap(text.to_string(), font, fg);
        let size = galley.size() + vec2(6.0, 2.0);
        let segs: Vec<(Pos2, Pos2)> = pts.windows(2).map(|w| (w[0], w[1])).rev().collect();
        let long = |(a, b): &(Pos2, Pos2)| a.distance(*b) >= size.x.min(size.y * 3.0);
        let segs: Vec<(Pos2, Pos2)> = segs
            .iter()
            .copied()
            .filter(long)
            .chain(segs.iter().copied().filter(|s| !long(s)))
            .collect();
        let spots: Vec<Pos2> = segs
            .iter()
            .flat_map(|(a, b)| [0.5_f32, 0.3, 0.7].map(|k| a.lerp(*b, k)))
            .collect();
        let free = |c: &Pos2| {
            let r = Rect::from_center_size(*c, size);
            !self.chips.iter().any(|q| q.expand(1.0).intersects(r))
                && !self.blocked.iter().any(|q| q.intersects(r))
        };
        let Some(at) = spots.iter().find(|c| free(c)).or(spots.first()).copied() else {
            return;
        };
        let r = Rect::from_center_size(at, size);
        if hot {
            self.p
                .rect_filled(r, CornerRadius::same(3), t.accent_color_primary);
        } else {
            self.p.rect(
                r,
                CornerRadius::same(3),
                t.code_bg_color,
                Stroke::new(1.0_f32, t.separator_color),
                StrokeKind::Inside,
            );
        }
        self.p.galley(r.center() - galley.size() / 2.0, galley, fg);
        self.chips.push(r);
    }

    /// The IR as 16 bit cells; `brackets` are the slices to draw under (or over) it, each with
    /// the lane y it runs along. Returns where each slice's wire starts.
    fn ir(&mut self, r: Rect, ir: u16, addr: usize, used: u16, lit: Lit) {
        let t = self.theme;
        let (fill, stroke) = self.colors(lit);
        self.p
            .rect(r, CornerRadius::same(4), fill, stroke, StrokeKind::Inside);
        let bit_w = r.width() / 16.0;
        let fields = ir_fields(ir);
        for (hi, lo, fname, value) in &fields {
            let x0 = r.min.x + (15 - hi) as f32 * bit_w;
            let x1 = r.min.x + (16 - lo) as f32 * bit_w;
            if *lo > 0 {
                self.p
                    .vline(x1, r.y_range(), Stroke::new(1.0_f32, t.separator_color));
            }
            let cap = if value.is_empty() {
                fname.to_string()
            } else if fname.is_empty() {
                value.clone()
            } else {
                format!("{fname} {value}")
            };
            let size = super::figure::fit(self.fs(7.0), &cap, x1 - x0);
            self.p.text(
                Pos2::new((x0 + x1) / 2.0, r.max.y - 1.0),
                Align2::CENTER_BOTTOM,
                &cap,
                FontId::proportional(size),
                t.secondary_text_color,
            );
        }
        for b in 0..16u32 {
            let cell = Rect::from_min_size(
                Pos2::new(r.min.x + (15 - b) as f32 * bit_w, r.min.y + 2.0),
                vec2(bit_w, r.height() * 0.55),
            );
            let on = used >> b & 1 == 1;
            if on {
                self.p.rect_filled(
                    cell.shrink(1.0),
                    CornerRadius::same(2),
                    t.accent_color_primary,
                );
            }
            let bit = if ir >> b & 1 == 1 { "1" } else { "0" };
            self.p.text(
                cell.center(),
                Align2::CENTER_CENTER,
                bit,
                FontId::monospace(self.fs(10.0).min(bit_w * 0.9)),
                if on {
                    t.on_accent()
                } else {
                    t.strong_text_color
                },
            );
        }
        self.hits.push((
            r,
            format!(
                "{}\nIR = x{ir:04X}: {}",
                help(N::Ir),
                disasm::disassemble(ir, addr, &HashMap::new())
            ),
        ));
    }

    /// A bracket under (below = true) or over IR bits hi..lo, with a stem from its middle to
    /// `lane`. Returns the stem's end and the bracket's far end on the lane.
    fn bracket(
        &mut self,
        ir: Rect,
        hi: u8,
        lo: u8,
        lane: f32,
        lit: Lit,
        below: bool,
    ) -> (Pos2, Pos2) {
        let bit_w = ir.width() / 16.0;
        let x0 = ir.min.x + (15 - hi) as f32 * bit_w + 1.5;
        let x1 = ir.min.x + (16 - lo) as f32 * bit_w - 1.5;
        let edge = if below { ir.max.y } else { ir.min.y };
        let col = match lit {
            Lit::Active | Lit::Live => self.theme.accent_color_primary,
            Lit::Trail => self.theme.accent_color_primary.gamma_multiply(0.35),
            Lit::Idle | Lit::ToGate => self.theme.secondary_text_color.gamma_multiply(0.4),
        };
        let w = match lit {
            Lit::Active => 2.0,
            Lit::Live => 1.5,
            _ => 1.0,
        } * self.tf.k.max(0.6);
        let dir = if below { 1.0 } else { -1.0 };
        let (t0, t1) = (edge + dir * 2.0 * self.tf.k, edge + dir * 6.0 * self.tf.k);
        let mid = (x0 + x1) / 2.0;
        let stroke = Stroke::new(w, col);
        self.p.add(Shape::line(
            vec![
                Pos2::new(x0, t0),
                Pos2::new(x0, t1),
                Pos2::new(x1, t1),
                Pos2::new(x1, t0),
            ],
            stroke,
        ));
        if (lane - t1) * dir > 0.0 {
            self.p.add(Shape::line(
                vec![Pos2::new(mid, t1), Pos2::new(mid, lane)],
                stroke,
            ));
        }
        if lit == Lit::Active {
            // the sliced bits
            self.p.rect_filled(
                Rect::from_min_max(Pos2::new(x0, edge.min(t0)), Pos2::new(x1, edge.max(t0))),
                CornerRadius::ZERO,
                col,
            );
        }
        (Pos2::new(mid, lane), Pos2::new(x1, lane))
    }

    /// The tooltip for what the pointer is over, with the thing outlined.
    fn hover(&self, at: Option<Pos2>) -> Option<String> {
        let at = at?;
        let t = self.theme;
        let wire = self
            .wires
            .iter()
            .map(|(pts, tip)| {
                (
                    pts.windows(2)
                        .map(|w| segment_distance(at, w[0], w[1]))
                        .fold(f32::INFINITY, f32::min),
                    pts,
                    tip,
                )
            })
            .filter(|(d, _, _)| *d <= 5.0)
            .min_by(|a, b| a.0.total_cmp(&b.0));
        let part = self
            .hits
            .iter()
            .filter(|(r, _)| r.contains(at))
            .min_by(|a, b| a.0.area().total_cmp(&b.0.area()));
        if let Some((_, pts, tip)) =
            wire.filter(|_| part.map_or(true, |(r, _)| !r.shrink(4.0).contains(at)))
        {
            self.p.add(Shape::line(
                pts.clone(),
                Stroke::new(3.0_f32, t.strong_text_color.gamma_multiply(0.8)),
            ));
            return Some(tip.clone());
        }
        let (r, tip) = part?;
        self.p.rect_stroke(
            r.expand(2.0),
            CornerRadius::same(4),
            Stroke::new(1.5_f32, t.strong_text_color),
            StrokeKind::Outside,
        );
        Some(tip.clone())
    }
}

/// An orthogonal path from `a` (leaving through side `sa`) to `b` (arriving through `sb`).
fn route(a: Pos2, sa: Side, b: Pos2, sb: Side) -> Vec<Pos2> {
    let horiz = |s: Side| matches!(s, Side::Left | Side::Right);
    match (horiz(sa), horiz(sb)) {
        (true, true) => {
            let mx = (a.x + b.x) / 2.0;
            vec![a, Pos2::new(mx, a.y), Pos2::new(mx, b.y), b]
        }
        (false, false) => {
            let my = (a.y + b.y) / 2.0;
            vec![a, Pos2::new(a.x, my), Pos2::new(b.x, my), b]
        }
        (true, false) => vec![a, Pos2::new(b.x, a.y), b],
        (false, true) => vec![a, Pos2::new(a.x, b.y), b],
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
enum Side {
    Left,
    Right,
    Top,
    Bottom,
}

/// Which sides of two boxes face each other (preferring top and bottom when the flow is
/// vertical).
fn facing(a: Rect, b: Rect, vertical: bool) -> (Side, Side) {
    if vertical {
        if b.min.y >= a.max.y - 2.0 {
            return (Side::Bottom, Side::Top);
        }
        if b.max.y <= a.min.y + 2.0 {
            return (Side::Top, Side::Bottom);
        }
    }
    if b.min.x >= a.max.x - 2.0 {
        (Side::Right, Side::Left)
    } else if b.max.x <= a.min.x + 2.0 {
        (Side::Left, Side::Right)
    } else if b.min.y >= a.max.y {
        (Side::Bottom, Side::Top)
    } else {
        (Side::Top, Side::Bottom)
    }
}

/// Points along a side of a box, one per connection, ordered to avoid crossings.
fn port(r: Rect, side: Side, i: usize, n: usize) -> Pos2 {
    let f = (i as f32 + 1.0) / (n as f32 + 1.0);
    match side {
        Side::Left => Pos2::new(r.min.x, egui::lerp(r.min.y..=r.max.y, f)),
        Side::Right => Pos2::new(r.max.x, egui::lerp(r.min.y..=r.max.y, f)),
        Side::Top => Pos2::new(egui::lerp(r.min.x..=r.max.x, f), r.min.y),
        Side::Bottom => Pos2::new(egui::lerp(r.min.x..=r.max.x, f), r.max.y),
    }
}

/// A box (by its exact corners) and one of its sides.
type BoxSide = ([u32; 4], Side);
/// The connections ending on a box side: (edge, this end is the source, where the other end is
/// along the side).
type SideEnds = Vec<(usize, bool, f32)>;

/// The ends of each connection between boxes, spreading several on one side.
fn assign_ports(
    edges: &[(usize, Rect, Rect)],
    vertical: bool,
) -> HashMap<usize, (Pos2, Side, Pos2, Side)> {
    // (box, side) -> [(edge, end is source, coordinate of the other end along the side)]
    let mut sides: HashMap<BoxSide, SideEnds> = HashMap::new();
    let key = |r: Rect| {
        [
            r.min.x.to_bits(),
            r.min.y.to_bits(),
            r.max.x.to_bits(),
            r.max.y.to_bits(),
        ]
    };
    let mut out = HashMap::new();
    let mut sides_of: HashMap<usize, (Side, Side)> = HashMap::new();
    for &(i, a, b) in edges {
        let (sa, sb) = facing(a, b, vertical);
        sides_of.insert(i, (sa, sb));
        let along = |s: Side, other: Rect| {
            if matches!(s, Side::Left | Side::Right) {
                other.center().y
            } else {
                other.center().x
            }
        };
        sides
            .entry((key(a), sa))
            .or_default()
            .push((i, true, along(sa, b)));
        sides
            .entry((key(b), sb))
            .or_default()
            .push((i, false, along(sb, a)));
    }
    let mut ends: HashMap<(usize, bool), Pos2> = HashMap::new();
    let rect_of: HashMap<usize, (Rect, Rect)> =
        edges.iter().map(|&(i, a, b)| (i, (a, b))).collect();
    for ((_, side), mut list) in sides {
        list.sort_by(|a, b| a.2.total_cmp(&b.2));
        let n = list.len();
        for (j, (i, src, _)) in list.into_iter().enumerate() {
            let (a, b) = rect_of[&i];
            ends.insert((i, src), port(if src { a } else { b }, side, j, n));
        }
    }
    for (i, (sa, sb)) in sides_of {
        out.insert(i, (ends[&(i, true)], sa, ends[&(i, false)], sb));
    }
    out
}

// ───────────────────────── the model ─────────────────────────

const MW: f32 = 1240.0;
const MH: f32 = 805.0;
const BUS_Y: f32 = 400.0;
/// IR's top and bottom in the model; register numbers leave over the top, offsets under it.
const IR_TOP: f32 = 44.0;
const IR_BOTTOM: f32 = 82.0;
/// Over IR: (field, lane y, margin x it runs down). The leftmost field has the lane nearest IR,
/// so no lane crosses another field's stem.
const OVER: [((u8, u8), f32, f32); 5] = [
    ((15, 12), 36.0, 24.0),
    ((11, 9), 28.0, 19.0),
    ((8, 6), 20.0, 14.0),
    ((5, 5), 12.0, 9.0),
    ((2, 0), 4.0, 4.0),
];
/// Under IR: (field, lane y). Wider fields start further left and run deeper.
const UNDER: [((u8, u8), f32); 5] = [
    ((4, 0), 100.0),
    ((5, 0), 130.0),
    ((7, 0), 160.0),
    ((8, 0), 190.0),
    ((10, 0), 220.0),
];

/// Where each part goes in the model (canvas units).
fn model_rect(n: N) -> Rect {
    let r = |x0: f32, y0: f32, x1: f32, y1: f32| {
        Rect::from_min_max(Pos2::new(x0, y0), Pos2::new(x1, y1))
    };
    match n {
        // the instruction and its fields, top left
        N::Ir => r(30.0, IR_TOP, 390.0, IR_BOTTOM),
        N::Sext5 => r(420.0, 90.0, 485.0, 110.0),
        N::Sext6 => r(420.0, 120.0, 485.0, 140.0),
        N::Zext => r(420.0, 150.0, 485.0, 170.0),
        N::Sext9 => r(420.0, 180.0, 485.0, 200.0),
        N::Sext11 => r(420.0, 210.0, 485.0, 230.0),
        N::Zero => r(440.0, 245.0, 475.0, 265.0),
        // address arithmetic and the PC
        N::Addr2Mux => r(520.0, 180.0, 600.0, 260.0),
        N::Addr1Mux => r(520.0, 285.0, 600.0, 335.0),
        N::Adder => r(640.0, 185.0, 720.0, 275.0),
        N::PcMux => r(760.0, 205.0, 860.0, 250.0),
        N::Pc => r(760.0, 290.0, 860.0, 345.0),
        N::PcInc => r(872.0, 222.0, 908.0, 250.0),
        N::PcDec => r(872.0, 330.0, 908.0, 358.0),
        N::MarMux => r(880.0, 110.0, 960.0, 160.0),
        // memory, top right
        N::Memory => r(1000.0, 110.0, 1200.0, 210.0),
        N::Mar => r(1000.0, 290.0, 1080.0, 345.0),
        N::Mdr => r(1120.0, 290.0, 1200.0, 345.0),
        // registers and the ALU, under the bus
        N::RegFile => r(330.0, 430.0, 520.0, 650.0),
        N::Sr2Mux => r(540.0, 560.0, 620.0, 600.0),
        N::Alu => r(580.0, 670.0, 720.0, 740.0),
        N::Logic => r(760.0, 440.0, 850.0, 485.0),
        N::Psr => r(760.0, 530.0, 920.0, 590.0),
        N::SetPriv => r(935.0, 430.0, 1010.0, 478.0),
        N::IntCmp => r(950.0, 540.0, 1020.0, 580.0),
        N::KbdPriority => r(1050.0, 540.0, 1120.0, 580.0),
        N::Control => r(30.0, 715.0, 250.0, 785.0),
        // the stack pointer, under the register file
        N::SpInc => r(340.0, 672.0, 380.0, 700.0),
        N::SpDec => r(395.0, 672.0, 435.0, 700.0),
        N::SavedUsp => r(450.0, 672.0, 540.0, 705.0),
        N::SavedSsp => r(450.0, 715.0, 540.0, 748.0),
        N::SpMux => r(340.0, 740.0, 435.0, 775.0),
        // vector tables, bottom right
        N::TabX00 => r(960.0, 630.0, 1000.0, 652.0),
        N::TabX01 => r(960.0, 660.0, 1000.0, 682.0),
        N::TableMux => r(1020.0, 630.0, 1080.0, 682.0),
        N::Table => r(1100.0, 630.0, 1160.0, 682.0),
        N::VecIntv => r(960.0, 700.0, 1010.0, 720.0),
        N::VecX00 => r(960.0, 724.0, 1010.0, 744.0),
        N::VecX01 => r(960.0, 748.0, 1010.0, 768.0),
        N::VecX02 => r(960.0, 772.0, 1010.0, 792.0),
        N::VectorMux => r(1030.0, 725.0, 1090.0, 770.0),
        N::VectorSel => r(1110.0, 725.0, 1165.0, 770.0),
        N::Vector => r(1180.0, 630.0, 1235.0, 682.0),
        N::Bus => r(15.0, BUS_Y - 6.0, MW - 5.0, BUS_Y + 6.0),
    }
}

/// Connections drawn along a set route, where the straight one would run through other parts.
fn model_route(e: &E, tf: Tf, rect: &dyn Fn(N) -> Rect) -> Option<Vec<Pos2>> {
    let p = |x: f32, y: f32| tf.p(x, y);
    let bus = rect(N::Bus);
    Some(match (e.from, e.to) {
        // the bus up the left of the PC into PCMUX
        (N::Bus, N::PcMux) => vec![
            Pos2::new(p(745.0, 0.0).x, bus.min.y),
            p(745.0, 240.0),
            p(760.0, 240.0),
        ],
        // the bus up into IR, left of the fields
        (N::Bus, N::Ir) => vec![Pos2::new(p(120.0, 0.0).x, bus.min.y), p(120.0, IR_BOTTOM)],
        // SPMUX round the left of the register file to the bus
        (N::SpMux, N::Bus) => vec![
            p(340.0, 757.0),
            p(310.0, 757.0),
            Pos2::new(p(310.0, 0.0).x, bus.max.y),
        ],
        // imm5 over the address muxes, down between them and the adder, across the bus
        (N::Sext5, N::Sr2Mux) => vec![
            p(485.0, 100.0),
            p(625.0, 100.0),
            p(625.0, 580.0),
            p(620.0, 580.0),
        ],
        // interrupts and BEN into control, along the bottom
        (N::IntCmp, N::Control) => vec![
            p(985.0, 580.0),
            p(985.0, 795.0),
            p(200.0, 795.0),
            p(200.0, 785.0),
        ],
        (N::Psr, N::Control) => vec![
            p(800.0, 590.0),
            p(800.0, 800.0),
            p(150.0, 800.0),
            p(150.0, 785.0),
        ],
        (N::Psr, N::IntCmp) => vec![p(920.0, 570.0), p(950.0, 570.0)],
        // into MARMUX over the top of PCMUX: the adder's result, and ZEXT above it
        (N::Adder, N::MarMux) => vec![
            p(720.0, 200.0),
            p(745.0, 200.0),
            p(745.0, 145.0),
            p(880.0, 145.0),
        ],
        (N::Zext, N::MarMux) => vec![
            p(485.0, 160.0),
            p(730.0, 160.0),
            p(730.0, 125.0),
            p(880.0, 125.0),
        ],
        _ => return None,
    })
}

/// Draw the model: every part, values in place, registers editable.
pub fn draw_model(
    ui: &mut egui::Ui,
    area: Rect,
    theme: &ThemeSettings,
    c: &Ctx<'_>,
    emulator: &mut Emulator,
) -> Option<String> {
    let tf = Tf::fit(area, MW, MH, 1.6);
    let mut pt = Painter {
        p: ui.painter_at(area),
        theme,
        tf,
        hits: Vec::new(),
        wires: Vec::new(),
        chips: Vec::new(),
        blocked: Vec::new(),
    };
    let used = c.used_bits();
    let lits: Vec<Lit> = EDGES.iter().map(|e| c.lit(e)).collect();
    let node_lit = |n: N| {
        let mut l = Lit::Idle;
        for (e, lit) in EDGES.iter().zip(&lits) {
            if e.from == n || e.to == n {
                // a part that a live wire touches is working (the true state)
                l = match (l, lit) {
                    (Lit::Active, _) | (_, Lit::Active) | (_, Lit::Live) => Lit::Active,
                    (Lit::Trail, _) | (_, Lit::Trail) => Lit::Trail,
                    _ => Lit::Idle,
                };
            }
        }
        l
    };

    // the bus
    {
        let r = tf.r(model_rect(N::Bus));
        let l = node_lit(N::Bus);
        let col = if l == Lit::Active {
            theme.accent_color_primary.gamma_multiply(0.35)
        } else {
            theme.secondary_text_color.gamma_multiply(0.25)
        };
        pt.p.rect_filled(r, CornerRadius::same(5), col);
        let (value, _) = content(N::Bus, c);
        let label = if value.is_empty() {
            "BUS".to_string()
        } else {
            format!("BUS  {value}")
        };
        pt.p.text(
            r.left_center() + vec2(6.0, 0.0),
            Align2::LEFT_CENTER,
            label,
            FontId::monospace(pt.fs(9.0)),
            if l == Lit::Active {
                theme.strong_text_color
            } else {
                theme.secondary_text_color
            },
        );
        pt.hits.push((r, help(N::Bus).into()));
    }

    // connections between boxes (not the bus or IR slices)
    let rect = |n: N| tf.r(model_rect(n));
    let boxed: Vec<(usize, Rect, Rect)> = EDGES
        .iter()
        .enumerate()
        .filter(|(_, e)| {
            e.from != N::Bus
                && e.to != N::Bus
                && !matches!(e.val, V::Bits(..))
                && model_route(e, tf, &rect).is_none()
        })
        .map(|(i, e)| (i, rect(e.from), rect(e.to)))
        .collect();
    let ports = assign_ports(&boxed, false);
    // bus connections: straight up or down, spread along the part's side facing the bus
    let mut bus_side: HashMap<N, Vec<usize>> = HashMap::new();
    let custom = |e: &E| model_route(e, tf, &rect).is_some();
    for (i, e) in EDGES.iter().enumerate() {
        if custom(e) {
            continue;
        }
        if e.from == N::Bus {
            bus_side.entry(e.to).or_default().push(i);
        } else if e.to == N::Bus {
            bus_side.entry(e.from).or_default().push(i);
        }
    }
    let bus = rect(N::Bus);
    let mut paths: Vec<(usize, Vec<Pos2>)> = Vec::new();
    for (n, list) in &bus_side {
        let r = rect(*n);
        let above = r.center().y < bus.center().y;
        let count = list.len();
        for (j, &i) in list.iter().enumerate() {
            let f = (j as f32 + 1.0) / (count as f32 + 1.0);
            let x = egui::lerp(r.min.x..=r.max.x, f);
            let (near, far) = if above {
                (Pos2::new(x, r.max.y), Pos2::new(x, bus.min.y))
            } else {
                (Pos2::new(x, r.min.y), Pos2::new(x, bus.max.y))
            };
            let pts = if EDGES[i].from == N::Bus {
                vec![far, near]
            } else {
                vec![near, far]
            };
            paths.push((i, pts));
        }
    }
    for (i, _, _) in &boxed {
        let (a, sa, b, sb) = ports[i];
        paths.push((*i, route(a, sa, b, sb)));
    }
    for (i, e) in EDGES.iter().enumerate() {
        if let Some(pts) = model_route(e, tf, &rect) {
            paths.push((i, pts));
        }
    }

    // IR slices: register numbers, the opcode and IR[5] over the bits, offsets under them
    let ir = rect(N::Ir);
    for (i, e) in EDGES.iter().enumerate() {
        let V::Bits(hi, lo) = e.val else { continue };
        let lit = lits[i];
        let dst = rect(e.to);
        let pts = if let Some((_, lane, margin)) = OVER.iter().find(|(f, _, _)| *f == (hi, lo)) {
            // over the top, out to the left margin and down
            let (stem, _) = pt.bracket(ir, hi, lo, tf.p(0.0, *lane).y, lit, false);
            let x = tf.p(*margin, 0.0).x;
            let (tx, ty) = match (e.to, hi) {
                (N::RegFile, 11) => (dst.min.x, egui::lerp(dst.min.y..=dst.max.y, 0.14)),
                (N::RegFile, 8) => (dst.min.x, egui::lerp(dst.min.y..=dst.max.y, 0.24)),
                (N::RegFile, _) => (dst.min.x, egui::lerp(dst.min.y..=dst.max.y, 0.34)),
                _ => (dst.min.x, dst.center().y),
            };
            if e.to == N::Sr2Mux {
                // under the register file, up into SR2MUX
                let y = tf.p(0.0, 660.0).y;
                let ux = tf.p(580.0, 0.0).x;
                vec![
                    stem,
                    Pos2::new(x, stem.y),
                    Pos2::new(x, y),
                    Pos2::new(ux, y),
                    Pos2::new(ux, dst.max.y),
                ]
            } else {
                vec![
                    stem,
                    Pos2::new(x, stem.y),
                    Pos2::new(x, ty),
                    Pos2::new(tx, ty),
                ]
            }
        } else if let Some((_, lane)) = UNDER.iter().find(|(f, _)| *f == (hi, lo)) {
            let lane_y = tf.p(0.0, *lane).y;
            let (stem, _) = pt.bracket(ir, hi, lo, lane_y, lit, true);
            if e.to == N::VectorSel {
                // trapvect8 also goes to the vector tables: down past the bus and along the bottom
                let x = tf.p(405.0, 0.0).x;
                let (bottom, right) = (tf.p(0.0, 803.0).y, tf.p(1180.0, 0.0).x);
                vec![
                    stem,
                    Pos2::new(x, lane_y),
                    Pos2::new(x, bottom),
                    Pos2::new(right, bottom),
                    Pos2::new(right, dst.center().y),
                    Pos2::new(dst.max.x, dst.center().y),
                ]
            } else {
                vec![
                    stem,
                    Pos2::new(dst.min.x, lane_y.clamp(dst.min.y + 2.0, dst.max.y - 2.0)),
                ]
            }
        } else {
            continue;
        };
        paths.push((i, pts));
    }

    pt.blocked = ALL_NODES
        .iter()
        .filter(|n| **n != N::Bus)
        .map(|n| rect(*n))
        .collect();
    // wires: idle, then trail, then active on top
    for pass in [Lit::Idle, Lit::ToGate, Lit::Trail, Lit::Live, Lit::Active] {
        for (i, pts) in &paths {
            let e = &EDGES[*i];
            if lits[*i] != pass {
                continue;
            }
            let value = c.value(e, lits[*i]);
            let tip = format!("{} → {}", name(e.from), name(e.to));
            pt.wire(pts.clone(), lits[*i], value, e.sig, tip, c.flow_time);
        }
    }

    // parts
    let editable = [N::Pc, N::Mar, N::Mdr];
    let mut edits: Vec<(N, Rect, u16, bool)> = Vec::new();
    for n in ALL_NODES {
        if matches!(n, N::Bus) {
            continue;
        }
        let r = rect(n);
        let l = node_lit(n);
        if n == N::Ir {
            pt.ir(r, c.snap.ir, c.ir_addr, used, l);
            continue;
        }
        if n == N::RegFile {
            reg_file(&mut pt, r, c, l, true);
            continue;
        }
        let (value, detail) = content(n, c);
        if editable.contains(&n) {
            // the editor draws the value on the second line
            pt.node(n, r, l, " ", &detail, None);
            let v = match n {
                N::Pc => c.snap.pc,
                N::Mar => c.snap.mar,
                _ => c.snap.mdr,
            };
            edits.push((n, r, v, !detail.is_empty()));
        } else {
            pt.node(n, r, l, &value, &detail, None);
        }
    }

    // edit in place: PC, MAR, MDR and the registers
    let base = ui_kit::Base::Hex;
    let size = (11.5 * tf.k).clamp(7.0, 16.0);
    for (n, r, v, has_detail) in edits {
        // sized to the four digits, so the value sits centred like the other parts'
        let w = (size * 0.62 * 4.0 + 10.0).min(r.width() - 4.0);
        // the value line: second of two lines, or the middle of three
        let y = r.min.y + r.height() * if has_detail { 0.5 } else { 0.75 };
        let cell = Rect::from_center_size(Pos2::new(r.center().x, y), vec2(w, size * 1.55));
        let mut child = ui.new_child(
            egui::UiBuilder::new()
                .max_rect(cell)
                .layout(egui::Layout::left_to_right(egui::Align::Center)),
        );
        let (resp, new) = ui_kit::editable_word_sized(
            &mut child,
            egui::Id::new(("model-edit", n)),
            v,
            base,
            w,
            theme.strong_text_color,
            None,
            size,
        );
        resp.explain(format!("{}\nClick to edit", help(n)));
        if let Some(v) = new {
            match n {
                N::Pc => emulator.pc.set(v),
                N::Mar => emulator.mar.set(v),
                _ => emulator.mdr.set(v),
            }
        }
    }
    let rf = rect(N::RegFile);
    for i in 0..8 {
        let row = reg_row(rf, i);
        let cell = Rect::from_min_max(
            Pos2::new(row.center().x - 4.0, row.min.y),
            Pos2::new(row.max.x - 4.0, row.max.y),
        );
        let mut child = ui.new_child(
            egui::UiBuilder::new()
                .max_rect(cell)
                .layout(egui::Layout::left_to_right(egui::Align::Center)),
        );
        let rsize = size.min(row.height() * 0.62);
        let (_, new) = ui_kit::editable_word_sized(
            &mut child,
            egui::Id::new(("model-reg", i)),
            c.snap.r[i],
            base,
            cell.width(),
            theme.primary_text_color,
            None,
            rsize,
        );
        if let Some(v) = new {
            emulator.r[i].set(v);
        }
    }

    pt.hover(c.hover)
}

const ALL_NODES: [N; 44] = [
    N::Ir,
    N::Control,
    N::Pc,
    N::PcInc,
    N::PcDec,
    N::PcMux,
    N::Zext,
    N::Sext11,
    N::Sext9,
    N::Sext6,
    N::Sext5,
    N::Zero,
    N::Addr1Mux,
    N::Addr2Mux,
    N::Adder,
    N::MarMux,
    N::RegFile,
    N::Sr2Mux,
    N::Alu,
    N::Logic,
    N::Psr,
    N::SetPriv,
    N::KbdPriority,
    N::IntCmp,
    N::Bus,
    N::Mar,
    N::Mdr,
    N::Memory,
    N::SpInc,
    N::SpDec,
    N::SavedUsp,
    N::SavedSsp,
    N::SpMux,
    N::TabX00,
    N::TabX01,
    N::TableMux,
    N::Table,
    N::VecIntv,
    N::VecX00,
    N::VecX01,
    N::VecX02,
    N::VectorMux,
    N::VectorSel,
    N::Vector,
];

fn reg_row(r: Rect, i: usize) -> Rect {
    let top = r.min.y + r.height() * 0.13;
    let h = (r.max.y - top) / 8.0;
    Rect::from_min_size(
        Pos2::new(r.min.x + 4.0, top + i as f32 * h),
        vec2(r.width() - 8.0, h),
    )
}

/// The register file: R0–R7, the ones read outlined, the one written filled. In the model the
/// values are drawn by the editors on top.
fn reg_file(pt: &mut Painter<'_>, r: Rect, c: &Ctx<'_>, lit: Lit, editors: bool) {
    let t = pt.theme;
    let (fill, stroke) = pt.colors(lit);
    pt.p.rect(r, CornerRadius::same(4), fill, stroke, StrokeKind::Inside);
    pt.p.text(
        Pos2::new(r.center().x, r.min.y + r.height() * 0.065),
        Align2::CENTER_CENTER,
        "REG FILE",
        FontId::proportional(pt.fs(9.5)),
        if lit == Lit::Active {
            t.accent_color_primary
        } else {
            t.secondary_text_color
        },
    );
    let read = c.rec.map_or(0, |r| r.regs_read);
    let written = c.rec.and_then(|r| r.reg_written);
    for i in 0..8 {
        let row = reg_row(r, i);
        let (is_r, is_w) = (read >> i & 1 == 1, written == Some(i as u8));
        if is_w {
            pt.p.rect_filled(
                row.shrink(1.0),
                CornerRadius::same(2),
                t.accent_color_primary.gamma_multiply(0.35),
            );
        } else if is_r {
            pt.p.rect_stroke(
                row.shrink(1.0),
                CornerRadius::same(2),
                Stroke::new(1.3_f32, t.accent_color_secondary),
                StrokeKind::Inside,
            );
        }
        let size = pt.fs(10.0).min(row.height() * 0.8);
        pt.p.text(
            Pos2::new(row.min.x + 6.0, row.center().y),
            Align2::LEFT_CENTER,
            format!("R{i}"),
            FontId::monospace(size),
            t.register_name_color,
        );
        if !editors {
            pt.p.text(
                Pos2::new(row.max.x - 6.0, row.center().y),
                Align2::RIGHT_CENTER,
                format!("x{:04X}", c.snap.r[i]),
                FontId::monospace(size),
                t.primary_text_color,
            );
        }
        let role = match (is_r, is_w) {
            (_, true) => "  written",
            (true, _) => "  read",
            _ => "",
        };
        pt.hits.push((
            row,
            format!("R{i} = {}{role}", disasm::value_text(c.snap.r[i])),
        ));
    }
    pt.hits.push((r, help(N::RegFile).into()));
}

// ───────────────────────── the focus view ─────────────────────────

/// A part in the focus view; a register read and written in this state appears twice.
type Key = (N, bool);

/// Draw only what this state uses, in the order data flows (left to right).
pub fn draw_focus(
    ui: &mut egui::Ui,
    area: Rect,
    theme: &ThemeSettings,
    c: &Ctx<'_>,
) -> Option<String> {
    let lits: Vec<Lit> = EDGES.iter().map(|e| c.lit(e)).collect();
    let active: Vec<usize> = (0..EDGES.len())
        .filter(|i| lits[*i] == Lit::Active)
        .collect();
    if active.is_empty() {
        let p = ui.painter_at(area);
        let text = if c.rec.is_some() {
            "This state only tests a condition: nothing moves on the datapath"
        } else {
            "Step an instruction to see what each state does"
        };
        p.text(
            area.center(),
            Align2::CENTER_CENTER,
            text,
            FontId::proportional(13.0),
            theme.secondary_text_color,
        );
        return None;
    }
    let sources: Vec<N> = active.iter().map(|i| EDGES[*i].from).collect();
    let key_from = |n: N| -> Key { (n, false) };
    let split = |n: N| is_register(n) && sources.contains(&n);
    let state = c.rec.map_or(0, |r| r.state);
    // the end a connection arrives at: a register read and written here has two; register
    // numbers select what is read, except DR (IR[11:9], unless a store reads it in state 23)
    let key_of = |e: &E| -> Key {
        let reads = match e.val {
            V::Bits(hi, lo) => e.to == N::RegFile && ((hi, lo) != (11, 9) || state == 23),
            _ => false,
        };
        (e.to, split(e.to) && !reads)
    };
    let mut nodes: Vec<Key> = Vec::new();
    for &i in &active {
        for k in [key_from(EDGES[i].from), key_of(&EDGES[i])] {
            if !nodes.contains(&k) {
                nodes.push(k);
            }
        }
    }
    nodes.sort();
    // layers: the longest path from a source
    let mut layer: HashMap<Key, usize> = nodes.iter().map(|k| (*k, 0)).collect();
    for _ in 0..nodes.len() {
        let mut changed = false;
        for &i in &active {
            let (a, b) = (key_from(EDGES[i].from), key_of(&EDGES[i]));
            if a == b {
                continue;
            }
            let want = layer[&a] + 1;
            if layer[&b] < want && want < nodes.len() {
                layer.insert(b, want);
                changed = true;
            }
        }
        if !changed {
            break;
        }
    }
    let layers = layer.values().max().copied().unwrap_or(0) + 1;
    let mut cols: Vec<Vec<Key>> = vec![Vec::new(); layers];
    for k in &nodes {
        cols[layer[k]].push(*k);
    }
    // order each column by where its inputs are
    for l in 1..layers {
        let prev = cols[l - 1].clone();
        let pos = |k: &Key| -> f32 {
            let ins: Vec<f32> = active
                .iter()
                .filter(|i| key_of(&EDGES[**i]) == *k)
                .filter_map(|i| prev.iter().position(|p| *p == key_from(EDGES[*i].from)))
                .map(|p| p as f32)
                .collect();
            if ins.is_empty() {
                f32::MAX / 2.0
            } else {
                ins.iter().sum::<f32>() / ins.len() as f32
            }
        };
        cols[l].sort_by(|a, b| pos(a).total_cmp(&pos(b)));
    }

    // the IR fields this state slices out
    let ir_slices = {
        let mut v: Vec<(u8, u8)> = active
            .iter()
            .filter_map(|i| {
                if let V::Bits(h, l) = EDGES[*i].val {
                    Some((h, l))
                } else {
                    None
                }
            })
            .collect();
        v.sort();
        v.dedup();
        v.len()
    };
    // sizes (canvas units)
    let size = |k: &Key, vertical: bool| -> egui::Vec2 {
        match k.0 {
            N::Ir => vec2(
                260.0,
                if vertical {
                    84.0
                } else {
                    74.0 + 11.0 * ir_slices as f32
                },
            ),
            N::RegFile => vec2(170.0, 40.0 + 26.0 * reg_rows(c, k.1).len().max(1) as f32),
            N::Bus => vec2(120.0, 60.0),
            N::Memory | N::Alu | N::Adder | N::Psr | N::Logic => vec2(190.0, 76.0),
            _ => vec2(150.0, 66.0),
        }
    };
    // the flow runs left to right or top to bottom, whichever draws it bigger
    let extent = |vertical: bool| {
        let (gap_main, gap_cross) = if vertical { (60.0, 26.0) } else { (80.0, 34.0) };
        let main_of = |v: egui::Vec2| if vertical { v.y } else { v.x };
        let cross_of = |v: egui::Vec2| if vertical { v.x } else { v.y };
        let main_len: Vec<f32> = cols
            .iter()
            .map(|col| {
                col.iter()
                    .map(|k| main_of(size(k, vertical)))
                    .fold(0.0, f32::max)
            })
            .collect();
        let cross_len: Vec<f32> = cols
            .iter()
            .map(|col| {
                col.iter()
                    .map(|k| cross_of(size(k, vertical)) + gap_cross)
                    .sum::<f32>()
                    - gap_cross
            })
            .collect();
        let total_main = main_len.iter().sum::<f32>() + gap_main * (layers as f32 - 1.0) + 40.0;
        let total_cross = cross_len.iter().copied().fold(0.0, f32::max) + 40.0;
        let (cw, ch) = if vertical {
            (total_cross, total_main)
        } else {
            (total_main, total_cross)
        };
        let k = (area.width() / cw).min(area.height() / ch);
        (
            k,
            cw,
            ch,
            main_len,
            cross_len,
            total_cross,
            gap_main,
            gap_cross,
        )
    };
    let (kh, ..) = extent(false);
    let (kv, ..) = extent(true);
    let vertical = kv > kh * 1.05;
    let (_, cw, ch, main_len, cross_len, total_cross, gap_main, gap_cross) = extent(vertical);
    let main_of = |v: egui::Vec2| if vertical { v.y } else { v.x };
    let cross_of = |v: egui::Vec2| if vertical { v.x } else { v.y };
    let size = |k: &Key| size(k, vertical);
    let _ = (cw, ch, cross_len, total_cross);
    // place row by row (or column by column): each part as near as it can be to the middle of
    // what feeds it (for an IR field, its bits), so wires run straight and don't cross
    let mut canvas: HashMap<Key, Rect> = HashMap::new();
    let mut m = 0.0;
    for (l, col) in cols.iter().enumerate() {
        let mut items: Vec<(Key, f32)> = col
            .iter()
            .map(|k| {
                let feeds: Vec<f32> = active
                    .iter()
                    .filter(|i| key_of(&EDGES[**i]) == *k)
                    .filter_map(|i| {
                        let e = &EDGES[*i];
                        let from = canvas.get(&key_from(e.from))?;
                        Some(match e.val {
                            V::Bits(hi, lo) if vertical => {
                                from.min.x
                                    + 8.0
                                    + (15.5 - (hi as f32 + lo as f32) / 2.0) * (from.width() - 16.0)
                                        / 16.0
                            }
                            _ if vertical => from.center().x,
                            _ => from.center().y,
                        })
                    })
                    .collect();
                let want = if feeds.is_empty() {
                    f32::NAN
                } else {
                    feeds.iter().sum::<f32>() / feeds.len() as f32
                };
                (*k, want)
            })
            .collect();
        items.sort_by(|a, b| match (a.1.is_nan(), b.1.is_nan()) {
            (false, false) => a.1.total_cmp(&b.1),
            (x, y) => x.cmp(&y),
        });
        // left to right without overlapping, each as close to where it wants to be as it can
        let mut end: Option<f32> = None;
        for (k, want) in items {
            let s = size(&k);
            let start = match (want.is_nan(), end) {
                (true, None) => 0.0,
                (true, Some(e)) => e + gap_cross,
                (false, None) => want - cross_of(s) / 2.0,
                (false, Some(e)) => (want - cross_of(s) / 2.0).max(e + gap_cross),
            };
            end = Some(start + cross_of(s));
            let along = m + (main_len[l] - main_of(s)) / 2.0;
            let r = if vertical {
                Rect::from_min_size(Pos2::new(start, along), s)
            } else {
                Rect::from_min_size(Pos2::new(along, start), s)
            };
            canvas.insert(k, r);
        }
        m += main_len[l] + gap_main;
    }
    // fit what was placed, with room at the sides for wires that skip a row
    let bbox = canvas.values().fold(Rect::NOTHING, |a, r| a.union(*r));
    let margin = if vertical {
        vec2(46.0, 16.0)
    } else {
        vec2(16.0, 46.0)
    };
    let bbox = bbox.expand2(margin);
    let tf = Tf::fit(area, bbox.width(), bbox.height(), 1.5);
    let rects: HashMap<Key, Rect> = canvas
        .iter()
        .map(|(k, r)| (*k, tf.r(r.translate(-bbox.min.to_vec2()))))
        .collect();

    let mut pt = Painter {
        p: ui.painter_at(area),
        theme,
        tf,
        hits: Vec::new(),
        wires: Vec::new(),
        chips: Vec::new(),
        blocked: rects.values().copied().collect(),
    };
    let used = c.used_bits();

    // IR slices leave IR's right side from the bracket under their bits
    let mut ir_lanes: HashMap<(u8, u8), Pos2> = HashMap::new();
    if let Some(ir) = rects.get(&(N::Ir, false)).copied() {
        let bits = Rect::from_min_max(
            ir.min + vec2(8.0, 22.0) * tf.k,
            Pos2::new(ir.max.x - 8.0 * tf.k, ir.min.y + 58.0 * tf.k),
        );
        let mut slices: Vec<(u8, u8)> = active
            .iter()
            .filter_map(|i| {
                if let V::Bits(h, l) = EDGES[*i].val {
                    Some((h, l))
                } else {
                    None
                }
            })
            .collect();
        slices.sort_by_key(|(h, l)| (*h as i32 - *l as i32, *h));
        slices.dedup();
        for (j, (hi, lo)) in slices.iter().enumerate() {
            // stacked lanes to leave by the right; straight down when the flow is vertical
            let lane = if vertical {
                bits.max.y + 8.0 * tf.k
            } else {
                bits.max.y + (12.0 + 11.0 * j as f32) * tf.k
            };
            let (stem, _) = pt.bracket(bits, *hi, *lo, lane, Lit::Active, true);
            ir_lanes.insert((*hi, *lo), stem);
        }
        pt.ir(bits, c.snap.ir, c.ir_addr, used, Lit::Active);
        pt.p.text(
            ir.min + vec2(8.0, 10.0) * tf.k,
            Align2::LEFT_CENTER,
            "IR",
            FontId::proportional(pt.fs(10.0)),
            theme.accent_color_primary,
        );
        pt.p.text(
            Pos2::new(ir.max.x - 8.0 * tf.k, ir.min.y + 10.0 * tf.k),
            Align2::RIGHT_CENTER,
            disasm::disassemble(c.snap.ir, c.ir_addr, &HashMap::new()),
            FontId::monospace(pt.fs(9.0)),
            theme.primary_text_color,
        );
        pt.p.rect_stroke(
            ir,
            CornerRadius::same(6),
            Stroke::new(1.0_f32, theme.separator_color),
            StrokeKind::Inside,
        );
    }

    // connections; an IR field's wire starts where it leaves IR (a point) so its end shares the
    // spread of ports with the other wires into the same part
    let ir_exit = |i: usize| -> Option<Rect> {
        let V::Bits(hi, lo) = EDGES[i].val else {
            return None;
        };
        let end = *ir_lanes.get(&(hi, lo))?;
        let ir = rects[&(N::Ir, false)];
        let at = if vertical {
            Pos2::new(end.x, ir.max.y + 6.0 * tf.k)
        } else {
            Pos2::new(ir.max.x + 10.0 * tf.k, end.y)
        };
        Some(Rect::from_center_size(at, vec2(0.5, 0.5)))
    };
    let mut boxed: Vec<(usize, Rect, Rect)> = Vec::new();
    for &i in &active {
        let e = &EDGES[i];
        let from = if matches!(e.val, V::Bits(..)) {
            match ir_exit(i) {
                Some(r) => r,
                None => continue,
            }
        } else {
            rects[&key_from(e.from)]
        };
        boxed.push((i, from, rects[&key_of(e)]));
    }
    let ports = assign_ports(&boxed, vertical);
    // a wire that skips a row goes round the side rather than through the parts in between
    let placed = rects.values().fold(Rect::NOTHING, |a, r| a.union(*r));
    let layer_of_from = |e: &E| {
        if matches!(e.val, V::Bits(..)) {
            layer[&(N::Ir, false)]
        } else {
            layer[&key_from(e.from)]
        }
    };
    let mut channels = (0usize, 0usize);
    let _ = layer_of_from;
    // does a path run through a part other than the two it joins?
    let crosses = |e: &E, pts: &[Pos2]| -> bool {
        let ends = [key_from(e.from), key_of(e)];
        rects
            .iter()
            .filter(|(k, _)| !ends.contains(k))
            .any(|(_, r)| {
                let inner = r.shrink(2.0);
                pts.windows(2)
                    .any(|w| inner.intersects(Rect::from_two_pos(w[0], w[1]).expand(0.5)))
            })
    };
    let blockers = |e: &E, pts: &[Pos2]| -> Vec<Rect> {
        let ends = [key_from(e.from), key_of(e)];
        rects
            .iter()
            .filter(|(k, _)| !ends.contains(k))
            .map(|(_, r)| *r)
            .filter(|r| {
                pts.windows(2).any(|w| {
                    r.shrink(2.0)
                        .intersects(Rect::from_two_pos(w[0], w[1]).expand(0.5))
                })
            })
            .collect()
    };
    let mut detour = |e: &E, a: Pos2, b: Pos2, straight: &[Pos2]| -> Option<Vec<Pos2>> {
        // only when the direct way is blocked
        if !crosses(e, straight) {
            return None;
        }
        // first try stepping just past the side of whatever is in the way
        let gap = 8.0 * tf.k;
        let off = 9.0 * tf.k;
        let mut best: Option<(f32, Vec<Pos2>)> = None;
        for r in blockers(e, straight) {
            let sides = if vertical {
                [r.min.x - gap, r.max.x + gap]
            } else {
                [r.min.y - gap, r.max.y + gap]
            };
            for j in sides {
                let pts = if vertical {
                    vec![
                        a,
                        Pos2::new(a.x, a.y + off),
                        Pos2::new(j, a.y + off),
                        Pos2::new(j, b.y - off),
                        Pos2::new(b.x, b.y - off),
                        b,
                    ]
                } else {
                    vec![
                        a,
                        Pos2::new(a.x + off, a.y),
                        Pos2::new(a.x + off, j),
                        Pos2::new(b.x - off, j),
                        Pos2::new(b.x - off, b.y),
                        b,
                    ]
                };
                if crosses(e, &pts) {
                    continue;
                }
                let len: f32 = pts.windows(2).map(|w| w[0].distance(w[1])).sum();
                if best.as_ref().map_or(true, |(l, _)| len < *l) {
                    best = Some((len, pts));
                }
            }
        }
        if let Some((_, pts)) = best {
            return Some(pts);
        }
        // each detour leaves and arrives at its own height, so none share a stretch
        let off = (9.0 + 6.0 * (channels.0 + channels.1) as f32) * tf.k;
        Some(if vertical {
            let left = (a.x + b.x) / 2.0 < placed.center().x;
            let n = if left {
                &mut channels.0
            } else {
                &mut channels.1
            };
            *n += 1;
            let cx = if left {
                placed.min.x - (10.0 + 8.0 * *n as f32) * tf.k
            } else {
                placed.max.x + (10.0 + 8.0 * *n as f32) * tf.k
            };
            vec![
                a,
                Pos2::new(a.x, a.y + off),
                Pos2::new(cx, a.y + off),
                Pos2::new(cx, b.y - off),
                Pos2::new(b.x, b.y - off),
                b,
            ]
        } else {
            let top = (a.y + b.y) / 2.0 < placed.center().y;
            let n = if top {
                &mut channels.0
            } else {
                &mut channels.1
            };
            *n += 1;
            let cy = if top {
                placed.min.y - (10.0 + 8.0 * *n as f32) * tf.k
            } else {
                placed.max.y + (10.0 + 8.0 * *n as f32) * tf.k
            };
            vec![
                a,
                Pos2::new(a.x + off, a.y),
                Pos2::new(a.x + off, cy),
                Pos2::new(b.x - off, cy),
                Pos2::new(b.x - off, b.y),
                b,
            ]
        })
    };
    for &i in &active {
        let e = &EDGES[i];
        let dst = rects[&key_of(e)];
        let _ = dst;
        let pts = if let V::Bits(hi, lo) = e.val {
            let Some(end) = ir_lanes.get(&(hi, lo)).copied() else {
                continue;
            };
            let Some((a, sa, b, sb)) = ports.get(&i).copied() else {
                continue;
            };
            // from the bracket, out of IR, then on to the part
            let mut v = vec![end];
            let straight = route(a, sa, b, sb);
            v.extend(detour(e, a, b, &straight).unwrap_or(straight));
            v
        } else {
            let (a, sa, b, sb) = ports[&i];
            let straight = route(a, sa, b, sb);
            detour(e, a, b, &straight).unwrap_or(straight)
        };
        let value = c.value(e, Lit::Active);
        let tip = format!("{} → {}", name(e.from), name(e.to));
        pt.wire(pts, Lit::Active, value, e.sig, tip, c.flow_time);
    }

    // parts, with what they do
    for k in &nodes {
        let r = rects[k];
        let n = k.0;
        match n {
            N::Ir => {}
            N::RegFile => focus_reg_file(&mut pt, r, c, k.1),
            _ => {
                let (mut value, mut detail) = content(n, c);
                // the value at the start of the state, for a register this state also loads
                let before = |p: &Snapshot| match n {
                    N::Pc => Some(p.pc),
                    N::Mar => Some(p.mar),
                    N::Mdr => Some(p.mdr),
                    _ => None,
                };
                let old = c.prev.as_ref().and_then(before);
                let title =
                    if is_register(n) && sources.contains(&n) && !k.1 && nodes.contains(&(n, true))
                    {
                        if let Some(v) = old {
                            value = format!("x{v:04X}");
                        }
                        detail = "at the start of the state".into();
                        Some(format!("{} (before)", name(n)))
                    } else if k.1 {
                        let now = before(&c.snap);
                        detail = match (old, now) {
                            (Some(o), Some(v)) if o != v => format!("was x{o:04X}"),
                            (Some(_), Some(_)) => "unchanged".into(),
                            _ => "loaded at the end of the state".into(),
                        };
                        Some(format!("{} (after)", name(n)))
                    } else {
                        None
                    };
                if detail.is_empty() {
                    detail = focus_detail(n, c);
                }
                pt.node(n, r, Lit::Active, &value, &detail, title.as_deref());
            }
        }
    }
    pt.hover(c.hover)
}

/// A line on what a part does in this state.
fn focus_detail(n: N, c: &Ctx<'_>) -> String {
    let a = c.act();
    match n {
        N::Bus => {
            let gate = [
                (Sig::GatePc, "GatePC"),
                (Sig::GateMdr, "GateMDR"),
                (Sig::GateAlu, "GateALU"),
                (Sig::GateMarMux, "GateMARMUX"),
                (Sig::GatePcMinus1, "GatePC-1"),
                (Sig::GateSp, "GateSP"),
                (Sig::GatePsr, "GatePSR"),
                (Sig::GateVector, "GateVector"),
            ]
            .iter()
            .find(|(s, _)| a.signal(*s))
            .map(|(_, n)| *n)
            .unwrap_or("");
            if gate.is_empty() {
                String::new()
            } else {
                format!("driven by {gate}")
            }
        }
        N::PcMux | N::MarMux | N::Addr1Mux | N::Addr2Mux | N::Sr2Mux | N::SpMux => {
            let (v, _) = content(n, c);
            if v.is_empty() {
                String::new()
            } else {
                format!("selects {v}")
            }
        }
        N::Control => c
            .rec
            .map(|r| crate::emulator::states::state_text(r.state).to_string())
            .unwrap_or_default(),
        _ => String::new(),
    }
}

/// The registers the register file shows in the focus view: those read, or the one written.
fn reg_rows(c: &Ctx<'_>, after: bool) -> Vec<usize> {
    let read = c.rec.map_or(0, |r| r.regs_read);
    let written = c.rec.and_then(|r| r.reg_written);
    let a = c.act();
    let mut rows: Vec<usize> = Vec::new();
    if after || !a.has(Dp::BusToReg) || read != 0 || a.has(Dp::Sr1ToSp) {
        for i in 0..8 {
            if read >> i & 1 == 1 && !after {
                rows.push(i);
            }
        }
        // the stack pointer logic reads R6
        if !after
            && (a.has(Dp::SpIncPath)
                || a.has(Dp::SpDecPath)
                || a.has(Dp::UspIn)
                || a.has(Dp::SspIn))
            && !rows.contains(&6)
        {
            rows.push(6);
        }
    }
    if let Some(w) = written {
        let only_dest = !c.rec.is_some_and(|r| r.regs_read != 0) || after;
        if only_dest && !rows.contains(&(w as usize)) {
            rows.push(w as usize);
        }
    }
    rows
}

fn focus_reg_file(pt: &mut Painter<'_>, r: Rect, c: &Ctx<'_>, after: bool) {
    let t = pt.theme;
    let (fill, stroke) = pt.colors(Lit::Active);
    pt.p.rect(r, CornerRadius::same(4), fill, stroke, StrokeKind::Inside);
    let title = if after {
        "REG FILE (after)"
    } else {
        "REG FILE"
    };
    pt.p.text(
        Pos2::new(r.center().x, r.min.y + 14.0 * pt.tf.k),
        Align2::CENTER_CENTER,
        title,
        FontId::proportional(pt.fs(9.5)),
        t.accent_color_primary,
    );
    let written = c.rec.and_then(|r| r.reg_written);
    let prev = c.prev.unwrap_or(c.snap);
    for (j, i) in reg_rows(c, after).into_iter().enumerate() {
        let row = Rect::from_min_size(
            r.min + vec2(8.0, 30.0 + 26.0 * j as f32) * pt.tf.k,
            vec2(r.width() - 16.0 * pt.tf.k, 22.0 * pt.tf.k),
        );
        let is_w = written == Some(i as u8);
        let shown_after = is_w && (after || c.rec.is_some_and(|r| r.regs_read == 0));
        let v = if shown_after { c.snap.r[i] } else { prev.r[i] };
        if shown_after {
            pt.p.rect_filled(
                row,
                CornerRadius::same(3),
                t.accent_color_primary.gamma_multiply(0.3),
            );
        } else {
            pt.p.rect_stroke(
                row,
                CornerRadius::same(3),
                Stroke::new(1.2_f32, t.accent_color_secondary),
                StrokeKind::Inside,
            );
        }
        let size = pt.fs(10.5);
        pt.p.text(
            Pos2::new(row.min.x + 6.0, row.center().y),
            Align2::LEFT_CENTER,
            format!("R{i}"),
            FontId::monospace(size),
            t.register_name_color,
        );
        let text = if shown_after && prev.r[i] != c.snap.r[i] {
            format!("x{:04X} ← was x{:04X}", v, prev.r[i])
        } else {
            format!("x{v:04X}")
        };
        let tsize = super::figure::fit(size, &text, row.width() * 0.7);
        pt.p.text(
            Pos2::new(row.max.x - 6.0, row.center().y),
            Align2::RIGHT_CENTER,
            text,
            FontId::monospace(tsize),
            t.strong_text_color,
        );
        pt.hits.push((
            row,
            format!(
                "R{i} = {}{}",
                disasm::value_text(v),
                if shown_after {
                    ", written in this state"
                } else {
                    ", read in this state"
                }
            ),
        ));
    }
    pt.hits.push((r, help(N::RegFile).into()));
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::emulator::trace::{cycles, merge};

    /// Draw the focus view and the model for every state of a program that uses every kind of
    /// instruction, a TRAP into the OS, an interrupt and an exception: none of them may panic.
    #[test]
    fn every_state_draws() {
        let program = r#"
            .ORIG x3000
            LEA R0, DATA
            LD R1, DATA
            LDI R2, PTR
            LDR R3, R0, #1
            ADD R4, R1, #-1
            ADD R4, R4, R1
            AND R5, R4, #3
            NOT R5, R5
            ST R5, DATA
            STI R5, PTR
            STR R5, R0, #2
            BRz SKIP
            JSR SUB
    SKIP    OUT
            RTI
    SUB     RET
    DATA    .FILL #5
    PTR     .FILL DATA
            .FILL #0
            .END
        "#;
        let mut emu = Emulator::new();
        emu.trace = true;
        let out = Emulator::parse_program(program, Some(&mut emu.metadata)).expect("assembles");
        emu.load_program(crate::emulator::LoadedProgram {
            machine_code: out.machine_code,
            orig_address: out.orig_address,
            ..Default::default()
        });
        emu.trace = true;
        emu.skip_os_emulation = false;
        let theme = ThemeSettings::dark_default();
        let ctx = egui::Context::default();
        let mut drawn = 0;
        for step in 0..1500 {
            if emu.halted {
                break;
            }
            if step == 300 {
                emu.push_input('k');
            }
            emu.micro_step_command();
            let log = cycles(&emu.micro_log);
            let Some(last) = log.last() else { continue };
            let Some(rec) = merge(&emu.micro_log[last.start..last.end]) else {
                continue;
            };
            for true_state in [false, true] {
                let extra = [0, 0, 0x8002, 1, 0x80];
                let live =
                    true_state.then(|| super::super::live::live(Some(&rec), &rec.snap, &extra));
                let _ = ctx.run(egui::RawInput::default(), |ctx| {
                    egui::CentralPanel::default().show(ctx, |ui| {
                        let c = Ctx {
                            rec: Some(&rec),
                            trail: Activity::default(),
                            snap: rec.snap,
                            prev: None,
                            extra,
                            flow_time: Some(1.0),
                            hover: Some(Pos2::new(100.0, 100.0)),
                            live: live.as_ref(),
                            ir_addr: 0x3000,
                        };
                        let frame = super::super::figure::Frame {
                            rec: Some(&rec),
                            trail: Activity::default(),
                            snap: &rec.snap,
                            prev: None,
                            extra,
                            flow_time: Some(1.0),
                            hover: Some(Pos2::new(100.0, 100.0)),
                            live: live.as_ref(),
                        };
                        for area in [
                            Rect::from_min_size(Pos2::ZERO, vec2(900.0, 600.0)),
                            Rect::from_min_size(Pos2::ZERO, vec2(300.0, 900.0)),
                        ] {
                            draw_focus(ui, area, &theme, &c);
                            draw_model(ui, area, &theme, &c, &mut emu.clone());
                            super::super::figure::draw(ui, area, &theme, &frame);
                        }
                    });
                });
            }
            drawn += 1;
        }
        assert!(drawn > 200, "drew {drawn} states");
    }
}
