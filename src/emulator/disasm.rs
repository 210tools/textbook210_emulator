//! A small disassembler that turns machine words back into readable assembly, using labels from
//! the last assembled program for PC-relative targets.

use std::collections::HashMap;

fn sext(v: u16, bits: u32) -> i32 {
    let shift = 16 - bits;
    (((v << shift) as i16) >> shift) as i32
}

fn reg(v: u16, lo: u32) -> String {
    format!("R{}", (v >> lo) & 7)
}

fn target(addr: usize, word: u16, bits: u32, labels: &HashMap<usize, String>) -> String {
    let t = (addr as i32 + 1 + sext(word & ((1 << bits) - 1), bits)) as u16 as usize;
    match labels.get(&t) {
        Some(l) => l.clone(),
        None => format!("x{t:04X}"),
    }
}

/// The address a PC-relative instruction refers to, if any.
pub fn pc_relative_target(addr: usize, word: u16) -> Option<usize> {
    let bits = match word >> 12 {
        0x0 | 0x2 | 0x3 | 0xA | 0xB | 0xE => 9,
        0x4 if word & 0x0800 != 0 => 11,
        _ => return None,
    };
    Some((addr as i32 + 1 + sext(word & ((1 << bits) - 1), bits)) as u16 as usize)
}

/// Disassemble `word` as if it were at `addr`.
pub fn disassemble(word: u16, addr: usize, labels: &HashMap<usize, String>) -> String {
    let op = word >> 12;
    match op {
        0x1 | 0x5 => {
            let name = if op == 0x1 { "ADD" } else { "AND" };
            if word & 0x20 != 0 {
                format!(
                    "{name} {}, {}, #{}",
                    reg(word, 9),
                    reg(word, 6),
                    sext(word & 0x1F, 5)
                )
            } else {
                format!(
                    "{name} {}, {}, {}",
                    reg(word, 9),
                    reg(word, 6),
                    reg(word, 0)
                )
            }
        }
        0x9 => format!("NOT {}, {}", reg(word, 9), reg(word, 6)),
        0x0 => {
            let n = word & 0x0800 != 0;
            let z = word & 0x0400 != 0;
            let p = word & 0x0200 != 0;
            if !n && !z && !p {
                return "NOP".to_string();
            }
            let mut name = "BR".to_string();
            if !(n && z && p) {
                if n {
                    name.push('n');
                }
                if z {
                    name.push('z');
                }
                if p {
                    name.push('p');
                }
            }
            format!("{name} {}", target(addr, word, 9, labels))
        }
        0xC => {
            if (word >> 6) & 7 == 7 {
                "RET".to_string()
            } else {
                format!("JMP {}", reg(word, 6))
            }
        }
        0x4 => {
            if word & 0x0800 != 0 {
                format!("JSR {}", target(addr, word, 11, labels))
            } else {
                format!("JSRR {}", reg(word, 6))
            }
        }
        0x2 | 0x3 | 0xA | 0xB | 0xE => {
            let name = match op {
                0x2 => "LD",
                0x3 => "ST",
                0xA => "LDI",
                0xB => "STI",
                _ => "LEA",
            };
            format!("{name} {}, {}", reg(word, 9), target(addr, word, 9, labels))
        }
        0x6 | 0x7 => {
            let name = if op == 0x6 { "LDR" } else { "STR" };
            format!(
                "{name} {}, {}, #{}",
                reg(word, 9),
                reg(word, 6),
                sext(word & 0x3F, 6)
            )
        }
        0x8 => "RTI".to_string(),
        0xF => match word & 0xFF {
            0x20 => "GETC".to_string(),
            0x21 => "OUT".to_string(),
            0x22 => "PUTS".to_string(),
            0x23 => "IN".to_string(),
            0x24 => "PUTSP".to_string(),
            0x25 => "HALT".to_string(),
            v => format!("TRAP x{v:02X}"),
        },
        _ => format!(".FILL x{word:04X}"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn disassembles_with_labels() {
        let mut labels = HashMap::new();
        labels.insert(0x3003, "MESSAGE".to_string());
        assert_eq!(disassemble(0xE002, 0x3000, &labels), "LEA R0, MESSAGE");
        assert_eq!(disassemble(0xF022, 0x3001, &labels), "PUTS");
        assert_eq!(disassemble(0x1261, 0x3000, &labels), "ADD R1, R1, #1");
        assert_eq!(disassemble(0x127F, 0x3000, &labels), "ADD R1, R1, #-1");
        assert_eq!(disassemble(0x0BFE, 0x3002, &labels), "BRnp x3001");
        assert_eq!(
            disassemble(0x0E00, 0x3002, &labels),
            "BR x3003".replace("x3003", "MESSAGE")
        );
        assert_eq!(disassemble(0xC1C0, 0x3000, &labels), "RET");
        assert_eq!(disassemble(0x6042, 0x3000, &labels), "LDR R0, R1, #2");
        assert_eq!(disassemble(0xD000, 0x3000, &labels), ".FILL xD000");
        assert_eq!(pc_relative_target(0x3000, 0xE002), Some(0x3003));
    }

    #[test]
    fn round_trips_assembled_code() {
        let src = ".ORIG x3000\nLOOP ADD R1, R1, #-1\nBRp LOOP\nLDR R0, R6, #-3\nNOT R2, R3\nJSR SUB\nHALT\nSUB RET\n.END";
        let out = crate::emulator::Emulator::parse_program(src, None).expect("assembles");
        let labels: HashMap<usize, String> =
            out.labels.iter().map(|(k, v)| (*v, k.clone())).collect();
        let text: Vec<String> = out
            .machine_code
            .iter()
            .enumerate()
            .map(|(i, w)| disassemble(*w, 0x3000 + i, &labels))
            .collect();
        assert_eq!(
            text,
            vec![
                "ADD R1, R1, #-1",
                "BRp LOOP",
                "LDR R0, R6, #-3",
                "NOT R2, R3",
                "JSR SUB",
                "HALT",
                "RET"
            ]
        );
    }
}

/// `x0012 (18)`, or with the signed value for negative numbers.
pub fn value_text(v: u16) -> String {
    if v & 0x8000 != 0 {
        format!("x{v:04X} ({})", v as i16)
    } else {
        format!("x{v:04X} ({v})")
    }
}

/// What the instruction at `addr` is about to read, with current values, e.g.
/// `R2 = x0009 (9), R3 = x0001 (1)` or `R1 = x3503 → mem[x3503] = x0007 (7)`.
/// Used to annotate the instruction the debugger is stopped at.
pub fn instruction_inputs(e: &crate::emulator::Emulator, addr: usize) -> Option<String> {
    let word = e.memory[addr].get();
    let reg = |n: u16| -> String { format!("R{n} = {}", value_text(e.r[n as usize].get())) };
    let mem = |a: usize| -> u16 { e.memory[a & 0xFFFF].get() };
    let r = |lo: u32| (word >> lo) & 7;
    let text = match word >> 12 {
        0x1 | 0x5 => {
            // an immediate, or the same register twice: one register to show
            if word & 0x20 != 0 || r(6) == r(0) {
                reg(r(6))
            } else {
                format!("{}, {}", reg(r(6)), reg(r(0)))
            }
        }
        0x9 => reg(r(6)),
        0x0 => {
            let (n, z, p) = e.get_nzp();
            let want = (
                (word >> 11) & 1 == 1,
                (word >> 10) & 1 == 1,
                (word >> 9) & 1 == 1,
            );
            if !want.0 && !want.1 && !want.2 {
                return None;
            }
            if want.0 && want.1 && want.2 {
                return Some("always taken".to_string());
            }
            let taken = (want.0 && n) || (want.1 && z) || (want.2 && p);
            format!(
                "N={} Z={} P={} → {}",
                n as u8,
                z as u8,
                p as u8,
                if taken { "taken" } else { "not taken" }
            )
        }
        0xC => reg(r(6)),
        0x4 if word & 0x0800 == 0 => reg(r(6)),
        0x2 => {
            let t = pc_relative_target(addr, word)?;
            format!("mem[x{t:04X}] = {}", value_text(mem(t)))
        }
        0xA => {
            let t = pc_relative_target(addr, word)?;
            let p = mem(t) as usize;
            format!(
                "mem[x{t:04X}] = x{p:04X} → mem[x{p:04X}] = {}",
                value_text(mem(p))
            )
        }
        0x6 => {
            let base = e.r[r(6) as usize].get();
            let off = (((word & 0x3F) << 10) as i16 >> 10) as i32;
            let a = (base as i32 + off) as u16 as usize;
            format!("{} → mem[x{a:04X}] = {}", reg(r(6)), value_text(mem(a)))
        }
        0xE => {
            let t = pc_relative_target(addr, word)?;
            format!("address x{t:04X}")
        }
        0x3 => {
            let t = pc_relative_target(addr, word)?;
            format!("{} → mem[x{t:04X}]", reg(r(9)))
        }
        0xB => {
            let t = pc_relative_target(addr, word)?;
            let p = mem(t);
            format!("{} → mem[x{p:04X}] (pointer at x{t:04X})", reg(r(9)))
        }
        0x7 => {
            let base = e.r[r(6) as usize].get();
            let off = (((word & 0x3F) << 10) as i16 >> 10) as i32;
            let a = (base as i32 + off) as u16 as usize;
            format!("{}, {} → mem[x{a:04X}]", reg(r(9)), reg(r(6)))
        }
        0xF => match word & 0xFF {
            0x21 => {
                let c = e.r[0].get() & 0xFF;
                if (32..127).contains(&c) {
                    format!("R0 = '{}'", c as u8 as char)
                } else {
                    reg(0)
                }
            }
            0x22 => {
                let start = e.r[0].get() as usize;
                let mut s = String::new();
                for i in 0..24 {
                    let c = mem(start + i);
                    if c == 0 {
                        break;
                    }
                    match c as u8 {
                        b'\n' => s.push_str("\\n"),
                        b if (32..127).contains(&b) => s.push(b as char),
                        _ => s.push('·'),
                    }
                    if i == 23 {
                        s.push('…');
                    }
                }
                format!("R0 = x{start:04X} → \"{s}\"")
            }
            0x24 => reg(0),
            _ => return None,
        },
        _ => return None,
    };
    Some(text)
}

#[cfg(test)]
mod input_tests {
    use super::*;
    use crate::emulator::Emulator;

    #[test]
    fn describes_inputs() {
        let mut e = Emulator::new();
        e.r[2].set(9);
        e.r[3].set(1);
        e.memory[0x3000].set(0x1682); // ADD R3, R2, R2
        assert_eq!(
            instruction_inputs(&e, 0x3000).as_deref(),
            Some("R2 = x0009 (9)")
        );
        e.memory[0x3000].set(0x1683); // ADD R3, R2, R3
        assert_eq!(
            instruction_inputs(&e, 0x3000).as_deref(),
            Some("R2 = x0009 (9), R3 = x0001 (1)")
        );
        e.r[1].set(0x3500);
        e.memory[0x3501].set(7);
        e.memory[0x3000].set(0x6041); // LDR R0, R1, #1
        assert_eq!(
            instruction_inputs(&e, 0x3000).as_deref(),
            Some("R1 = x3500 (13568) → mem[x3501] = x0007 (7)")
        );
        e.set_z();
        e.memory[0x3000].set(0x0802); // BRn
        assert_eq!(
            instruction_inputs(&e, 0x3000).as_deref(),
            Some("N=0 Z=1 P=0 → not taken")
        );
    }
}
