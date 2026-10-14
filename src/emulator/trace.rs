//! A record of what each micro-op did in terms of the LC-3 datapath (Patt & Patel, Figure C.3):
//! which parts and wires carried data, which control signals were asserted, and the values
//! involved. The datapath pane draws these.

use super::micro_op::{DataDestination, DataSource, MAluOp, MachineFlag, MicroOp};

/// A part or wire of the datapath diagram. Used as a bit index into [`Activity::parts`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum Dp {
    Bus,
    GatePc,
    Pc,
    PcInc,
    PcMux,
    BusToPcMux,
    AdderToPcMux,
    MarMux,
    GateMarMux,
    Zext,
    ZextToMarMux,
    AdderToMarMux,
    Adder,
    Addr1Mux,
    Addr2Mux,
    PcToAddr1,
    RegToAddr1,
    Sext11,
    Sext9,
    Sext6,
    ZeroToAddr2,
    RegFile,
    BusToReg,
    Sr1Out,
    Sr2Out,
    Sext5,
    Sr2Mux,
    Alu,
    GateAlu,
    Ir,
    BusToIr,
    IrToControl,
    Control,
    Logic,
    Nzp,
    BusToLogic,
    Mdr,
    GateMdr,
    BusToMdr,
    Mar,
    BusToMar,
    MarToMem,
    Memory,
    MemToMdr,
    MdrToMem,
    // Figure C.8's additions for interrupts and exceptions
    PcDec,
    GatePcDec,
    Sr1ToSp,
    UspIn,
    UspOut,
    SspIn,
    SspOut,
    SpIncPath,
    SpDecPath,
    SpMux,
    GateSp,
    SavedUsp,
    SavedSsp,
    Priv,
    PrivMux,
    SetPriv,
    GatePsr,
    BusToPsr,
    NzpMux,
    Priority,
    PriorityMux,
    Table,
    TableMux,
    GateVector,
    Vector,
    VectorSel,
    IrToVector,
    VectorMux,
    /// The interrupting device's priority, into the comparator and PSRMUX
    IntPriorityIn,
    /// INT, from the priority comparator to the microsequencer
    IntSignal,
    /// The VectorMUX inputs
    VecIntv,
    VecX00,
    VecX01,
    VecX02,
    /// The TableMUX inputs
    TabX00,
    TabX01,
}

impl Dp {
    /// Every part, in order.
    pub const ALL: [Dp; 81] = [
        Dp::Bus,
        Dp::GatePc,
        Dp::Pc,
        Dp::PcInc,
        Dp::PcMux,
        Dp::BusToPcMux,
        Dp::AdderToPcMux,
        Dp::MarMux,
        Dp::GateMarMux,
        Dp::Zext,
        Dp::ZextToMarMux,
        Dp::AdderToMarMux,
        Dp::Adder,
        Dp::Addr1Mux,
        Dp::Addr2Mux,
        Dp::PcToAddr1,
        Dp::RegToAddr1,
        Dp::Sext11,
        Dp::Sext9,
        Dp::Sext6,
        Dp::ZeroToAddr2,
        Dp::RegFile,
        Dp::BusToReg,
        Dp::Sr1Out,
        Dp::Sr2Out,
        Dp::Sext5,
        Dp::Sr2Mux,
        Dp::Alu,
        Dp::GateAlu,
        Dp::Ir,
        Dp::BusToIr,
        Dp::IrToControl,
        Dp::Control,
        Dp::Logic,
        Dp::Nzp,
        Dp::BusToLogic,
        Dp::Mdr,
        Dp::GateMdr,
        Dp::BusToMdr,
        Dp::Mar,
        Dp::BusToMar,
        Dp::MarToMem,
        Dp::Memory,
        Dp::MemToMdr,
        Dp::MdrToMem,
        Dp::PcDec,
        Dp::GatePcDec,
        Dp::Sr1ToSp,
        Dp::UspIn,
        Dp::UspOut,
        Dp::SspIn,
        Dp::SspOut,
        Dp::SpIncPath,
        Dp::SpDecPath,
        Dp::SpMux,
        Dp::GateSp,
        Dp::SavedUsp,
        Dp::SavedSsp,
        Dp::Priv,
        Dp::PrivMux,
        Dp::SetPriv,
        Dp::GatePsr,
        Dp::BusToPsr,
        Dp::NzpMux,
        Dp::Priority,
        Dp::PriorityMux,
        Dp::Table,
        Dp::TableMux,
        Dp::GateVector,
        Dp::Vector,
        Dp::VectorSel,
        Dp::IrToVector,
        Dp::VectorMux,
        Dp::IntPriorityIn,
        Dp::IntSignal,
        Dp::VecIntv,
        Dp::VecX00,
        Dp::VecX01,
        Dp::VecX02,
        Dp::TabX00,
        Dp::TabX01,
    ];
}

/// A control signal from Table C.1. Used as a bit index into [`Activity::signals`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum Sig {
    LdMar,
    LdMdr,
    LdIr,
    LdBen,
    LdReg,
    LdCc,
    LdPc,
    GatePc,
    GateMdr,
    GateAlu,
    GateMarMux,
    PcMuxInc,
    PcMuxBus,
    PcMuxAdder,
    Addr1Pc,
    Addr1Base,
    Addr2Zero,
    Addr2Off6,
    Addr2Off9,
    Addr2Off11,
    MarMuxZext,
    MarMuxAdder,
    AlukAdd,
    AlukAnd,
    AlukNot,
    AlukPassA,
    Sr2MuxImm,
    MioEn,
    RwRead,
    RwWrite,
    GatePcMinus1,
    GateSp,
    GatePsr,
    GateVector,
    LdSavedSsp,
    LdSavedUsp,
    LdPriv,
    SetPriv,
    LdPriority,
    LdTable,
    LdVector,
    SpMuxInc,
    SpMuxDec,
    SpMuxUsp,
    SpMuxSsp,
    TableMux,
    VectorMux,
}

impl Sig {
    pub const ALL: [Sig; 47] = [
        Sig::LdMar,
        Sig::LdMdr,
        Sig::LdIr,
        Sig::LdBen,
        Sig::LdReg,
        Sig::LdCc,
        Sig::LdPc,
        Sig::GatePc,
        Sig::GateMdr,
        Sig::GateAlu,
        Sig::GateMarMux,
        Sig::PcMuxInc,
        Sig::PcMuxBus,
        Sig::PcMuxAdder,
        Sig::Addr1Pc,
        Sig::Addr1Base,
        Sig::Addr2Zero,
        Sig::Addr2Off6,
        Sig::Addr2Off9,
        Sig::Addr2Off11,
        Sig::MarMuxZext,
        Sig::MarMuxAdder,
        Sig::AlukAdd,
        Sig::AlukAnd,
        Sig::AlukNot,
        Sig::AlukPassA,
        Sig::Sr2MuxImm,
        Sig::MioEn,
        Sig::RwRead,
        Sig::RwWrite,
        Sig::GatePcMinus1,
        Sig::GateSp,
        Sig::GatePsr,
        Sig::GateVector,
        Sig::LdSavedSsp,
        Sig::LdSavedUsp,
        Sig::LdPriv,
        Sig::SetPriv,
        Sig::LdPriority,
        Sig::LdTable,
        Sig::LdVector,
        Sig::SpMuxInc,
        Sig::SpMuxDec,
        Sig::SpMuxUsp,
        Sig::SpMuxSsp,
        Sig::TableMux,
        Sig::VectorMux,
    ];

    pub fn name(self) -> &'static str {
        match self {
            Sig::LdMar => "LD.MAR",
            Sig::LdMdr => "LD.MDR",
            Sig::LdIr => "LD.IR",
            Sig::LdBen => "LD.BEN",
            Sig::LdReg => "LD.REG",
            Sig::LdCc => "LD.CC",
            Sig::LdPc => "LD.PC",
            Sig::GatePc => "GatePC",
            Sig::GateMdr => "GateMDR",
            Sig::GateAlu => "GateALU",
            Sig::GateMarMux => "GateMARMUX",
            Sig::PcMuxInc => "PCMUX=PC+1",
            Sig::PcMuxBus => "PCMUX=BUS",
            Sig::PcMuxAdder => "PCMUX=ADDER",
            Sig::Addr1Pc => "ADDR1MUX=PC",
            Sig::Addr1Base => "ADDR1MUX=BaseR",
            Sig::Addr2Zero => "ADDR2MUX=0",
            Sig::Addr2Off6 => "ADDR2MUX=off6",
            Sig::Addr2Off9 => "ADDR2MUX=PCoff9",
            Sig::Addr2Off11 => "ADDR2MUX=PCoff11",
            Sig::MarMuxZext => "MARMUX=ZEXT",
            Sig::MarMuxAdder => "MARMUX=ADDER",
            Sig::AlukAdd => "ALUK=ADD",
            Sig::AlukAnd => "ALUK=AND",
            Sig::AlukNot => "ALUK=NOT",
            Sig::AlukPassA => "ALUK=PASSA",
            Sig::Sr2MuxImm => "SR2MUX=imm5",
            Sig::MioEn => "MIO.EN",
            Sig::RwRead => "R.W=RD",
            Sig::RwWrite => "R.W=WR",
            Sig::GatePcMinus1 => "GatePC-1",
            Sig::GateSp => "GateSP",
            Sig::GatePsr => "GatePSR",
            Sig::GateVector => "GateVector",
            Sig::LdSavedSsp => "LD.SavedSSP",
            Sig::LdSavedUsp => "LD.SavedUSP",
            Sig::LdPriv => "LD.Priv",
            Sig::SetPriv => "Set.Priv",
            Sig::LdPriority => "LD.Priority",
            Sig::LdTable => "LD.Table",
            Sig::LdVector => "LD.Vector",
            Sig::SpMuxInc => "SPMUX=SP+1",
            Sig::SpMuxDec => "SPMUX=SP-1",
            Sig::SpMuxUsp => "SPMUX=Saved.USP",
            Sig::SpMuxSsp => "SPMUX=Saved.SSP",
            Sig::TableMux => "TableMUX",
            Sig::VectorMux => "VectorMUX",
        }
    }
}

/// Which parts, wires and signals were active.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Activity {
    pub parts: u128,
    pub signals: u64,
}

impl Activity {
    pub fn has(&self, p: Dp) -> bool {
        self.parts & (1 << p as u8) != 0
    }
    pub fn signal(&self, s: Sig) -> bool {
        self.signals & (1 << s as u8) != 0
    }
    pub fn is_empty(&self) -> bool {
        self.parts == 0 && self.signals == 0
    }
    pub fn add(&mut self, parts: &[Dp], signals: &[Sig]) {
        for p in parts {
            self.parts |= 1 << *p as u8;
        }
        for s in signals {
            self.signals |= 1 << *s as u8;
        }
    }
}

/// The machine's visible state after a step, so a replay can show values as they were.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Snapshot {
    pub pc: u16,
    pub ir: u16,
    pub mar: u16,
    pub mdr: u16,
    pub r: [u16; 8],
    /// bit 2 = N, bit 1 = Z, bit 0 = P
    pub nzp: u8,
}

/// One step of an instruction as the datapath sees it.
#[derive(Debug, Clone)]
pub struct MicroRecord {
    /// Register transfer text, e.g. `MAR <- PC`
    pub text: String,
    pub activity: Activity,
    /// The value on the bus (or produced by the adder/ALU/memory) in this step
    pub value: Option<u16>,
    /// Index of the instruction cycle phase (0 = fetch .. 5 = store)
    pub phase: usize,
    /// Bit mask of the general purpose registers read in this step
    pub regs_read: u8,
    pub reg_written: Option<u8>,
    /// State after the step
    pub snap: Snapshot,
    /// The state machine state (Figures C.2 and C.7) this step happened in
    pub state: u8,
    /// Marks the start of a state; carries no transfer itself
    pub marker: bool,
    /// The value on each wire (and part) this step used
    pub wires: Vec<(Dp, u16)>,
}

impl MicroRecord {
    pub fn new(text: String, activity: Activity, value: Option<u16>, phase: usize) -> Self {
        Self {
            text,
            activity,
            value,
            phase,
            regs_read: 0,
            reg_written: None,
            snap: Snapshot::default(),
            state: 0,
            marker: false,
            wires: Vec::new(),
        }
    }

    pub fn with_wires(mut self, wires: &[(Dp, u16)]) -> Self {
        self.wires.extend_from_slice(wires);
        self
    }

    /// The value on a wire in this step, if it carried one.
    pub fn wire(&self, dp: Dp) -> Option<u16> {
        self.wires.iter().find(|(d, _)| *d == dp).map(|(_, v)| *v)
    }
}

/// Registers a micro-op reads and writes: (read mask, written register).
pub fn registers_of(op: &MicroOp) -> (u8, Option<u8>) {
    let bit = |s: &DataSource| match s {
        DataSource::Register(n) => 1u8 << (*n & 7),
        _ => 0,
    };
    match op {
        MicroOp::Transfer {
            source,
            destination,
        } => (
            bit(source),
            match destination {
                DataDestination::Register(n) => Some((*n & 7) as u8),
                _ => None,
            },
        ),
        MicroOp::Alu {
            operand1, operand2, ..
        } => (bit(operand1) | bit(operand2), None),
        MicroOp::Address { base, .. } => (bit(base), None),
        _ => (0, None),
    }
}

/// The datapath activity of a micro-op (before values are known).
pub fn activity_of(op: &MicroOp) -> Activity {
    use Dp::*;
    let mut a = Activity::default();
    match op {
        MicroOp::Transfer {
            source,
            destination,
        } => {
            let to_pc_directly = matches!(destination, DataDestination::PC)
                && matches!(source, DataSource::PcPlusOne | DataSource::AddrOut);
            let to_saved_sp = matches!(
                destination,
                DataDestination::SavedSsp | DataDestination::SavedUsp
            );
            match source {
                DataSource::Register(_) if to_saved_sp => a.add(&[RegFile, Sr1ToSp], &[]),
                DataSource::PcMinusOne => a.add(&[Pc, PcDec, GatePcDec, Bus], &[Sig::GatePcMinus1]),
                DataSource::SpMinusOne => a.add(
                    &[RegFile, Sr1ToSp, SpDecPath, SpMux, GateSp, Bus],
                    &[Sig::SpMuxDec, Sig::GateSp],
                ),
                DataSource::SpPlusOne => a.add(
                    &[RegFile, Sr1ToSp, SpIncPath, SpMux, GateSp, Bus],
                    &[Sig::SpMuxInc, Sig::GateSp],
                ),
                DataSource::SavedSsp => a.add(
                    &[SavedSsp, SspOut, SpMux, GateSp, Bus],
                    &[Sig::SpMuxSsp, Sig::GateSp],
                ),
                DataSource::SavedUsp => a.add(
                    &[SavedUsp, UspOut, SpMux, GateSp, Bus],
                    &[Sig::SpMuxUsp, Sig::GateSp],
                ),
                DataSource::TableVector => {
                    a.add(&[Table, Vector, GateVector, Bus], &[Sig::GateVector])
                }
                DataSource::PSR => a.add(&[Priv, Priority, Nzp, GatePsr, Bus], &[Sig::GatePsr]),
                DataSource::IrLow8 => a.add(&[IrToVector, VectorSel], &[]),
                DataSource::Const8(v) if matches!(destination, DataDestination::Table) => a.add(
                    &[if *v == 0 { TabX00 } else { TabX01 }, TableMux],
                    &[Sig::TableMux],
                ),
                DataSource::Const8(v) => a.add(
                    &[
                        [VecX00, VecX01, VecX02][(*v as usize).min(2)],
                        VectorMux,
                        VectorSel,
                    ],
                    &[Sig::VectorMux],
                ),
                DataSource::Intv => a.add(&[VecIntv, VectorMux, VectorSel], &[Sig::VectorMux]),
                DataSource::IntPriority => a.add(&[IntPriorityIn, PriorityMux], &[]),
                DataSource::Zero => a.add(&[SetPriv, PrivMux], &[Sig::SetPriv]),
                DataSource::PC => a.add(&[Pc, GatePc, Bus], &[Sig::GatePc]),
                DataSource::PcPlusOne => a.add(&[Pc, PcInc, PcMux], &[Sig::PcMuxInc]),
                DataSource::MDR => a.add(&[Mdr, GateMdr, Bus], &[Sig::GateMdr]),
                DataSource::AluOut => a.add(&[Alu, GateAlu, Bus], &[Sig::GateAlu]),
                DataSource::AddrOut if to_pc_directly => {
                    a.add(&[Adder, AdderToPcMux, PcMux], &[Sig::PcMuxAdder])
                }
                DataSource::AddrOut => a.add(
                    &[Adder, AdderToMarMux, MarMux, GateMarMux, Bus],
                    &[Sig::MarMuxAdder, Sig::GateMarMux],
                ),
                DataSource::Register(_) => a.add(
                    &[RegFile, Sr1Out, Alu, GateAlu, Bus],
                    &[Sig::AlukPassA, Sig::GateAlu],
                ),
                DataSource::TrapVector(_) | DataSource::Constant(_) => a.add(
                    &[Ir, Zext, ZextToMarMux, MarMux, GateMarMux, Bus],
                    &[Sig::MarMuxZext, Sig::GateMarMux],
                ),
                _ => a.add(&[Bus], &[]),
            }
            match destination {
                DataDestination::MAR => a.add(&[BusToMar, Mar], &[Sig::LdMar]),
                DataDestination::MDR => a.add(&[BusToMdr, Mdr], &[Sig::LdMdr]),
                DataDestination::IR => a.add(&[BusToIr, Ir], &[Sig::LdIr]),
                DataDestination::Register(_) => a.add(&[BusToReg, RegFile], &[Sig::LdReg]),
                DataDestination::PC if to_pc_directly => a.add(&[Pc], &[Sig::LdPc]),
                DataDestination::PC => a.add(&[BusToPcMux, PcMux, Pc], &[Sig::PcMuxBus, Sig::LdPc]),
                DataDestination::PSR => a.add(
                    &[BusToPsr, PrivMux, PriorityMux, NzpMux, Priv, Priority, Nzp],
                    &[Sig::LdPriv, Sig::LdPriority, Sig::LdCc],
                ),
                DataDestination::Priv => a.add(&[Priv], &[Sig::LdPriv]),
                DataDestination::SavedSsp => a.add(&[SspIn, SavedSsp], &[Sig::LdSavedSsp]),
                DataDestination::SavedUsp => a.add(&[UspIn, SavedUsp], &[Sig::LdSavedUsp]),
                DataDestination::Table => a.add(&[Table], &[Sig::LdTable]),
                DataDestination::Vector => a.add(&[Vector], &[Sig::LdVector]),
                DataDestination::Priority => a.add(&[Priority], &[Sig::LdPriority]),
                _ => {}
            }
        }
        MicroOp::Alu {
            operation,
            operand2,
            ..
        } => {
            a.add(&[RegFile, Sr1Out, Alu], &[]);
            match operation {
                MAluOp::Add => a.add(&[], &[Sig::AlukAdd]),
                MAluOp::And => a.add(&[], &[Sig::AlukAnd]),
                MAluOp::Not => a.add(&[], &[Sig::AlukNot]),
            }
            match operand2 {
                DataSource::Register(_) => a.add(&[Sr2Out, Sr2Mux], &[]),
                DataSource::Immediate(_) => a.add(&[Ir, Sext5, Sr2Mux], &[Sig::Sr2MuxImm]),
                _ => {}
            }
        }
        MicroOp::Address { base, bits, .. } => {
            a.add(&[Adder, Addr1Mux, Addr2Mux], &[]);
            match base {
                DataSource::PC => a.add(&[Pc, PcToAddr1], &[Sig::Addr1Pc]),
                _ => a.add(&[RegFile, RegToAddr1], &[Sig::Addr1Base]),
            }
            match bits {
                11 => a.add(&[Ir, Sext11], &[Sig::Addr2Off11]),
                9 => a.add(&[Ir, Sext9], &[Sig::Addr2Off9]),
                6 => a.add(&[Ir, Sext6], &[Sig::Addr2Off6]),
                _ => a.add(&[ZeroToAddr2], &[Sig::Addr2Zero]),
            }
        }
        MicroOp::SetFlag(MachineFlag::UpdateCondCodes(_)) => {
            a.add(&[Bus, BusToLogic, Logic, NzpMux, Nzp], &[Sig::LdCc])
        }
        // the write itself is recorded when it happens at the end of the phase
        MicroOp::SetFlag(MachineFlag::WriteMemory) => {}
        MicroOp::PhaseTransition(super::micro_op::CycleState::Decode) => {
            a.add(&[Ir, IrToControl, Control], &[Sig::LdBen])
        }
        _ => {}
    }
    a
}

/// The activity of a memory read (MDR <- mem[MAR]).
pub fn memory_read() -> Activity {
    let mut a = Activity::default();
    a.add(
        &[Dp::Mar, Dp::MarToMem, Dp::Memory, Dp::MemToMdr, Dp::Mdr],
        &[Sig::MioEn, Sig::RwRead, Sig::LdMdr],
    );
    a
}

/// The activity of a memory write (mem[MAR] <- MDR).
pub fn memory_write() -> Activity {
    let mut a = Activity::default();
    a.add(
        &[Dp::Mar, Dp::MarToMem, Dp::Mdr, Dp::MdrToMem, Dp::Memory],
        &[Sig::MioEn, Sig::RwWrite],
    );
    a
}

/// One state of the state machine (a clock cycle): a marker record and the steps after it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Cycle {
    pub state: u8,
    /// Range of records in the log
    pub start: usize,
    pub end: usize,
}

/// Group a micro-op log into the states it went through.
pub fn cycles(log: &[MicroRecord]) -> Vec<Cycle> {
    let mut out: Vec<Cycle> = Vec::new();
    for (i, r) in log.iter().enumerate() {
        match out.last_mut() {
            Some(c) if !r.marker => c.end = i + 1,
            _ => out.push(Cycle {
                state: r.state,
                start: i,
                end: i + 1,
            }),
        }
    }
    out
}

/// Everything a state did, as one record: the union of its steps' activity, the value it put
/// on the bus (or else the last value it produced), and the machine state after it.
pub fn merge(records: &[MicroRecord]) -> Option<MicroRecord> {
    let first = records.first()?;
    let steps: Vec<&MicroRecord> = records.iter().filter(|r| !r.marker).collect();
    let mut m = MicroRecord::new(
        steps
            .iter()
            .map(|r| r.text.as_str())
            .collect::<Vec<_>>()
            .join(", "),
        Activity::default(),
        None,
        first.phase,
    );
    m.state = first.state;
    m.snap = records.last()?.snap;
    for r in &steps {
        m.activity.parts |= r.activity.parts;
        m.activity.signals |= r.activity.signals;
        m.regs_read |= r.regs_read;
        if r.reg_written.is_some() {
            m.reg_written = r.reg_written;
        }
        for &(dp, v) in &r.wires {
            if m.wire(dp).is_none() {
                m.wires.push((dp, v));
            }
        }
    }
    m.value = steps
        .iter()
        .find(|r| r.activity.has(Dp::Bus) && r.value.is_some())
        .or_else(|| steps.iter().find(|r| r.activity.has(Dp::Memory)))
        .or_else(|| steps.iter().rev().find(|r| r.value.is_some()))
        .and_then(|r| r.value);
    Some(m)
}
