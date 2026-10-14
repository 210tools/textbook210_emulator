use std::fmt::Display;

pub use add::AddOp;
pub use and::AndOp;
pub use br::BrOp;
pub use jmp::JmpOp;
pub use jsr::JsrOp;
pub use ld::LdOp;
pub use ldi::LdiOp;
pub use ldr::LdrOp;
pub use lea::LeaOp;
pub use not::NotOp;
pub use rti::RtiOp;
pub use st::StOp;
pub use sti::StiOp;
pub use str::StrOp;
pub use trap::TrapOp;

use super::{BitAddressable, EmulatorCell};

mod add;
mod and;
mod br;
mod jmp;
pub mod jsr;
mod ld;
mod ldi;
mod ldr;
mod lea;
mod not;
mod rti;
mod st;
mod sti;
mod str;
mod trap;

#[derive(Debug, Clone)]
/// This encodes the key data used in the state machine to decide the next action to take
pub enum CpuState {
    Fetch,                    // Fetch instruction from memory location pointed by PC into IR
    Decode,                   // Decode instruction in IR, identify opcode and operands
    EvaluateAddress(OpCode),  // Calculate memory address for operands or target (if needed)
    FetchOperands(OpCode),    // Fetch operands from registers or memory
    ExecuteOperation(OpCode), // Perform the operation (ALU, branch check, PC update, etc.)
    StoreResult(OpCode),      // Write the result back to register or memory (if needed)
    /// Running the exception entry sequence (Figure C.7: states 44, 13 or 48-61, then 45-55)
    Exception,
}

impl Display for CpuState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CpuState::Fetch => write!(f, "Fetch"),
            CpuState::Decode => write!(f, "Decode"),
            CpuState::EvaluateAddress(op) => write!(f, "Evaluate Address {op}"),
            CpuState::FetchOperands(op) => write!(f, "Fetch Operands {op}"),
            CpuState::ExecuteOperation(op) => write!(f, "Execute Operation {op}"),
            CpuState::StoreResult(op) => write!(f, "Store Result {op}"),
            CpuState::Exception => write!(f, "Exception"),
        }
    }
}

/// Represents the decoded operation type.
#[derive(Debug, Clone)]
pub enum OpCode {
    Add(AddOp),
    And(AndOp),
    Br(BrOp),
    Jmp(JmpOp), // Includes RET
    Jsr(JsrOp), // Includes JSRR
    Ld(LdOp),
    Ldi(LdiOp),
    Ldr(LdrOp),
    Lea(LeaOp),
    Not(NotOp),
    Rti(RtiOp),
    St(StOp),
    Sti(StiOp),
    Str(StrOp),
    Trap(TrapOp),
}

impl CpuState {
    pub fn to_instruction(&self) -> Option<OpCode> {
        match self {
            CpuState::Fetch => None,
            CpuState::Decode => None,
            CpuState::EvaluateAddress(op_code) => Some(op_code),
            CpuState::FetchOperands(op_code) => Some(op_code),
            CpuState::ExecuteOperation(op_code) => Some(op_code),
            CpuState::StoreResult(op_code) => Some(op_code),
            CpuState::Exception => None,
        }
        .cloned()
    }
}

impl OpCode {
    /// Decodes an instruction from the machine state (usually from IR)
    /// and returns the corresponding OpCode variant containing the decoded operation details.
    pub fn from_instruction(instruction: EmulatorCell) -> Option<OpCode> {
        let opcode_val = instruction.range(15..12).get();

        match opcode_val {
            // Call the specific decode method for each opcode
            0x1 => Some(OpCode::Add(AddOp::decode(instruction))),
            0x5 => Some(OpCode::And(AndOp::decode(instruction))),
            0x0 => Some(OpCode::Br(BrOp::decode(instruction))),
            0xC => Some(OpCode::Jmp(JmpOp::decode(instruction))),
            0x4 => Some(OpCode::Jsr(JsrOp::decode(instruction))),
            0x2 => Some(OpCode::Ld(LdOp::decode(instruction))),
            0xA => Some(OpCode::Ldi(LdiOp::decode(instruction))),
            0x6 => Some(OpCode::Ldr(LdrOp::decode(instruction))),
            0xE => Some(OpCode::Lea(LeaOp::decode(instruction))),
            0x9 => Some(OpCode::Not(NotOp::decode(instruction))),
            0x8 => Some(OpCode::Rti(RtiOp::decode(instruction))),
            0x3 => Some(OpCode::St(StOp::decode(instruction))),
            0xB => Some(OpCode::Sti(StiOp::decode(instruction))),
            0x7 => Some(OpCode::Str(StrOp::decode(instruction))),
            0xF => Some(OpCode::Trap(TrapOp::decode(instruction))),
            // Opcode 13 (0xD) is unused/reserved in standard LC-3.
            _ => None, // Return None for invalid/unused opcode
        }
    }
}

impl OpCode {
    /// The micro-op plan generator for this instruction.
    pub fn plan_generator(&self) -> &dyn crate::emulator::micro_op::MicroOpGenerator {
        match self {
            OpCode::Add(op) => op,
            OpCode::And(op) => op,
            OpCode::Br(op) => op,
            OpCode::Jmp(op) => op,
            OpCode::Jsr(op) => op,
            OpCode::Ld(op) => op,
            OpCode::Ldi(op) => op,
            OpCode::Ldr(op) => op,
            OpCode::Lea(op) => op,
            OpCode::Not(op) => op,
            OpCode::Rti(op) => op,
            OpCode::St(op) => op,
            OpCode::Sti(op) => op,
            OpCode::Str(op) => op,
            OpCode::Trap(op) => op,
        }
    }
}

impl Display for OpCode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            OpCode::Add(op) => write!(f, "{op}"),
            OpCode::And(op) => write!(f, "{op}"),
            OpCode::Br(op) => write!(f, "{op}"),
            OpCode::Jmp(op) => write!(f, "{op}"),
            OpCode::Jsr(op) => write!(f, "{op}"),
            OpCode::Ld(op) => write!(f, "{op}"),
            OpCode::Ldi(op) => write!(f, "{op}"),
            OpCode::Ldr(op) => write!(f, "{op}"),
            OpCode::Lea(op) => write!(f, "{op}"),
            OpCode::Not(op) => write!(f, "{op}"),
            OpCode::Rti(op) => write!(f, "{op}"),
            OpCode::St(op) => write!(f, "{op}"),
            OpCode::Sti(op) => write!(f, "{op}"),
            OpCode::Str(op) => write!(f, "{op}"),
            OpCode::Trap(op) => write!(f, "{op}"),
        }
    }
}

// Trait defining the interface for LC-3 operations during different phases of the instruction cycle.
pub trait Op: std::fmt::Debug + Display {
    /// **Decode Phase:**
    /// Extract operand specifiers (register numbers, immediate values, offsets)
    /// from the instruction stored in `machine_state.ir`.
    fn decode(instruction: EmulatorCell) -> Self;
}
