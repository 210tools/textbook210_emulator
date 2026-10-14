use super::Op;
use crate::emulator::micro_op::{
    CycleState, DataDestination as D, DataSource as S, MicroOp, MicroOpGenerator,
};
use crate::emulator::{BitAddressable, EmulatorCell, Exception, PSR_ADDR};
use std::collections::HashMap;

#[derive(Debug, Clone)]
/// This is executed by the os on service routines to return control to the callee
pub struct RtiOp;

impl MicroOpGenerator for RtiOp {
    fn generate_plan(&self) -> HashMap<CycleState, Vec<MicroOp>> {
        let mut plan = HashMap::new();
        // Figure C.7: pop PC, then PSR, then switch back to the user stack if returning to
        // User mode (state 59) or do nothing (state 51).
        plan.insert(
            CycleState::Execute,
            vec![
                MicroOp::State(8),
                micro_op!(MAR <- R(6)),
                MicroOp::new_custom(
                    |emu| {
                        if emu.memory[PSR_ADDR].index(15).get() == 1 {
                            Err(Exception::PrivilegeViolation)
                        } else {
                            Ok(())
                        }
                    },
                    "[PSR[15]]: RTI in User mode is a privilege mode exception (state 44)"
                        .to_owned(),
                ),
                MicroOp::State(36),
                micro_op!(-> Execute), // MDR <- M[MAR]
                MicroOp::State(38),
                micro_op!(PC <- MDR),
                MicroOp::State(39),
                MicroOp::transfer(D::MAR, S::SpPlusOne),
                MicroOp::transfer(D::Register(6), S::SpPlusOne),
                MicroOp::State(40),
                micro_op!(-> Execute), // MDR <- M[MAR]
                MicroOp::State(42),
                micro_op!(PSR <- MDR),
                MicroOp::State(34),
                MicroOp::transfer(D::Register(6), S::SpPlusOne),
                MicroOp::new_custom(
                    |emu| {
                        if emu.memory[PSR_ADDR].index(15).get() == 0 {
                            emu.skip_state = Some(59);
                        }
                        Ok(())
                    },
                    "[PSR[15]]: back to User mode goes to state 59, else state 51".to_owned(),
                ),
                MicroOp::State(59),
                MicroOp::transfer(D::SavedSsp, S::Register(6)),
                MicroOp::transfer(D::Register(6), S::SavedUsp),
                MicroOp::new_custom(
                    |emu| {
                        emu.skip_state = Some(51);
                        Ok(())
                    },
                    "then to state 18".to_owned(),
                ),
                MicroOp::State(51),
            ],
        );
        plan
    }
}

impl Op for RtiOp {
    fn decode(_ir: EmulatorCell) -> Self {
        // RTI has a fixed opcode (1000) and no operands in the instruction itself.
        Self
    }
}
use std::fmt;

impl fmt::Display for RtiOp {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // RTI has no operands encoded in the instruction to display.
        write!(f, "RTI")
    }
}
