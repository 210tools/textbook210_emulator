use crate::emulator::micro_op::{CycleState, MicroOp, MicroOpGenerator};
use crate::emulator::{BitAddressable, EmulatorCell};
use std::collections::HashMap;
use std::fmt;

use super::Op;

#[derive(Debug, Clone)]
/// The add operation
pub enum AddOp {
    /// We have been suplied a 5 bit value to add to some register we have not yet fetched
    Immidiate {
        dr: EmulatorCell,
        sr1: EmulatorCell,
        imm5: EmulatorCell,
    },
    /// We have been suplied a register we have not yet fetched to add to some register we have not yet fetched
    Register {
        dr: EmulatorCell,
        sr1: EmulatorCell,
        sr2: EmulatorCell,
    },
}

impl MicroOpGenerator for AddOp {
    fn generate_plan(&self) -> HashMap<CycleState, Vec<MicroOp>> {
        let mut plan = HashMap::new();

        match self {
            AddOp::Immidiate { dr, sr1, imm5 } => {
                // Execute phase
                plan.insert(
                    CycleState::Execute,
                    vec![micro_op!(ALU_OUT <- R(sr1.get()) + IMM(imm5.sext(4).get() as i16))],
                );

                // Store Result phase
                plan.insert(
                    CycleState::StoreResult,
                    vec![
                        micro_op!(R(dr.get()) <- AluOut),
                        micro_op!(SET_CC(dr.get())),
                    ],
                );
            }
            AddOp::Register { dr, sr1, sr2 } => {
                // Execute phase
                plan.insert(
                    CycleState::Execute,
                    vec![micro_op!(ALU_OUT <- R(sr1.get()) + R(sr2.get()))],
                );

                // Store Result phase
                plan.insert(
                    CycleState::StoreResult,
                    vec![
                        micro_op!(R(dr.get()) <- AluOut),
                        micro_op!(SET_CC(dr.get())),
                    ],
                );
            }
        }

        plan
    }
}

impl Op for AddOp {
    fn decode(ir: EmulatorCell) -> Self {
        let span = tracing::trace_span!("ADD_decode", op = ir.get());
        let _enter = span.enter();

        // LAYOUT: 0001 | DR | SR1 | ImmTBit | (Register || Immediate)
        let dr = ir.range(11..9);
        let sr1 = ir.range(8..6);

        // Check immediate mode (bit[5])
        match ir.index(5).get() {
            0 => {
                // Register mode
                let sr2 = ir.range(2..0);
                AddOp::Register { dr, sr1, sr2 }
            }
            1 => {
                // Immediate mode
                // imm5 is bits 4..0
                let imm5 = ir.range(4..0);

                AddOp::Immidiate { dr, sr1, imm5 }
            }
            _ => unreachable!("Bit 5 can only be 0 or 1"),
        }
    }
}

impl fmt::Display for AddOp {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AddOp::Immidiate { dr, sr1, imm5 } => {
                // Sign extend imm5 for correct decimal representation
                let imm_value = imm5.sext(4).get() as i16; // sext from bit 4 for 5 bits
                write!(
                    f,
                    "ADD R{}, R{}, #{:?} (x{:02X})",
                    dr.get(),
                    sr1.get(),
                    imm_value,
                    imm5.get() & 0x1F // Mask to 5 bits for hex
                )
            }
            AddOp::Register { dr, sr1, sr2 } => {
                write!(f, "ADD R{}, R{}, R{}", dr.get(), sr1.get(), sr2.get())
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::emulator::micro_op::{
        CycleState, DataDestination, DataSource, MachineFlag, MicroOp,
    };

    #[test]
    fn test_add_register_micro_op_generation() {
        // Create an instruction: ADD R1, R2, R3 (0001 001 010 0 00 011)
        let ir = EmulatorCell::new(0b0001_001_010_0_00_011);
        let add_op = AddOp::decode(ir);

        let plan = add_op.generate_plan();

        // Should have Execute and StoreResult phases
        assert!(plan.contains_key(&CycleState::Execute));
        assert!(plan.contains_key(&CycleState::StoreResult));

        // Execute phase
        let execute_ops = &plan[&CycleState::Execute];
        assert_eq!(execute_ops.len(), 1);

        // Check ALU operation
        match &execute_ops[0] {
            MicroOp::Alu {
                operation,
                operand1,
                operand2,
            } => {
                assert!(matches!(operation, crate::emulator::micro_op::MAluOp::Add));
                assert!(matches!(operand1, DataSource::Register(2)));
                assert!(matches!(operand2, DataSource::Register(3)));
            }
            _ => panic!("Expected ALU ADD operation"),
        }

        // Store Result phase
        let store_ops = &plan[&CycleState::StoreResult];
        assert_eq!(store_ops.len(), 2);

        // Check result transfer
        match &store_ops[0] {
            MicroOp::Transfer {
                source,
                destination,
            } => {
                assert!(matches!(source, DataSource::AluOut));
                assert!(matches!(destination, DataDestination::Register(1)));
            }
            _ => panic!("Expected transfer from ALU_OUT to R1"),
        }

        // Check condition code update
        match &store_ops[1] {
            MicroOp::SetFlag(MachineFlag::UpdateCondCodes(1)) => (),
            _ => panic!("Expected SET_CC(1)"),
        }
    }

    #[test]
    fn test_add_immediate_micro_op_generation() {
        // Create an instruction: ADD R1, R2, #5 (0001 001 010 1 00101)
        let ir = EmulatorCell::new(0b0001_001_010_1_00101);
        let add_op = AddOp::decode(ir);

        let plan = add_op.generate_plan();

        // Execute phase
        let execute_ops = &plan[&CycleState::Execute];

        // Check ALU operation with immediate
        match &execute_ops[0] {
            MicroOp::Alu {
                operation,
                operand1,
                operand2,
            } => {
                assert!(matches!(operation, crate::emulator::micro_op::MAluOp::Add));
                assert!(matches!(operand1, DataSource::Register(2)));
                assert!(matches!(operand2, DataSource::Immediate(5)));
            }
            _ => panic!("Expected ALU ADD operation with immediate"),
        }
    }

    #[test]
    fn test_add_immediate_negative_micro_op_generation() {
        // Create an instruction: ADD R1, R2, #-1 (0001 001 010 1 11111)
        let ir = EmulatorCell::new(0b0001_001_010_1_11111);
        let add_op = AddOp::decode(ir);

        let plan = add_op.generate_plan();

        // Check ALU operation with negative immediate
        let execute_ops = &plan[&CycleState::Execute];
        match &execute_ops[0] {
            MicroOp::Alu {
                operation,
                operand1,
                operand2,
            } => {
                assert!(matches!(operation, crate::emulator::micro_op::MAluOp::Add));
                assert!(matches!(operand1, DataSource::Register(2)));
                assert!(matches!(operand2, DataSource::Immediate(-1)));
            }
            _ => panic!("Expected ALU ADD operation with negative immediate"),
        }
    }

    #[test]
    fn test_add_display_format() {
        // Test register mode display
        let ir_reg = EmulatorCell::new(0b0001_001_010_0_00_011);
        let add_reg = AddOp::decode(ir_reg);
        assert_eq!(format!("{add_reg}"), "ADD R1, R2, R3");

        // Test immediate mode display
        let ir_imm = EmulatorCell::new(0b0001_001_010_1_00101);
        let add_imm = AddOp::decode(ir_imm);
        assert_eq!(format!("{add_imm}"), "ADD R1, R2, #5 (x05)");

        // Test negative immediate display
        let ir_neg = EmulatorCell::new(0b0001_001_010_1_11111);
        let add_neg = AddOp::decode(ir_neg);
        assert_eq!(format!("{add_neg}"), "ADD R1, R2, #-1 (x1F)");
    }
}
