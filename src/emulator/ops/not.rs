use crate::emulator::micro_op::{CycleState, MicroOp, MicroOpGenerator};
use crate::emulator::{BitAddressable, EmulatorCell};
use std::collections::HashMap;

use super::Op;

#[derive(Debug, Clone)]
/// Preform the alu Not op on some register then save the result into some other register
pub enum NotOp {
    Decoded {
        dr: EmulatorCell, // Destination register index
        sr: EmulatorCell, // Source register index
    },
}

impl MicroOpGenerator for NotOp {
    fn generate_plan(&self) -> HashMap<CycleState, Vec<MicroOp>> {
        let mut plan = HashMap::new();

        match self {
            NotOp::Decoded { dr, sr } => {
                // Execute phase
                plan.insert(
                    CycleState::Execute,
                    vec![micro_op!(ALU_OUT <- NOT R(sr.get()))],
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

impl Op for NotOp {
    fn decode(ir: EmulatorCell) -> Self {
        // LAYOUT: 1001 | DR | SR | 111111
        let dr = ir.range(11..9);
        let sr = ir.range(8..6);

        NotOp::Decoded { dr, sr }
    }
}
use std::fmt;

impl fmt::Display for NotOp {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            NotOp::Decoded { dr, sr } => {
                write!(f, "NOT R{}, R{}", dr.get(), sr.get())
            }
        }
    }
}
