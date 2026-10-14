use crate::emulator::micro_op::{
    CycleState, DataDestination, DataSource, MAluOp, MachineFlag, MicroOp,
};
use crate::emulator::states;
use crate::emulator::trace::{self, Activity, MicroRecord};
use crate::emulator::{
    area_from_address, AluOp, CpuState, Emulator, EmulatorCell, Exception, OpCode, KBDR_ADDR,
    KBSR_ADDR, MCR_ADDR, PSR_ADDR,
};

/// Manages the execution state and flow of micro-operations within an instruction cycle
#[derive(Clone)]
pub struct CpuPhaseState {
    /// The complete execution plan for the current instruction (6 phases)
    execution_plan: Vec<Vec<MicroOp>>,
    /// Current phase index (0-5)
    pub current_phase: usize,
    /// Current micro-op index within the current phase
    pub micro_op_index: usize,
    /// Flag indicating if the instruction is complete
    instruction_complete: bool,
    /// Flag indicating if a memory read is pending between phases
    pub memory_read_pending: bool,
    /// Flag indicating if a memory write is pending between phases
    pub memory_write_pending: bool,
    temp_register: EmulatorCell,
}

impl CpuPhaseState {
    /// Create a new phase state with the given execution plan
    pub fn new(execution_plan: Vec<Vec<MicroOp>>) -> Self {
        let span = tracing::trace_span!("CpuPhaseState::new", plan_phases = execution_plan.len());
        let _enter = span.enter();

        tracing::trace!(
            "Created new phase state with {} phases",
            execution_plan.len()
        );

        Self {
            execution_plan,
            current_phase: 0,
            micro_op_index: 0,
            instruction_complete: false,
            memory_read_pending: false,
            memory_write_pending: false,
            temp_register: EmulatorCell::new(0),
        }
    }

    /// Get the micro-ops for a phase
    pub fn phase_ops(&self, i: usize) -> &[MicroOp] {
        if i < self.execution_plan.len() {
            &self.execution_plan[i]
        } else {
            &[]
        }
    }

    /// Check if the instruction is complete
    pub fn is_instruction_complete(&self) -> bool {
        self.instruction_complete
    }
}

impl Emulator {
    /// Run one state of the state machine (Figures C.2 and C.7), i.e. one clock cycle: from a
    /// `State` marker up to the next state the machine enters.
    pub fn state_step(&mut self) -> Result<(), String> {
        // asleep (MCR[14]): no clock cycles until a device requests an interrupt. Its priority
        // doesn't matter, so the OS can sleep with the interrupt masked and take it on waking.
        if self.asleep && !self.fetched && self.exception.is_none() {
            self.update_devices();
            if !self.interrupt_requested() {
                return Ok(());
            }
            self.asleep = false;
        }
        if let Some(exc) = self.exception.take() {
            self.handle_exception(exc);
        }
        if matches!(self.cpu_state, CpuState::Fetch) && !self.fetched {
            self.fetch()?;
        }
        self.update_devices();
        let mut entered = false;
        loop {
            let phase = self.execute_state.current_phase;
            if phase >= self.execute_state.execution_plan.len() {
                self.execute_state.instruction_complete = true;
                self.finish_instruction();
                return Ok(());
            }
            let index = self.execute_state.micro_op_index;
            if index >= self.execute_state.execution_plan[phase].len() {
                // end of a phase: pending memory reads and writes finish this state
                self.handle_implicit_memory_operations()?;
                self.enter_next_phase();
                continue;
            }
            if let MicroOp::State(n) = self.execute_state.execution_plan[phase][index] {
                if self.skip_state != Some(n) {
                    if entered {
                        return Ok(());
                    }
                    entered = true;
                }
            }
            self.execute_micro_op()?;
            self.execute_state.micro_op_index += 1;
        }
    }

    /// Back to fetch once the plan is done.
    pub(crate) fn finish_instruction(&mut self) {
        self.cpu_state = CpuState::Fetch;
        self.instructions += 1;
        self.fetched = false;
    }

    /// Move to the next phase of the plan, updating the displayed CPU state.
    fn enter_next_phase(&mut self) {
        self.execute_state.current_phase += 1;
        self.execute_state.micro_op_index = 0;
        if self.execute_state.current_phase >= self.execute_state.execution_plan.len() {
            self.execute_state.instruction_complete = true;
            return;
        }
        if let Some(MicroOp::PhaseTransition(phase)) =
            self.execute_state.execution_plan[self.execute_state.current_phase].first()
        {
            let op = || {
                self.cpu_state.to_instruction().or_else(|| {
                    (self.ir.get() >> 12 != 0xD)
                        .then(|| OpCode::from_instruction(self.ir))
                        .flatten()
                })
            };
            self.cpu_state = match phase {
                CycleState::Fetch => CpuState::Fetch,
                CycleState::Decode => CpuState::Decode,
                CycleState::EvaluateAddress => {
                    op().map_or(CpuState::Exception, CpuState::EvaluateAddress)
                }
                CycleState::FetchOperands => {
                    op().map_or(CpuState::Exception, CpuState::FetchOperands)
                }
                CycleState::Execute => op().map_or(CpuState::Exception, CpuState::ExecuteOperation),
                CycleState::StoreResult => op().map_or(CpuState::Exception, CpuState::StoreResult),
            };
        }
    }

    /// Handle implicit memory operations that occur between phases
    fn handle_implicit_memory_operations(&mut self) -> Result<(), String> {
        let span = tracing::trace_span!("implicit_memory_ops");
        let _enter = span.enter();

        // Check if there's a pending memory write
        if self.execute_state.memory_write_pending {
            let addr = self.mar.get() as usize;
            let value = self.mdr.get();

            // Check write permissions
            let addr_cell = EmulatorCell::new(addr as u16);
            let area = area_from_address(&addr_cell);
            if !area.can_write(&self.priv_level()) {
                self.exception = Some(Exception::AccessControlViolation);
                return Err("Access control violation during memory write".to_string());
            }

            if addr < self.memory.len() {
                self.memory[addr].set(value);
                self.push_micro_record(
                    MicroRecord::new(
                        format!("mem[x{addr:04X}] <- MDR"),
                        trace::memory_write(),
                        Some(value),
                        self.execute_state.current_phase,
                    )
                    .with_wires(&[
                        (trace::Dp::MarToMem, addr as u16),
                        (trace::Dp::MdrToMem, value),
                    ]),
                );
                tracing::trace!("Implicit memory write: [0x{:04X}] <- 0x{:04X}", addr, value);
                if addr == MCR_ADDR {
                    // MCR[15] = 0 stops the clock: HALT. MCR is also the debugger's run flag
                    // (exactly x8000 while running), so other bits don't stay set.
                    if value & 0x8000 == 0 {
                        self.halted = true;
                        self.memory[addr].set(0);
                    } else {
                        self.memory[addr].set(0x8000);
                        // MCR[14], wait for interrupt: the clock stops until a device asks to
                        // interrupt
                        if value & 0x4000 != 0 {
                            self.asleep = true;
                        }
                    }
                }
            } else {
                return Err(format!("Memory write address out of bounds: 0x{addr:04X}"));
            }

            self.execute_state.memory_write_pending = false;
            self.execute_state.memory_read_pending = false;
            return Ok(());
        }

        // Check if there's a pending memory read (MAR was set in previous phase)
        if self.execute_state.memory_read_pending {
            let addr = self.mar.get() as usize;

            // Check read permissions
            let addr_cell = EmulatorCell::new(addr as u16);
            let area = area_from_address(&addr_cell);
            if !area.can_read(&self.priv_level()) {
                self.exception = Some(Exception::AccessControlViolation);
                return Err("Access control violation during memory read".to_string());
            }

            if addr < self.memory.len() {
                let value = self.memory[addr].get();
                self.mdr.set(value);
                self.push_micro_record(
                    MicroRecord::new(
                        format!("MDR <- mem[x{addr:04X}]"),
                        trace::memory_read(),
                        Some(value),
                        self.execute_state.current_phase,
                    )
                    .with_wires(&[
                        (trace::Dp::MarToMem, addr as u16),
                        (trace::Dp::MemToMdr, value),
                    ]),
                );
                tracing::trace!(
                    "Implicit memory read: [0x{:04X}] -> MDR = 0x{:04X}",
                    addr,
                    value
                );
            } else {
                return Err(format!("Memory read address out of bounds: 0x{addr:04X}"));
            }

            // reading KBDR takes the key: KBSR[15] clears, the interrupt enable stays
            if addr == KBDR_ADDR {
                let kbsr = self.memory[KBSR_ADDR].get();
                self.memory[KBSR_ADDR].set(kbsr & 0x7FFF);
            }

            self.execute_state.memory_read_pending = false;
        }

        Ok(())
    }

    /// Execute the plan's current micro-operation.
    fn execute_micro_op(&mut self) -> Result<(), String> {
        let (phase, index) = (
            self.execute_state.current_phase,
            self.execute_state.micro_op_index,
        );
        // take the op out while it runs (a custom op borrows the emulator mutably)
        let op = std::mem::replace(
            &mut self.execute_state.execution_plan[phase][index],
            MicroOp::State(0),
        );
        let result = self.run_op(&op);
        if let Some(slot) = self
            .execute_state
            .execution_plan
            .get_mut(phase)
            .and_then(|p| p.get_mut(index))
        {
            *slot = op;
        }
        result
    }

    /// Run a micro-op, recording what it did on the datapath when tracing.
    fn run_op(&mut self, op: &MicroOp) -> Result<(), String> {
        if let Some(skip) = self.skip_state {
            match op {
                MicroOp::State(n) if *n == skip => return Ok(()),
                MicroOp::State(_) => self.skip_state = None,
                // the ops of a state the machine does not enter
                _ => return Ok(()),
            }
        }
        if let MicroOp::State(n) = op {
            self.cycles += 1;
            self.states_in_instruction += 1;
            self.state_no = *n;
            if states::CHECK_STATES.contains(n) {
                self.check_state = *n;
            }
            if self.trace {
                let mut record = MicroRecord::new(
                    String::new(),
                    Activity::default(),
                    None,
                    self.execute_state.current_phase,
                );
                record.marker = true;
                self.push_micro_record(record);
            }
            return Ok(());
        }
        if !self.trace {
            return self.exec_op(op);
        }
        // custom ops record what they do through the ops they run inline
        let record_it = !matches!(op, MicroOp::Message(_) | MicroOp::Custom(..))
            && !matches!(op, MicroOp::PhaseTransition(p) if *p != CycleState::Decode);
        if !record_it {
            return self.exec_op(op);
        }
        let activity = trace::activity_of(op);
        let text = match op {
            MicroOp::PhaseTransition(_) => "BEN <- IR[11]&N + IR[10]&Z + IR[9]&P".to_string(),
            _ => op.to_string(),
        };
        let (regs_read, reg_written) = trace::registers_of(op);
        let phase = self.execute_state.current_phase;
        // what flows into the op, read before it changes anything
        let mut wires = self.wire_inputs(op);
        let carried = match op {
            MicroOp::Transfer { source, .. } => self.get_source_value(source).ok().map(|c| c.get()),
            _ => None,
        };

        let result = self.exec_op(op);

        if !activity.is_empty() {
            let value = match op {
                MicroOp::Transfer { destination, .. } => Some(self.destination_value(destination)),
                MicroOp::Alu { .. } => Some(self.alu.alu_out.get()),
                MicroOp::Address { .. } => Some(self.addr_out.get()),
                MicroOp::SetFlag(MachineFlag::UpdateCondCodes(r)) => {
                    Some(self.r[*r as usize & 7].get())
                }
                _ => None,
            };
            // every other wire the op used carries what it moved or produced
            if let MicroOp::SetFlag(MachineFlag::UpdateCondCodes(_)) = op {
                let (n, z, p) = self.get_nzp();
                let nzp = (n as u16) << 2 | (z as u16) << 1 | p as u16;
                wires.extend([
                    (trace::Dp::Logic, nzp),
                    (trace::Dp::NzpMux, nzp),
                    (trace::Dp::Nzp, nzp),
                ]);
            }
            if let Some(v) = carried.or(value) {
                for dp in trace::Dp::ALL {
                    if activity.has(dp) && !wires.iter().any(|(d, _)| *d == dp) {
                        wires.push((dp, v));
                    }
                }
            }
            let mut record = MicroRecord::new(text, activity, value, phase);
            record.regs_read = regs_read;
            record.reg_written = reg_written;
            record.wires = wires;
            self.push_micro_record(record);
        }
        result
    }

    /// Record state 18 seeing INT, so the datapath shows the interrupt request.
    pub(crate) fn note_int(&mut self) {
        let mut a = Activity::default();
        a.add(&[trace::Dp::IntPriorityIn, trace::Dp::IntSignal], &[]);
        self.push_micro_record(
            MicroRecord::new(
                "[INT] = 1".to_owned(),
                a,
                None,
                self.execute_state.current_phase,
            )
            .with_wires(&[
                (trace::Dp::IntPriorityIn, super::KEYBOARD_PRIORITY),
                (trace::Dp::IntSignal, 1),
            ]),
        );
    }

    /// The values on the wires into a micro-op that differ from what it moves: an operation's
    /// inputs (the ALU's and the address adder's), and the stack pointer and PC before the
    /// incrementer or decrementer.
    fn wire_inputs(&self, op: &MicroOp) -> Vec<(trace::Dp, u16)> {
        use trace::Dp;
        let val = |s: &DataSource| self.get_source_value(s).map(|c| c.get()).unwrap_or(0);
        let r6 = self.r[6].get();
        let pc = self.pc.get();
        match op {
            MicroOp::Transfer { source, .. } => match source {
                DataSource::PcMinusOne => vec![(Dp::PcDec, pc)],
                DataSource::SpMinusOne | DataSource::SpPlusOne | DataSource::Register(_) => {
                    vec![(Dp::Sr1ToSp, r6), (Dp::UspIn, r6), (Dp::SspIn, r6)]
                }
                DataSource::TrapVector(_) | DataSource::Constant(_) => {
                    vec![(Dp::Zext, self.ir.get() & 0xFF)]
                }
                _ => vec![],
            },
            MicroOp::Alu {
                operand1, operand2, ..
            } => {
                let (a, b) = (val(operand1), val(operand2));
                let mut w = vec![(Dp::Sr1Out, a), (Dp::Sr2Mux, b)];
                match operand2 {
                    DataSource::Register(_) => w.push((Dp::Sr2Out, b)),
                    _ => w.push((Dp::Sext5, b)),
                }
                w
            }
            MicroOp::Address { base, offset, bits } => {
                let b = val(base);
                let off = *offset as u16;
                let mut w = vec![
                    (Dp::Addr1Mux, b),
                    (Dp::Addr2Mux, if *bits == 0 { 0 } else { off }),
                ];
                w.push((
                    if matches!(base, DataSource::PC) {
                        Dp::PcToAddr1
                    } else {
                        Dp::RegToAddr1
                    },
                    b,
                ));
                w.push(match bits {
                    11 => (Dp::Sext11, off),
                    9 => (Dp::Sext9, off),
                    6 => (Dp::Sext6, off),
                    _ => (Dp::ZeroToAddr2, 0),
                });
                w
            }
            MicroOp::SetFlag(MachineFlag::UpdateCondCodes(r)) => {
                vec![(Dp::BusToLogic, self.r[*r as usize & 7].get())]
            }
            MicroOp::PhaseTransition(_) => vec![(Dp::IrToControl, self.ir.get())],
            _ => vec![],
        }
    }

    /// The registers the datapath views show, as they are now.
    pub fn snapshot(&self) -> trace::Snapshot {
        let (n, z, p) = self.get_nzp();
        trace::Snapshot {
            pc: self.pc.get(),
            ir: self.ir.get(),
            mar: self.mar.get(),
            mdr: self.mdr.get(),
            r: std::array::from_fn(|i| self.r[i].get()),
            nzp: (n as u8) << 2 | (z as u8) << 1 | p as u8,
        }
    }

    fn push_micro_record(&mut self, mut record: MicroRecord) {
        if !self.trace {
            return;
        }
        record.state = self.state_no;
        record.snap = self.snapshot();
        if self.micro_log.len() < 64 {
            self.micro_log.push(record);
        }
    }

    fn destination_value(&self, d: &DataDestination) -> u16 {
        match d {
            DataDestination::Register(n) => self.r[*n as usize & 7].get(),
            DataDestination::PC => self.pc.get(),
            DataDestination::IR => self.ir.get(),
            DataDestination::MAR => self.mar.get(),
            DataDestination::MDR => self.mdr.get(),
            DataDestination::PSR => self.memory[PSR_ADDR].get(),
            DataDestination::AluOut => self.alu.alu_out.get(),
            DataDestination::Temp => self.execute_state.temp_register.get(),
            DataDestination::Priv => self.memory[PSR_ADDR].get() >> 15,
            DataDestination::SavedSsp => self.saved_ssp.get(),
            DataDestination::SavedUsp => self.saved_usp.get(),
            DataDestination::Table => self.table.get(),
            DataDestination::Vector => self.vector.get(),
            DataDestination::Priority => self.memory[PSR_ADDR].get() >> 8 & 7,
        }
    }

    fn exec_op(&mut self, op: &MicroOp) -> Result<(), String> {
        let span = tracing::trace_span!("execute_micro_op");
        let _enter = span.enter();

        match op {
            MicroOp::Transfer {
                source,
                destination,
            } => {
                let value = self.get_source_value(source)?;
                tracing::trace!(
                    "Transfer: {} -> {} (value: 0x{:04X})",
                    source,
                    destination,
                    value.get()
                );
                self.set_destination_value(&destination.clone(), value.get())?;
            }

            MicroOp::Alu {
                operation,
                operand1,
                operand2,
            } => {
                let val1 = self.get_source_value(operand1)?;
                let val2 = self.get_source_value(operand2)?;
                self.alu.op = Some(match operation {
                    MAluOp::Add => AluOp::Add(val1, val2),
                    MAluOp::And => AluOp::And(val1, val2),
                    MAluOp::Not => AluOp::Not(val1),
                });
                if let Some(alu_op) = self.alu.op.take() {
                    self.alu.alu_out = alu_op.execute();
                }
            }

            MicroOp::Address { base, offset, .. } => {
                let b = self.get_source_value(base)?;
                self.addr_out.set(b.get().wrapping_add(*offset as u16));
            }

            MicroOp::PhaseTransition(phase) => {
                tracing::trace!("Phase transition: {}", phase);
                self.handle_implicit_memory_operations()?;
            }

            MicroOp::SetFlag(flag) => {
                match flag {
                    MachineFlag::UpdateCondCodes(reg_num) => {
                        tracing::trace!("Updated condition codes for R{}", reg_num);
                        self.update_flags(*reg_num as usize);
                    }
                    // the write happens at the end of the state (with its ACV check)
                    MachineFlag::WriteMemory => self.execute_state.memory_write_pending = true,
                }
            }

            MicroOp::Message(msg) => {
                tracing::trace!("Message: {}", msg);
            }

            MicroOp::State(_) => {}

            MicroOp::Custom(f, _) => match f.clone()(self) {
                Ok(()) => (),
                Err(err) => {
                    self.exception = Some(err.clone());
                    return Err(format!("{err:?}"));
                }
            },
        }

        Ok(())
    }

    /// Set a value to a data destination
    fn set_destination_value(
        &mut self,
        destination: &DataDestination,
        value: u16,
    ) -> Result<(), String> {
        let span =
            tracing::trace_span!("set_destination_value", dest = %destination, value = value);
        let _enter = span.enter();

        match destination {
            DataDestination::Register(reg_num) => {
                if *reg_num > 7 {
                    return Err(format!("Invalid register number: {reg_num}"));
                }
                self.r[*reg_num as usize].set(value);
                self.reg_writes[*reg_num as usize] = Some(self.register_access());
                tracing::trace!("Write R{} <- 0x{:04X}", reg_num, value);
            }

            DataDestination::PC => {
                self.pc.set(value);
                tracing::trace!("Write PC <- 0x{:04X}", value);
            }

            DataDestination::IR => {
                self.ir.set(value);
                tracing::trace!("Write IR <- 0x{:04X}", value);
            }

            DataDestination::MAR => {
                self.mar.set(value);
                // Setting MAR triggers a memory read that will happen between phases
                self.execute_state.memory_read_pending = true;
                tracing::trace!("Write MAR <- 0x{:04X} (memory read pending)", value);
            }

            DataDestination::MDR => {
                self.mdr.set(value);
                tracing::trace!("Write MDR <- 0x{:04X}", value);
            }

            DataDestination::PSR => {
                self.memory[PSR_ADDR].set(value);
                tracing::trace!("Write PSR <- 0x{:04X}", value);
            }

            DataDestination::AluOut => {
                self.alu.alu_out.set(value);
                tracing::trace!("Write ALU_OUT <- 0x{:04X}", value);
            }

            DataDestination::Temp => {
                self.execute_state.temp_register.set(value);
                tracing::trace!("Write TEMP <- 0x{:04X}", value);
            }

            DataDestination::Priv => {
                let psr = self.memory[PSR_ADDR].get() & 0x7FFF;
                self.memory[PSR_ADDR].set(psr | (value & 1) << 15);
            }
            DataDestination::SavedSsp => self.saved_ssp.set(value),
            DataDestination::SavedUsp => self.saved_usp.set(value),
            DataDestination::Table => self.table.set(value & 0xFF),
            DataDestination::Vector => self.vector.set(value & 0xFF),
            DataDestination::Priority => {
                let psr = self.memory[PSR_ADDR].get() & !0x0700;
                self.memory[PSR_ADDR].set(psr | (value & 7) << 8);
            }
        }

        Ok(())
    }
    /// Get the value from a data source
    fn get_source_value(&self, source: &DataSource) -> Result<EmulatorCell, String> {
        let span = tracing::trace_span!("get_source_value", source = %source);
        let _enter = span.enter();

        let value = match source {
            DataSource::Register(reg_num) => {
                if *reg_num > 7 {
                    return Err(format!("Invalid register number: {reg_num}"));
                }
                let value = self.r[*reg_num as usize].get();
                self.reg_reads[*reg_num as usize].set(Some(self.register_access()));
                tracing::trace!("Read R{} = 0x{:04X}", reg_num, value);
                value
            }

            DataSource::PC => {
                let value = self.pc.get();
                tracing::trace!("Read PC = 0x{:04X}", value);
                value
            }

            DataSource::IR => {
                let value = self.ir.get();
                tracing::trace!("Read IR = 0x{:04X}", value);
                value
            }

            DataSource::MAR => {
                let value = self.mar.get();
                tracing::trace!("Read MAR = 0x{:04X}", value);
                value
            }

            DataSource::MDR => {
                let value = self.mdr.get();
                tracing::trace!("Read MDR = 0x{:04X}", value);
                value
            }

            DataSource::PSR => {
                let value = self.memory[PSR_ADDR].get();
                tracing::trace!("Read PSR = 0x{:04X}", value);
                value
            }

            DataSource::AluOut => {
                let value = self.alu.alu_out.get();
                tracing::trace!("Read ALU_OUT = 0x{:04X}", value);
                value
            }

            DataSource::Temp => {
                let value = self.execute_state.temp_register.get();
                tracing::trace!("Read TEMP = 0x{:04X}", value);
                value
            }

            DataSource::AddrOut => self.addr_out.get(),

            DataSource::PcPlusOne => self.pc.get().wrapping_add(1),
            DataSource::PcMinusOne => self.pc.get().wrapping_sub(1),
            DataSource::SpPlusOne => self.r[6].get().wrapping_add(1),
            DataSource::SpMinusOne => self.r[6].get().wrapping_sub(1),
            DataSource::SavedSsp => self.saved_ssp.get(),
            DataSource::SavedUsp => self.saved_usp.get(),
            DataSource::TableVector => (self.table.get() & 0xFF) << 8 | (self.vector.get() & 0xFF),
            DataSource::IrLow8 => self.ir.get() & 0xFF,
            DataSource::Const8(v) => *v as u16,
            DataSource::Zero => 0,
            DataSource::Intv => super::KEYBOARD_INTV,
            DataSource::IntPriority => super::KEYBOARD_PRIORITY,

            DataSource::Immediate(imm) => {
                let value = *imm as u16;
                tracing::trace!("Immediate value: 0x{:04X}", value);
                value
            }

            DataSource::PCOffset(offset) => {
                let value = *offset as u16;
                tracing::trace!("PC offset: 0x{:04X}", value);
                value
            }

            DataSource::TrapVector(vector) => {
                let value = *vector as u16;
                tracing::trace!("Trap vector: 0x{:02X}", vector);
                value
            }

            DataSource::Constant(constant) => {
                tracing::trace!("Constant: 0x{:04X}", constant);
                *constant
            }
        };

        Ok(EmulatorCell::new(value))
    }
}
#[cfg(test)]
mod tests {
    use tracing_test::traced_test;

    use super::*;

    use crate::micro_op;

    /// Run a hand-made plan to the end, one state at a time.
    fn run_plan(emulator: &mut Emulator) {
        emulator.fetched = true;
        while emulator.fetched {
            emulator.state_step().unwrap();
        }
    }

    fn create_test_emulator() -> Emulator {
        let mut emulator = Emulator::new();
        // Set up some initial state for testing
        emulator.r[1].set(0x1000);
        emulator.r[2].set(0x2000);
        emulator.pc.set(0x3000);
        emulator
    }

    #[test]
    fn test_simple_transfer() {
        let mut emulator = create_test_emulator();

        let plan = vec![vec![micro_op!(-> Execute), micro_op!(R(0) <- R(1))]];

        let phase_state = CpuPhaseState::new(plan);
        emulator.execute_state = phase_state;

        // Step through all micro-ops
        run_plan(&mut emulator);

        assert_eq!(emulator.r[0].get(), 0x1000);
    }

    #[test]
    fn test_alu_operations() {
        let mut emulator = create_test_emulator();
        emulator.r[1].set(5);
        emulator.r[2].set(3);

        let plan = vec![vec![
            micro_op!(-> Execute),
            micro_op!(ALU_OUT <- R(1) + R(2)),
            micro_op!(R(0) <- AluOut),
        ]];

        let phase_state = CpuPhaseState::new(plan);
        emulator.execute_state = phase_state;
        run_plan(&mut emulator);

        assert_eq!(emulator.r[0].get(), 8);
        assert_eq!(emulator.alu.alu_out.get(), 8);
    }

    #[test]
    fn test_alu_and_operation() {
        let mut emulator = create_test_emulator();
        emulator.r[1].set(0b1100);
        emulator.r[2].set(0b1010);

        let plan = vec![vec![
            micro_op!(-> Execute),
            micro_op!(ALU_OUT <- R(1) & R(2)),
            micro_op!(R(0) <- AluOut),
        ]];

        let phase_state = CpuPhaseState::new(plan);
        emulator.execute_state = phase_state;
        run_plan(&mut emulator);

        assert_eq!(emulator.r[0].get(), 0b1000);
    }

    #[test]
    fn test_alu_not_operation() {
        let mut emulator = create_test_emulator();
        emulator.r[1].set(0b0000_1111_0000_1111);

        let plan = vec![vec![
            micro_op!(-> Execute),
            micro_op!(ALU_OUT <- NOT R(1)),
            micro_op!(R(0) <- AluOut),
        ]];

        let phase_state = CpuPhaseState::new(plan);
        emulator.execute_state = phase_state;
        run_plan(&mut emulator);

        assert_eq!(emulator.r[0].get(), 0b1111_0000_1111_0000);
    }

    #[test]
    fn test_immediate_values() {
        let mut emulator = create_test_emulator();
        emulator.r[1].set(10);

        let plan = vec![vec![
            micro_op!(-> Execute),
            micro_op!(ALU_OUT <- R(1) + IMM(5)),
            micro_op!(R(0) <- AluOut),
        ]];

        let phase_state = CpuPhaseState::new(plan);
        emulator.execute_state = phase_state;
        run_plan(&mut emulator);

        assert_eq!(emulator.r[0].get(), 15);
    }

    #[test]
    fn test_pc_operations() {
        let mut emulator = create_test_emulator();

        let plan = vec![vec![
            micro_op!(-> Execute),
            micro_op!(ALU_OUT <- PC + C(1)),
            micro_op!(PC <- AluOut),
        ]];

        let phase_state = CpuPhaseState::new(plan);
        emulator.execute_state = phase_state;
        run_plan(&mut emulator);

        assert_eq!(emulator.pc.get(), 0x3001);
    }

    #[traced_test]
    #[test]
    fn test_memory_operations() {
        let mut emulator = create_test_emulator();
        emulator.memory[0x4000].set(0xABCD);
        emulator.mar.set(0x4000);

        let plan = vec![
            vec![micro_op!(-> Execute)], // implicit load
            vec![micro_op!(-> Execute), micro_op!(R(0) <- MDR)],
        ];

        let mut phase_state = CpuPhaseState::new(plan);
        phase_state.memory_read_pending = true; // simulate modifying mar
        emulator.execute_state = phase_state;
        run_plan(&mut emulator);

        assert_eq!(emulator.r[0].get(), 0xABCD);
    }

    #[test]
    fn test_condition_codes() {
        let mut emulator = create_test_emulator();
        emulator.r[1].set(0x8000); // Negative value

        let plan = vec![vec![
            micro_op!(-> Execute),
            micro_op!(R(0) <- R(1)),
            micro_op!(SET_CC(0)),
        ]];

        let phase_state = CpuPhaseState::new(plan);
        emulator.execute_state = phase_state;
        run_plan(&mut emulator);

        let (n, z, p) = emulator.get_nzp();
        assert!(n, "Negative flag should be set");
        assert!(!z, "Zero flag should not be set");
        assert!(!p, "Positive flag should not be set");
    }

    #[test]
    fn test_memory_write() {
        let mut emulator = create_test_emulator();
        emulator.mar.set(0x4000);
        emulator.mdr.set(0xBEEF);

        let plan = vec![vec![
            micro_op!(-> Execute),
            micro_op!(SET_FLAG(WriteMemory)),
            micro_op!(-> Execute), // write
        ]];

        let phase_state = CpuPhaseState::new(plan);
        emulator.execute_state = phase_state;
        run_plan(&mut emulator);

        assert_eq!(emulator.memory[0x4000].get(), 0xBEEF);
    }

    #[test]
    fn state_markers_split_a_plan_into_clock_cycles() {
        let mut emulator = create_test_emulator();
        emulator.r[1].set(42);

        let plan = vec![vec![
            micro_op!(-> Execute),
            MicroOp::State(1),
            micro_op!(R(0) <- R(1)),
            MicroOp::State(2),
            micro_op!(ALU_OUT <- R(0) + IMM(1)),
            MicroOp::State(3),
            micro_op!(R(2) <- AluOut),
        ]];
        emulator.execute_state = CpuPhaseState::new(plan);
        emulator.fetched = true;

        emulator.state_step().unwrap();
        assert_eq!((emulator.state_no, emulator.r[0].get()), (1, 42));
        emulator.state_step().unwrap();
        assert_eq!((emulator.state_no, emulator.alu.alu_out.get()), (2, 43));
        assert_eq!(emulator.r[2].get(), 0x2000, "state 3 hasn't run yet");
        emulator.state_step().unwrap();
        assert_eq!((emulator.state_no, emulator.r[2].get()), (3, 43));
        assert!(!emulator.fetched, "the plan is done after its last state");
    }

    #[test]
    #[traced_test]
    fn test_complex_add_instruction() {
        let mut emulator = create_test_emulator();
        emulator.r[1].set(10);
        emulator.r[2].set(5);

        // Simulate a complete ADD R0, R1, R2 instruction
        let plan = vec![
            vec![
                micro_op!(-> Fetch),
                micro_op!(MAR <- PC),
                micro_op!(ALU_OUT <- PC + C(1)),
                micro_op!(PC <- AluOut),
            ],
            vec![
                micro_op!(-> Decode),
                micro_op!(IR <- MDR),
                micro_op!(MSG "ADD instruction decoded"),
            ],
            vec![
                micro_op!(-> EvaluateAddress),
                micro_op!(MSG "No address evaluation needed for ADD"),
            ],
            vec![
                micro_op!(-> FetchOperands),
                micro_op!(MSG "Operands already in registers"),
            ],
            vec![micro_op!(-> Execute), micro_op!(ALU_OUT <- R(1) + R(2))],
            vec![
                micro_op!(-> StoreResult),
                micro_op!(R(0) <- AluOut),
                micro_op!(SET_CC(0)),
            ],
        ];

        let phase_state = CpuPhaseState::new(plan);
        emulator.execute_state = phase_state;
        run_plan(&mut emulator);

        assert_eq!(emulator.r[0].get(), 15);
        assert_eq!(emulator.pc.get(), 0x3001);

        let (n, z, p) = emulator.get_nzp();
        assert!(p, "Positive flag should be set");
        assert!(!z && !n, "Zero and negative flags should not be set");
    }
}
