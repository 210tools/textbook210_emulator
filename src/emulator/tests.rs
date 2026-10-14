use tracing_test::traced_test;

use crate::emulator::{parse::ParseOutput, BitAddressable, Emulator, EmulatorCell};

#[traced_test]
#[test]
fn test_full_program_execution() {
    tracing::info_span!("test_full_program_execution").in_scope(|| {
        tracing::info!("Starting comprehensive program execution test with all instructions");

        let program = r#"
            .ORIG x3000

            BR CODE
            
            ; First subroutine
            SUBROUTINE
                ADD R0, R0, #5      ; Add 5 to R0
                RET                 ; Return using R7

            ; Initialize registers
            CODE AND R0, R0, #0      ; Clear R0
            AND R1, R1, #0      ; Clear R1
            AND R2, R2, #0      ; Clear R2
            AND R3, R3, #0      ; Clear R3
            AND R4, R4, #0      ; Clear R4
            AND R5, R5, #0      ; Clear R5
            AND R6, R6, #0      ; Clear R6

            ; Load indirect and set up base register
            LDI R1, POINTER     ; Load R1 indirectly from pointer
            ADD R2, R1, #10     ; Add an immediate to R1

            ; Test branching
            ADD R3, R3, #1      ; Set R3 to 1
            BRp SKIP_SECTION    ; Branch if positive

            ; This section should be skipped
            ADD R3, R3, #15     ; Would set R3 to 16 if executed

            SKIP_SECTION:
            ADD R3, R3, #2      ; R3 now equals 3

            ; Test bitwise operations
            NOT R4, R3          ; Complement R3 into R4
            AND R5, R4, R1      ; Bitwise AND R4 and R1

            ; Load effective address
            LEA R6, SUBROUTINE  ; Load address of subroutine into R6

            ; Test register indirect jump
            JSRR R6             ; Jump to subroutine at address in R6

            ; After return, do a relative jump
            JSR SECOND_SUB      ; Jump to second subroutine using PC-relative

            ; After return from both subroutines, test load/store
            LD R1, DATA_VAL      ; Load address of DATA_VAL
            ADD R0, R0, R1       ; Add DATA_VAL address to R0
            ST R0, DATA_VAL      ; Store directly
            LD R0, DATA_VAL      ; Load value directly

            LD R2, REGISTER_STORE_ADDRESS ; store at x4000
            STR R0, R2, #0       ; Store using register-based addressing

            ; Load register-based
            LDR R3, R2, #0      ; Load the value we just stored

            ; Test NOT and store
            NOT R3, R3          ; Complement the value
            ST R3, RESULT       ; Store directly

            ; Test indirect store
            LEA R1, RESULT_PTR  ; Load address of result pointer
            STI R0, RESULT_PTR  ; Store indirectly


            ; Jump unconditionally past the data
            BRNZP END_PROG

            ; Data section should be skipped
            DATA_VAL: .FILL x0041    ; ASCII 'A' (65)
            RESULT:   .FILL x0000    ; To store direct result
            POINTER:  .FILL PTR_VAL  ; Pointer to a value
            PTR_VAL:  .FILL x00BE    ; Value 190 to load indirectly
            RESULT_PTR: .FILL INDIRECT_RESULT ; Pointer to indirect result
            INDIRECT_RESULT: .FILL x0000 ; To store indirect result
            REGISTER_STORE_ADDRESS: .FILL x4000


            ; Second subroutine
            SECOND_SUB:
                ADD R0, R0, #3      ; Add 3 to R0
                AND R5, R5, #0      ; Clear R5
                ADD R5, R5, #1      ; Set R5 to 1
                BRnzp RETURN_SUB    ; Branch always to return
                ADD R5, R5, #10     ; Shouldn't be executed

            RETURN_SUB:
                RET                 ; Return using R7

            END_PROG:
                OUT                 ; Output the character in R0
                HALT                ; Stop execution


            REN: .fill #-10

            .END
            "#;

        tracing::debug!(
            program = program,
            "Complex assembled program to test all instructions"
        );

        let mut machine_state = Emulator::new();

        // Parse the program
        tracing::debug!("Parsing program");
        let parse_result = Emulator::parse_program(program, None);

        // Check if parsing was successful
        assert!(parse_result.is_ok(), "Program parsing should succeed");

        let ParseOutput {
            labels,
            orig_address,
            machine_code,
            ..
        } = parse_result.unwrap();

        tracing::debug!("Loading program into emulator");
        machine_state.flash_memory(machine_code, orig_address);

        // Verify the program was loaded correctly
        assert_eq!(
            machine_state.pc.get(),
            0x0200,
            "PC should be set to OS origin address"
        );

        // Execute the program with a maximum number of steps
        tracing::debug!("Beginning program execution");
        machine_state.start_running();
        let result = machine_state.run(None);

        tracing::debug!("Program execution completed");
        tracing::debug!("Result: {:?}", result);

        // Verify execution completed successfully
        assert!(result.is_ok(), "Program execution should succeed");

        // Verify the machine halted
        assert!(!machine_state.running(), "Machine should have halted");

        // Verify the results in memory
        let direct_result_address = *labels.get("RESULT").unwrap();
        let expected_direct_result = 0xFFB6;

        // 2. Register-based store:
        let register_store_address = 0x4000;
        let expected_register_result = 0x49;

        // 3. Indirect store: Original value (A=0x41) = 0x41
        let indirect_result_address = *labels.get("INDIRECT_RESULT").unwrap();
        let expected_indirect_result = 0x49;

        tracing::debug!(
            pc = format!("0x{:04X}", machine_state.pc.get()),
            expected_direct = format!("0x{:04X}", expected_direct_result),
            direct_addr = format!("0x{:04X}", direct_result_address),
            actual_direct = format!(
                "0x{:04X}",
                machine_state.memory[direct_result_address].get()
            ),
            expected_register = format!("0x{:04X}", expected_register_result),
            register_addr = format!("0x{:04X}", register_store_address),
            actual_register = format!(
                "0x{:04X}",
                machine_state.memory[register_store_address].get()
            ),
            expected_indirect = format!("0x{:04X}", expected_indirect_result),
            indirect_addr = format!("0x{:04X}", indirect_result_address),
            actual_indirect = format!(
                "0x{:04X}",
                machine_state.memory[indirect_result_address].get()
            ),
            "Final machine state"
        );

        // Verify direct storage result
        assert_eq!(
            machine_state.memory[direct_result_address].get(),
            expected_direct_result,
            "Direct result should be stored correctly in memory"
        );

        // Verify register-based store result
        assert_eq!(
            machine_state.memory[register_store_address].get(),
            expected_register_result,
            "Register-based result should be stored correctly in memory"
        );

        // Verify indirect store result
        assert_eq!(
            machine_state.memory[indirect_result_address].get(),
            expected_indirect_result,
            "Indirect result should be stored correctly in memory"
        );

        tracing::info!("Comprehensive program execution test completed successfully");
    });
}

#[traced_test]
#[test]
fn test_cell_index_and_range() {
    tracing::info_span!("test_cell_index_and_range").in_scope(|| {
        tracing::info!("Starting test for BitAddressable index and range");

        // Test the index function
        let test_cell = EmulatorCell::new(0b1010_1100_0011_0101); // 0xAC35

        tracing::debug!(
            cell_value = format!("0x{:04X}", test_cell.get()),
            binary = format!("0b{:016b}", test_cell.get()),
            "Testing index operation on test cell"
        );

        // Test individual bit extraction
        let bit15 = test_cell.index(15).get();
        let bit10 = test_cell.index(10).get();
        let bit5 = test_cell.index(5).get();
        let bit0 = test_cell.index(0).get();

        tracing::debug!(
            bit15 = bit15,
            bit10 = bit10,
            bit5 = bit5,
            bit0 = bit0,
            "Individual bit extraction results"
        );

        // Expected values from binary 1010110000110101
        assert_eq!(bit15, 1, "Bit 15 should be 1");
        assert_eq!(bit10, 1, "Bit 10 should be 1");
        assert_eq!(bit5, 1, "Bit 5 should be 1");
        assert_eq!(bit0, 1, "Bit 0 should be 1");

        // Test the range function
        tracing::debug!(
            cell_value = format!("0x{:04X}", test_cell.get()),
            "Testing range operation on test cell"
        );

        // Extract different ranges
        let range_15_12 = test_cell.range(15..12).get(); // 1010
        let range_11_8 = test_cell.range(11..8).get(); // 1100
        let range_7_4 = test_cell.range(7..4).get(); // 0011
        let range_3_0 = test_cell.range(3..0).get(); // 0101

        tracing::debug!(
            range_15_12 = format!("0x{:X}", range_15_12),
            range_11_8 = format!("0x{:X}", range_11_8),
            range_7_4 = format!("0x{:X}", range_7_4),
            range_3_0 = format!("0x{:X}", range_3_0),
            "Range extraction results"
        );

        assert_eq!(range_15_12, 0xA, "Range 15..12 should be 0xA");
        assert_eq!(range_11_8, 0xC, "Range 11..8 should be 0xC");
        assert_eq!(range_7_4, 0x3, "Range 7..4 should be 0x3");
        assert_eq!(range_3_0, 0x5, "Range 3..0 should be 0x5");

        // Test sign extension with range and sext
        let negative_value = test_cell.range(15..12).sext(3).get(); // Sign-extend 1010
        let positive_value = test_cell.range(3..0).sext(3).get(); // Sign-extend 0101

        tracing::debug!(
            negative_range = format!("0b{:04b}", test_cell.range(15..12).get()),
            negative_extended = format!("0x{:04X}", negative_value),
            positive_range = format!("0b{:04b}", test_cell.range(3..0).get()),
            positive_extended = format!("0x{:04X}", positive_value),
            "Sign extension results"
        );

        // 1010 sign-extended from bit 3 should have 1s in upper bits
        assert_eq!(
            negative_value, 0xFFFA,
            "Negative value sign-extended from bit 3 should be 0xFFFA"
        );
        // 0101 sign-extended from bit 3 should keep 0s in upper bits
        assert_eq!(
            positive_value, 0x0005,
            "Positive value sign-extended from bit 3 should be 0x0005"
        );

        tracing::info!("BitAddressable index and range test completed successfully");
    });
}

#[traced_test]
#[test]
fn test_c_println_assembly() {
    tracing::info_span!("test_c_println_assembly").in_scope(|| {
        tracing::info!("Starting test for C-generated assembly code");

        // Load assembly file
        let assembly_content = include_str!("../../asm_tests/c-println.asm");

        tracing::debug!(
            assembly_size = assembly_content.len(),
            "Loaded assembly file from c-println.asm"
        );

        // Parse the program
        tracing::debug!("Parsing C-generated assembly program");
        let parse_result = Emulator::parse_program(assembly_content, None);

        // Check if parsing was successful
        assert!(parse_result.is_ok(), "Assembly parsing should succeed");

        let ParseOutput {
            machine_code,
            labels,
            orig_address,
            ..
        } = parse_result.unwrap();

        tracing::debug!(
            instruction_count = machine_code.len(),
            label_count = labels.len(),
            origin = format!("0x{:04X}", orig_address),
            "Assembly parsed successfully"
        );

        // Create an emulator and load the program
        let mut machine_state = Emulator::new();
        tracing::debug!("Loading assembly program into emulator");
        machine_state.flash_memory(machine_code, orig_address);

        // Execute the program with a maximum number of steps
        tracing::debug!("Beginning C-generated assembly program execution");
        let max_steps = 10000; // Prevent infinite loops
        machine_state.start_running();
        let result = machine_state.run(Some(max_steps));

        tracing::debug!("Program execution completed");
        tracing::debug!("Result: {:?}", result);

        // Verify execution completed successfully
        assert!(result.is_ok(), "Assembly execution should succeed");

        // Verify the machine halted
        assert!(!machine_state.running(), "Machine should have halted");

        // Log the output
        tracing::debug!(output = machine_state.output, "Assembly program output");

        // Verify some output was produced
        assert!(
            !machine_state.output.is_empty(),
            "Assembly program should produce output"
        );

        tracing::info!("C-generated assembly test completed successfully");
    });
}

// Individual Opcode Tests
#[cfg(test)]
fn run_instruction_test(
    initial_pc: u16,
    instruction: u16,
    setup_fn: impl FnOnce(&mut Emulator),
    assert_fn: impl FnOnce(&Emulator),
) {
    let mut machine = Emulator::new();
    machine.pc.set(initial_pc);
    machine.memory[initial_pc as usize].set(instruction);

    // Apply initial setup
    setup_fn(&mut machine);

    // Run the single instruction step
    machine.step();
    assert!(
        machine.exception.is_none(),
        "Instruction step failed: {:?}",
        machine.exception
    );

    // Assert final state
    assert_fn(&machine);
}

#[traced_test]
#[test]
fn test_add_register() {
    run_instruction_test(
        0x3000,
        0b0001_001_010_0_00_011, // ADD R1, R2, R3
        |m| {
            m.r[2].set(5);
            m.r[3].set(10);
        },
        |m| {
            assert_eq!(m.r[1].get(), 15, "R1 should be 5 + 10 = 15");
            assert_eq!(m.pc.get(), 0x3001, "PC should increment");
            let (n, z, p) = m.get_nzp();
            assert!(p, "Positive flag should be set");
            assert!(!z, "Zero flag should be clear");
            assert!(!n, "Negative flag should be clear");
        },
    );
}

#[traced_test]
#[test]
fn test_add_immediate() {
    run_instruction_test(
        0x3000,
        0b0001_001_010_1_00101, // ADD R1, R2, #5
        |m| {
            m.r[2].set(10);
        },
        |m| {
            assert_eq!(m.r[1].get(), 15, "R1 should be 10 + 5 = 15");
            assert_eq!(m.pc.get(), 0x3001, "PC should increment");
            let (.., p) = m.get_nzp();
            assert!(p, "Positive flag should be set");
        },
    );
}

#[traced_test]
#[test]
fn test_add_immediate_negative() {
    run_instruction_test(
        0x3000,
        0b0001_001_010_1_11111, // ADD R1, R2, #-1
        |m| {
            m.r[2].set(5);
        },
        |m| {
            assert_eq!(m.r[1].get(), 4, "R1 should be 5 + (-1) = 4");
            assert_eq!(m.pc.get(), 0x3001, "PC should increment");
            let (.., p) = m.get_nzp();
            assert!(p, "Positive flag should be set");
        },
    );
}

#[traced_test]
#[test]
fn test_and_register() {
    run_instruction_test(
        0x3000,
        0b0101_001_010_0_00_011, // AND R1, R2, R3
        |m| {
            m.r[2].set(0b1100);
            m.r[3].set(0b1010);
        },
        |m| {
            assert_eq!(
                m.r[1].get(),
                0b1000,
                "R1 should be 0b1100 & 0b1010 = 0b1000"
            );
            assert_eq!(m.pc.get(), 0x3001, "PC should increment");
            let (.., p) = m.get_nzp();
            assert!(p, "Positive flag should be set");
        },
    );
}

#[traced_test]
#[test]
fn test_and_immediate() {
    run_instruction_test(
        0x3000,
        0b0101_001_010_1_01010, // AND R1, R2, #10 (0b01010)
        |m| {
            m.r[2].set(0b1111); // 15
        },
        |m| {
            assert_eq!(m.r[1].get(), 0b01010, "R1 should be 0b1111 & 0b01010 = 10");
            assert_eq!(m.pc.get(), 0x3001, "PC should increment");
            let (.., p) = m.get_nzp();
            assert!(p, "Positive flag should be set");
        },
    );
}

#[traced_test]
#[test]
fn test_br_taken() {
    run_instruction_test(
        0x3000,
        0b0000_0_0_1_000001010, // BRP +10 (PC -> 0x300B)
        |m| {
            // Set positive flag in PSR
            m.set_p();
        },
        |m| {
            assert_eq!(m.pc.get(), 0x300B, "PC should jump to 0x3000 + 1 + 10");
        },
    );
}

#[traced_test]
#[test]
fn test_br_not_taken() {
    run_instruction_test(
        0x3000,
        0b0000_1_0_0_000001010, // BRn +10
        |m| {
            // Set positive flag (condition doesn't match)
            m.set_p()
        },
        |m| {
            assert_eq!(m.pc.get(), 0x3001, "PC should only increment");
        },
    );
}

#[traced_test]
#[test]
fn test_jmp() {
    run_instruction_test(
        0x3000,
        0b1100_000_011_000000, // JMP R3
        |m| {
            m.r[3].set(0x4000); // Set target address in R3
        },
        |m| {
            assert_eq!(m.pc.get(), 0x4000, "PC should jump to address in R3");
        },
    );
}

#[traced_test]
#[test]
fn test_ret() {
    run_instruction_test(
        0x3000,
        0b1100_000_111_000000, // RET (JMP R7)
        |m| {
            m.r[7].set(0x5000); // Set return address in R7
        },
        |m| {
            assert_eq!(m.pc.get(), 0x5000, "PC should jump to address in R7");
        },
    );
}

#[traced_test]
#[test]
fn test_jsr() {
    run_instruction_test(
        0x3000,
        0b0100_1_00000010000, // JSR +16 (Target 0x3011)
        |_| {},               // No specific setup needed
        |m| {
            assert_eq!(
                m.r[7].get(),
                0x3001,
                "R7 should contain the return address (PC+1)"
            );
            assert_eq!(
                m.pc.get(),
                0x3011,
                "PC should jump to 0x3000 + 1 + 16 = 0x3011"
            );
        },
    );
}

#[traced_test]
#[test]
fn test_jsrr() {
    run_instruction_test(
        0x3000,
        0b0100_0_00_011_000000, // JSRR R3
        |m| {
            m.r[3].set(0x6000); // Set subroutine address in R3
        },
        |m| {
            assert_eq!(
                m.r[7].get(),
                0x3001,
                "R7 should contain the return address (PC+1)"
            );
            assert_eq!(m.pc.get(), 0x6000, "PC should jump to address in R3");
        },
    );
}

#[traced_test]
#[test]
fn test_ld() {
    run_instruction_test(
        0x3000,
        0b0010_001_000000101, // LD R1, +5 (Load from 0x3006)
        |m| {
            m.memory[0x3006].set(0xABCD); // Value to load
        },
        |m| {
            assert_eq!(m.r[1].get(), 0xABCD, "R1 should contain value from memory");
            assert_eq!(m.pc.get(), 0x3001, "PC should increment");
            let (n, ..) = m.get_nzp();
            assert!(n, "Negative flag should be set (0xABCD)");
        },
    );
}

#[traced_test]
#[test]
fn test_ldr() {
    run_instruction_test(
        0x3000,
        0b0110_001_010_000101, // LDR R1, R2, #5
        |m| {
            m.r[2].set(0x4000); // Base address
            m.memory[0x4005].set(0x1234); // Value to load (0x4000 + 5)
        },
        |m| {
            assert_eq!(m.r[1].get(), 0x1234, "R1 should contain value from memory");
            assert_eq!(m.pc.get(), 0x3001, "PC should increment");
            let (.., p) = m.get_nzp();
            assert!(p, "Positive flag should be set (0x1234)");
        },
    );
}

#[traced_test]
#[test]
fn test_lea() {
    run_instruction_test(
        0x3000,
        0b1110_001_000000101, // LEA R1, +5 (Load address 0x3006)
        |_| {},
        |m| {
            assert_eq!(
                m.r[1].get(),
                0x3006,
                "R1 should contain the effective address 0x3000 + 1 + 5"
            );
            assert_eq!(m.pc.get(), 0x3001, "PC should increment");
        },
    );
}

#[traced_test]
#[test]
fn test_not() {
    run_instruction_test(
        0x3000,
        0b1001_001_010_111111, // NOT R1, R2
        |m| {
            m.r[2].set(0b0000_1111_0000_1111); // Input value
        },
        |m| {
            assert_eq!(
                m.r[1].get(),
                0b1111_0000_1111_0000,
                "R1 should contain the bitwise NOT"
            );
            assert_eq!(m.pc.get(), 0x3001, "PC should increment");
            let (n, ..) = m.get_nzp();
            assert!(n, "Negative flag should be set");
        },
    );
}

#[traced_test]
#[test]
fn test_st() {
    run_instruction_test(
        0x3000,
        0b0011_001_000000101, // ST R1, +5 (Store to 0x3006)
        |m| {
            m.r[1].set(0xFACE); // Value to store
        },
        |m| {
            assert_eq!(
                m.memory[0x3006].get(),
                0xFACE,
                "Memory at 0x3006 should contain value from R1"
            );
            assert_eq!(m.pc.get(), 0x3001, "PC should increment");
        },
    );
}

#[traced_test]
#[test]
fn test_str() {
    run_instruction_test(
        0x3000,
        0b0111_001_010_000101, // STR R1, R2, #5
        |m| {
            m.r[1].set(0xBEEF); // Value to store
            m.r[2].set(0x4000); // Base address
        },
        |m| {
            assert_eq!(
                m.memory[0x4005].get(),
                0xBEEF,
                "Memory at 0x4005 should contain value from R1"
            );
            assert_eq!(m.pc.get(), 0x3001, "PC should increment");
        },
    );
}

// --- debugger commands ---

fn loaded(program: &str) -> Emulator {
    loaded_with(crate::emulator::OsKind::Interrupt, program)
}

fn loaded_with(os: crate::emulator::OsKind, program: &str) -> Emulator {
    let mut emu = Emulator::with_os(os);
    emu.trace = true;
    let out = Emulator::parse_program(program, Some(&mut emu.metadata)).expect("program assembles");
    emu.load_program(crate::emulator::LoadedProgram {
        machine_code: out.machine_code,
        orig_address: out.orig_address,
        ..Default::default()
    });
    emu.speed = 1000;
    emu.ticks_between_updates = 1;
    emu.skip_os_emulation = true;
    // boot the OS until it hands over to the user program
    while emu.pc.get() != 0x3000 {
        emu.step_into();
        assert!(!emu.halted, "halted while booting");
    }
    emu
}

fn run_until_stopped(emu: &mut Emulator) {
    for _ in 0..10_000 {
        if !emu.running() {
            return;
        }
        emu.update();
    }
    panic!("machine never stopped");
}

const CALL_PROGRAM: &str = r#"
    .ORIG x3000
    JSR SUB
    ADD R1, R1, #1
    HALT
SUB ADD R0, R0, #2
    ST R7, SAVE7
    JSR SUB2
    LD R7, SAVE7
    ADD R0, R0, #3
    RET
SUB2 ADD R2, R2, #7
    RET
SAVE7 .FILL 0
    .END
"#;

#[test]
fn step_over_runs_the_whole_subroutine() {
    let mut emu = loaded(CALL_PROGRAM);
    emu.step_over();
    run_until_stopped(&mut emu);
    assert_eq!(emu.pc.get(), 0x3001);
    assert_eq!(emu.r[0].get(), 5);
    assert_eq!(emu.r[2].get(), 7);
    assert_eq!(emu.stop_reason, crate::emulator::StopReason::Stepped);
    emu.step_over();
    assert_eq!(emu.pc.get(), 0x3002);
    assert_eq!(emu.r[1].get(), 1);
}

#[test]
fn step_into_then_step_out_returns_to_caller() {
    let mut emu = loaded(CALL_PROGRAM);
    emu.step_into();
    assert_eq!(emu.pc.get(), 0x3003, "step into enters the subroutine");
    emu.step_into(); // ADD R0, R0, #2
    emu.step_into(); // ST R7, SAVE7
    emu.step_into(); // JSR SUB2
    assert_eq!(emu.pc.get(), 0x3009);
    emu.step_out(); // back into SUB
    run_until_stopped(&mut emu);
    assert_eq!(emu.pc.get(), 0x3006);
    emu.step_out(); // back to the main program
    run_until_stopped(&mut emu);

    assert_eq!(emu.pc.get(), 0x3001);
    assert_eq!(emu.r[0].get(), 5);
}

#[test]
fn breakpoints_stop_run_and_resume_continues() {
    let mut emu = loaded(CALL_PROGRAM);
    emu.breakpoints.insert(0x3007);
    emu.resume();
    run_until_stopped(&mut emu);
    assert_eq!(emu.pc.get(), 0x3007);
    assert_eq!(
        emu.stop_reason,
        crate::emulator::StopReason::Breakpoint(0x3007)
    );
    emu.resume();
    run_until_stopped(&mut emu);
    assert!(emu.halted);
    assert_eq!(emu.stop_reason, crate::emulator::StopReason::Halted);
}

#[test]
fn typed_keys_are_queued_until_read() {
    let mut emu = loaded(
        r#"
        .ORIG x3000
        GETC
        OUT
        GETC
        OUT
        GETC
        OUT
        HALT
        .END
    "#,
    );
    emu.output.clear();
    for c in "abc".chars() {
        emu.push_input(c);
    }
    emu.resume();
    run_until_stopped(&mut emu);
    assert!(emu.halted);
    assert!(emu.output.starts_with("abc"), "output was {:?}", emu.output);
}

#[test]
fn hard_reset_reloads_the_program() {
    let mut emu = loaded(CALL_PROGRAM);
    emu.memory[0x3000].set(0);
    emu.breakpoints.insert(0x3002);
    emu.hard_reset();
    assert_eq!(emu.memory[0x3000].get(), 0x4802);
    assert!(emu.breakpoints.contains(&0x3002));
    assert_eq!(emu.pc.get(), 0x200);
}

#[test]
fn two_programs_stay_loaded_together() {
    let mut emu = Emulator::new();
    let code = Emulator::parse_program(
        ".ORIG x3000\nLD R0, DATA\nHALT\nDATA .FILL x3500\n.END",
        None,
    )
    .unwrap();
    let data = Emulator::parse_program(".ORIG x3500\n.FILL #7\n.FILL #9\n.END", None).unwrap();
    emu.load_program(crate::emulator::LoadedProgram {
        source_id: 1,
        machine_code: code.machine_code,
        orig_address: 0x3000,
        ..Default::default()
    });
    emu.load_program(crate::emulator::LoadedProgram {
        source_id: 2,
        machine_code: data.machine_code,
        orig_address: 0x3500,
        ..Default::default()
    });
    assert_eq!(emu.memory[0x3500].get(), 7);
    assert_eq!(emu.memory[0x3000].get() >> 12, 0b0010, "code still loaded");
    // re-assembling the code keeps the data
    let code2 = Emulator::parse_program(
        ".ORIG x3000\nLD R1, DATA\nHALT\nDATA .FILL x3500\n.END",
        None,
    )
    .unwrap();
    emu.load_program(crate::emulator::LoadedProgram {
        source_id: 1,
        machine_code: code2.machine_code,
        orig_address: 0x3000,
        ..Default::default()
    });
    assert_eq!(emu.memory[0x3501].get(), 9);
    assert_eq!(emu.loaded_programs.len(), 2);
    assert_eq!(emu.program_at(0x3501).map(|p| p.source_id), Some(2));
}

const TRACE_PROGRAM: &str = r#"
    .ORIG x3000
    LD R1, VAL
    LDR R2, R1, #0
    ADD R3, R1, #2
    BRp SKIP
    ADD R3, R3, #1
SKIP JSRR R1
    HALT
VAL .FILL x3006
    .END
"#;

fn trace_texts(emu: &Emulator) -> Vec<String> {
    emu.micro_log
        .iter()
        .filter(|r| !r.marker)
        .map(|r| r.text.clone())
        .collect()
}

/// The state machine states the last instruction went through.
fn states_of(emu: &Emulator) -> Vec<u8> {
    crate::emulator::trace::cycles(&emu.micro_log)
        .iter()
        .map(|c| c.state)
        .collect()
}

#[test]
fn micro_log_follows_the_datapath() {
    use crate::emulator::trace::{Dp, Sig};
    let mut emu = loaded(TRACE_PROGRAM);

    emu.step_into(); // LD R1, VAL
    let texts = trace_texts(&emu);
    assert_eq!(texts[0], "MAR <- PC", "{texts:?}");
    assert!(texts.iter().any(|t| t == "PC <- PC + 1"), "{texts:?}");
    assert!(
        texts.iter().any(|t| t.starts_with("ADDR <- PC")),
        "{texts:?}"
    );
    assert!(texts.iter().any(|t| t == "MAR <- ADDR"), "{texts:?}");
    assert_eq!(emu.r[1].get(), 0x3006);

    // the fetch's PC increment goes through PCMUX, not the bus
    let inc = emu
        .micro_log
        .iter()
        .find(|r| r.text == "PC <- PC + 1")
        .unwrap();
    assert!(inc.activity.signal(Sig::PcMuxInc) && inc.activity.signal(Sig::LdPc));
    assert!(!inc.activity.has(Dp::Bus));
    assert_eq!(inc.value, Some(0x3001));

    // the address adder feeds MAR through MARMUX
    let mar = emu
        .micro_log
        .iter()
        .find(|r| r.text == "MAR <- ADDR")
        .unwrap();
    assert!(mar.activity.signal(Sig::GateMarMux) && mar.activity.signal(Sig::MarMuxAdder));
    assert_eq!(mar.value, Some(0x3007));
    let addr = emu
        .micro_log
        .iter()
        .find(|r| r.text.starts_with("ADDR <- PC"))
        .unwrap();
    assert!(addr.activity.signal(Sig::Addr1Pc) && addr.activity.signal(Sig::Addr2Off9));

    // memory reads are recorded
    assert!(emu
        .micro_log
        .iter()
        .any(|r| r.activity.signal(Sig::RwRead) && r.value == Some(0x3006)));

    emu.step_into(); // LDR R2, R1, #0
    let addr = emu
        .micro_log
        .iter()
        .find(|r| r.text.starts_with("ADDR <-"))
        .unwrap();
    assert!(addr.activity.signal(Sig::Addr1Base) && addr.activity.signal(Sig::Addr2Off6));
    assert_eq!(addr.value, Some(0x3006));

    emu.step_into(); // ADD R3, R1, #2
    let alu = emu
        .micro_log
        .iter()
        .find(|r| r.activity.signal(Sig::AlukAdd))
        .unwrap();
    assert!(alu.activity.signal(Sig::Sr2MuxImm));
    assert_eq!(emu.r[3].get(), 0x3008);

    emu.step_into(); // BRp SKIP (taken)
    assert_eq!(emu.pc.get(), 0x3005);
    assert!(emu
        .micro_log
        .iter()
        .any(|r| r.activity.signal(Sig::PcMuxAdder)));

    emu.step_into(); // JSRR R1: the target is read before R7 is written
    assert_eq!(emu.pc.get(), 0x3006);
    assert_eq!(emu.r[7].get(), 0x3006);
}

#[test]
fn untaken_branch_does_not_load_pc_from_adder() {
    use crate::emulator::trace::Sig;
    let mut emu = loaded(
        r#"
        .ORIG x3000
        AND R0, R0, #0
        BRp NOPE
        HALT
NOPE    HALT
        .END
    "#,
    );
    emu.step_into();
    emu.step_into();
    assert_eq!(emu.pc.get(), 0x3002);
    assert!(!emu
        .micro_log
        .iter()
        .any(|r| r.activity.signal(Sig::PcMuxAdder)));
}

#[test]
fn nothing_is_traced_unless_asked() {
    let mut emu = loaded(TRACE_PROGRAM);
    emu.trace = false;
    emu.step_into();
    assert!(emu.micro_log.is_empty());
    assert_eq!(emu.r[1].get(), 0x3006);
}

#[test]
fn jsrr_r7_jumps_to_old_r7() {
    let mut emu = loaded(
        r#"
        .ORIG x3000
        LEA R7, TARGET
        JSRR R7
        HALT
TARGET  ADD R0, R0, #1
        HALT
        .END
    "#,
    );
    emu.step_into();
    emu.step_into();
    assert_eq!(emu.pc.get(), 0x3003);
    assert_eq!(emu.r[7].get(), 0x3002);
}

const STATES_PROGRAM: &str = r#"
    .ORIG x3000
    ADD R0, R0, #1
    AND R1, R1, #0
    NOT R2, R1
    LEA R3, DATA
    LD R4, DATA
    LDR R5, R3, #0
    LDI R6, PTR
    ST R4, DATA
    STR R4, R3, #0
    STI R4, PTR
    BRz SKIP
    BRp NEXT
SKIP ADD R0, R0, #0
NEXT LEA R1, SUB
    JSRR R1
    JSR SUB
    TRAP x21
    HALT
SUB RET
DATA .FILL x3013
PTR .FILL x3013
    .END
"#;

/// Every instruction follows the states of Patt & Patel Figures C.2 and C.7 exactly.
#[test]
fn instructions_follow_the_state_machine() {
    let mut emu = loaded(STATES_PROGRAM);
    let fetch = [18u8, 33, 28, 30, 32];
    let expect: &[(&str, &[u8])] = &[
        ("ADD", &[1]),
        ("AND", &[5]),
        ("NOT", &[9]),
        ("LEA", &[14]),
        ("LD", &[2, 35, 25, 27]),
        ("LDR", &[6, 35, 25, 27]),
        ("LDI", &[10, 17, 24, 26, 35, 25, 27]),
        ("ST", &[3, 23, 16]),
        ("STR", &[7, 23, 16]),
        ("STI", &[11, 19, 29, 31, 23, 16]),
        ("BRz not taken", &[0]),
        ("BRp taken", &[0, 22]),
        ("LEA", &[14]),
        ("JSRR", &[4, 20]),
    ];
    for (name, states) in expect {
        emu.step_into();
        let want: Vec<u8> = fetch.iter().chain(states.iter()).copied().collect();
        assert_eq!(states_of(&emu), want, "{name}");
    }
    // RET from the subroutine
    emu.step_into();
    assert_eq!(states_of(&emu), [18, 33, 28, 30, 32, 12]);
    emu.step_into(); // JSR SUB
    assert_eq!(states_of(&emu), [18, 33, 28, 30, 32, 4, 21]);
    emu.step_into(); // RET
}

#[test]
fn trap_and_rti_follow_figure_c7() {
    let mut emu = loaded(STATES_PROGRAM);
    while emu.memory[emu.pc.get() as usize].get() != 0xF021 {
        emu.step_into();
    }
    let user_sp = emu.r[6].get();
    let return_to = emu.pc.get() + 1;
    // step into the OS: TRAP from User mode goes through state 45 to switch stacks
    emu.skip_os_emulation = false;
    emu.step_into();
    assert_eq!(
        states_of(&emu),
        [18, 33, 28, 30, 32, 15, 47, 45, 37, 41, 43, 46, 52, 54, 53, 55]
    );
    assert_eq!(emu.saved_usp.get(), user_sp);
    assert_eq!(
        emu.memory[emu.r[6].get() as usize].get(),
        return_to,
        "PC pushed"
    );
    // run to the service routine's RTI
    while emu.memory[emu.pc.get() as usize].get() >> 12 != 0x8 {
        emu.step_into();
    }
    emu.step_into();
    assert_eq!(
        states_of(&emu),
        [18, 33, 28, 30, 32, 8, 36, 38, 39, 40, 42, 34, 59]
    );
    assert_eq!(emu.pc.get(), return_to);
    assert_eq!(emu.r[6].get(), user_sp);
}

#[test]
fn exceptions_enter_through_the_state_machine() {
    // RTI in User mode: privilege mode exception, state 44
    let mut emu = loaded(
        r#"
        .ORIG x3000
        RTI
        .END
    "#,
    );
    emu.skip_os_emulation = false;
    emu.step_into();
    assert_eq!(
        states_of(&emu),
        [18, 33, 28, 30, 32, 8, 44, 45, 37, 41, 43, 46, 52, 54, 53, 55]
    );
    assert_eq!(
        emu.pc.get(),
        emu.memory[0x0100].get(),
        "privilege vector x00"
    );
    assert_eq!(
        emu.memory[emu.r[6].get() as usize].get(),
        0x3000,
        "PC - 1 pushed"
    );

    // LD from privileged memory: ACV found in state 35, exception state 57
    let mut emu = loaded(
        r#"
        .ORIG x3000
        LDI R0, OSPTR
        HALT
OSPTR   .FILL x0200
        .END
    "#,
    );
    emu.skip_os_emulation = false;
    emu.step_into();
    assert_eq!(
        states_of(&emu),
        [18, 33, 28, 30, 32, 10, 17, 24, 26, 35, 57, 45, 37, 41, 43, 46, 52, 54, 53, 55]
    );
    assert_eq!(emu.pc.get(), emu.memory[0x0102].get(), "ACV vector x02");

    // opcode 1101: illegal opcode exception, state 13
    let mut emu = loaded(
        r#"
        .ORIG x3000
        .FILL xD000
        .END
    "#,
    );
    emu.skip_os_emulation = false;
    emu.step_into();
    assert_eq!(
        states_of(&emu),
        [18, 33, 28, 30, 32, 13, 45, 37, 41, 43, 46, 52, 54, 53, 55]
    );
    assert_eq!(
        emu.pc.get(),
        emu.memory[0x0101].get(),
        "illegal opcode vector x01"
    );
}

/// The micro step command runs one state (clock cycle) at a time, and mixes with whole steps.
#[test]
fn micro_step_runs_one_state() {
    let mut emu = loaded(STATES_PROGRAM);
    let cycles = emu.cycles;
    // ADD R0, R0, #1: 18 33 28 30 32 1
    for want in [
        vec![18u8],
        vec![18, 33],
        vec![18, 33, 28],
        vec![18, 33, 28, 30],
        vec![18, 33, 28, 30, 32],
    ] {
        emu.micro_step_command();
        assert_eq!(states_of(&emu), want);
        assert!(!matches!(emu.cpu_state, crate::emulator::CpuState::Fetch) || want.len() < 5);
    }
    assert_eq!(emu.r[0].get(), 0, "not executed yet");
    emu.micro_step_command();
    assert_eq!(states_of(&emu), [18, 33, 28, 30, 32, 1]);
    assert_eq!(emu.r[0].get(), 1);
    assert_eq!(emu.cycles - cycles, 6, "one cycle per state");
    assert!(matches!(emu.cpu_state, crate::emulator::CpuState::Fetch));

    // AND: two micro steps, then finish it with step into
    emu.micro_step_command();
    emu.micro_step_command();
    assert_eq!(states_of(&emu), [18, 33]);
    emu.step_into();
    assert_eq!(states_of(&emu), [18, 33, 28, 30, 32, 5]);
    assert_eq!(emu.pc.get(), 0x3002);

    // skipped states are not stepped into: BRz not taken is 0, then done
    while emu.memory[emu.pc.get() as usize].get() >> 12 != 0 {
        emu.step_into();
    }
    for _ in 0..6 {
        emu.micro_step_command();
    }
    assert_eq!(states_of(&emu), [18, 33, 28, 30, 32, 0]);
    assert!(matches!(emu.cpu_state, crate::emulator::CpuState::Fetch));
}

/// Stepping back restores the whole machine a state at a time, back to the instruction's start.
#[test]
fn step_back_rewinds_the_machine() {
    let mut emu = loaded(STATES_PROGRAM);
    // ADD R0, R0, #1 one state at a time
    for _ in 0..6 {
        emu.micro_step_command();
    }
    assert_eq!(emu.r[0].get(), 1);
    assert!(emu.can_step_back());
    emu.step_back();
    assert_eq!(states_of(&emu), [18, 33, 28, 30, 32]);
    assert_eq!(emu.r[0].get(), 0, "state 1 undone");
    assert_eq!(emu.pc.get(), 0x3001);
    for _ in 0..5 {
        emu.step_back();
    }
    assert_eq!(emu.pc.get(), 0x3000, "back to the start of the instruction");
    assert!(
        !emu.can_step_back(),
        "not past the start of the instruction"
    );
    // and forwards again
    emu.step_into();
    assert_eq!(emu.r[0].get(), 1);

    // a store is undone too
    while emu.memory[emu.pc.get() as usize].get() >> 12 != 0x3 {
        emu.step_into();
    }
    let addr = 0x3013;
    emu.memory[addr].set(0x1234);
    emu.step_into(); // ST R4, DATA
    assert_ne!(emu.memory[addr].get(), 0x1234);
    emu.step_back(); // undo state 16, the write
    assert_eq!(emu.memory[addr].get(), 0x1234);
    assert_eq!(*states_of(&emu).last().unwrap(), 23);
}

#[test]
fn st_state_3_loads_mar() {
    let mut emu = loaded(STATES_PROGRAM);
    while emu.memory[emu.pc.get() as usize].get() >> 12 != 0x3 {
        emu.step_into();
    }
    let at = emu.pc.get();
    for _ in 0..6 {
        emu.micro_step_command();
    }
    assert_eq!(states_of(&emu), [18, 33, 28, 30, 32, 3]);
    let off = ((emu.ir.get() << 7) as i16 >> 7) as u16;
    assert_eq!(
        emu.mar.get(),
        at.wrapping_add(1).wrapping_add(off),
        "MAR <- PC + off9 in state 3"
    );
    let last = crate::emulator::trace::cycles(&emu.micro_log)
        .last()
        .copied()
        .unwrap();
    let texts: Vec<_> = emu.micro_log[last.start..last.end]
        .iter()
        .map(|r| r.text.clone())
        .collect();
    println!("{texts:?}");
}

const INPUT_PROGRAM: &str = r#"
        .ORIG x3000
        GETC
        OUT
        GETC
        OUT
        GETC
        OUT
        HALT
        .END
    "#;

#[test]
fn both_oses_read_the_keyboard() {
    use crate::emulator::OsKind;
    for os in OsKind::ALL {
        // keys typed before the program asks for them
        let mut emu = loaded_with(os, INPUT_PROGRAM);
        emu.output.clear();
        for c in "abc".chars() {
            emu.push_input(c);
        }
        emu.resume();
        run_until_stopped(&mut emu);
        assert!(emu.halted, "{os:?}");
        assert!(
            emu.output.starts_with("abc"),
            "{os:?}: output was {:?}",
            emu.output
        );

        // keys typed while the program waits in GETC
        let mut emu = loaded_with(os, INPUT_PROGRAM);
        emu.output.clear();
        emu.resume();
        for c in "xyz".chars() {
            for _ in 0..20 {
                emu.update();
            }
            assert!(
                emu.running() && !emu.halted,
                "{os:?}: stopped waiting for input"
            );
            emu.push_input(c);
        }
        run_until_stopped(&mut emu);
        assert!(emu.halted, "{os:?}");
        assert!(
            emu.output.starts_with("xyz"),
            "{os:?}: output was {:?}",
            emu.output
        );
    }
}

const BUSY_PROGRAM: &str = r#"
        .ORIG x3000
        AND R0, R0, #0
LOOP    ADD R0, R0, #1
        BRnzp LOOP
        .END
    "#;

#[test]
fn the_keyboard_interrupts_through_state_49() {
    use crate::emulator::{KBSR_ADDR, PSR_ADDR};
    let mut emu = loaded(BUSY_PROGRAM);
    emu.skip_os_emulation = false;
    assert_eq!(
        emu.memory[KBSR_ADDR].get() & 0x4000,
        0x4000,
        "the OS enables the keyboard interrupt"
    );
    for _ in 0..5 {
        emu.step_into();
    }
    let user_sp = emu.r[6].get();
    let user_psr = emu.memory[PSR_ADDR].get();
    emu.push_input('k');
    // the key arrives during this instruction; the next fetch sees INT
    emu.step_into();
    let (interrupted, count) = (emu.pc.get(), emu.r[0].get());
    emu.step_into();
    assert_eq!(
        states_of(&emu),
        [18, 49, 45, 37, 41, 43, 46, 52, 54, 53, 55]
    );
    assert_eq!(
        emu.pc.get(),
        emu.memory[0x0180].get(),
        "PC <- M[x0180], the keyboard's vector"
    );
    let psr = emu.memory[PSR_ADDR].get();
    assert_eq!(psr & 0x8000, 0, "Supervisor mode");
    assert_eq!(psr >> 8 & 7, 4, "priority 4");
    assert_eq!(emu.saved_usp.get(), user_sp);
    let sp = emu.r[6].get() as usize;
    assert_eq!(
        emu.memory[sp].get(),
        interrupted,
        "the interrupted instruction's address is pushed"
    );
    assert_eq!(
        emu.memory[sp + 1].get(),
        user_psr,
        "the program's PSR is pushed"
    );

    // the service routine takes the key and returns to the program, which carries on
    while emu.pc.get() < 0x3000 {
        emu.step_into();
    }
    assert_eq!(
        states_of(&emu),
        [18, 33, 28, 30, 32, 8, 36, 38, 39, 40, 42, 34, 59]
    );
    assert_eq!(emu.pc.get(), interrupted);
    assert_eq!(
        emu.r[0].get(),
        count,
        "the program's registers are untouched"
    );
    assert_eq!(emu.r[6].get(), user_sp);
    assert_eq!(emu.memory[PSR_ADDR].get(), user_psr);
    assert_eq!(emu.memory[KBSR_ADDR].get(), 0x4000, "the key was read");
    let os = crate::emulator::os_source_of(crate::emulator::OsKind::Interrupt);
    let buffer = (0..0x3000)
        .find(|a| {
            os.address_to_line
                .get(a)
                .is_some_and(|l| os.text.lines().nth(l - 1).unwrap().starts_with("KB_BUFFER"))
        })
        .unwrap();
    assert_eq!(
        emu.memory[buffer].get(),
        'k' as u16,
        "KBDINT buffered the key"
    );
}

#[test]
fn the_polling_os_never_takes_an_interrupt() {
    use crate::emulator::KBSR_ADDR;
    let mut emu = loaded_with(crate::emulator::OsKind::Polling, BUSY_PROGRAM);
    emu.skip_os_emulation = false;
    assert_eq!(emu.memory[KBSR_ADDR].get() & 0x4000, 0);
    emu.push_input('k');
    for _ in 0..20 {
        emu.step_into();
        assert!(emu.pc.get() >= 0x3000, "the program keeps running");
        assert!(!states_of(&emu).contains(&49));
    }
    assert_eq!(emu.memory[KBSR_ADDR].get(), 0x8000, "the key waits in KBDR");
}

#[test]
fn skipping_the_os_hides_interrupts_from_steps() {
    let mut emu = loaded(BUSY_PROGRAM);
    emu.step_into();
    emu.push_input('k');
    // the key arrives during this step and interrupts before the next; each step still runs
    // one instruction of the program
    let mut pcs = Vec::new();
    for _ in 0..4 {
        emu.step_into();
        pcs.push(emu.pc.get());
    }
    assert_eq!(pcs, [0x3002, 0x3001, 0x3002, 0x3001]);
    assert_eq!(
        emu.memory[crate::emulator::KBSR_ADDR].get(),
        0x4000,
        "the interrupt took the key"
    );
}

#[test]
fn step_out_is_not_ended_by_an_interrupt() {
    let mut emu = loaded(
        r#"
        .ORIG x3000
        JSR SUB
        HALT
SUB     AND R1, R1, #0
        LD R2, N
LOOP    ADD R1, R1, #1
        ADD R2, R2, #-1
        BRp LOOP
        RET
N       .FILL #40
        .END
    "#,
    );
    for os_skipped in [true, false] {
        emu.hard_reset();
        while emu.pc.get() != 0x3000 {
            emu.step_into();
        }
        emu.skip_os_emulation = os_skipped;
        emu.step_into(); // into SUB
        emu.push_input('k');
        emu.step_out();
        run_until_stopped(&mut emu);
        assert_eq!(
            emu.pc.get(),
            0x3001,
            "skip OS {os_skipped}: stopped after SUB returned"
        );
        assert_eq!(emu.r[1].get(), 40);
    }
}

#[test]
fn switching_os_relabels_memory() {
    use crate::emulator::OsKind;
    let mut emu = loaded_with(OsKind::Interrupt, BUSY_PROGRAM);
    let kbdint = emu.metadata.labels["KBDINT"];
    emu.os = OsKind::Polling;
    emu.hard_reset();
    assert!(
        !emu.metadata.labels.contains_key("KBDINT"),
        "the polling OS has no keyboard ISR"
    );
    assert_ne!(
        emu.metadata.addr_to_label.get(&kbdint).map(String::as_str),
        Some("KBDINT")
    );
    assert_eq!(
        emu.metadata.labels["LOOP"], 0x3001,
        "the program's labels stay"
    );
    let getc = emu.memory[0x20].get() as usize;
    assert_eq!(emu.metadata.addr_to_label[&getc], "TRAP_GETC");
}

#[test]
fn getc_sleeps_until_the_keyboard_interrupts() {
    use crate::emulator::{OsKind, MCR_ADDR};
    let mut emu = loaded_with(OsKind::Interrupt, INPUT_PROGRAM);
    emu.output.clear();
    emu.resume();
    for _ in 0..50 {
        emu.update();
    }
    assert!(emu.asleep, "GETC put the machine to sleep");
    assert!(emu.running() && emu.waiting_for_input());
    assert_eq!(
        emu.memory[MCR_ADDR].get(),
        0x8000,
        "MCR[14] doesn't stay set"
    );
    let cycles = emu.cycles;
    for _ in 0..50 {
        emu.update();
    }
    assert_eq!(emu.cycles, cycles, "no clock cycles while asleep");

    emu.push_input('a');
    for _ in 0..50 {
        emu.update();
    }
    assert_eq!(emu.output, "a");
    assert!(emu.asleep, "asleep again in the next GETC");

    // the polling OS spins instead
    let mut emu = loaded_with(OsKind::Polling, INPUT_PROGRAM);
    emu.resume();
    emu.update();
    let cycles = emu.cycles;
    for _ in 0..20 {
        emu.update();
    }
    assert!(!emu.asleep);
    assert!(emu.cycles > cycles, "polling runs the clock");
}

#[test]
fn stepping_a_sleeping_machine_waits() {
    use crate::emulator::OsKind;
    let mut emu = loaded_with(OsKind::Interrupt, INPUT_PROGRAM);
    emu.skip_os_emulation = false;
    for _ in 0..200 {
        if emu.asleep {
            break;
        }
        emu.step_into();
    }
    assert!(emu.asleep);
    let pc = emu.pc.get();
    emu.step_into();
    emu.micro_step_command();
    assert_eq!(emu.pc.get(), pc, "nothing runs while asleep");
    emu.push_input('z');
    // waking up, GETC unmasks the keyboard (STI R3, OS_PSR), and the interrupt is taken
    emu.step_into();
    assert!(!emu.asleep);
    assert_eq!(
        emu.memory[crate::emulator::PSR_ADDR].get() >> 8 & 7,
        0,
        "unmasked"
    );
    emu.step_into();
    assert_eq!(emu.pc.get(), emu.memory[0x0180].get(), "KBDINT runs");
}

#[test]
fn programs_cannot_run_past_xffff() {
    assert!(
        Emulator::parse_program(".ORIG xFFFF\n.FILL #1\n.END", None).is_ok(),
        "the last word is fine"
    );
    assert!(Emulator::parse_program(".ORIG xFFFF\n.FILL #1\n.FILL #2\n.END", None).is_err());
    assert!(Emulator::parse_program(".ORIG xFFF0\n.BLKW #17\n.END", None).is_err());
    assert!(Emulator::parse_program(".ORIG xFFFE\n.STRINGZ \"ab\"\n.END", None).is_err());
}

#[test]
fn a_breakpoint_does_not_refire_while_asleep() {
    use crate::emulator::{OsKind, StopReason};
    let mut emu = loaded_with(OsKind::Interrupt, INPUT_PROGRAM);
    emu.resume();
    for _ in 0..50 {
        emu.update();
    }
    assert!(emu.asleep);
    // a breakpoint where GETC wakes up
    let wake = emu.pc.get() as usize;
    emu.breakpoints.insert(wake);
    emu.halted_on_breakpoint = true; // as if Run was pressed on it
    for _ in 0..20 {
        emu.update();
    }
    assert!(
        emu.running(),
        "still asleep, not stopped at {:?}",
        emu.stop_reason
    );
    assert!(!matches!(emu.stop_reason, StopReason::Breakpoint(_)));
}

#[test]
fn stepping_back_keeps_keys_typed_since() {
    let mut emu = loaded(BUSY_PROGRAM);
    emu.step_into();
    emu.micro_step_command();
    emu.micro_step_command();
    emu.push_input('q');
    emu.step_back();
    // still waiting in the queue, or already delivered to KBDR by the replay
    let kbdr = emu.memory[crate::emulator::KBDR_ADDR].get();
    assert!(
        emu.input_queue.contains(&'q') || kbdr == 'q' as u16,
        "the key was lost"
    );
}

#[test]
fn odd_psr_values_do_not_panic() {
    let mut emu = loaded(BUSY_PROGRAM);
    emu.memory[crate::emulator::PSR_ADDR].set(0x8000); // no condition code
    assert_eq!(emu.get_nzp(), (false, false, false));
    emu.memory[crate::emulator::PSR_ADDR].set(0x8007);
    emu.step_into();
}

#[test]
fn restarting_reboots_the_os_but_keeps_user_memory() {
    use crate::emulator::KBSR_ADDR;
    let mut emu = loaded(BUSY_PROGRAM);
    emu.memory[0x4000].set(1234);
    emu.memory[KBSR_ADDR].set(0xC000);
    emu.push_input('z');
    let fresh = emu.soft_reset();
    assert_eq!(fresh.memory[0x4000].get(), 1234);
    assert_eq!(fresh.memory[KBSR_ADDR].get(), 0, "devices start fresh");
    assert_eq!(fresh.input_queue.len(), 1, "typed keys are kept");
    assert_eq!(fresh.pc.get(), 0x200);
}

#[test]
fn the_trace_records_what_each_wire_carries() {
    use crate::emulator::trace::{cycles, merge, Dp};
    let mut emu = loaded(
        r#"
        .ORIG x3000
        LD R1, N
        ADD R2, R1, #-3
        LDR R3, R1, #1
        HALT
N       .FILL #7
        .END
    "#,
    );
    let state = |emu: &Emulator, n: u8| {
        let c = cycles(&emu.micro_log)
            .into_iter()
            .find(|c| c.state == n)
            .expect("state ran");
        merge(&emu.micro_log[c.start..c.end]).unwrap()
    };
    emu.step_into();
    // state 2: MAR <- PC + SEXT(PCoffset9); the fetch's PC + 1 is x3001
    let s2 = state(&emu, 2);
    assert_eq!(s2.wire(Dp::Sext9), Some(3));
    assert_eq!(s2.wire(Dp::PcToAddr1), Some(0x3001));
    assert_eq!(s2.wire(Dp::AdderToMarMux), Some(0x3004));
    assert_eq!(state(&emu, 25).wire(Dp::MemToMdr), Some(7));
    emu.step_into();
    let s1 = state(&emu, 1);
    assert_eq!(s1.wire(Dp::Sr1Out), Some(7), "SR1 = R1");
    assert_eq!(s1.wire(Dp::Sext5), Some((-3i16) as u16));
    assert_eq!(s1.wire(Dp::GateAlu), Some(4), "the ALU's result on the bus");
    assert_eq!(s1.wire(Dp::NzpMux), Some(0b001), "P");
    emu.step_into();
    let s6 = state(&emu, 6);
    assert_eq!(s6.wire(Dp::RegToAddr1), Some(7), "BaseR = R1");
    assert_eq!(s6.wire(Dp::Sext6), Some(1));
}

/// The datapath's "before" values for an instruction's first state come from the machine as
/// the instruction began (state 18 loads PC, so its old value is only there).
#[test]
fn first_state_knows_the_machine_before_it() {
    let mut emu = loaded(STATES_PROGRAM);
    emu.trace = true;
    let pc = emu.pc.get();
    emu.micro_step_command();
    assert_eq!(states_of(&emu), [18]);
    assert_eq!(emu.micro_log_before.pc, pc);
    assert_eq!(emu.micro_log.last().unwrap().snap.pc, pc + 1);
}

/// Everything the fast path must leave as the state machine does.
fn machine(e: &Emulator) -> String {
    let mem: u64 = e.memory.iter().enumerate().fold(0u64, |h, (i, c)| {
        h.wrapping_mul(31).wrapping_add((c.get() as u64) ^ i as u64)
    });
    format!(
        "pc {:04X} r {:04X?} ir {:04X} mar {:04X} mdr {:04X} psr {:04X} ssp {:04X} usp {:04X} table {:02X} vector {:02X} cycles {} instructions {} halted {} asleep {} output {:?} mem {mem:X}",
        e.pc.get(),
        e.r.iter().map(|r| r.get()).collect::<Vec<_>>(),
        e.ir.get(),
        e.mar.get(),
        e.mdr.get(),
        e.memory[crate::emulator::PSR_ADDR].get(),
        e.saved_ssp.get(),
        e.saved_usp.get(),
        e.table.get() & 0xFF,
        e.vector.get() & 0xFF,
        e.cycles,
        e.instructions,
        e.halted,
        e.asleep,
        e.output,
    )
}

/// The fast path and the state machine agree after every instruction, from boot to HALT: every
/// opcode, traps from User mode, RTI, keyboard interrupts and both OSes.
#[test]
fn fast_path_matches_the_state_machine() {
    let program = r#"
        .ORIG x3000
        LEA R0, MSG
        PUTS
        ADD R1, R1, #5
        ADD R2, R1, R1
        AND R3, R2, #6
        AND R4, R2, R1
        NOT R5, R1
        LD R6, DATA
        LDI R1, PTR
        LDR R2, R0, #1
        ST R1, SPOT
        STI R2, PTR2
        STR R3, R0, #2
        LEA R7, SUB
        JSRR R7
        JSR SUB
        AND R1, R1, #0
        ADD R1, R1, #3
LOOP    ADD R1, R1, #-1
        BRp LOOP
        BRz NEXT
        HALT
NEXT    GETC
        OUT
        IN
        ADD R2, R0, #0
        GETC
        OUT
        HALT
SUB     ADD R4, R4, #1
        RET
DATA    .FILL x1234
PTR     .FILL DATA
PTR2    .FILL SPOT
SPOT    .BLKW 1
MSG     .STRINGZ "hi"
        .END
    "#;
    for os in [
        crate::emulator::OsKind::Interrupt,
        crate::emulator::OsKind::Polling,
    ] {
        let make = || {
            let mut emu = Emulator::with_os(os);
            let out = Emulator::parse_program(program, Some(&mut emu.metadata))
                .expect("program assembles");
            emu.load_program(crate::emulator::LoadedProgram {
                machine_code: out.machine_code,
                orig_address: out.orig_address,
                ..Default::default()
            });
            for c in "xyzabc".chars() {
                emu.push_input(c);
            }
            emu.start_running();
            emu
        };
        let (mut slow, mut fast) = (make(), make());
        let mut n = 0;
        while !slow.halted {
            assert!(
                !slow.asleep || slow.interrupt_requested(),
                "{os:?}: asleep with nothing to wake it"
            );
            slow.step();
            fast.run_fast(1, None);
            assert_eq!(
                machine(&fast),
                machine(&slow),
                "{os:?}: after instruction {n} at x{:04X}",
                slow.currently_executing
            );
            n += 1;
            assert!(n < 20_000, "{os:?}: never halted");
        }
        assert!(fast.halted);
        assert!(slow.output.contains("hi"), "{:?}", slow.output);
    }
}

/// Exceptions go through the state machine from the fast path too.
#[test]
fn fast_path_hands_exceptions_to_the_state_machine() {
    for program in [
        ".ORIG x3000\n.FILL xD000\n.END",
        ".ORIG x3000\nRTI\n.END",
        ".ORIG x3000\nLDI R0, P\nP .FILL x0200\n.END",
    ] {
        let mut slow = loaded(program);
        let mut fast = loaded(program);
        slow.start_running();
        fast.start_running();
        slow.step();
        fast.run_fast(1, None);
        assert_eq!(machine(&fast), machine(&slow), "{program}");
        assert!(
            fast.pc.get() < 0x3000,
            "{program}: in the exception handler"
        );
    }
}

/// Run at Max speed: a long loop finishes in a few updates.
#[test]
fn max_speed_runs_fast() {
    let mut emu = loaded(
        r#"
        .ORIG x3000
        LD R1, COUNT
LOOP    ADD R1, R1, #-1
        BRp LOOP
        HALT
COUNT   .FILL #30000
        .END
    "#,
    );
    emu.max_speed = true;
    emu.resume();
    for _ in 0..50 {
        if emu.halted {
            break;
        }
        emu.update();
    }
    assert!(emu.halted, "60k instructions in 50 updates");
    assert_eq!(emu.r[1].get(), 0);
}

/// How fast each path runs (cargo test --release speed_report -- --ignored --nocapture).
#[test]
#[ignore]
fn speed_report() {
    let program = r#"
        .ORIG x3000
        LEA R4, COUNT
        LD R1, COUNT
OUTER   LD R2, COUNT
INNER   LDR R3, R4, #0
        ADD R2, R2, #-1
        BRp INNER
        ADD R1, R1, #-1
        BRp OUTER
        HALT
COUNT   .FILL #2000
        .END
    "#;
    for fast in [false, true] {
        let mut emu = loaded(program);
        emu.trace = false;
        emu.start_running();
        let t = std::time::Instant::now();
        let i0 = emu.instructions;
        let c0 = emu.cycles;
        while !emu.halted {
            if fast {
                emu.run_fast(u64::MAX, None);
            } else {
                emu.step();
            }
        }
        let s = t.elapsed().as_secs_f64();
        println!(
            "{}: {} instructions in {s:.3}s = {:.1}M instructions/s, {:.1}M cycles/s",
            if fast { "fast path" } else { "state machine" },
            emu.instructions - i0,
            (emu.instructions - i0) as f64 / s / 1e6,
            (emu.cycles - c0) as f64 / s / 1e6
        );
    }
}
