use crate::emulator::micro_op::{CycleState, MicroOp, MicroOpGenerator};
use crate::emulator::{BitAddressable, EmulatorCell};
use std::collections::HashMap;

use super::Op;

#[derive(Debug, Clone)]
/// Load the effective adress of some offset from PC
pub struct LeaOp {
    pub dr: EmulatorCell,        // Destination Register index
    pub pc_offset: EmulatorCell, // PCoffset9 (sign-extended)
}

impl MicroOpGenerator for LeaOp {
    fn generate_plan(&self) -> HashMap<CycleState, Vec<MicroOp>> {
        let mut plan = HashMap::new();

        // Evaluate Address phase - calculate effective address
        plan.insert(
            CycleState::EvaluateAddress,
            vec![micro_op!(ALU_OUT <- PC + PCOFFSET(self.pc_offset.get() as i16))],
        );

        // Store Result phase - store effective address in destination register
        plan.insert(
            CycleState::StoreResult,
            vec![micro_op!(R(self.dr.get()) <- AluOut)],
        );

        plan
    }
}

impl Op for LeaOp {
    fn decode(ir: EmulatorCell) -> Self {
        // LAYOUT: 1110 | DR | PCoffset9
        let dr = ir.range(11..9);
        // Extract and sign-extend PCoffset9 during decode
        let pc_offset = ir.range(8..0).sext(8);

        Self { dr, pc_offset }
    }
}

impl std::fmt::Display for LeaOp {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let dr_index = self.dr.get();
        let offset_val = self.pc_offset.get() as i16; // Cast to signed for proper display

        write!(
            f,
            "LEA R{}, #{} (x{:03X})",
            dr_index,
            offset_val,
            self.pc_offset.get() & 0x1FF
        )?;

        Ok(())
    }
}
