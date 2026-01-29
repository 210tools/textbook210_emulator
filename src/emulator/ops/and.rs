use crate::emulator::BitAddressable;

use crate::emulator::micro_op::{CycleState, MicroOp, MicroOpGenerator};
use crate::emulator::EmulatorCell;
use std::collections::HashMap;

use super::Op;

#[derive(Debug, Clone)]
/// The AND alu op
pub enum AndOp {
    /// We have been given some 5 bit value to AND with a register we have not yet fetched
    Immediate {
        dr: EmulatorCell,
        sr1: EmulatorCell,
        imm5: EmulatorCell,
    },
    /// We have been given some register we have nopt yet fetched to AND with a register we have not yet fetched
    Register {
        dr: EmulatorCell,
        sr1: EmulatorCell,
        sr2: EmulatorCell,
    },
}

impl MicroOpGenerator for AndOp {
    fn generate_plan(&self) -> HashMap<CycleState, Vec<MicroOp>> {
        let mut plan = HashMap::new();

        match self {
            AndOp::Immediate { dr, sr1, imm5 } => {
                // Execute phase
                plan.insert(
                    CycleState::Execute,
                    vec![micro_op!(ALU_OUT <- R(sr1.get()) & IMM(imm5.sext(4).get() as i16))],
                );

                // Store Result phase
                plan.insert(
                    CycleState::StoreResult,
                    vec![
                        micro_op!(R(dr.get()) <- AluOut),
                        micro_op!(SET_CC(dr.get())),
                    ],
                );
            }
            AndOp::Register { dr, sr1, sr2 } => {
                // Execute phase
                plan.insert(
                    CycleState::Execute,
                    vec![micro_op!(ALU_OUT <- R(sr1.get()) & R(sr2.get()))],
                );

                // Store Result phase
                plan.insert(
                    CycleState::StoreResult,
                    vec![
                        micro_op!(R(dr.get()) <- AluOut),
                        micro_op!(SET_CC(dr.get())),
                    ],
                );
            }
        }

        plan
    }
}

impl Op for AndOp {
    fn decode(ir: EmulatorCell) -> Self {
        let span = tracing::trace_span!("AND_decode", op = ir.get());
        let _enter = span.enter();

        // LAYOUT: 0101 | DR | SR1 | ImmTBit | (Register || Immediate)
        let dr = ir.range(11..9);
        let sr1 = ir.range(8..6);

        // Check immediate mode (bit[5])
        match ir.index(5).get() {
            0 => {
                // Register mode
                let sr2 = ir.range(2..0);
                AndOp::Register { dr, sr1, sr2 }
            }
            1 => {
                // Immediate mode
                // imm5 is bits 4..0
                let imm5 = ir.range(4..0);
                AndOp::Immediate { dr, sr1, imm5 }
            }
            _ => unreachable!("Bit 5 can only be 0 or 1"),
        }
    }
}

use std::fmt;

impl fmt::Display for AndOp {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AndOp::Register { dr, sr1, sr2 } => {
                write!(f, "AND R{}, R{}, R{}", dr.get(), sr1.get(), sr2.get())
            }
            AndOp::Immediate { dr, sr1, imm5 } => {
                // Get the raw 5-bit value
                let imm_val_5bit = imm5.get() & 0x1F;
                // Calculate the sign-extended value (as i16 for display)
                let imm_val_sext = imm5.sext(4).get() as i16;
                write!(
                    f,
                    "AND R{}, R{}, #{} (x{:02X})",
                    dr.get(),
                    sr1.get(),
                    imm_val_sext, // Display sign-extended decimal
                    imm_val_5bit  // Display raw 5-bit hex
                )
            }
        }
    }
}
