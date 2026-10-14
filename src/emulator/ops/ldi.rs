use crate::emulator::micro_op::{CycleState, MicroOp, MicroOpGenerator};
use crate::emulator::{BitAddressable, EmulatorCell};
use std::collections::HashMap;

use super::Op;

#[derive(Debug, Clone)]
/// Load indirectly from an offset so we load Mem[Mem[PC + PCoffset9]]
pub struct LdiOp {
    pub dr: EmulatorCell,        // Destination Register index
    pub pc_offset: EmulatorCell, // PCoffset9 (sign-extended)
}

impl MicroOpGenerator for LdiOp {
    fn generate_plan(&self) -> HashMap<CycleState, Vec<MicroOp>> {
        let mut plan = HashMap::new();

        // Evaluate Address phase - calculate pointer address
        plan.insert(
            CycleState::EvaluateAddress,
            vec![
                MicroOp::State(10),
                MicroOp::addr_pc(self.pc_offset.get() as i16, 9),
            ],
        );

        // Fetch Operands phase - first memory read for pointer
        plan.insert(
            CycleState::FetchOperands,
            vec![
                micro_op!(MAR <- AddrOut),
                MicroOp::State(17),
                MicroOp::State(24),
                // First memory read happens implicitly: MDR <- MEM[MAR]
            ],
        );
        // Then second fetch happens with MDR as new address

        plan.insert(
            CycleState::Execute,
            vec![
                MicroOp::State(26),
                micro_op!(MAR <- MDR),
                MicroOp::State(35),
                MicroOp::State(25),
                // Second memory read happens implicitly: MDR <- MEM[MAR]
            ],
        );

        // Store Result phase - move final loaded value to destination register
        plan.insert(
            CycleState::StoreResult,
            vec![
                MicroOp::State(27),
                micro_op!(R(self.dr.get()) <- MDR),
                micro_op!(SET_CC(self.dr.get())),
            ],
        );

        plan
    }
}

impl Op for LdiOp {
    fn decode(ir: EmulatorCell) -> Self {
        // LAYOUT: 1010 | DR | PCoffset9
        let dr = ir.range(11..9);
        // Extract and sign-extend PCoffset9
        let pc_offset = ir.range(8..0).sext(8);

        Self { dr, pc_offset }
    }
}

use std::fmt;

impl fmt::Display for LdiOp {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // Format offset as signed decimal
        let offset_val = self.pc_offset.get() as i16; // Cast to signed for proper display

        write!(
            f,
            "LDI R{}, #{} (x{:03X})",
            self.dr.get(),
            offset_val,
            self.pc_offset.get() & 0x1FF // Mask to 9 bits for hex
        )?;
        Ok(())
    }
}
