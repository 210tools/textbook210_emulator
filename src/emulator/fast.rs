//! The fast path: whole instructions executed directly with a `match` on the opcode, for
//! running at high speed when nobody is watching the states. It leaves the machine exactly as
//! the state machine would (registers, memory, MAR, MDR, the saved stack pointers, Table and
//! Vector, the cycle and instruction counts), but records no micro-ops. Anything unusual (an
//! interrupt, an exception, a reserved opcode) is handed to the state machine for that one
//! instruction.

use super::{CpuState, Emulator, RegisterAccess, KBDR_ADDR, KBSR_ADDR, MCR_ADDR, PSR_ADDR};

/// How many clock cycles (states) each instruction takes, fetch (18, 33, 28, 30) and decode
/// (32) included. Only BR (taken or not), TRAP (from User mode or not) and RTI vary.
const FETCH_DECODE: u64 = 5;

#[inline(always)]
fn sext(v: u16, bits: u32) -> u16 {
    let shift = 16 - bits;
    (((v << shift) as i16) >> shift) as u16
}

/// What a User mode program may read: the trap vector table and user space.
#[inline(always)]
fn user_can_read(a: u16) -> bool {
    a < 0x0100 || (0x3000..0xFE00).contains(&a)
}

/// What a User mode program may write: user space.
#[inline(always)]
fn user_can_write(a: u16) -> bool {
    (0x3000..0xFE00).contains(&a)
}

impl Emulator {
    /// Can the fast path take over: running, at an instruction boundary, with no step command
    /// (step over, step out) to watch for?
    pub fn can_run_fast(&self) -> bool {
        self.running()
            && !self.stepping
            && !self.fetched
            && matches!(self.cpu_state, CpuState::Fetch)
            && self.exception.is_none()
            && self.run_target.is_none()
            && self.step_out_depth.is_none()
            && !self.stop_at_next_instruction
    }

    /// Run whole instructions until `max_cycles` clock cycles have passed, the deadline (if
    /// any) is reached, or the machine stops (HALT, a breakpoint, or asleep with nothing to wake
    /// it). Returns whether anything ran.
    pub fn run_fast(&mut self, max_cycles: u64, deadline: Option<web_time::Instant>) -> bool {
        let start = self.cycles;
        let mut ran = false;
        // the datapath's last instruction would be stale
        if !self.micro_log.is_empty() {
            self.micro_log.clear();
            self.micro_log_id = self.micro_log_id.wrapping_add(1);
        }
        let mut n: u32 = 0;
        while self.cycles - start < max_cycles && self.can_run_fast() {
            n = n.wrapping_add(1);
            if n % 4096 == 0 && deadline.is_some_and(|d| web_time::Instant::now() >= d) {
                break;
            }
            // in the order the state machine does them: devices while asleep, INT at state 18,
            // then the devices at the start of the instruction's states
            if self.asleep {
                self.update_devices();
                if !self.interrupt_requested() {
                    break;
                }
                self.asleep = false;
            }
            let pc = self.pc.get();
            if !self.breakpoints.is_empty()
                && self.breakpoints.contains(&(pc as usize))
                && !self.halted_on_breakpoint
            {
                self.stop_running();
                self.halted_on_breakpoint = true;
                self.stop_reason = super::StopReason::Breakpoint(pc);
                break;
            }
            self.halted_on_breakpoint = false;
            ran = true;
            if self.interrupt_pending() {
                self.step();
                continue;
            }
            self.update_devices();
            if !self.execute_fast() {
                // the state machine handles interrupts and exceptions
                self.step();
            }
        }
        ran
    }

    #[inline(always)]
    fn reg(&self, n: u16) -> u16 {
        self.r[(n & 7) as usize].get()
    }

    #[inline(always)]
    fn set_reg_cc(&mut self, n: u16, v: u16, access: RegisterAccess) {
        self.set_reg(n, v, access);
        let cc = if v == 0 {
            0b010
        } else if v & 0x8000 != 0 {
            0b100
        } else {
            0b001
        };
        let psr = self.memory[PSR_ADDR].get();
        self.memory[PSR_ADDR].set(psr & !7 | cc);
    }

    #[inline(always)]
    fn set_reg(&mut self, n: u16, v: u16, access: RegisterAccess) {
        self.r[(n & 7) as usize].set(v);
        self.reg_writes[(n & 7) as usize] = Some(access);
    }

    #[inline(always)]
    fn note_read(&self, n: u16, access: RegisterAccess) {
        self.reg_reads[(n & 7) as usize].set(Some(access));
    }

    /// A memory read (MDR <- M[MAR]): reading KBDR takes the key.
    #[inline(always)]
    fn read_mem(&mut self, a: u16) -> u16 {
        self.mar.set(a);
        let v = self.memory[a as usize].get();
        self.mdr.set(v);
        if a as usize == KBDR_ADDR {
            let kbsr = self.memory[KBSR_ADDR].get();
            self.memory[KBSR_ADDR].set(kbsr & 0x7FFF);
            // the next state's devices update delivers the next key straight away
            self.update_devices();
        }
        v
    }

    /// A memory write (M[MAR] <- MDR): MCR[15] = 0 halts, MCR[14] sleeps.
    #[inline(always)]
    fn write_mem(&mut self, a: u16, v: u16) {
        self.mar.set(a);
        self.mdr.set(v);
        self.memory[a as usize].set(v);
        if a as usize == MCR_ADDR {
            if v & 0x8000 == 0 {
                self.halted = true;
                self.memory[MCR_ADDR].set(0);
            } else {
                self.memory[MCR_ADDR].set(0x8000);
                if v & 0x4000 != 0 {
                    self.asleep = true;
                }
            }
        }
    }

    /// Execute the instruction at PC directly. Returns false, having changed nothing, when it
    /// would raise an exception: the state machine runs it instead.
    fn execute_fast(&mut self) -> bool {
        let pc = self.pc.get();
        let psr = self.memory[PSR_ADDR].get();
        let user = psr & 0x8000 != 0;
        if user && !user_can_read(pc) {
            return false;
        }
        let ir = self.memory[pc as usize].get();
        let op = ir >> 12;
        let dr = ir >> 9 & 7;
        let sr1 = ir >> 6 & 7;
        let pc1 = pc.wrapping_add(1);
        let off9 = pc1.wrapping_add(sext(ir & 0x1FF, 9));
        let access = RegisterAccess {
            addr: pc as usize,
            instruction: self.instructions,
        };
        let readable = |a: u16| !user || user_can_read(a);
        let writable = |a: u16| !user || user_can_write(a);

        // exceptions are the state machine's: check before changing anything
        let ok = match op {
            0x2 => readable(off9),
            0x6 => readable(self.reg(sr1).wrapping_add(sext(ir & 0x3F, 6))),
            0xA => readable(off9) && readable(self.memory[off9 as usize].get()),
            0x3 => writable(off9),
            0x7 => writable(self.reg(sr1).wrapping_add(sext(ir & 0x3F, 6))),
            0xB => readable(off9) && writable(self.memory[off9 as usize].get()),
            0x8 => !user,
            0xD => false,
            _ => true,
        };
        if !ok {
            return false;
        }

        // fetch: MAR <- PC, PC <- PC + 1, MDR <- M[MAR], IR <- MDR
        self.currently_executing = pc as usize;
        self.took_interrupt = false;
        self.mar.set(pc);
        self.mdr.set(ir);
        self.ir.set(ir);
        self.pc.set(pc1);
        let states: u64 = match op {
            // ADD, AND
            0x1 | 0x5 => {
                self.note_read(sr1, access);
                let b = if ir & 0x20 != 0 {
                    sext(ir & 0x1F, 5)
                } else {
                    self.note_read(ir, access);
                    self.reg(ir)
                };
                let a = self.reg(sr1);
                let v = if op == 0x1 { a.wrapping_add(b) } else { a & b };
                self.set_reg_cc(dr, v, access);
                1
            }
            // NOT
            0x9 => {
                self.note_read(sr1, access);
                let v = !self.reg(sr1);
                self.set_reg_cc(dr, v, access);
                1
            }
            // BR
            0x0 => {
                if dr & (psr & 7) != 0 {
                    self.pc.set(off9);
                    2
                } else {
                    1
                }
            }
            // JMP (RET)
            0xC => {
                self.note_read(sr1, access);
                self.pc.set(self.reg(sr1));
                1
            }
            // JSR, JSRR
            0x4 => {
                let target = if ir & 0x800 != 0 {
                    pc1.wrapping_add(sext(ir & 0x7FF, 11))
                } else {
                    self.note_read(sr1, access);
                    self.reg(sr1)
                };
                self.set_reg(7, pc1, access);
                self.pc.set(target);
                2
            }
            // LD
            0x2 => {
                let v = self.read_mem(off9);
                self.set_reg_cc(dr, v, access);
                4
            }
            // LDR
            0x6 => {
                self.note_read(sr1, access);
                let a = self.reg(sr1).wrapping_add(sext(ir & 0x3F, 6));
                let v = self.read_mem(a);
                self.set_reg_cc(dr, v, access);
                4
            }
            // LDI
            0xA => {
                let a = self.read_mem(off9);
                let v = self.read_mem(a);
                self.set_reg_cc(dr, v, access);
                7
            }
            // LEA
            0xE => {
                self.set_reg(dr, off9, access);
                1
            }
            // ST
            0x3 => {
                self.note_read(dr, access);
                let v = self.reg(dr);
                self.write_mem(off9, v);
                3
            }
            // STR
            0x7 => {
                self.note_read(dr, access);
                self.note_read(sr1, access);
                let a = self.reg(sr1).wrapping_add(sext(ir & 0x3F, 6));
                let v = self.reg(dr);
                self.write_mem(a, v);
                3
            }
            // STI
            0xB => {
                self.note_read(dr, access);
                let a = self.read_mem(off9);
                let v = self.reg(dr);
                self.write_mem(a, v);
                6
            }
            // TRAP: to Supervisor mode and stack, push PSR and PC, PC <- M[x00'trapvect8]
            0xF => {
                self.table.set(0x00);
                self.vector.set(ir & 0xFF);
                self.memory[PSR_ADDR].set(psr & 0x7FFF);
                if user {
                    self.saved_usp.set(self.r[6].get());
                    self.r[6].set(self.saved_ssp.get());
                }
                let sp = self.r[6].get().wrapping_sub(1);
                self.r[6].set(sp);
                self.write_mem(sp, psr);
                let sp = sp.wrapping_sub(1);
                self.r[6].set(sp);
                self.write_mem(sp, pc1);
                let target = self.read_mem(ir & 0xFF);
                self.pc.set(target);
                if user {
                    11
                } else {
                    10
                }
            }
            // RTI: pop PC and PSR, back to the user stack if returning to User mode
            0x8 => {
                let sp = self.r[6].get();
                let new_pc = self.read_mem(sp);
                self.pc.set(new_pc);
                let sp = sp.wrapping_add(1);
                self.r[6].set(sp);
                let new_psr = self.read_mem(sp);
                self.memory[PSR_ADDR].set(new_psr);
                self.r[6].set(sp.wrapping_add(1));
                if new_psr & 0x8000 != 0 {
                    self.saved_ssp.set(self.r[6].get());
                    self.r[6].set(self.saved_usp.get());
                }
                8
            }
            _ => unreachable!("reserved opcode handled above"),
        };
        self.cycles += FETCH_DECODE + states;
        self.instructions += 1;
        true
    }
}
