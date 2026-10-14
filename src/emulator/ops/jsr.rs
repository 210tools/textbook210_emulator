use crate::emulator::micro_op::{CycleState, MicroOp, MicroOpGenerator};
use crate::emulator::{BitAddressable, EmulatorCell};
use std::collections::HashMap;

use super::Op;

#[derive(Debug, Clone)]
/// Are we looking at jsr or jsrr
pub enum JsrMode {
    /// JSR: jump to a sub-routine the adress at pc + imm11
    Relative { pc_offset: EmulatorCell },
    /// JSRR: jump to a sub-routine the adress stored at a given register
    Register { base_r: EmulatorCell },
}

#[derive(Debug, Clone)]
/// Jump to a sub routine either directly or via pc offset
pub struct JsrOp {
    /// are we looking at jst or jsrr?
    pub mode: JsrMode,
}

impl MicroOpGenerator for JsrOp {
    fn generate_plan(&self) -> HashMap<CycleState, Vec<MicroOp>> {
        let mut plan = HashMap::new();

        // Evaluate Address phase - save return address and calculate target
        plan.insert(
            CycleState::EvaluateAddress,
            vec![
                // state 4 tests IR[11]; state 21 or 20 does R7 <- PC and loads PC in one cycle,
                // so the target is computed from the old R7 (JSRR R7 must work)
                MicroOp::State(4),
                MicroOp::State(match &self.mode {
                    JsrMode::Relative { .. } => 21,
                    JsrMode::Register { .. } => 20,
                }),
                match &self.mode {
                    JsrMode::Relative { pc_offset } => MicroOp::addr_pc(pc_offset.get() as i16, 11),
                    JsrMode::Register { base_r } => MicroOp::addr_reg(base_r.get(), 0, 0),
                },
                micro_op!(R(7) <- PC), // Save return address in R7
            ],
        );

        // Execute phase - jump to target address
        plan.insert(
            CycleState::Execute,
            vec![
                micro_op!(MSG "Jump to subroutine at target address".to_string()),
                micro_op!(PC <- AddrOut),
            ],
        );

        plan
    }
}

impl Op for JsrOp {
    fn decode(ir: EmulatorCell) -> Self {
        // LAYOUT: 0100 | ToggleBit | rest of bits (PCoffset11 or BaseR)

        let mode = if ir.index(11).get() == 1 {
            // JSR: Use PC-relative addressing
            // Extract and sign-extend PCoffset11
            let pc_offset = ir.range(10..0).sext(10);
            JsrMode::Relative { pc_offset }
        } else {
            // JSRR: Get address from base register
            let base_r = ir.range(8..6);
            JsrMode::Register { base_r }
        };

        Self { mode }
    }
}

use std::fmt;

impl fmt::Display for JsrOp {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // Display implementation based on the state after decode.
        match &self.mode {
            JsrMode::Relative { pc_offset } => {
                // JSR: Display with PC-relative offset
                let offset_val = pc_offset.get() as i16; // Cast to signed for decimal display
                write!(
                    f,
                    "JSR #{} (x{:03X})",
                    offset_val,
                    pc_offset.get() & 0x7FF // Mask to 11 bits for hex
                )?;
            }
            JsrMode::Register { base_r } => {
                // JSRR: Display with base register
                write!(f, "JSRR R{}", base_r.get())?;
            }
        }
        Ok(())
    }
}
