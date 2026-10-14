use crate::emulator::micro_op::{CycleState, MicroOp, MicroOpGenerator};
use crate::emulator::{BitAddressable, Emulator, EmulatorCell};
use std::collections::HashMap;

use super::Op;

#[derive(Debug, Clone)]

pub struct BrOp {
    /// Do we match on negitive?
    pub n_bit: EmulatorCell,
    /// Do we match on zero?
    pub z_bit: EmulatorCell,
    /// Do we match on positive?
    pub p_bit: EmulatorCell,
    /// Offset from pc to take if we match
    pub pc_offset: EmulatorCell,
}

impl MicroOpGenerator for BrOp {
    fn generate_plan(&self) -> HashMap<CycleState, Vec<MicroOp>> {
        let mut plan = HashMap::new();
        // Decode (state 32) computes BEN from IR[11:9] and the condition codes. State 0 tests it,
        // and only a taken branch goes to state 22, which adds PCoffset9 to the PC.
        let n_cond = self.n_bit.get() == 1;
        let z_cond = self.z_bit.get() == 1;
        let p_cond = self.p_bit.get() == 1;
        let offset = self.pc_offset.get() as i16;

        let mut if_cond = vec![];
        if n_cond {
            if_cond.push("N");
        }
        if z_cond {
            if_cond.push("Z");
        }
        if p_cond {
            if_cond.push("P");
        }
        let pseudo_str = if if_cond.is_empty() {
            "[BEN] is always 0: BR with no conditions never branches".to_string()
        } else {
            format!("[BEN] ({}): state 22 only if set", if_cond.join(" or "))
        };

        plan.insert(
            CycleState::Execute,
            vec![
                MicroOp::State(0),
                MicroOp::new_custom(
                    move |emu: &mut Emulator| {
                        let (psr_n, psr_z, psr_p) = emu.get_nzp();
                        let ben = (n_cond && psr_n) || (z_cond && psr_z) || (p_cond && psr_p);
                        if !ben {
                            emu.skip_state = Some(22);
                        }
                        Ok(())
                    },
                    pseudo_str,
                ),
                MicroOp::State(22),
                MicroOp::addr_pc(offset, 9),
                micro_op!(PC <- AddrOut),
            ],
        );

        plan
    }
}

impl Op for BrOp {
    fn decode(ir: EmulatorCell) -> Self {
        // LAYOUT: 0000 | N | Z | P | PCoffset9

        // Extract and sign-extend PCoffset9
        let pc_offset = ir.range(8..0).sext(8);

        Self {
            n_bit: ir.index(11),
            z_bit: ir.index(10),
            p_bit: ir.index(9),
            pc_offset,
        }
    }
}

impl std::fmt::Display for BrOp {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut op_name = "BR".to_string();
        if self.n_bit.get() == 1 {
            op_name.push('N');
        }
        if self.z_bit.get() == 1 {
            op_name.push('Z');
        }
        if self.p_bit.get() == 1 {
            op_name.push('P');
        }

        // Format offset as signed decimal
        let offset_val = self.pc_offset.get() as i16; // Cast to signed for proper display

        write!(
            f,
            "{} #{} (x{:03X})",
            op_name,
            offset_val,
            self.pc_offset.get() & 0x1FF
        )?;
        Ok(())
    }
}
