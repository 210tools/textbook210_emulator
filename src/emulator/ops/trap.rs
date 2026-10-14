use super::Op;
use crate::emulator::micro_op::{
    CycleState, DataDestination as D, DataSource as S, MicroOp, MicroOpGenerator,
};
use crate::emulator::{states, BitAddressable, EmulatorCell};
use std::collections::HashMap;

#[derive(Debug, Clone)]
/// TRAP works like a special kind of jump instruction.
/// 1. Pushes the current PC (return address) onto the system stack
/// 2. Pushes PSR (processor status register) onto the system stack
/// 3. Switches the CPU to Supervisor mode.
/// 4. Switches the Stack Pointer (R6) from User SP (USP) to Supervisor SP (SSP).
/// 5. Reads the starting address of the trap handler routine from the Trap Vector Table (Memory[0x0000 + ZEXT(trapvect8)]).
/// 6. Jumps to that handler routine address.
pub struct TrapOp {
    pub trap_vector: EmulatorCell, // The 8-bit vector number from the instruction
}

impl MicroOpGenerator for TrapOp {
    fn generate_plan(&self) -> HashMap<CycleState, Vec<MicroOp>> {
        let mut plan = HashMap::new();
        // Figure C.7: state 15, state 47, state 45 if the program was in User mode, then the
        // states that push PSR and PC and load PC from the trap vector table (37 to 55).
        let mut ops = vec![
            MicroOp::State(15),
            MicroOp::transfer(D::Table, S::Const8(0x00)),
            // PC is incremented here and decremented in state 43, which interrupts share
            micro_op!(PC <- PcPlusOne),
            MicroOp::transfer(D::MDR, S::PSR),
            MicroOp::State(47),
            MicroOp::transfer(D::Vector, S::IrLow8),
            MicroOp::transfer(D::Priv, S::Zero),
        ];
        ops.extend(states::to_supervisor_stack());
        ops.extend(states::push_and_vector());
        plan.insert(CycleState::Execute, ops);
        plan
    }
}

impl Op for TrapOp {
    fn decode(ir: EmulatorCell) -> Self {
        // LAYOUT: 1111 | 0000 | trapvect8
        let trap_vector = ir.range(7..0); // ZEXT occurs implicitly via range + EmulatorCell

        Self { trap_vector }
    }
}
use std::fmt;

impl fmt::Display for TrapOp {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let vector_val = self.trap_vector.get(); // Get the 8-bit vector value

        // Check for common trap aliases
        match vector_val {
            0x20 => write!(f, "GETC"),
            0x21 => write!(f, "OUT"),
            0x22 => write!(f, "PUTS"),
            0x23 => write!(f, "IN"),
            0x24 => write!(f, "PUTSP"),
            0x25 => write!(f, "HALT"),
            _ => write!(f, "TRAP x{vector_val:02X}"), // Fallback for unknown vectors
        }?;

        Ok(())
    }
}
