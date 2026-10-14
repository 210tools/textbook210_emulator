use crate::emulator::micro_op::{CycleState, MicroOp, MicroOpGenerator};
use crate::emulator::{BitAddressable, EmulatorCell};
use std::collections::HashMap;

use super::Op;

#[derive(Debug, Clone)]
/// Store some register value at `mem[mem[pc+pc_offset]]`
pub struct StiOp {
    pub sr: EmulatorCell,        // Source Register index
    pub pc_offset: EmulatorCell, // PCoffset9 (sign-extended)
}

impl MicroOpGenerator for StiOp {
    fn generate_plan(&self) -> HashMap<CycleState, Vec<MicroOp>> {
        let mut plan = HashMap::new();

        // Evaluate Address phase - calculate pointer address
        plan.insert(
            CycleState::EvaluateAddress,
            vec![
                MicroOp::State(11),
                MicroOp::addr_pc(self.pc_offset.get() as i16, 9),
            ],
        );

        // Fetch Operands phase - read pointer to get final address
        plan.insert(
            CycleState::FetchOperands,
            vec![
                micro_op!(MAR <- AddrOut),
                MicroOp::State(19),
                MicroOp::State(29),
            ],
        );
        // Memory read happens implicitly: MDR <- MEM[MAR] (gets indirect address)

        // Store Result phase - trigger memory write
        plan.insert(
            CycleState::StoreResult,
            vec![
                MicroOp::State(31),
                micro_op!(MAR <- MDR), // Set MAR to indirect address
                MicroOp::State(23),
                micro_op!(MDR <- R(self.sr.get())), // Load value to store
                MicroOp::State(16),
                micro_op!(SET_FLAG(WriteMemory)),
            ],
        );

        plan
    }
}

impl Op for StiOp {
    fn decode(ir: EmulatorCell) -> Self {
        // LAYOUT: 1011 | SR | PCoffset9

        // Extract and sign-extend PCoffset9
        let pc_offset = ir.range(8..0).sext(8);

        Self {
            sr: ir.range(11..9),
            pc_offset,
        }
    }
}
use std::fmt;

impl fmt::Display for StiOp {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // Display based on the state after decode (sr and pc_offset are known)

        let sr_index = self.sr.get();
        // Sign-extend the 9-bit offset for correct decimal representation
        let offset_val = self.pc_offset.sext(8).get() as i16;

        write!(
            f,
            "STI R{}, #{} (x{:03X})",
            sr_index,
            offset_val,                   // Display signed decimal offset
            self.pc_offset.get() & 0x1FF  // Display raw 9-bit hex offset
        )?;
        Ok(())
    }
}
