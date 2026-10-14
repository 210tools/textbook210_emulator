use crate::emulator::micro_op::{CycleState, MicroOp, MicroOpGenerator};
use crate::emulator::{BitAddressable, EmulatorCell};
use std::collections::HashMap;

use super::Op;
#[derive(Debug, Clone)]
pub struct JmpOp {
    /// Base register index, this is added to the offset to calculate where to jump
    pub base_r: EmulatorCell,
}

impl MicroOpGenerator for JmpOp {
    fn generate_plan(&self) -> HashMap<CycleState, Vec<MicroOp>> {
        let mut plan = HashMap::new();

        // Evaluate Address phase - get target address from register
        plan.insert(
            CycleState::EvaluateAddress,
            vec![
                MicroOp::State(12),
                MicroOp::addr_reg(self.base_r.get(), 0, 0),
            ],
        );

        // Execute phase - update PC to target address
        plan.insert(
            CycleState::Execute,
            vec![
                micro_op!(MSG format!("Jump to address in R{}", self.base_r.get())),
                micro_op!(PC <- AddrOut),
            ],
        );

        plan
    }
}

impl Op for JmpOp {
    fn decode(ir: EmulatorCell) -> Self {
        // LAYOUT: 1100 | 000 | BaseR | 000000
        let base_r = ir.range(8..6);

        Self { base_r }
    }
}

use std::fmt;

impl fmt::Display for JmpOp {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let base_r_index = self.base_r.get();

        if base_r_index == 7 {
            write!(f, "RET")?;
        } else {
            write!(f, "JMP R{base_r_index}")?;
        }
        Ok(())
    }
}
