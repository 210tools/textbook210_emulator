use crate::emulator::micro_op::{CycleState, MicroOp, MicroOpGenerator};
use crate::emulator::{BitAddressable, EmulatorCell};
use std::collections::HashMap;

use super::Op;

#[derive(Debug, Clone)]
/// Store some register value at adress of some other register value
pub struct StrOp {
    pub sr: EmulatorCell,      // Source Register index
    pub base_r: EmulatorCell,  // Base Register index
    pub offset6: EmulatorCell, // offset6 (sign-extended)
}

impl MicroOpGenerator for StrOp {
    fn generate_plan(&self) -> HashMap<CycleState, Vec<MicroOp>> {
        let mut plan = HashMap::new();

        // Evaluate Address phase - calculate effective address from base + offset
        plan.insert(
            CycleState::EvaluateAddress,
            vec![
                MicroOp::State(7),
                MicroOp::addr_reg(self.base_r.get(), self.offset6.get() as i16, 6),
            ],
        );

        // Store Result phase - trigger memory write
        plan.insert(
            CycleState::StoreResult,
            vec![
                micro_op!(MAR <- AddrOut),
                MicroOp::State(23),
                micro_op!(MDR <- R(self.sr.get())),
                MicroOp::State(16),
                micro_op!(SET_FLAG(WriteMemory)),
            ],
        );

        plan
    }
}

impl Op for StrOp {
    fn decode(ir: EmulatorCell) -> Self {
        // LAYOUT: 0111 | SR | BaseR | offset6
        let sr = ir.range(11..9);
        let base_r = ir.range(8..6);
        // Extract and sign-extend offset6
        let offset6 = ir.range(5..0).sext(5);

        Self {
            sr,
            base_r,
            offset6,
        }
    }
}
use std::fmt;

impl fmt::Display for StrOp {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // Get the raw 6-bit offset value
        let offset_val_6bit = self.offset6.get() & 0x3F;
        // Calculate the sign-extended value (as i16 for display)
        let offset_val_sext = if (offset_val_6bit >> 5) & 1 == 1 {
            // Negative number, extend with 1s
            (offset_val_6bit as i16) | !0x3F // or (offset_val_6bit as i16) - 64
        } else {
            offset_val_6bit as i16
        };

        write!(
            f,
            "STR R{}, R{}, #{} (x{:02X})",
            self.sr.get(),
            self.base_r.get(),
            offset_val_sext, // Display sign-extended decimal
            offset_val_6bit  // Display raw 6-bit hex
        )?;
        Ok(())
    }
}
