//! The true state of the datapath: in hardware every combinational part works all the time, so
//! every wire carries a value whether or not this state uses it. Only the gates (onto the bus),
//! the load signals (into registers) and memory's enable decide what happens. This works out
//! which wires are live in a state and what is on each.

use crate::emulator::trace::{Activity, Dp, MicroRecord, Sig, Snapshot};
use std::collections::HashMap;

/// What is live in one state.
#[derive(Default)]
pub struct Live {
    /// Wires and parts carrying a value
    pub parts: Activity,
    /// Wires into a gate that is off: live up to the gate, dead after it
    pub to_gate: Activity,
    /// The value on each live wire
    pub values: HashMap<Dp, u16>,
}

impl Live {
    pub fn has(&self, d: Dp) -> bool {
        self.parts.has(d)
    }
    pub fn stops_at_gate(&self, d: Dp) -> bool {
        self.to_gate.has(d)
    }
    pub fn value(&self, d: Dp) -> Option<u16> {
        self.values.get(&d).copied()
    }
}

/// The wires from a part through a gate onto the bus, with the gate's signal.
pub const GATES: [(Dp, Sig); 8] = [
    (Dp::GatePc, Sig::GatePc),
    (Dp::GateMdr, Sig::GateMdr),
    (Dp::GateAlu, Sig::GateAlu),
    (Dp::GateMarMux, Sig::GateMarMux),
    (Dp::GatePcDec, Sig::GatePcMinus1),
    (Dp::GateSp, Sig::GateSp),
    (Dp::GatePsr, Sig::GatePsr),
    (Dp::GateVector, Sig::GateVector),
];

/// Wires fed by the bus (and what hangs off them): live only while a gate drives it.
const FROM_BUS: [Dp; 10] = [
    Dp::Bus,
    Dp::BusToPcMux,
    Dp::BusToReg,
    Dp::BusToIr,
    Dp::BusToLogic,
    Dp::Logic,
    Dp::NzpMux,
    Dp::BusToMdr,
    Dp::BusToMar,
    Dp::BusToPsr,
];

fn sext(v: u16, bits: u32) -> u16 {
    let shift = 16 - bits;
    (((v << shift) as i16) >> shift) as u16
}

fn nzp_of(v: u16) -> u16 {
    if v == 0 {
        0b010
    } else if v & 0x8000 != 0 {
        0b100
    } else {
        0b001
    }
}

/// What is live in the state `rec`, from the machine as the state began (`before`).
/// `extra` is (Saved_SSP, Saved_USP, PSR, Table, Vector).
pub fn live(rec: Option<&MicroRecord>, before: &Snapshot, extra: &[u16; 5]) -> Live {
    let act = rec.map(|r| r.activity).unwrap_or_default();
    let traced = |d: Dp| rec.and_then(|r| r.wire(d)).filter(|_| act.has(d));
    let on = |s: Sig| act.signal(s);
    let [saved_ssp, saved_usp, psr, table, vector] = *extra;
    let b = before;
    let ir = b.ir;
    let reg = |n: u16| b.r[(n & 7) as usize];

    // the register file's outputs: SR1 is IR[8:6] unless this state reads another (a store's
    // SR, or R6 for the stack)
    let sr1 = traced(Dp::Sr1Out)
        .or(traced(Dp::RegToAddr1))
        .unwrap_or(reg(ir >> 6));
    let sr2 = reg(ir);
    let r6 = b.r[6];
    let sext5 = sext(ir & 0x1F, 5);
    let sr2mux = if ir & 0x20 != 0 { sext5 } else { sr2 };
    // muxes take the input their select picks; with no select asserted, input 0
    let addr1 = if on(Sig::Addr1Base) { sr1 } else { b.pc };
    let addr2 = if on(Sig::Addr2Off6) {
        sext(ir & 0x3F, 6)
    } else if on(Sig::Addr2Off9) {
        sext(ir & 0x1FF, 9)
    } else if on(Sig::Addr2Off11) {
        sext(ir & 0x7FF, 11)
    } else {
        0
    };
    let adder = traced(Dp::Adder).unwrap_or(addr1.wrapping_add(addr2));
    let zext = ir & 0xFF;
    let marmux = if on(Sig::MarMuxAdder) { adder } else { zext };
    let alu = traced(Dp::Alu).unwrap_or(if on(Sig::AlukAnd) {
        sr1 & sr2mux
    } else if on(Sig::AlukNot) {
        !sr1
    } else if on(Sig::AlukPassA) {
        sr1
    } else {
        sr1.wrapping_add(sr2mux)
    });
    let spmux = if on(Sig::SpMuxDec) {
        r6.wrapping_sub(1)
    } else if on(Sig::SpMuxUsp) {
        saved_usp
    } else if on(Sig::SpMuxSsp) {
        saved_ssp
    } else {
        r6.wrapping_add(1)
    };
    let driven = act.has(Dp::Bus);
    let bus = traced(Dp::Bus)
        .or(rec.and_then(|r| r.value))
        .filter(|_| driven);
    let pcmux = if on(Sig::PcMuxBus) {
        bus.unwrap_or(0)
    } else if on(Sig::PcMuxAdder) {
        adder
    } else {
        b.pc.wrapping_add(1)
    };
    let tab = if on(Sig::TableMux) { 1 } else { 0 };
    let vecmux = traced(Dp::VectorMux).unwrap_or(crate::emulator::KEYBOARD_INTV);

    let mut values: HashMap<Dp, u16> = HashMap::new();
    let mut set = |d: Dp, v: u16| {
        values.insert(d, v);
    };
    set(Dp::Pc, b.pc);
    set(Dp::GatePc, b.pc);
    set(Dp::PcToAddr1, b.pc);
    set(Dp::PcInc, b.pc.wrapping_add(1));
    set(Dp::PcDec, b.pc);
    set(Dp::GatePcDec, b.pc.wrapping_sub(1));
    set(Dp::PcMux, pcmux);
    set(Dp::AdderToPcMux, adder);
    set(Dp::Adder, adder);
    set(Dp::Addr1Mux, addr1);
    set(Dp::Addr2Mux, addr2);
    set(Dp::RegToAddr1, sr1);
    set(Dp::Sext11, sext(ir & 0x7FF, 11));
    set(Dp::Sext9, sext(ir & 0x1FF, 9));
    set(Dp::Sext6, sext(ir & 0x3F, 6));
    set(Dp::Sext5, sext5);
    set(Dp::ZeroToAddr2, 0);
    set(Dp::Zext, zext);
    set(Dp::ZextToMarMux, zext);
    set(Dp::AdderToMarMux, adder);
    set(Dp::MarMux, marmux);
    set(Dp::GateMarMux, marmux);
    set(Dp::Sr1Out, sr1);
    set(Dp::Sr2Out, sr2);
    set(Dp::Sr2Mux, sr2mux);
    set(Dp::Alu, alu);
    set(Dp::GateAlu, alu);
    set(Dp::Ir, ir);
    set(Dp::IrToControl, ir);
    set(Dp::Nzp, b.nzp as u16);
    set(Dp::Mdr, b.mdr);
    set(Dp::GateMdr, b.mdr);
    set(Dp::MdrToMem, b.mdr);
    set(Dp::Mar, b.mar);
    set(Dp::MarToMem, b.mar);
    set(Dp::Sr1ToSp, r6);
    set(Dp::UspIn, r6);
    set(Dp::SspIn, r6);
    set(Dp::SpIncPath, r6.wrapping_add(1));
    set(Dp::SpDecPath, r6.wrapping_sub(1));
    set(Dp::UspOut, saved_usp);
    set(Dp::SspOut, saved_ssp);
    set(Dp::SavedUsp, saved_usp);
    set(Dp::SavedSsp, saved_ssp);
    set(Dp::SpMux, spmux);
    set(Dp::GateSp, spmux);
    set(Dp::GatePsr, psr);
    set(Dp::Priv, psr >> 15);
    set(Dp::Priority, psr >> 8 & 7);
    set(Dp::SetPriv, 0);
    set(Dp::PrivMux, if on(Sig::SetPriv) { 0 } else { psr >> 15 });
    set(Dp::IntPriorityIn, crate::emulator::KEYBOARD_PRIORITY);
    set(
        Dp::PriorityMux,
        if on(Sig::LdPriority) {
            crate::emulator::KEYBOARD_PRIORITY
        } else {
            psr >> 8 & 7
        },
    );
    set(Dp::IntSignal, traced(Dp::IntSignal).unwrap_or(0));
    set(Dp::TabX00, 0);
    set(Dp::TabX01, 1);
    set(Dp::TableMux, tab);
    set(Dp::Table, table & 0xFF);
    set(Dp::VecIntv, crate::emulator::KEYBOARD_INTV);
    set(Dp::VecX00, 0);
    set(Dp::VecX01, 1);
    set(Dp::VecX02, 2);
    set(Dp::VectorMux, vecmux);
    set(Dp::IrToVector, zext);
    set(
        Dp::VectorSel,
        if on(Sig::VectorMux) { vecmux } else { zext },
    );
    set(Dp::Vector, vector & 0xFF);
    set(Dp::GateVector, (table & 0xFF) << 8 | (vector & 0xFF));
    if let Some(v) = bus {
        for d in FROM_BUS {
            set(d, v);
        }
        set(Dp::Logic, nzp_of(v));
        set(Dp::NzpMux, nzp_of(v));
    }
    // what the state actually put on its wires wins
    if let Some(r) = rec {
        for (d, v) in &r.wires {
            if act.has(*d) {
                values.insert(*d, *v);
            }
        }
    }

    let mut parts = Activity::default();
    let mut to_gate = Activity::default();
    for d in Dp::ALL {
        let gated = GATES.iter().find(|(g, _)| *g == d);
        if act.has(d) {
            parts.add(&[d], &[]);
        } else if let Some((_, s)) = gated {
            if on(*s) {
                parts.add(&[d], &[]);
            } else {
                to_gate.add(&[d], &[]);
            }
        } else if FROM_BUS.contains(&d) {
            if driven {
                parts.add(&[d], &[]);
            }
        } else if matches!(d, Dp::Memory | Dp::MemToMdr) {
            // memory only answers when enabled
        } else if values.contains_key(&d) || matches!(d, Dp::RegFile | Dp::Control) {
            parts.add(&[d], &[]);
        }
    }
    Live {
        parts,
        to_gate,
        values,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn idle_wires_carry_values_and_gates_stop_them() {
        // ADD R2, R1, #-3 with R1 = 7, in a state that uses nothing
        let mut b = Snapshot {
            pc: 0x3001,
            ir: 0x147D,
            ..Default::default()
        };
        b.r[1] = 7;
        let l = live(None, &b, &[0; 5]);
        assert_eq!(l.value(Dp::Sr1Out), Some(7));
        assert_eq!(l.value(Dp::Sr2Mux), Some(0xFFFD));
        assert_eq!(
            l.value(Dp::Alu),
            Some(4),
            "the ALU adds whether or not anyone looks"
        );
        assert_eq!(l.value(Dp::PcInc), Some(0x3002));
        assert!(l.has(Dp::Alu) && l.has(Dp::Sext5));
        assert!(
            !l.has(Dp::GateAlu) && l.stops_at_gate(Dp::GateAlu),
            "no gate open: stops at it"
        );
        assert!(
            !l.has(Dp::Bus) && !l.has(Dp::BusToMar),
            "nothing drives the bus"
        );
        assert!(!l.has(Dp::MemToMdr), "memory is not enabled");
    }
}
