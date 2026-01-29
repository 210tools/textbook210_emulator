#![allow(clippy::unusual_byte_groupings)] // so we can group bits by instruction parts
#![allow(clippy::reversed_empty_ranges)] // We want to use ranges for bis like we have in class (big:small)

/// Run the low level ops
pub mod executor;
/// Manage the low level ops that each instruction is broken down into
#[macro_use]
pub mod micro_op;
/// Spec for each op so they can be executed
pub mod ops;
/// Convert a seris of lines of lc3 code into emulator cells reporting errors
pub mod parse;
#[cfg(test)]
/// Tests for emulation layer
mod tests;

use std::{collections::HashSet, ops::Range};

pub use ops::{CpuState, OpCode};
use parse::ParseOutput;

use crate::emulator::{
    executor::CpuPhaseState,
    micro_op::{CycleState, MicroOpGenerator},
    parse::CompilationArtifacts,
};

/// The amount of steps to skip when os skips are enabled and we are in OS memory space
pub const MAX_OS_STEPS: usize = 1000;

// Device registers at memory addresses xFE00-xFFFF
/// Keyboard status ADDR, KBSR[15] = 1 when we have input to be read
pub const KBSR_ADDR: usize = 0xFE00;
/// Keyboard destination ADDR, KBDR[7:0] = last typed in ascii
pub const KBDR_ADDR: usize = 0xFE02;
/// Display Status ADDR, DSR[15] = 1 when display service ready to display a new char (always 1 in this emulator)
pub const DSR_ADDR: usize = 0xFE04;
/// Display destination register, when DDR[7:0] set we write the ascii char contained to the output
pub const DDR_ADDR: usize = 0xFE06;
/// Program Status register, Contains Privlage mode and condition codes, PSR[15] = 0 when in superviser mode and 1 when user mode,
///  PSR[2] = N, PSR[1] = Z, PSR[0] = P
pub const PSR_ADDR: usize = 0xFFFC;
/// Machine control register, when MCR[15] = 1 the program is running. To halt it is cleared.
pub const MCR_ADDR: usize = 0xFFFE;

pub struct Emulator {
    // --- not involved in the state machine ---
    /// How many cycles to run per update call
    pub speed: u32,
    /// How many update calls to wait before doing cycles equal to speed
    pub ticks_between_updates: u32,
    /// What is the current tick, only really used for modulo so we could wrap it around
    pub tick: u64,
    /// Do we jump over os instructions accouding to the [`MAX_OS_STEPS`] var
    pub skip_os_emulation: bool,
    /// The summation of all MEM[DDR] sets aka the 'output' of the emulator
    pub output: String,
    /// Some associated data for the most recent set of compiled programs
    pub metadata: CompilationArtifacts,
    pub breakpoints: HashSet<usize>,
    pub currently_executing: usize,
    /// Has the machine been halted by the OS/program
    pub halted: bool,
    // -----------------------------------------

    // Why in a Box? Becuase array sits on stack and takes alot of memory.
    // wasm was unhappy so I put it on the heap using Box
    // TODO: How much faster is it on the stack? mabye it should be a compile time distinction
    /// Holds all the instructions and data that the state machine munches on
    pub memory: Box<[EmulatorCell; 65536]>,
    /// The alu component (this manages ADD, NOT and AND operations)
    pub alu: Alu,

    /// Registers  R0-R7
    pub r: [EmulatorCell; 8],

    /// Program Counter. This stores the Adress that we will use to populate the IR on the next fetch cycle.
    /// IR <- MEM[PC]
    pub pc: EmulatorCell,

    /// The adress of the memory location to read from the memory on next reading phase
    pub mar: EmulatorCell,
    /// Stores the input or output of the memory, we set it then do a write phase to set MEM[mar] <- mdr.
    /// It also gets set on a read phase MEM[mar] -> mdr
    pub mdr: EmulatorCell,

    /// Stores the instruction we are in the process of ececuting the first 4 bits are the op etc.
    /// Set in the fetch phase (IR <- MEM[pc])
    pub ir: EmulatorCell,

    /// CPU state. fetch -> decode -> evaluate address etc
    /// Each instruction relys on its own struct of the instruction after the decode stage
    pub cpu_state: CpuState,

    /// In a phase this does the heavy lifting in terms of running the micro ops.
    pub execute_state: CpuPhaseState,

    // Saved stack pointers. These are used when going between os service routines and user code.
    // For example when trap is executed from user code saved_usp <- R6 && R6 <- saved_ssp
    // and when RTI is executed saved_ssp <- R6 && R6 <- saved_usp
    /// The last known R6 used in an OS service
    pub saved_ssp: EmulatorCell,
    /// The last known R6 used in user code
    pub saved_usp: EmulatorCell,

    /// If our stste machine has reached an exeption state than this stores the particulars
    pub exception: Option<Exception>,
}

impl Emulator {
    pub fn new() -> Emulator {
        let mut emulator = Self {
            halted: false,
            speed: 1,
            ticks_between_updates: 2,
            tick: 0,
            skip_os_emulation: true,
            currently_executing: 0,
            metadata: CompilationArtifacts::default(),
            breakpoints: HashSet::new(),
            memory: Box::new([EmulatorCell::new(0); 65536]),
            r: [EmulatorCell::new(0); 8],
            pc: EmulatorCell::new(0x200), // start of os
            mar: EmulatorCell::new(0),
            mdr: EmulatorCell::new(0),
            ir: EmulatorCell::new(0),
            output: String::new(),
            cpu_state: CpuState::Fetch,
            execute_state: CpuPhaseState::new(Vec::new()), // this is empty before we execute the first op
            alu: Alu::default(),
            exception: None,
            saved_ssp: EmulatorCell::new(0),
            saved_usp: EmulatorCell::new(0),
        };

        let parse_output = Emulator::parse_program(
            include_str!("../oses/simpleos.asm"),
            Some(&mut emulator.metadata),
        );

        tracing::debug!("OS parse_output: {:?}", parse_output);

        if let Ok(ParseOutput {
            machine_code,
            orig_address,
            ..
        }) = parse_output
        {
            emulator.flash_memory(machine_code, orig_address);
        } else {
            debug_assert!(false, "INVALID DEFAULT OS!!!");
        }

        emulator.memory[DSR_ADDR].set(0x8000); // ready for a char
        emulator.memory[PSR_ADDR].set(0x0002); // Z=1, N=0, P=0 Supervisor
        emulator.memory[MCR_ADDR].set(0x0000); // machine is "stopped"

        emulator
    }

    /// Are we running in superviser or user mode?
    pub fn priv_level(&self) -> PrivilegeLevel {
        match self.memory[PSR_ADDR].index(15).get() {
            0 => PrivilegeLevel::Supervisor,
            1 => PrivilegeLevel::User,
            _ => unreachable!(),
        }
    }

    /// Reset all registers and set PC to x200
    pub fn soft_reset(&self) -> Self {
        Self {
            output: self.output.clone(),
            pc: EmulatorCell::new(0x200),
            speed: self.speed,
            skip_os_emulation: self.skip_os_emulation,
            memory: self.memory.clone(),
            metadata: self.metadata.clone(),
            ..Default::default()
        }
    }

    /// Change the privlage mode.
    pub fn set_priv_level(&mut self, level: PrivilegeLevel) {
        let psr_val = self.memory[PSR_ADDR].get();
        match level {
            PrivilegeLevel::User => self.memory[PSR_ADDR].set(psr_val | 0x8000),
            PrivilegeLevel::Supervisor => self.memory[PSR_ADDR].set(psr_val & !0x8000),
        }
    }

    pub fn running(&self) -> bool {
        self.memory[MCR_ADDR].get() == 0x8000
    }

    /// Change the running state of the emulator to true.
    pub fn start_running(&mut self) {
        self.memory[MCR_ADDR].set(0x8000);
    }

    /// Change the running state of the emulator to false
    pub fn stop_running(&mut self) {
        self.memory[MCR_ADDR].set(0);
    }

    /// Set the negitive condition bit (reseting the others)
    pub fn set_n(&mut self) {
        let psr = self.memory[PSR_ADDR].get();
        let new_psr = (psr & 0xFFF8) | 0x0004;
        self.memory[PSR_ADDR].set(new_psr);
    }

    /// Set the zero condition bit (reseting the others)
    pub fn set_z(&mut self) {
        let psr = self.memory[PSR_ADDR].get();
        let new_psr = (psr & 0xFFF8) | 0x0002;
        self.memory[PSR_ADDR].set(new_psr);
    }

    /// Set the positive condition bit (reseting the others)
    pub fn set_p(&mut self) {
        let psr = self.memory[PSR_ADDR].get();
        let new_psr = (psr & 0xFFF8) | 0x0001;
        self.memory[PSR_ADDR].set(new_psr);
    }

    /// Get (n,z,p) as bools. Only one must be true at all times
    pub fn get_nzp(&self) -> (bool, bool, bool) {
        let psr = self.memory[PSR_ADDR].get();
        let n = (psr & 0x0004) != 0;
        let z = (psr & 0x0002) != 0;
        let p = (psr & 0x0001) != 0;
        debug_assert!(
            (n as u8 + z as u8 + p as u8) == 1,
            "Exactly one of N, Z, P must be true"
        );
        (n, z, p)
    }

    /// Input one char so that the os can read it.
    // TODO: change this if/when we do interuption based input
    pub fn set_in_char(&mut self, c: char) {
        if c.is_ascii() {
            self.memory[KBDR_ADDR].set(c as u16);
            self.memory[KBSR_ADDR].set(0x8000); // indicates new char avalible
        }
    }

    /// Given a register we update flages based on value
    pub fn update_flags(&mut self, reg_index: usize) {
        let value = self.r[reg_index].get();

        // Check if the value is negative (bit 15 is 1)
        let is_negative = (value >> 15) & 1 == 1;

        // Set negative flag
        if is_negative {
            self.set_n();
        } else if value == 0 {
            self.set_z();
        } else {
            self.set_p();
        }
    }
}

#[derive(Debug, Default, Clone, Copy)]
/// The core data structure for the emulator. Every value is stored via this using .get() and .set(). Stores a LC3 word (16 bits) and wether that word has changed
pub struct EmulatorCell(u16, bool);

impl EmulatorCell {
    #[inline(always)]
    pub fn new(value: u16) -> Self {
        Self(value, true)
    }

    /// get the word
    #[inline(always)]
    pub fn get(&self) -> u16 {
        self.0
    }

    /// Set the word
    #[inline(always)]
    pub fn set(&mut self, value: u16) {
        if value != self.0 {
            self.0 = value;
            self.1 = true;
        }
    }

    /// Check if value has changed, reseting changed.
    #[inline(always)]
    pub fn changed(&mut self) -> bool {
        let changed = self.1;
        self.1 = false;
        changed
    }

    /// Check if value has changed without reseting changed.
    #[inline(always)]
    pub fn changed_peek(&self) -> bool {
        self.1
    }

    /// Sign extend from bit position to 16 bits
    /// bits to the left of pos must be 0
    #[inline(always)]
    pub fn sext(&self, bit_pos: u8) -> Self {
        let value = self.0;
        let is_negative = (value >> bit_pos) & 1 == 1;

        if is_negative {
            // Set all bits above bit_pos to 1
            // mask example: using 8 bits for simplicity:
            // number = 00001010
            // bit_pos = 3  ^
            // bit_pos+1 =4^
            // 1<<4 = 00010000
            // -1 = 00001111
            // ! = 11110000
            let mask = !((1 << (bit_pos + 1)) - 1);
            Self(value | mask, true)
        } else {
            *self
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
/// User: Can run code, read and write as long as it is not:
///  - RTI
///  - reading or writing outside of 0x3000..=0xFDFF
///    (excluding alowing reads in x0000 - x00FF Trap vector table)
///  
///
/// Supervisor: Can read or write anywhere
pub enum PrivilegeLevel {
    User,
    Supervisor,
}

pub fn area_from_address(addr: &EmulatorCell) -> MemoryArea {
    match addr.get() {
        0x0000..=0x00FF => MemoryArea::TrapVectorTable,
        0x0100..=0x01FF => MemoryArea::IntrruptVectorTable,
        0x0200..=0x2FFF => MemoryArea::OperatingSystem,
        0x3000..=0xFDFF => MemoryArea::UserSpace,
        0xFE00..=0xFFFF => MemoryArea::DeviceRegisters,
    }
}

pub enum MemoryArea {
    TrapVectorTable,     // x0000 - x00FF (Read only for userspace)
    IntrruptVectorTable, // x0100 - x01FF (No permissions for userspace)
    OperatingSystem,     // x0200 - x2FFF (No permissions for userspace)
    UserSpace,           // x3000 - xFDFF (rwx for userspace)
    DeviceRegisters,     // xFE00 - xFFFF (No permissions for userspace)
}

impl MemoryArea {
    pub fn can_read(&self, level: &PrivilegeLevel) -> bool {
        match level {
            PrivilegeLevel::User => match self {
                MemoryArea::TrapVectorTable => true,
                MemoryArea::IntrruptVectorTable => false,
                MemoryArea::OperatingSystem => false,
                MemoryArea::UserSpace => true,
                MemoryArea::DeviceRegisters => false,
            },
            PrivilegeLevel::Supervisor => true, // Supervisor can read anything
        }
    }

    pub fn can_write(&self, level: &PrivilegeLevel) -> bool {
        match level {
            PrivilegeLevel::User => match self {
                MemoryArea::TrapVectorTable => false,
                MemoryArea::IntrruptVectorTable => false,
                MemoryArea::OperatingSystem => false,
                MemoryArea::UserSpace => true,
                MemoryArea::DeviceRegisters => false,
            },
            // Supervisor generally can write, but maybe some areas mabye read-only even for supervisor (trap table for safety?)
            // Let's assume supervisor can write everywhere for now, adjust if needed.
            PrivilegeLevel::Supervisor => true,
        }
    }
}

#[derive(Debug, Clone)]
pub enum Exception {
    PrivilegeViolation,
    IllegalInstruction,
    AccessControlViolation,
}

impl Exception {
    fn get_handler_address(&self) -> usize {
        // Base address of the Interrupt Vector Table
        const IVT_BASE: usize = 0x0100;
        // TODO: Should we make more?
        match self {
            Exception::PrivilegeViolation => IVT_BASE, // Vector x00 in IVT for Privilege Violation
            Exception::IllegalInstruction => IVT_BASE + 0x01, // Vector x01 in IVT for Illegal Opcode
            Exception::AccessControlViolation => IVT_BASE + 0x02, // Using x02 for Access Control
        }
    }
}

#[derive(Debug, Clone, Default)]
/// This component of the state machine takes some operation then a number of cells and outputs the result of a arthmatic op
pub struct Alu {
    pub op: Option<AluOp>,
    pub alu_out: EmulatorCell,
}

#[derive(Debug, Clone)]
pub enum AluOp {
    Add(EmulatorCell, EmulatorCell),
    And(EmulatorCell, EmulatorCell),
    Not(EmulatorCell),
}

impl AluOp {
    pub fn execute(&self) -> EmulatorCell {
        EmulatorCell::new(match self {
            AluOp::Add(a, b) => a.get().wrapping_add(b.get()),
            AluOp::And(a, b) => a.get() & b.get(),
            AluOp::Not(a) => !a.get(),
        })
    }
}

impl Default for Emulator {
    fn default() -> Self {
        Self::new()
    }
}

/// emulator logic core
impl Emulator {
    /// this will be called every clock cycle of the state machine (bool if emulator state changed)
    pub fn update(&mut self) -> bool {
        let mut os_steps = 0;
        let mut changed = false;

        self.tick = self.tick.wrapping_add(1);

        if self.running() {
            // Automatic stepping logic when running
            if self.tick % self.ticks_between_updates as u64 == 0 {
                let mut i = 0;
                while self.running() && i < self.speed {
                    // Check for breakpoints
                    let current_pc = self.pc.get() as usize;

                    if self.breakpoints.contains(&current_pc)
                        && matches!(self.cpu_state, CpuState::Fetch)
                    // Break *before* fetching the instruction at the breakpoint
                    {
                        self.stop_running();
                        log::info!("Breakpoint hit at address 0x{current_pc:04X}");
                        break;
                    }

                    changed = true;
                    let _ = self.micro_step();
                    i += 1;

                    if i >= self.speed {
                        break;
                    }
                }
            }

            // Skip OS code if enabled during running mode
            // Limit OS skipping to avoid freezing
            if self.skip_os_emulation {
                while self.pc.get() < 0x3000 && os_steps < MAX_OS_STEPS && self.running() {
                    changed = true;
                    self.step();
                    os_steps += 1;
                }
            }
        }
        changed
    }

    // --- Core Instruction Cycle Phases ---

    /// **Fetch Phase:** Read instruction from memory at PC into IR, increment PC.
    fn fetch(&mut self) -> Result<(), String> {
        let pc_value = self.pc.get();
        let memory_area = area_from_address(&self.pc);

        self.currently_executing = pc_value as usize;

        // Check read permission for PC address
        if !memory_area.can_read(&self.priv_level()) {
            self.exception = Some(Exception::AccessControlViolation);
            return Err(format!(
                "Fetch Access Violation: Cannot read PC address 0x{pc_value:04X}"
            ));
        }

        // Get the micro-op generator for the instruction
        let opcode = OpCode::from_instruction(self.memory[pc_value as usize])
            .expect("Failed to decode instruction");
        let micro_op_gen: &dyn MicroOpGenerator = match &opcode {
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
        };

        // Get the plan for the specific instruction phases
        let mut op_plan_map = micro_op_gen.generate_plan();

        // Create the full 6-phase execution plan
        let full_plan = vec![
            // Phase 0: Fetch
            vec![
                micro_op!(-> Fetch),
                micro_op!(MAR <- PC),
                micro_op!(ALU_OUT <- PC + C(1)),
                micro_op!(PC <- AluOut),
                micro_op!(-> Fetch),
                micro_op!(IR <- MDR),
            ],
            // Phase 1: Decode
            vec![
                micro_op!(-> Decode),
                micro_op!(MSG format!("Instruction decoded: {}", opcode)),
            ],
            // Phase 2: EvaluateAddress
            {
                let mut v = vec![micro_op!(-> EvaluateAddress)];
                v.extend(
                    op_plan_map
                        .remove(&CycleState::EvaluateAddress)
                        .unwrap_or_default(),
                );
                v
            },
            // Phase 3: FetchOperands
            {
                let mut v = vec![micro_op!(-> FetchOperands)];
                v.extend(
                    op_plan_map
                        .remove(&CycleState::FetchOperands)
                        .unwrap_or_default(),
                );
                v
            },
            // Phase 4: Execute
            {
                let mut v = vec![micro_op!(-> Execute)];
                v.extend(op_plan_map.remove(&CycleState::Execute).unwrap_or_default());
                v
            },
            // Phase 5: StoreResult
            {
                let mut v = vec![micro_op!(-> StoreResult)];
                v.extend(
                    op_plan_map
                        .remove(&CycleState::StoreResult)
                        .unwrap_or_default(),
                );
                v
            },
        ];
        self.execute_state = CpuPhaseState::new(full_plan);

        Ok(())
    }

    /// **Decode Phase:** Decode instruction in IR, determine OpCode.
    fn decode(&mut self) -> Result<OpCode, String> {
        match OpCode::from_instruction(self.ir) {
            Some(op) => Ok(op),
            None => {
                self.exception = Some(Exception::IllegalInstruction);
                Err(format!(
                    "Decode Error: Illegal opcode in IR=0x{:04X}",
                    self.ir.get()
                ))
            }
        }
    }

    /// **Handle Exception:** Switch to supervisor mode, save state, jump to handler.
    fn handle_exception(&mut self, exception: Exception) {
        tracing::warn!("Handling Exception: {:?}", exception);

        // 1. Get handler address
        let handler_addr = exception.get_handler_address();
        let handler_addr = self.memory[handler_addr];

        let psr_val = self.memory[PSR_ADDR].get(); // save the curr psr before changing priv

        // 2. Switch to Supervisor Mode & Stack
        if matches!(self.priv_level(), PrivilegeLevel::User) {
            self.saved_usp = self.r[6]; // Save User SP
            self.r[6] = self.saved_ssp; // Load Supervisor SP
        }
        self.set_priv_level(PrivilegeLevel::Supervisor);

        // 3. Push PSR and PC onto the Supervisor Stack (R6)
        let ssp = self.r[6].get();
        let psr_addr = ssp.wrapping_sub(1);
        let pc_addr = ssp.wrapping_sub(2);

        // Check stack write permissions (should be writable in Supervisor mode)
        // Basic check: Ensure stack pointer is within valid memory range
        if pc_addr > 1 && pc_addr < (self.memory.len() - 1) as u16 {
            self.memory[psr_addr as usize].set(psr_val);
            self.memory[pc_addr as usize].set(self.pc.get());
            self.r[6].set(pc_addr); // Update SSP
        } else {
            // Stack Overflow/Underflow - This is a critical error, potentially halt or double fault
            tracing::error!(
                "CRITICAL: Stack pointer R6=0x{:04X} out of bounds during exception handling.",
                ssp
            );
            self.stop_running(); // Halt on severe stack error
            return;
        }

        // 4. Set PC to handler address
        self.pc = handler_addr;

        // 5. Clear the exception state
        self.exception = None;

        // 6. Reset CPU state to Fetch for the handler routine
        self.cpu_state = CpuState::Fetch;
    }

    /// **Micro Step:** Execute one phase of the instruction cycle.
    pub fn micro_step(&mut self) -> Result<(), String> {
        tracing::trace!(cpu_state = ?self.cpu_state, "Entering micro_step");

        debug_assert!(self.running(), "attermpting run but not running");
        // --- Check for and Handle Exceptions First ---
        if let Some(exc) = self.exception.clone() {
            tracing::info!(
                exception = format!("{:?}", exc),
                "Handling exception in micro_step"
            );
            // Clone to avoid borrow checker issues if handle_exception modifies self.exception
            self.handle_exception(exc);
            // After handling, the state is reset, so we can return Ok and the next micro_step will fetch the handler.
            tracing::debug!("Exception handled, returning from micro_step");
        }

        if matches!(self.cpu_state, CpuState::Fetch) {
            self.fetch()?;
        }
        if matches!(self.cpu_state, CpuState::Decode) {
            self.decode()?;
        }

        // Give devices a chance to do their things
        tracing::trace!("Updating devices");
        self.update_devices();
        tracing::trace!(
            kbsr = format!("0x{:04X}", self.memory[KBSR_ADDR].get()),
            kbdr = format!("0x{:04X}", self.memory[KBDR_ADDR].get()),
            dsr = format!("0x{:04X}", self.memory[DSR_ADDR].get()),
            ddr = format!("0x{:04X}", self.memory[DDR_ADDR].get()),
            "Device registers after update"
        );

        // --- Execute Current CPU State Phase ---

        self.step_phase()?;

        if self.execute_state.is_instruction_complete() {
            self.cpu_state = CpuState::Fetch;
        }

        Ok(())
    }

    /// **Step:** Execute one full instruction cycle (multiple micro-steps).
    pub fn step(&mut self) {
        let input_running = self.running();

        self.start_running();

        // Execute micro-steps until we return to the Fetch state, completing one instruction.
        if matches!(self.cpu_state, CpuState::Fetch) {
            let _ = self.micro_step(); // Step over  Fetch
        }
        while !matches!(self.cpu_state, CpuState::Fetch) && self.running() {
            // Continue micro-stepping until Fetch is reached or an exception occurs
            let _ = self.micro_step(); // we ignore becuase it should already set an exeption
        }

        // Check if somehow not running anymore (e.g. HALT)
        if !self.running() {
            return;
        }

        debug_assert!(matches!(self.cpu_state, CpuState::Fetch), "invalid step");

        if !input_running {
            self.stop_running();
        }
    }

    /// **Run:** Execute instructions until HALT, error, input wait, or max_steps.
    pub fn run(&mut self, max_steps: Option<usize>) -> Result<(), String> {
        self.start_running();
        let mut steps = 0;

        loop {
            if !self.running() {
                tracing::info!("Execution halted.");
                return Ok(());
            }

            if let Some(max) = max_steps {
                if steps >= max {
                    tracing::info!("Reached maximum steps ({}), stopping execution.", max);
                    self.stop_running();
                    return Ok(());
                }
            }

            // Execute one full instruction step
            self.step();

            // Step completed successfully (or halted, or paused for input, or exception pending)
            // Check running state again in case step caused HALT
            if !self.running() {
                tracing::info!("Execution halted by instruction.");
                return Ok(());
            }
            // Check for pending exception after the step finished
            if self.exception.is_some() {
                tracing::warn!("Exception pending after step, will be handled on next cycle.");
                // Continue loop, exception handler runs at start of next micro_step
            }

            if max_steps.is_some() {
                steps += 1;
            }
        }
    }

    // Custom device ideas:
    // time
    // file-system
    // pixel display
    // For now keep it simple
    fn update_devices(&mut self) {
        // Check if a program is trying to write to display
        let dsr_value = self.memory[DSR_ADDR].get();
        if (dsr_value & 0x8000) != 0 {
            // Display is ready to receive character
            let ddr_value = self.memory[DDR_ADDR].get();
            // Check if a value has been written to DDR that hasn't been processed
            if (ddr_value & 0xFF) != 0 {
                // Extract ASCII character
                let character = (ddr_value & 0xFF) as u8 as char;
                // Convert to character and add to output
                self.output.push(character);
                // Clear DDR after processing
                self.memory[DDR_ADDR].set(0);
            }
        }
    }
}

/// if we can bit adress a type then we can index into the bits.
pub trait BitAddressable {
    fn index(&self, addr: u8) -> Self;
    fn range(&self, slice: Range<u8>) -> Self;
}

impl BitAddressable for EmulatorCell {
    fn index(&self, addr: u8) -> Self {
        assert!(addr < 16, "Address out of range");
        Self((self.0 >> addr) & 1, true)
    }

    fn range(&self, slice: Range<u8>) -> Self {
        // Reversed range: bigger (start) to smaller (end)
        assert!(slice.start >= slice.end, "Invalid range");
        let start = slice.start;
        let end = slice.end;
        let width = (start + 1) - end;
        let mask = ((1 << width) - 1) << end;
        Self((self.0 & mask) >> end, true)
    }
}
