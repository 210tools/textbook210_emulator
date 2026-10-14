//! The LC-3 state machine of Patt & Patel, 3rd edition, Figures C.2 and C.7.
//!
//! Instruction plans are written as micro-ops that follow the ISA description; `MicroOp::State`
//! markers group them into the numbered states of the state machine, one clock cycle each.
//! This module has the state descriptions and the sequences TRAP and exceptions share.

use super::micro_op::{DataDestination as D, DataSource as S, MicroOp};
use super::{Emulator, Exception};
use crate::micro_op;

/// What the book's state machine does in a state, as printed in Figures C.2 and C.7.
pub fn state_text(n: u8) -> &'static str {
    match n {
        0 => "[BEN]",
        1 => "DR<-SR1+OP2, set CC",
        2 => "MAR<-PC+off9, set ACV",
        3 => "MAR<-PC+off9, set ACV",
        4 => "[IR[11]]",
        5 => "DR<-SR1&OP2, set CC",
        6 => "MAR<-B+off6, set ACV",
        7 => "MAR<-B+off6, set ACV",
        8 => "MAR<-SP, [PSR[15]]",
        9 => "DR<-NOT(SR), set CC",
        10 => "MAR<-PC+off9, set ACV",
        11 => "MAR<-PC+off9, set ACV",
        12 => "PC<-BaseR",
        13 => "Table<-x01, Vector<-x01, MDR<-PSR, PSR[15]<-0, [PSR[15]]",
        14 => "DR<-PC+off9",
        15 => "Table<-x00, PC<-PC+1, MDR<-PSR",
        16 => "M[MAR]<-MDR",
        17 => "[ACV]",
        18 => "MAR<-PC, PC<-PC+1, set ACV, [INT]",
        19 => "[ACV]",
        20 => "R7<-PC, PC<-BaseR",
        21 => "R7<-PC, PC<-PC+off11",
        22 => "PC<-PC+off9",
        23 => "MDR<-SR, [ACV]",
        24 => "MDR<-M[MAR]",
        25 => "MDR<-M[MAR]",
        26 => "MAR<-MDR, set ACV",
        27 => "DR<-MDR, set CC",
        28 => "MDR<-M",
        29 => "MDR<-M[MAR]",
        30 => "IR<-MDR",
        31 => "MAR<-MDR, set ACV",
        32 => "BEN<-IR[11]&N + IR[10]&Z + IR[9]&P, [IR[15:12]]",
        33 => "[ACV]",
        34 => "SP<-SP+1, [PSR[15]]",
        35 => "[ACV]",
        36 => "MDR<-M",
        37 => "MAR, SP<-SP-1",
        38 => "PC<-MDR",
        39 => "MAR, SP<-SP+1",
        40 => "MDR<-M",
        41 => "Write (M[MAR]<-MDR)",
        42 => "PSR<-MDR",
        43 => "MDR<-PC-1",
        44 => "Table<-x01, Vector<-x00, MDR<-PSR, PSR[15]<-0",
        45 => "Saved_USP<-SP, SP<-Saved_SSP",
        46 => "MAR, SP<-SP-1",
        47 => "Vector<-IR[7:0], PSR[15]<-0, [PSR[15]]",
        48 | 56 | 57 | 60 | 61 => "Table<-x01, Vector<-x02, MDR<-PSR, PSR[15]<-0",
        49 => "Table<-x01, Vector<-INTV, PSR[10:8]<-Priority, MDR<-PSR, PSR[15]<-0, [PSR[15]]",
        51 => "Nothing",
        52 => "Write (M[MAR]<-MDR)",
        53 => "MDR<-M",
        54 => "MAR<-Table'Vector",
        55 => "PC<-MDR",
        59 => "Saved_SSP<-SP, SP<-Saved_USP",
        _ => "",
    }
}

/// States that test something (ACV, privilege, the opcode) and can branch to an exception.
pub const CHECK_STATES: [u8; 7] = [33, 35, 17, 19, 23, 8, 32];

/// Memory access states, which do not happen when the check before them raises an exception.
pub const ACCESS_STATES: [u8; 10] = [28, 24, 25, 29, 16, 36, 40, 41, 52, 53];

/// The state an exception enters, given the check state that raised it, and its vector.
pub fn exception_state(exception: &Exception, check_state: u8) -> (u8, u8) {
    match exception {
        Exception::PrivilegeViolation => (44, 0x00),
        Exception::IllegalInstruction => (13, 0x01),
        Exception::AccessControlViolation => (
            match check_state {
                17 => 56,
                35 => 57,
                19 => 61,
                23 => 48,
                _ => 60,
            },
            0x02,
        ),
    }
}

/// `[PSR[15]]` after PSR[15] <- 0: if the program was in User mode (the old PSR, now in MDR,
/// has bit 15 set), state 45 switches to the supervisor stack; otherwise it is skipped.
pub fn to_supervisor_stack() -> Vec<MicroOp> {
    vec![
        MicroOp::new_custom(
            |emu: &mut Emulator| {
                if emu.mdr.get() & 0x8000 == 0 {
                    emu.skip_state = Some(45);
                }
                Ok(())
            },
            "[PSR[15]]: state 45 only if the program was in User mode".to_owned(),
        ),
        MicroOp::State(45),
        MicroOp::transfer(D::SavedUsp, S::Register(6)),
        MicroOp::transfer(D::Register(6), S::SavedSsp),
    ]
}

/// States 37 to 55: push PSR (already in MDR) and PC, then load PC from the vector table.
/// Shared by TRAP and by exceptions and interrupts.
pub fn push_and_vector() -> Vec<MicroOp> {
    vec![
        MicroOp::State(37),
        MicroOp::transfer(D::MAR, S::SpMinusOne),
        MicroOp::transfer(D::Register(6), S::SpMinusOne),
        MicroOp::State(41),
        micro_op!(SET_FLAG(WriteMemory)),
        micro_op!(-> Execute), // M[MAR] <- MDR (the PSR)
        MicroOp::State(43),
        MicroOp::transfer(D::MDR, S::PcMinusOne),
        MicroOp::State(46),
        MicroOp::transfer(D::MAR, S::SpMinusOne),
        MicroOp::transfer(D::Register(6), S::SpMinusOne),
        MicroOp::State(52),
        micro_op!(SET_FLAG(WriteMemory)),
        micro_op!(-> Execute), // M[MAR] <- MDR (the PC)
        MicroOp::State(54),
        MicroOp::transfer(D::MAR, S::TableVector),
        MicroOp::State(53),
        micro_op!(-> Execute), // MDR <- M[MAR], the service routine's address
        MicroOp::State(55),
        micro_op!(PC <- MDR),
    ]
}

/// What state 18 does when INT is set, then state 49 (Figure C.7): the PC was already
/// incremented, MAR's read never happens, and the keyboard's vector and priority are loaded.
pub fn interrupt_entry() -> Vec<MicroOp> {
    let mut ops = vec![
        MicroOp::new_custom(
            |emu: &mut Emulator| {
                // the fetch MAR started is abandoned
                emu.execute_state.memory_read_pending = false;
                emu.note_int();
                Ok(())
            },
            "[INT] = 1: the keyboard interrupts (KBSR[15] and KBSR[14] set, priority 4 > PSR[10:8])".to_owned(),
        ),
        MicroOp::State(49),
        MicroOp::transfer(D::Table, S::Const8(0x01)),
        MicroOp::transfer(D::Vector, S::Intv),
        MicroOp::transfer(D::MDR, S::PSR),
        MicroOp::transfer(D::Priority, S::IntPriority),
        MicroOp::transfer(D::Priv, S::Zero),
    ];
    ops.extend(to_supervisor_stack());
    ops.extend(push_and_vector());
    ops
}

/// The exception entry sequence (Figure C.7): state 44, 13 or 48/56/57/60/61, then 45 if the
/// program was in User mode, then 37 to 55.
pub fn exception_entry(state: u8, vector: u8) -> Vec<MicroOp> {
    let mut ops = vec![
        MicroOp::State(state),
        MicroOp::transfer(D::Table, S::Const8(0x01)),
        MicroOp::transfer(D::Vector, S::Const8(vector)),
        MicroOp::transfer(D::MDR, S::PSR),
        MicroOp::transfer(D::Priv, S::Zero),
    ];
    ops.extend(to_supervisor_stack());
    ops.extend(push_and_vector());
    ops
}
