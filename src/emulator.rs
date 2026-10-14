#![allow(clippy::unusual_byte_groupings)] // so we can group bits by instruction parts
#![allow(clippy::reversed_empty_ranges)] // We want to use ranges for bis like we have in class (big:small)

/// Turn machine words back into assembly text
pub mod disasm;
/// Run the low level ops
pub mod executor;
mod fast;
/// What each micro-op did on the datapath, for the datapath view
pub mod states;
pub mod trace;
/// Manage the low level ops that each instruction is broken down into
#[macro_use]
pub mod micro_op;
/// Spec for each op so they can be executed
pub mod ops;
/// Convert a series of lines of LC-3 code into emulator cells reporting errors
pub mod parse;
#[cfg(test)]
/// Tests for emulation layer
mod tests;

use crate::emulator::micro_op::MicroOp;
use std::{
    collections::{HashSet, VecDeque},
    ops::Range,
};

pub use ops::{CpuState, OpCode};
use parse::ParseOutput;

use crate::emulator::{
    executor::CpuPhaseState,
    micro_op::{CycleState, MicroOpGenerator},
    parse::CompilationArtifacts,
};

/// The bundled operating system source and where each of its lines ended up in memory.
pub struct OsSource {
    pub text: &'static str,
    /// address -> 1-based line
    pub address_to_line: std::collections::HashMap<usize, usize>,
    /// 1-based line -> address
    pub line_to_address: std::collections::HashMap<usize, usize>,
}

/// Which operating system the machine boots. Both have the same traps; they differ in how the
/// keyboard is read, so the two can be compared.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, serde::Serialize, serde::Deserialize)]
pub enum OsKind {
    /// The keyboard interrupts the program (KBSR[14] set); its service routine buffers each
    /// key and GETC reads the buffer
    #[default]
    Interrupt,
    /// GETC polls KBSR[15] until a key is ready; the keyboard never interrupts
    Polling,
}

impl OsKind {
    pub const ALL: [OsKind; 2] = [OsKind::Interrupt, OsKind::Polling];

    pub fn file_name(self) -> &'static str {
        match self {
            OsKind::Interrupt => "interrupt_os.asm",
            OsKind::Polling => "polling_os.asm",
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            OsKind::Interrupt => "Interrupt-driven OS",
            OsKind::Polling => "Polling OS",
        }
    }

    pub fn about(self) -> &'static str {
        match self {
            OsKind::Interrupt => "The keyboard interrupts the running program; a service routine buffers each key and GETC reads the buffer",
            OsKind::Polling => "GETC loops on KBSR until a key is ready; the keyboard never interrupts",
        }
    }

    fn text(self) -> &'static str {
        match self {
            OsKind::Interrupt => include_str!("../oses/interrupt_os.asm"),
            OsKind::Polling => include_str!("../oses/polling_os.asm"),
        }
    }
}

static OS_KIND: std::sync::atomic::AtomicU8 = std::sync::atomic::AtomicU8::new(0);

/// The OS the machine boots (on its next reset).
pub fn os_kind() -> OsKind {
    match OS_KIND.load(std::sync::atomic::Ordering::Relaxed) {
        1 => OsKind::Polling,
        _ => OsKind::Interrupt,
    }
}

/// Choose the OS the machine boots. Takes effect on the next reset.
pub fn set_os_kind(kind: OsKind) {
    OS_KIND.store(kind as u8, std::sync::atomic::Ordering::Relaxed);
}

/// The source map of the OS the machine boots.
pub fn os_source() -> &'static OsSource {
    os_source_of(os_kind())
}

/// An OS's source map, assembled once. User programs overwrite the emulator's own metadata,
/// so the OS keeps its own copy for the read-only OS editor.
pub fn os_source_of(kind: OsKind) -> &'static OsSource {
    static OSES: [std::sync::OnceLock<OsSource>; 2] =
        [std::sync::OnceLock::new(), std::sync::OnceLock::new()];
    OSES[kind as usize].get_or_init(|| {
        let text = kind.text();
        let mut artifacts = CompilationArtifacts::default();
        let _ = Emulator::parse_program(text, Some(&mut artifacts));
        OsSource {
            text,
            address_to_line: artifacts.address_to_line,
            line_to_address: artifacts.line_to_address,
        }
    })
}

/// The amount of steps to skip when os skips are enabled and we are in OS memory space
pub const MAX_OS_STEPS: usize = 1000;
/// Cycles per update from which Run uses the fast path (the datapath can't follow anyway)
pub const FAST_SPEED: u32 = 2000;
/// ... or from which it does when the datapath isn't on screen
pub const FAST_SPEED_UNTRACED: u32 = 100;
/// How long one update may run at Max speed (or at most at any fast speed), in ms
pub const MAX_SPEED_SLICE_MS: u64 = 12;

// Device registers at memory addresses xFE00-xFFFF
/// Keyboard status ADDR, KBSR[15] = 1 when we have input to be read
pub const KBSR_ADDR: usize = 0xFE00;
/// The keyboard's interrupt vector (INTV) and priority level, as in the book
pub const KEYBOARD_INTV: u16 = 0x80;
pub const KEYBOARD_PRIORITY: u16 = 4;
/// Keyboard destination ADDR, KBDR[7:0] = last typed in ascii
pub const KBDR_ADDR: usize = 0xFE02;
/// Display Status ADDR, DSR[15] = 1 when display service ready to display a new char (always 1 in this emulator)
pub const DSR_ADDR: usize = 0xFE04;
/// Display destination register, when DDR[7:0] set we write the ascii char contained to the output
pub const DDR_ADDR: usize = 0xFE06;
/// Processor status register: PSR[15] = 0 in supervisor mode and 1 in user mode, PSR[10:8] the
/// priority (interrupts need a higher one), PSR[2:0] the condition codes N, Z, P.
pub const PSR_ADDR: usize = 0xFFFC;
/// Machine control register: MCR[15] = 1 while the clock runs; clearing it halts. Setting MCR[14]
/// (this emulator's wait for interrupt) stops the clock until a device asks to interrupt.
pub const MCR_ADDR: usize = 0xFFFE;

#[derive(Clone)]
pub struct Emulator {
    // --- not involved in the state machine ---
    /// How many cycles to run per update call
    pub speed: u32,
    /// How many update calls to wait before doing cycles equal to speed
    pub ticks_between_updates: u32,
    /// Run as fast as possible: each update runs for a slice of the frame instead of a number
    /// of cycles
    pub max_speed: bool,
    /// What is the current tick, only really used for modulo so we could wrap it around
    pub tick: u64,
    /// Do we jump over os instructions according to the [`MAX_OS_STEPS`] var
    pub skip_os_emulation: bool,
    /// The summation of all MEM[DDR] sets aka the 'output' of the emulator
    pub output: String,
    /// Some associated data for the most recent set of compiled programs
    pub metadata: CompilationArtifacts,
    pub breakpoints: HashSet<usize>,
    pub currently_executing: usize,
    /// Has the machine been halted by the OS/program
    pub halted: bool,
    /// Is the machine currently on a breakpoint and has already halted once from it
    pub halted_on_breakpoint: bool,
    // -----------------------------------------

    // Why in a Box? Because the array would sit on the stack and takes a lot of memory.
    // wasm was unhappy so I put it on the heap using Box
    // TODO: How much faster is it on the stack? maybe it should be a compile time distinction
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

    /// Stores the instruction we are in the process of executing the first 4 bits are the op etc.
    /// Set in the fetch phase (IR <- MEM[pc])
    pub ir: EmulatorCell,

    /// CPU state. fetch -> decode -> evaluate address etc
    /// Each instruction relies on its own struct of the instruction after the decode stage
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
    /// Figure C.8's Table and Vector registers: MAR <- Table'Vector addresses the trap and
    /// interrupt vector tables
    pub table: EmulatorCell,
    pub vector: EmulatorCell,
    /// The state machine state (Figures C.2 and C.7) the last `MicroOp::State` entered
    pub state_no: u8,
    /// The last state that checked ACV, privilege or the opcode, which picks the exception state
    pub check_state: u8,
    /// A state the state machine does not enter this time (e.g. BR's state 22 when BEN is 0):
    /// its marker and ops are skipped
    pub skip_state: Option<u8>,
    /// The current instruction has been fetched (its plan is set up)
    pub fetched: bool,
    /// The machine as it was before the current (or last) instruction, so a step can go back a
    /// state by re-running from there. Kept only while stepping (it is a copy of the machine).
    pub rewind: Option<Box<Emulator>>,
    /// States the machine has entered in the current (or last) instruction
    pub states_in_instruction: usize,
    /// A step command is running
    pub stepping: bool,
    /// The last fetch took an interrupt instead of an instruction (state 18 -> 49)
    pub took_interrupt: bool,
    /// The OS wrote MCR[14] (wait for interrupt): the clock is stopped until a device
    /// requests an interrupt
    pub asleep: bool,

    /// If the state machine has reached an exception state, the particulars
    pub exception: Option<Exception>,

    // --- debugger state (not part of the LC-3) ---
    /// Characters typed into the terminal that the program has not read yet
    pub input_queue: VecDeque<char>,
    /// Keys typed so far, so stepping back keeps the ones typed since the snapshot
    pub keys_typed: u64,
    /// The OS this machine booted
    pub os: OsKind,
    /// Stop when the PC reaches this address at an instruction boundary (step over)
    pub run_target: Option<usize>,
    /// Call depth tracking for "step out", `Some(depth)` while stepping out
    pub step_out_depth: Option<u32>,
    /// Set when a step out has seen the returning instruction, stop at the next boundary
    pub stop_at_next_instruction: bool,
    /// Why the machine last stopped, for the status bar
    pub stop_reason: StopReason,
    /// Clock cycles (states of the state machine) since the last reset
    pub cycles: u64,
    /// Number of instructions completed since the last reset
    pub instructions: u64,
    /// Output latch of the address adder (ADDR1MUX + ADDR2MUX)
    pub addr_out: EmulatorCell,
    /// The datapath steps of the current (or just finished) instruction
    pub micro_log: Vec<trace::MicroRecord>,
    /// Bumped whenever `micro_log` starts a new instruction
    pub micro_log_id: u64,
    /// The machine as `micro_log`'s instruction began, before its first state
    pub micro_log_before: trace::Snapshot,
    /// Record `micro_log` (only needed while the datapath is on screen)
    pub trace: bool,
    /// The instruction that most recently wrote each register (for the debugger)
    pub reg_writes: [Option<RegisterAccess>; 8],
    /// The instruction that most recently read each register. A `Cell` because reads happen
    /// through `&self`.
    pub reg_reads: [std::cell::Cell<Option<RegisterAccess>>; 8],
    /// Programs assembled by the editors, in load order, so a reset can reload them all.
    /// Several can be loaded at once (e.g. code at x3000 and test data at x3500).
    pub loaded_programs: Vec<LoadedProgram>,
}

/// Where a register was last read or written.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RegisterAccess {
    /// Address of the instruction that did it
    pub addr: usize,
    /// Instruction count when it happened, to tell how long ago
    pub instruction: u64,
}

/// A program image that can be flashed back into memory after a reset, with its source map.
#[derive(Debug, Clone, Default)]
pub struct LoadedProgram {
    /// Which editor document it came from
    pub source_id: u64,
    pub name: String,
    pub machine_code: Vec<u16>,
    pub orig_address: usize,
    /// The source lines it was assembled from
    pub source: Vec<String>,
    /// address -> 1-based line
    pub address_to_line: std::collections::HashMap<usize, usize>,
    /// 1-based line -> address
    pub line_to_address: std::collections::HashMap<usize, usize>,
}

impl LoadedProgram {
    pub fn range(&self) -> std::ops::Range<usize> {
        self.orig_address..self.orig_address + self.machine_code.len()
    }
    pub fn contains(&self, addr: usize) -> bool {
        self.range().contains(&addr)
    }
}

/// Why execution stopped, shown in the toolbar.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum StopReason {
    /// Freshly reset or loaded
    #[default]
    Ready,
    /// The user pressed pause
    Paused,
    /// A step command completed
    Stepped,
    Breakpoint(u16),
    Halted,
}

impl Emulator {
    /// A machine that boots the OS chosen with [`set_os_kind`].
    pub fn new() -> Emulator {
        Self::with_os(os_kind())
    }

    /// A machine that boots `os`.
    pub fn with_os(os: OsKind) -> Emulator {
        let mut emulator = Self {
            halted: false,
            halted_on_breakpoint: false,
            speed: 1,
            ticks_between_updates: 2,
            max_speed: false,
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
            table: EmulatorCell::new(0),
            vector: EmulatorCell::new(0),
            state_no: 18,
            check_state: 18,
            skip_state: None,
            fetched: false,
            rewind: None,
            states_in_instruction: 0,
            stepping: false,
            took_interrupt: false,
            asleep: false,
            input_queue: VecDeque::new(),
            keys_typed: 0,
            os,
            run_target: None,
            step_out_depth: None,
            stop_at_next_instruction: false,
            stop_reason: StopReason::Ready,
            cycles: 0,
            instructions: 0,
            loaded_programs: Vec::new(),
            reg_writes: [None; 8],
            reg_reads: Default::default(),
            addr_out: EmulatorCell::new(0),
            micro_log: Vec::new(),
            micro_log_before: trace::Snapshot::default(),
            micro_log_id: 0,
            trace: false,
        };

        let parse_output =
            Emulator::parse_program(emulator.os.text(), Some(&mut emulator.metadata));

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

    /// Are we running in supervisor or user mode?
    pub fn priv_level(&self) -> PrivilegeLevel {
        match self.memory[PSR_ADDR].index(15).get() {
            0 => PrivilegeLevel::Supervisor,
            1 => PrivilegeLevel::User,
            _ => unreachable!(),
        }
    }

    /// Reset all registers and set PC to x200
    pub fn soft_reset(&self) -> Self {
        // a freshly booted OS and devices (its keyboard buffer, KBSR...), with user memory kept
        let mut fresh = Emulator::with_os(self.os);
        for a in 0x3000..0xFE00 {
            fresh.memory[a] = self.memory[a];
        }
        fresh.output = self.output.clone();
        fresh.speed = self.speed;
        fresh.ticks_between_updates = self.ticks_between_updates;
        fresh.max_speed = self.max_speed;
        fresh.skip_os_emulation = self.skip_os_emulation;
        fresh.trace = self.trace;
        fresh.metadata = self.metadata.clone();
        fresh.breakpoints = self.breakpoints.clone();
        fresh.loaded_programs = self.loaded_programs.clone();
        fresh.input_queue = self.input_queue.clone();
        fresh.keys_typed = self.keys_typed;
        fresh
    }

    /// Load (or replace) a program and reset the machine so it starts from the OS again.
    /// Other loaded programs stay in memory.
    pub fn load_program(&mut self, program: LoadedProgram) {
        self.loaded_programs
            .retain(|p| p.source_id != program.source_id);
        // a program that this one completely covers is gone from memory anyway
        let new_range = program.range();
        self.loaded_programs
            .retain(|p| !(new_range.start <= p.orig_address && p.range().end <= new_range.end));
        self.loaded_programs.push(program);
        self.hard_reset();
    }

    /// Forget all loaded programs and reset the machine.
    pub fn unload_programs(&mut self) {
        self.loaded_programs.clear();
        self.breakpoints.clear();
        self.hard_reset();
    }

    pub(crate) fn register_access(&self) -> RegisterAccess {
        RegisterAccess {
            addr: self.currently_executing,
            instruction: self.instructions,
        }
    }

    /// The source line (document name, 0-based line, text) for an address, from the loaded
    /// programs or the OS.
    pub fn source_line(&self, addr: usize) -> Option<(String, usize, String)> {
        if let Some(p) = self.program_at(addr) {
            let l = p.address_to_line.get(&addr)?.saturating_sub(1);
            return Some((
                p.name.clone(),
                l,
                p.source.get(l).cloned().unwrap_or_default(),
            ));
        }
        if addr < 0x3000 {
            let os = os_source_of(self.os);
            let l = os.address_to_line.get(&addr)?.saturating_sub(1);
            return Some((
                "OS".to_string(),
                l,
                os.text.lines().nth(l).unwrap_or("").to_string(),
            ));
        }
        None
    }

    /// The loaded program whose words include `addr` (the most recently loaded wins).
    pub fn program_at(&self, addr: usize) -> Option<&LoadedProgram> {
        self.loaded_programs.iter().rev().find(|p| p.contains(addr))
    }

    /// Reset the whole machine (CPU, memory and devices) and reload the loaded programs.
    /// Speed, the skip OS setting and breakpoints are kept.
    pub fn hard_reset(&mut self) {
        let mut fresh = Emulator::with_os(self.os);
        fresh.speed = self.speed;
        fresh.ticks_between_updates = self.ticks_between_updates;
        fresh.max_speed = self.max_speed;
        fresh.skip_os_emulation = self.skip_os_emulation;
        fresh.trace = self.trace;
        fresh.breakpoints = std::mem::take(&mut self.breakpoints);
        // labels: the OS just booted (it may be another one) and the loaded programs'
        let mut metadata = std::mem::take(&mut self.metadata);
        let in_program = |a: &usize| self.loaded_programs.iter().any(|p| p.contains(*a));
        metadata.addr_to_label.retain(|a, _| in_program(a));
        metadata.labels.retain(|_, a| in_program(a));
        for (a, l) in std::mem::take(&mut fresh.metadata.addr_to_label) {
            metadata.addr_to_label.entry(a).or_insert(l);
        }
        for (l, a) in std::mem::take(&mut fresh.metadata.labels) {
            metadata.labels.entry(l).or_insert(a);
        }
        fresh.metadata = metadata;
        for program in &self.loaded_programs {
            fresh.flash_memory(program.machine_code.clone(), program.orig_address);
        }
        fresh.loaded_programs = std::mem::take(&mut self.loaded_programs);
        *self = fresh;
    }

    /// Queue a character typed by the user. It is delivered through KBDR when the program has
    /// read the previous one.
    pub fn push_input(&mut self, c: char) {
        if c.is_ascii() {
            self.keys_typed += 1;
            self.input_queue.push_back(c);
        }
    }

    /// The INT signal state 18 tests: the keyboard has a key (KBSR[15]), may interrupt
    /// (KBSR[14]) and its priority is above the running program's (PSR[10:8]).
    pub fn interrupt_pending(&self) -> bool {
        let priority = self.memory[PSR_ADDR].get() >> 8 & 7;
        self.interrupt_requested() && KEYBOARD_PRIORITY > priority
    }

    /// A device asks to interrupt (the keyboard has a key and KBSR[14] is set), whatever the
    /// program's priority. This wakes a sleeping machine.
    pub fn interrupt_requested(&self) -> bool {
        self.memory[KBSR_ADDR].get() & 0xC000 == 0xC000
    }

    /// Is the program waiting on the keyboard (OS polling KBSR with nothing typed)?
    pub fn waiting_for_input(&self) -> bool {
        if self.asleep {
            return self.running() && self.input_queue.is_empty();
        }
        self.running()
            && self.input_queue.is_empty()
            && self.memory[KBSR_ADDR].get() & 0x8000 == 0
            && {
                // Are we inside the GETC or IN service routine?
                let pc = self.pc.get();
                [0x20usize, 0x23].iter().any(|v| {
                    let start = self.memory[*v].get();
                    pc >= start && pc < start.wrapping_add(0x30)
                })
            }
    }

    // --- Debugger commands ---

    /// Start or resume execution. A halted program is restarted from the OS entry point.
    pub fn resume(&mut self) {
        if self.halted {
            self.halted = false;
            *self = self.soft_reset();
        }
        self.clear_step_targets();
        // Don't stop again on the breakpoint we are currently sitting on
        self.halted_on_breakpoint = self.breakpoints.contains(&(self.pc.get() as usize));
        self.start_running();
    }

    /// Pause execution.
    pub fn pause(&mut self) {
        self.clear_step_targets();
        self.stop_running();
        self.stop_reason = StopReason::Paused;
    }

    fn clear_step_targets(&mut self) {
        self.run_target = None;
        self.step_out_depth = None;
        self.stop_at_next_instruction = false;
    }

    /// Skip over OS code after a step when "skip OS" is enabled.
    fn skip_os_after_step(&mut self) {
        if self.skip_os_emulation {
            let mut os_steps = 0;
            // the OS's instructions aren't snapshotted (Back can't go into skipped OS code)
            let stepping = std::mem::replace(&mut self.stepping, false);
            while self.pc.get() < 0x3000
                && os_steps < MAX_OS_STEPS
                && self.running()
                && !self.asleep
            {
                self.step();
                os_steps += 1;
            }
            self.stepping = stepping;
            if os_steps > 0 {
                self.rewind = None;
            }
        }
    }

    fn prepare_step(&mut self) -> bool {
        if self.halted {
            return false;
        }
        self.stepping = true;
        self.clear_step_targets();
        let was_running = self.running();
        self.start_running();
        was_running
    }

    fn finish_step(&mut self, was_running: bool) {
        self.stepping = false;
        if !was_running && self.running() {
            self.stop_running();
        }
        if self.halted {
            self.stop_reason = StopReason::Halted;
        } else if !self.running() {
            self.stop_reason = StopReason::Stepped;
        }
    }

    /// Execute one micro step: one state of the state machine (one clock cycle).
    pub fn micro_step_command(&mut self) {
        let was_running = self.prepare_step();
        if self.halted {
            return;
        }
        let _ = self.state_step();
        if matches!(self.cpu_state, CpuState::Fetch) && !self.fetched {
            self.skip_os_after_step();
        }
        self.finish_step(was_running);
    }

    /// Whether `step_back` can go back a state: not past the start of the current instruction.
    pub fn can_step_back(&self) -> bool {
        self.rewind.is_some() && self.states_in_instruction > 0 && !self.running()
    }

    /// Go back one state of the state machine: restore the machine from before the current
    /// instruction and run it to the state before this one. Everything (registers, memory,
    /// devices, output, the micro-op log) goes back with it.
    pub fn step_back(&mut self) {
        if !self.can_step_back() {
            return;
        }
        let Some(snapshot) = self.rewind.take() else {
            return;
        };
        let target = self.states_in_instruction - 1;
        let mut machine = (*snapshot).clone();
        // settings the user may have changed since are not part of the machine
        machine.breakpoints = std::mem::take(&mut self.breakpoints);
        machine.speed = self.speed;
        machine.ticks_between_updates = self.ticks_between_updates;
        machine.max_speed = self.max_speed;
        machine.skip_os_emulation = self.skip_os_emulation;
        machine.trace = true;
        machine.stepping = true;
        machine.rewind = Some(snapshot);
        // the snapshot is from just before this instruction's fetch
        machine.states_in_instruction = 0;
        // keys typed since the snapshot are still to be read
        let typed = self.keys_typed.saturating_sub(machine.keys_typed) as usize;
        let late: Vec<char> = self
            .input_queue
            .iter()
            .rev()
            .take(typed)
            .rev()
            .copied()
            .collect();
        machine.input_queue.extend(late);
        machine.keys_typed = self.keys_typed;
        while machine.states_in_instruction < target {
            if machine.state_step().is_err() && machine.exception.is_none() {
                break;
            }
        }
        machine.stepping = false;
        machine.stop_running();
        machine.stop_reason = StopReason::Stepped;
        *self = machine;
    }

    /// Execute one whole instruction, stepping into subroutines.
    pub fn step_into(&mut self) {
        let was_running = self.prepare_step();
        if self.halted {
            return;
        }
        self.step();
        let mut interrupted = self.took_interrupt;
        self.skip_os_after_step();
        // with the OS skipped an interrupt is invisible, so step the instruction it delayed
        let mut tries = 0;
        while interrupted && self.skip_os_emulation && self.running() && tries < 4 {
            self.step();
            interrupted = self.took_interrupt;
            self.skip_os_after_step();
            tries += 1;
        }
        self.finish_step(was_running);
    }

    /// Execute one instruction, running subroutine calls to completion.
    pub fn step_over(&mut self) {
        if self.halted {
            return;
        }
        let at_boundary = matches!(self.cpu_state, CpuState::Fetch) && !self.fetched;
        let instr = self.memory[self.pc.get() as usize].get();
        let opcode = instr >> 12;
        let is_call = opcode == 0b0100 || (opcode == 0b1111 && !self.skip_os_emulation);
        if at_boundary && is_call {
            self.clear_step_targets();
            self.run_target = Some(self.pc.get().wrapping_add(1) as usize);
            self.halted_on_breakpoint = true; // don't stop on a breakpoint at the call itself
            self.start_running();
        } else {
            self.step_into();
        }
    }

    /// Run until the current subroutine returns.
    pub fn step_out(&mut self) {
        if self.halted {
            return;
        }
        self.clear_step_targets();
        self.step_out_depth = Some(0);
        self.halted_on_breakpoint = true;
        self.start_running();
    }

    /// Called at each instruction boundary while running. Returns true if execution should stop.
    fn check_step_targets(&mut self) -> bool {
        let pc = self.pc.get() as usize;
        if self.stop_at_next_instruction {
            self.clear_step_targets();
            return true;
        }
        if self.run_target == Some(pc) {
            self.clear_step_targets();
            return true;
        }
        // the OS's own calls and returns don't count when it is skipped
        let tracked = !(self.skip_os_emulation && pc < 0x3000);
        if let Some(depth) = self.step_out_depth.filter(|_| tracked) {
            let instr = self.memory[pc].get();
            let opcode = instr >> 12;
            // an interrupt is a call to its service routine, which returns with RTI
            let interrupt = self.interrupt_pending();
            let is_ret =
                !interrupt && ((opcode == 0b1100 && (instr >> 6) & 0b111 == 7) || opcode == 0b1000);
            let is_call = if interrupt {
                !self.skip_os_emulation
            } else {
                opcode == 0b0100 || (opcode == 0b1111 && !self.skip_os_emulation)
            };
            if is_call {
                self.step_out_depth = Some(depth + 1);
            } else if is_ret {
                if depth == 0 {
                    self.stop_at_next_instruction = true;
                } else {
                    self.step_out_depth = Some(depth - 1);
                }
            }
        }
        false
    }

    /// Change the privilege mode.
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

    /// Set the negative condition bit (resetting the others)
    pub fn set_n(&mut self) {
        let psr = self.memory[PSR_ADDR].get();
        let new_psr = (psr & 0xFFF8) | 0x0004;
        self.memory[PSR_ADDR].set(new_psr);
    }

    /// Set the zero condition bit (resetting the others)
    pub fn set_z(&mut self) {
        let psr = self.memory[PSR_ADDR].get();
        let new_psr = (psr & 0xFFF8) | 0x0002;
        self.memory[PSR_ADDR].set(new_psr);
    }

    /// Set the positive condition bit (resetting the others)
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
        // normally exactly one is set, but the PSR can be edited (or written by a program), so
        // any combination is shown as it is
        (n, z, p)
    }

    /// Given a register we update the flags based on value
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
/// The core data structure for the emulator. Every value is stored via this using .get() and .set(). Stores an LC-3 word (16 bits)
pub struct EmulatorCell(u16);

impl EmulatorCell {
    #[inline(always)]
    pub fn new(value: u16) -> Self {
        Self(value)
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
        }
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
            Self(value | mask)
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

#[derive(Debug, Clone, Default)]
/// This component of the state machine takes some operation then a number of cells and outputs the result of an arithmetic op
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
            // fast enough that nobody watches the states: whole instructions at once
            let fast = self.max_speed
                || self.speed >= FAST_SPEED
                || (!self.trace && self.speed >= FAST_SPEED_UNTRACED);
            let fast = fast && self.can_run_fast();
            if fast {
                let (cycles, deadline) = if self.max_speed {
                    (
                        u64::MAX,
                        Some(
                            web_time::Instant::now()
                                + web_time::Duration::from_millis(MAX_SPEED_SLICE_MS),
                        ),
                    )
                } else {
                    (
                        self.speed as u64,
                        Some(
                            web_time::Instant::now()
                                + web_time::Duration::from_millis(MAX_SPEED_SLICE_MS),
                        ),
                    )
                };
                changed |= self.run_fast(cycles, deadline);
            } else if self.tick % self.ticks_between_updates as u64 == 0 {
                let mut i = 0;
                while self.running() && i < self.speed {
                    // asleep: only the devices run (a key may wake it). The boundary checks
                    // below were done before it went to sleep.
                    if self.asleep {
                        let _ = self.state_step();
                        if self.asleep {
                            break;
                        }
                        changed = true;
                        i += 1;
                        continue;
                    }
                    // Check for breakpoints
                    let current_pc = self.pc.get() as usize;

                    if self.breakpoints.contains(&current_pc)
                        && !self.halted_on_breakpoint
                        && matches!(self.cpu_state, CpuState::Fetch)
                        && !self.fetched
                    // Break before fetching the instruction at the breakpoint
                    {
                        self.stop_running();
                        self.clear_step_targets();
                        log::info!("Breakpoint hit at address 0x{current_pc:04X}");
                        self.halted_on_breakpoint = true;
                        self.stop_reason = StopReason::Breakpoint(current_pc as u16);
                        changed = true;
                        break;
                    }

                    if matches!(self.cpu_state, CpuState::Fetch)
                        && !self.fetched
                        && (self.run_target.is_some()
                            || self.step_out_depth.is_some()
                            || self.stop_at_next_instruction)
                        && self.check_step_targets()
                    {
                        self.stop_running();
                        self.stop_reason = StopReason::Stepped;
                        changed = true;
                        break;
                    }

                    changed = true;
                    // whole states, so the machine only ever stops between clock cycles
                    let _ = self.state_step();
                    i += 1;

                    self.halted_on_breakpoint = false;

                    if i >= self.speed {
                        break;
                    }
                }
            }

            // Skip OS code if enabled during running mode
            // Limit OS skipping to avoid freezing
            // (the fast path runs the OS at full speed anyway)
            if self.skip_os_emulation && !fast {
                while self.pc.get() < 0x3000
                    && os_steps < MAX_OS_STEPS
                    && self.running()
                    && !self.asleep
                {
                    changed = true;
                    self.step();
                    os_steps += 1;
                }
            }
            if !self.running() && self.halted {
                self.stop_reason = StopReason::Halted;
                self.clear_step_targets();
            }
        }
        changed
    }

    // --- Core Instruction Cycle Phases ---

    /// **Fetch Phase:** Read instruction from memory at PC into IR, increment PC.
    fn fetch(&mut self) -> Result<(), String> {
        // remember the machine before this instruction, to step back through its states
        self.rewind = None;
        if self.stepping {
            let snapshot = self.clone();
            self.rewind = Some(Box::new(snapshot));
        }
        self.states_in_instruction = 0;
        let pc_value = self.pc.get();
        let memory_area = area_from_address(&self.pc);

        self.currently_executing = pc_value as usize;
        self.fetched = true;
        self.skip_state = None;

        // State 18 loads MAR and increments the PC; state 33 checks ACV (Figure C.2)
        let mut fetch_phase = vec![
            micro_op!(-> Fetch),
            MicroOp::State(18),
            micro_op!(MAR <- PC),
            micro_op!(PC <- PcPlusOne),
        ];
        self.took_interrupt = self.interrupt_pending();
        if self.took_interrupt {
            // [INT]: the keyboard interrupts instead of the instruction at PC being fetched
            fetch_phase.extend(states::interrupt_entry());
            self.execute_state = CpuPhaseState::new(vec![fetch_phase]);
            self.micro_log.clear();
            self.micro_log_before = self.snapshot();
            self.micro_log_id = self.micro_log_id.wrapping_add(1);
            return Ok(());
        }
        fetch_phase.push(MicroOp::State(33));
        if !memory_area.can_read(&self.priv_level()) {
            fetch_phase.push(MicroOp::new_custom(
                |_| Err(Exception::AccessControlViolation),
                "[ACV] = 1: user mode can't fetch from privileged memory".to_owned(),
            ));
            self.execute_state = CpuPhaseState::new(vec![fetch_phase]);
            self.micro_log.clear();
            self.micro_log_before = self.snapshot();
            self.micro_log_id = self.micro_log_id.wrapping_add(1);
            return Ok(());
        }
        fetch_phase.extend([
            MicroOp::State(28),
            micro_op!(-> Fetch), // MDR <- M[MAR]
            MicroOp::State(30),
            micro_op!(IR <- MDR),
            MicroOp::State(32),
        ]);

        // Opcode 1101 is reserved: decoding it is an illegal opcode exception (state 13)
        let word = self.memory[pc_value as usize];
        if word.get() >> 12 == 0xD {
            let decode_phase = vec![
                micro_op!(-> Decode),
                MicroOp::new_custom(
                    |_| Err(Exception::IllegalInstruction),
                    "IR[15:12] = 1101 is reserved: illegal opcode".to_owned(),
                ),
            ];
            self.execute_state = CpuPhaseState::new(vec![fetch_phase, decode_phase]);
            self.micro_log.clear();
            self.micro_log_before = self.snapshot();
            self.micro_log_id = self.micro_log_id.wrapping_add(1);
            return Ok(());
        }

        // Get the micro-op generator for the instruction
        let opcode = OpCode::from_instruction(word).expect("opcode 1101 is handled above");
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
            fetch_phase,
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
        self.micro_log.clear();
        self.micro_log_before = self.snapshot();
        self.micro_log_id = self.micro_log_id.wrapping_add(1);

        Ok(())
    }

    /// **Handle Exception:** run the exception entry states of Figure C.7 as micro-ops: they
    /// switch to Supervisor mode and stack, push PSR and PC - 1, and load PC from the vector
    /// table. The states continue the faulting instruction's micro-op log.
    fn handle_exception(&mut self, exception: Exception) {
        tracing::warn!("Handling Exception: {:?}", exception);
        self.exception = None;
        let (state, vector) = states::exception_state(&exception, self.check_state);
        // the memory access after the failed check never happened
        while self
            .micro_log
            .last()
            .is_some_and(|r| r.marker && states::ACCESS_STATES.contains(&r.state))
        {
            self.micro_log.pop();
        }
        self.execute_state = CpuPhaseState::new(vec![states::exception_entry(state, vector)]);
        self.cpu_state = CpuState::Exception;
    }

    /// **Step:** Execute one full instruction cycle (several states).
    pub fn step(&mut self) {
        let input_running = self.running();

        self.start_running();

        // Run states until the instruction (and any exception it raised) is done. It may have
        // been started already by micro steps.
        if !self.fetched {
            let _ = self.state_step();
        }
        while self.fetched && self.running() {
            // errors set an exception, which the next state runs
            let _ = self.state_step();
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

    /// **Run:** Execute instructions until HALT, error, max_steps, or the machine goes to sleep
    /// waiting for an interrupt (e.g. GETC with nothing typed).
    pub fn run(&mut self, max_steps: Option<usize>) -> Result<(), String> {
        self.start_running();
        let mut steps = 0;

        loop {
            if !self.running() {
                tracing::info!("Execution halted.");
                return Ok(());
            }
            if self.asleep && !self.interrupt_requested() && self.input_queue.is_empty() {
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
        // Deliver queued keyboard input once the previous character has been read
        if self.memory[KBSR_ADDR].get() & 0x8000 == 0 {
            if let Some(c) = self.input_queue.pop_front() {
                self.memory[KBDR_ADDR].set(c as u16);
                // ready; KBSR[14], interrupt enable, is the OS's to set
                let kbsr = self.memory[KBSR_ADDR].get();
                self.memory[KBSR_ADDR].set(kbsr | 0x8000);
            }
        }
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
        Self((self.0 >> addr) & 1)
    }

    fn range(&self, slice: Range<u8>) -> Self {
        // Reversed range: bigger (start) to smaller (end)
        assert!(slice.start >= slice.end, "Invalid range");
        let start = slice.start;
        let end = slice.end;
        let width = (start + 1) - end;
        let mask = ((1 << width) - 1) << end;
        Self((self.0 & mask) >> end)
    }
}
