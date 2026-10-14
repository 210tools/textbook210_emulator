use crate::emulator::micro_op::{CycleState, MicroOp, MicroOpGenerator};
use crate::emulator::{BitAddressable, EmulatorCell};
use std::collections::HashMap;

use super::Op;

#[derive(Debug, Clone)]
/// load from given register and offset Mem[Base_r + offset6]
pub struct LdrOp {
    pub dr: EmulatorCell,      // Destination Register index
    pub base_r: EmulatorCell,  // Base Register index
    pub offset6: EmulatorCell, // offset6 (sign-extended)
}

impl MicroOpGenerator for LdrOp {
    fn generate_plan(&self) -> HashMap<CycleState, Vec<MicroOp>> {
        let mut plan = HashMap::new();

        // Evaluate Address phase - calculate effective address from base + offset
        plan.insert(
            CycleState::EvaluateAddress,
            vec![
                MicroOp::State(6),
                MicroOp::addr_reg(self.base_r.get(), self.offset6.get() as i16, 6),
            ],
        );

        // Fetch Operands phase - set MAR for memory read
        plan.insert(
            CycleState::FetchOperands,
            vec![
                micro_op!(MAR <- AddrOut),
                MicroOp::State(35),
                MicroOp::State(25),
            ],
        );
        // Memory read MDR <- MEM[MAR] happens implicitly between phases

        // Store Result phase - move loaded value to destination register
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

impl Op for LdrOp {
    fn decode(ir: EmulatorCell) -> Self {
        // LAYOUT: 0110 | DR | BaseR | offset6
        let dr = ir.range(11..9);
        let base_r = ir.range(8..6);
        // Extract and sign-extend offset6
        let offset6 = ir.range(5..0).sext(5);

        Self {
            dr,
            base_r,
            offset6,
        }
    }
}
use std::fmt;

impl fmt::Display for LdrOp {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let dr_index = self.dr.get();
        let base_r_index = self.base_r.get();
        // offset6 is already sign-extended from decode
        let offset_val_signed = self.offset6.get() as i16;
        let offset_val_raw = self.offset6.get() & 0x3F; // Get raw 6 bits

        write!(
            f,
            "LDR R{dr_index}, R{base_r_index}, #{offset_val_signed} (x{offset_val_raw:02X})"
        )?;
        Ok(())
    }
}
