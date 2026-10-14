//! Jargon explained where it appears. Text shown with [`text`] underlines every term the
//! glossary knows; hovering a term opens a bubble that explains it, and the jargon in that
//! explanation can be hovered in turn. [`Explain::explain`] is `on_hover_text` with this.
//!
//! The bubbles stay open while the pointer is over them (or over a bubble opened from them),
//! so the pointer can move into one to hover its terms.

use egui::text::{CCursor, LayoutJob, TextFormat};
use egui::{Color32, CursorIcon, Id, Rect, Response, Sense, Stroke, TextStyle, Tooltip, Ui};

/// How a term is spotted in text.
enum Match {
    /// Any of these words, whole.
    Words(&'static [&'static str]),
    /// A whole word that starts with this (and has more after it).
    Prefix(&'static str),
    /// A whole word that ends with this (and has more before it).
    Suffix(&'static str),
}

use Match::*;

struct Term {
    m: Match,
    text: &'static str,
}

/// Every term, with what it means. The words are case-sensitive.
static TERMS: &[Term] = &[
    // ── registers and parts
    Term { m: Words(&["BEN", "branch-enable bit", "branch enable"]), text: "Branch ENable: a 1-bit register set while decoding (state 32) to IR[11]·N + IR[10]·Z + IR[9]·P, that is, whether any condition the BR asks for is true. State 0 checks it: 1 means BR takes the branch (state 22), 0 means back to fetch." },
    Term { m: Words(&["INT"]), text: "The interrupt request signal: 1 when a device with interrupts enabled is ready and its priority is higher than the running program's. Control checks it at state 18, the start of each fetch." },
    Term { m: Words(&["INTV"]), text: "Interrupt vector: the 8-bit number a device sends with its interrupt. The keyboard's is x80, so the address of its service routine is in memory x0180 (the interrupt vector table at x0100, plus x80)." },
    Term { m: Words(&["PSR", "processor status register", "Processor status register"]), text: "Processor Status Register: PSR[15] is the privilege (0 Supervisor, 1 User), PSR[10:8] the priority, and PSR[2:0] the condition codes N Z P. Its address is xFFFC." },
    Term { m: Words(&["PC"]), text: "Program Counter: the address of the next instruction to fetch. Fetch adds 1 to it; branches, jumps, TRAP and interrupts load it with a new address." },
    Term { m: Words(&["IR"]), text: "Instruction Register: the 16-bit instruction being executed. Wires take its fields (opcode, register numbers, offsets) straight to the parts that need them." },
    Term { m: Words(&["MAR"]), text: "Memory Address Register: the address memory reads or writes." },
    Term { m: Words(&["MDR"]), text: "Memory Data Register: the word just read from memory, or the word about to be written." },
    Term { m: Words(&["ALU"]), text: "Arithmetic Logic Unit: does ADD, AND or NOT on its two 16-bit inputs, A and B (or passes A through)." },
    Term { m: Words(&["ALUK"]), text: "The 2-bit control signal that picks the ALU's operation: ADD, AND, NOT or PASSA." },
    Term { m: Words(&["PASSA"]), text: "The ALU passes input A through unchanged (used to copy a register onto the bus)." },
    Term { m: Words(&["REG FILE", "register file", "Register file"]), text: "The register file: R0–R7. It reads two registers at once (SR1 and SR2) and writes one (DR), picked by register numbers from the IR." },
    Term { m: Words(&["LOGIC"]), text: "Works out N, Z or P (negative, zero, positive) from the value on the bus, for the condition codes." },
    Term { m: Words(&["Set.Priv"]), text: "The constant written into PSR[15] when entering an interrupt, exception or TRAP: 0, Supervisor mode." },
    Term { m: Words(&["Saved.SSP", "SSP"]), text: "Saved Supervisor Stack Pointer: where the OS's stack pointer is kept while a user program runs (R6 then holds the user's)." },
    Term { m: Words(&["Saved.USP", "USP"]), text: "Saved User Stack Pointer: where the user program's stack pointer is kept while the OS runs (R6 then holds the supervisor's)." },
    Term { m: Words(&["Table"]), text: "The high byte of a vector table entry's address: x00 for the trap vector table (x0000–x00FF), x01 for interrupts and exceptions (x0100–x01FF)." },
    Term { m: Words(&["Vector"]), text: "The low byte of a vector table entry's address: which entry. Table'Vector (the two bytes side by side) is the entry's address." },
    Term { m: Words(&["Table'Vector"]), text: "Table and Vector side by side, a 16-bit address: the vector table entry that holds the service routine's address." },
    Term { m: Words(&["MIO.EN"]), text: "Memory I/O enable: when asserted, memory (or a device register) is read or written this cycle." },
    Term { m: Words(&["R.W", "R/W"]), text: "Read or write: whether memory reads M[MAR] into MDR or writes MDR to M[MAR]." },
    Term { m: Words(&["MUX", "mux", "multiplexer"]), text: "Multiplexer: a switch with several inputs and one output. A control signal picks which input goes through." },
    Term { m: Words(&["PCMUX"]), text: "The multiplexer for the PC's next value: PC + 1, the address adder (branches, jumps) or the bus (TRAP, RTI, interrupts)." },
    Term { m: Words(&["ADDR1MUX"]), text: "The multiplexer for the address adder's base: the PC or a register (BaseR)." },
    Term { m: Words(&["ADDR2MUX"]), text: "The multiplexer for the address adder's offset: 0, offset6, PCoffset9 or PCoffset11, sign-extended from the IR." },
    Term { m: Words(&["MARMUX"]), text: "The multiplexer for an address on the bus: the address adder's sum, or ZEXT(trapvect8) for TRAP." },
    Term { m: Words(&["SR2MUX"]), text: "The multiplexer for the ALU's B input: register SR2, or imm5 sign-extended. IR[5] picks." },
    Term { m: Words(&["SPMUX"]), text: "The multiplexer for the new stack pointer: R6 + 1 (pop), R6 − 1 (push), Saved.SSP or Saved.USP." },
    Term { m: Words(&["PSRMUX"]), text: "The multiplexers that feed the PSR: from the bus (RTI), or N Z P from LOGIC, Set.Priv and the interrupt's priority." },
    Term { m: Suffix("MUX"), text: "A multiplexer: a switch with several inputs and one output. A control signal picks which input goes through." },
    Term { m: Words(&["bus", "Bus"]), text: "The bus: one shared 16-bit wire that carries a value between parts. Each clock cycle at most one gate drives it, and any register whose load signal is asserted takes the value." },
    Term { m: Prefix("Gate"), text: "A gate: a switch (tri-state buffer) between a part's output and the bus. While its signal is asserted, that value is on the bus." },
    Term { m: Words(&["gate", "gates"]), text: "A switch (tri-state buffer) between a part's output and the bus. While its signal is asserted, that value is on the bus." },
    Term { m: Prefix("LD."), text: "A load signal. When asserted, the register takes the value at its input at the end of the clock cycle; otherwise it keeps its value." },
    Term { m: Words(&["SEXT", "sign-extends", "sign-extend", "sign-extended", "sign extend"]), text: "Sign-extend: widen a two's complement number to 16 bits by copying its top bit, so a negative offset stays negative (5-bit 11111, −1, becomes xFFFF)." },
    Term { m: Words(&["ZEXT", "zero-extends", "zero-extend", "zero-extended"]), text: "Zero-extend: widen a number to 16 bits by filling the top with 0s. TRAP's 8-bit trapvect8 is zero-extended into an address." },
    Term { m: Words(&["two's complement"]), text: "How the LC-3 stores negative numbers: −n is NOT n, plus 1. The top bit is 1 for negative numbers, so 16 bits hold −32768 to 32767." },
    Term { m: Words(&["address adder"]), text: "Adds a base (ADDR1MUX: the PC or a register) and an offset (ADDR2MUX) to make an address, for loads, stores, branches and jumps." },
    // ── instruction fields
    Term { m: Words(&["opcode", "opcodes"]), text: "IR[15:12]: the 4 bits that say which instruction this is (0001 ADD, 0101 AND, 0000 BR, …). Control decodes it in state 32." },
    Term { m: Words(&["DR"]), text: "Destination Register: the register an instruction writes, usually IR[11:9]." },
    Term { m: Words(&["SR1"]), text: "Source Register 1: IR[8:6]. Its value goes to the ALU's A input and to ADDR1MUX (as BaseR)." },
    Term { m: Words(&["SR2"]), text: "Source Register 2: IR[2:0] (when IR[5] is 0). Its value goes to SR2MUX, the ALU's B input." },
    Term { m: Words(&["SR"]), text: "Source Register: the register an instruction reads, e.g. the value ST stores (IR[11:9]) or NOT inverts (IR[8:6])." },
    Term { m: Words(&["BaseR"]), text: "Base Register: IR[8:6] in LDR, STR, JMP and JSRR. Its value is the address adder's base." },
    Term { m: Words(&["imm5"]), text: "A 5-bit immediate: IR[4:0] in ADD and AND when IR[5] is 1, a constant from −16 to 15, sign-extended." },
    Term { m: Words(&["offset6"]), text: "IR[5:0] in LDR and STR: a constant from −32 to 31, sign-extended and added to BaseR." },
    Term { m: Words(&["PCoffset9"]), text: "IR[8:0]: a constant from −256 to 255, sign-extended and added to the PC (already incremented). LD, ST, LDI, STI, LEA and BR use it." },
    Term { m: Words(&["PCoffset11"]), text: "IR[10:0] in JSR: a constant from −1024 to 1023, sign-extended and added to the PC." },
    Term { m: Words(&["trapvect8", "trap vector"]), text: "IR[7:0] in TRAP: which service routine. Its address is in the trap vector table at x0000 + trapvect8 (HALT is x25, so M[x0025])." },
    Term { m: Words(&["nzp"]), text: "In a BR instruction, IR[11:9]: which of the condition codes N, Z, P to branch on. BRnzp (111) always branches." },
    // ── state and control
    Term { m: Words(&["condition codes", "N Z P", "N, Z, P", "N, Z or P", "NZP"]), text: "Condition codes N, Z, P: three bits in the PSR saying whether the last value written to a register was Negative, Zero or Positive. Exactly one is 1. BR tests them." },
    Term { m: Words(&["state machine", "control logic", "control unit"]), text: "The control unit is a finite state machine. Each clock cycle it is in one state (numbered as in Patt & Patel's Figure C.2), asserts that state's control signals, then picks the next state from the opcode, BEN, INT, R (memory ready) and so on." },
    Term { m: Words(&["control signals", "control signal", "signals"]), text: "Wires from the control unit that switch the datapath: load a register (LD.*), drive the bus (Gate*), pick a MUX input, choose the ALU's operation. Each state asserts its own set." },
    Term { m: Words(&["asserted", "asserts", "assert"]), text: "A signal is asserted when it is set to 1 (on), so the thing it controls happens this clock cycle." },
    Term { m: Words(&["clock cycle", "clock cycles", "cycle"]), text: "One tick of the clock: one state of the state machine. Values flow through the wires during the cycle, and registers load at its end." },
    Term { m: Words(&["datapath"]), text: "The registers, MUXes, ALU, adders and bus that move and change values, all switched by the control signals." },
    Term { m: Words(&["fetch"]), text: "Getting the next instruction: state 18 (MAR ← PC, PC ← PC + 1), 33 (MDR ← M[MAR]) and 35 (IR ← MDR)." },
    Term { m: Words(&["decode", "decodes", "decoding"]), text: "State 32: control looks at the opcode in IR[15:12] to pick the instruction's first state, and sets BEN for BR." },
    Term { m: Words(&["micro-op", "micro-ops", "micro step"]), text: "One small register transfer in a state, such as MAR ← PC. A state does all its micro-ops in the same clock cycle." },
    // ── privilege and interrupts
    Term { m: Words(&["privilege", "Supervisor mode", "supervisor mode", "User mode", "user mode"]), text: "PSR[15]. 0 is Supervisor mode: the OS, allowed everything. 1 is User mode: programs may not touch system memory (x0000–x2FFF) or the device registers (xFE00 up), or run RTI." },
    Term { m: Words(&["priority", "PL0", "PL1", "PL2", "PL3", "PL4", "PL5", "PL6", "PL7"]), text: "Priority levels PL0–PL7, in PSR[10:8]: a device can only interrupt if its priority is higher than the running program's. User programs run at PL0; the keyboard is PL4." },
    Term { m: Words(&["stack pointer", "stack pointers", "R6"]), text: "R6 is the stack pointer: the address of the top of the stack. Push is R6 ← R6 − 1 then a write; pop is a read then R6 ← R6 + 1. The OS and the user program each have their own stack, swapped through Saved.SSP and Saved.USP." },
    Term { m: Words(&["push", "pop"]), text: "The stack grows down. Push: R6 ← R6 − 1, then write M[R6]. Pop: read M[R6], then R6 ← R6 + 1." },
    Term { m: Words(&["interrupt", "interrupts"]), text: "A device asking for attention. At the next fetch the CPU saves PSR and PC on the supervisor stack, switches to Supervisor mode at the device's priority, and jumps to the device's service routine. RTI goes back." },
    Term { m: Words(&["exception", "exceptions"]), text: "The program did something not allowed: a privilege mode violation (x00), an illegal opcode (x01) or an ACV (x02). Handled like an interrupt, through the interrupt vector table." },
    Term { m: Words(&["ACV", "access control violation"]), text: "Access Control Violation: a User mode program used an address in system space (below x3000, or a device register from xFE00)." },
    Term { m: Words(&["service routine", "service routines", "ISR"]), text: "OS code that handles a TRAP, an interrupt or an exception. Its address is in a vector table." },
    Term { m: Words(&["vector table", "vector tables", "interrupt vector table", "trap vector table"]), text: "Tables of service routine addresses in memory: the trap vector table at x0000–x00FF and the interrupt vector table (interrupts and exceptions) at x0100–x01FF." },
    Term { m: Words(&["vector", "vectors"]), text: "A number that picks an entry in a vector table, the address of a service routine: trapvect8 for TRAP, INTV for an interrupt, x00–x02 for exceptions." },
    Term { m: Words(&["polling"]), text: "Waiting for a device by reading its status register in a loop until it says ready. Simple, but the CPU does nothing else meanwhile." },
    // ── device registers
    Term { m: Words(&["KBSR"]), text: "Keyboard Status Register, xFE00: bit 15 is 1 when a key is waiting in KBDR; bit 14 enables the keyboard interrupt." },
    Term { m: Words(&["KBDR"]), text: "Keyboard Data Register, xFE02: the last key typed (ASCII). Reading it clears KBSR[15]." },
    Term { m: Words(&["DSR"]), text: "Display Status Register, xFE04: bit 15 is 1 when the display is ready for another character." },
    Term { m: Words(&["DDR"]), text: "Display Data Register, xFE06: writing a character here prints it." },
    Term { m: Words(&["MCR"]), text: "Machine Control Register, xFFFE: clearing bit 15 stops the clock (HALT). In this emulator, setting bit 14 sleeps until an interrupt." },
    // ── instructions
    Term { m: Words(&["ADD"]), text: "ADD DR, SR1, SR2 or imm5: DR ← SR1 + the second operand. Sets N Z P." },
    Term { m: Words(&["AND"]), text: "AND DR, SR1, SR2 or imm5: DR ← SR1 AND the second operand, bit by bit. Sets N Z P." },
    Term { m: Words(&["NOT"]), text: "NOT DR, SR: DR ← every bit of SR flipped. Sets N Z P." },
    Term { m: Words(&["BR"]), text: "BR(nzp) PCoffset9: if any condition code it names is set (BEN), PC ← PC + offset. BRnzp always branches." },
    Term { m: Words(&["LD"]), text: "LD DR, PCoffset9: DR ← M[PC + offset]. Sets N Z P." },
    Term { m: Words(&["LDI"]), text: "LDI DR, PCoffset9: DR ← M[M[PC + offset]], loading through a pointer. Sets N Z P." },
    Term { m: Words(&["LDR"]), text: "LDR DR, BaseR, offset6: DR ← M[BaseR + offset]. Sets N Z P." },
    Term { m: Words(&["LEA"]), text: "LEA DR, PCoffset9: DR ← PC + offset, the address itself (no memory read)." },
    Term { m: Words(&["ST"]), text: "ST SR, PCoffset9: M[PC + offset] ← SR." },
    Term { m: Words(&["STI"]), text: "STI SR, PCoffset9: M[M[PC + offset]] ← SR, storing through a pointer." },
    Term { m: Words(&["STR"]), text: "STR SR, BaseR, offset6: M[BaseR + offset] ← SR." },
    Term { m: Words(&["JMP", "RET"]), text: "JMP BaseR: PC ← BaseR. RET is JMP R7, returning from a subroutine." },
    Term { m: Words(&["JSR", "JSRR"]), text: "Call a subroutine: R7 ← PC (the return address), then PC ← PC + PCoffset11 (JSR) or BaseR (JSRR)." },
    Term { m: Words(&["TRAP"]), text: "TRAP trapvect8: call an OS service routine. The CPU switches to Supervisor mode, pushes PSR and PC, and loads the PC from the trap vector table." },
    Term { m: Words(&["RTI"]), text: "Return from interrupt (or trap): pop the PC and PSR off the supervisor stack, going back to the interrupted program. Only allowed in Supervisor mode." },
    Term { m: Words(&["HALT"]), text: "TRAP x25: the OS stops the machine by clearing MCR[15]." },
    Term { m: Words(&["GETC"]), text: "TRAP x20: R0 ← the next key typed. The interrupt OS sleeps until the keyboard interrupt; the polling OS loops on KBSR." },
    Term { m: Words(&["OUT"]), text: "TRAP x21: print the character in R0." },
    Term { m: Words(&["PUTS"]), text: "TRAP x22: print the string at R0, one character per word, up to a zero." },
    Term { m: Words(&["IN"]), text: "TRAP x23: print a prompt, then read a key into R0 and echo it." },
    Term { m: Words(&["PUTSP"]), text: "TRAP x24: print the string at R0, two characters per word." },
];

/// How deep bubbles nest (a term's explanation's terms' explanations…).
const MAX_DEPTH: usize = 4;

/// One term found in a text: bytes `start..end`, term number.
fn find_terms(text: &str, skip: &[usize]) -> Vec<(usize, usize, usize)> {
    let bytes = text.as_bytes();
    let word = |b: u8| b.is_ascii_alphanumeric() || b == b'_';
    // a run of name characters from `i` (letters, digits, dots, dashes), without a trailing dot or dash
    let name_end = |i: usize| {
        let mut e = i;
        while e < bytes.len() && (word(bytes[e]) || matches!(bytes[e], b'.' | b'-' | b'\'' | b'*'))
        {
            e += 1;
        }
        while e > i && matches!(bytes[e - 1], b'.' | b'-' | b'\'') {
            e -= 1;
        }
        e
    };
    let mut seen = vec![false; TERMS.len()];
    for &s in skip {
        seen[s] = true;
    }
    let mut out = Vec::new();
    let mut i = 0;
    while i < bytes.len() {
        if !text.is_char_boundary(i) || (i > 0 && word(bytes[i - 1])) || !word(bytes[i]) {
            i += 1;
            continue;
        }
        let rest = &text[i..];
        let mut best: Option<(usize, usize)> = None; // (length, term)
        for (k, t) in TERMS.iter().enumerate() {
            let len = match &t.m {
                Words(ws) => ws
                    .iter()
                    .filter(|w| {
                        rest.starts_with(**w) && !bytes.get(i + w.len()).is_some_and(|b| word(*b))
                    })
                    .map(|w| w.len())
                    .max(),
                Prefix(p) => {
                    let e = name_end(i);
                    (rest.starts_with(p) && e > i + p.len()).then_some(e - i)
                }
                Suffix(s) => {
                    let e = name_end(i);
                    (e - i > s.len() && text[i..e].ends_with(s)).then_some(e - i)
                }
            };
            if let Some(len) = len {
                if best.is_none_or_shorter(len) {
                    best = Some((len, k));
                }
            }
        }
        match best {
            Some((len, k)) => {
                if !seen[k] {
                    seen[k] = true;
                    out.push((i, i + len, k));
                }
                i += len;
            }
            None => i += 1,
        }
    }
    out
}

trait Longer {
    fn is_none_or_shorter(&self, len: usize) -> bool;
}

impl Longer for Option<(usize, usize)> {
    fn is_none_or_shorter(&self, len: usize) -> bool {
        self.map_or(true, |(l, _)| l < len)
    }
}

/// Show `text` wrapped, with the glossary's terms underlined and explained on hover.
pub fn text(ui: &mut Ui, text: &str) -> Response {
    text_at(ui, text, 0, &[], None, None)
}

/// [`text`] at a font size and in a colour.
pub fn text_styled(ui: &mut Ui, text: &str, size: f32, color: Color32) -> Response {
    text_at(ui, text, 0, &[], Some(size), Some(color))
}

fn text_at(
    ui: &mut Ui,
    text: &str,
    depth: usize,
    skip: &[usize],
    size: Option<f32>,
    color: Option<Color32>,
) -> Response {
    let mut font = TextStyle::Body.resolve(ui.style());
    if let Some(size) = size {
        font.size = size;
    }
    let color = color.unwrap_or_else(|| ui.visuals().text_color());
    let link = ui.visuals().hyperlink_color;
    let terms = if depth < MAX_DEPTH {
        find_terms(text, skip)
    } else {
        Vec::new()
    };

    let mut job = LayoutJob::default();
    job.wrap.max_width = ui.available_width();
    let plain = TextFormat {
        font_id: font.clone(),
        color,
        ..Default::default()
    };
    let term = TextFormat {
        font_id: font.clone(),
        color: link,
        underline: Stroke::new(1.0_f32, link.gamma_multiply(0.45)),
        ..Default::default()
    };
    let mut at = 0;
    for &(s, e, _) in &terms {
        job.append(&text[at..s], 0.0, plain.clone());
        job.append(&text[s..e], 0.0, term.clone());
        at = e;
    }
    job.append(&text[at..], 0.0, plain);

    let base = ui.next_auto_id();
    let galley = ui.painter().layout_job(job);
    let (rect, response) = ui.allocate_exact_size(galley.size(), Sense::hover());
    ui.painter().galley(rect.min, galley.clone(), color);

    for &(s, e, k) in &terms {
        // the term's characters, row by row
        let (cs, ce) = (text[..s].chars().count(), text[..e].chars().count());
        let mut hit = Rect::NOTHING;
        for c in cs..ce {
            let (a, b) = (
                galley.pos_from_cursor(CCursor::new(c)),
                galley.pos_from_cursor(CCursor::new(c + 1)),
            );
            let r = if (a.center().y - b.center().y).abs() < 1.0 {
                a.union(b)
            } else {
                a
            };
            hit = hit.union(r.translate(rect.min.to_vec2()));
        }
        let r = ui
            .interact(hit, base.with(("term", s)), Sense::click())
            .on_hover_cursor(CursorIcon::Help);
        let name = &text[s..e];
        // the bubble doesn't link again what is linked above it
        let chain: Vec<usize> = skip
            .iter()
            .copied()
            .chain(terms.iter().map(|t| t.2))
            .collect();
        bubble(&r, depth + 1, None, |ui| {
            ui.label(egui::RichText::new(name).strong());
            text_at(ui, TERMS[k].text, depth + 1, &chain, None, None);
        });
    }
    response
}

/// A bubble that was shown: who opened it, where it is, what it hangs from, and when the
/// pointer was last on that or on the bubble (or a bubble opened from it).
#[derive(Clone, Copy)]
struct Shown {
    owner: Id,
    bubble: Rect,
    anchor: Rect,
    last_on: f64,
}

/// The bubbles shown, by depth. `prev` is last frame's.
#[derive(Clone, Default)]
struct Open {
    pass: u64,
    prev: Vec<Option<Shown>>,
    cur: Vec<Option<Shown>>,
}

fn open_state<R>(ctx: &egui::Context, f: impl FnOnce(&mut Open) -> R) -> R {
    let pass = ctx.cumulative_pass_nr();
    ctx.data_mut(|d| {
        let o = d.get_temp_mut_or_default::<Open>(Id::new("glossary_open"));
        if o.pass != pass {
            o.prev = std::mem::take(&mut o.cur);
            o.pass = pass;
        }
        f(o)
    })
}

/// How long the pointer may take to cross from a widget into its bubble.
const GRACE: f64 = 0.7;

/// Should the bubble `owner` opened at `depth` last frame stay open? It does while the pointer
/// is on it (or on a bubble opened from it), and for a moment while the pointer crosses
/// between the widget and the bubble. Also returns when the pointer was last on it.
fn keep_open(ctx: &egui::Context, owner: Id, depth: usize) -> Option<f64> {
    let (pointer, now) = ctx.input(|i| (i.pointer.latest_pos(), i.time));
    let p = pointer?;
    open_state(ctx, |o| {
        let s = o
            .prev
            .get(depth)
            .copied()
            .flatten()
            .filter(|s| s.owner == owner)?;
        if o.prev[depth..]
            .iter()
            .flatten()
            .any(|s| s.bubble.expand(2.0).contains(p))
        {
            return Some(now);
        }
        (now - s.last_on < GRACE && s.anchor.union(s.bubble).contains(p)).then_some(s.last_on)
    })
}

/// A hover bubble for `resp` at nesting `depth`, below `anchor` (default: the widget).
fn bubble(resp: &Response, depth: usize, anchor: Option<Rect>, add: impl FnOnce(&mut Ui)) -> bool {
    let ctx = &resp.ctx;
    let anchor = anchor.unwrap_or(resp.rect);
    let kept = keep_open(ctx, resp.id, depth);
    let natural = resp.enabled() && Tooltip::should_show_tooltip(resp, true);
    if !natural && kept.is_none() {
        return false;
    }
    let shown = Tooltip::always_open(ctx.clone(), resp.layer_id, resp.id, anchor).show(|ui| {
        ui.set_max_width(ui.spacing().tooltip_width);
        add(ui)
    });
    if let Some(shown) = shown {
        let now = ctx.input(|i| i.time);
        let on_anchor = ctx.pointer_latest_pos().is_some_and(|p| anchor.contains(p));
        let last_on = if on_anchor { now } else { kept.unwrap_or(now) };
        let s = Shown {
            owner: resp.id,
            bubble: shown.response.rect,
            anchor,
            last_on,
        };
        open_state(ctx, |o| {
            if o.cur.len() <= depth {
                o.cur.resize(depth + 1, None);
            }
            o.cur[depth] = Some(s);
        });
    }
    true
}

/// `on_hover_text`, with the jargon in it explained.
pub trait Explain {
    fn explain(self, text: impl AsRef<str>) -> Self;
}

impl Explain for Response {
    fn explain(self, tip: impl AsRef<str>) -> Self {
        bubble(&self, 0, None, |ui| {
            text(ui, tip.as_ref());
        });
        self
    }
}

/// The tooltip for a drawing where `target` is what the pointer is over now (its tooltip),
/// e.g. a part of the datapath. The bubble stays where it opened, so the pointer can move into
/// it, and keeps its text while the pointer is on it.
pub fn canvas(resp: &Response, target: Option<String>) {
    let ctx = &resp.ctx;
    let key = resp.id.with("glossary_canvas");
    let stored: Option<(String, Rect)> = ctx.data(|d| d.get_temp(key));
    let tip = if let Some(s) = stored
        .as_ref()
        .filter(|_| keep_open(ctx, resp.id, 0).is_some())
    {
        Some(s.clone())
    } else if let Some(t) = target {
        let at = match &stored {
            Some((old, r)) if *old == t => *r,
            _ => Rect::from_center_size(
                ctx.pointer_latest_pos().unwrap_or(resp.rect.center()),
                egui::vec2(24.0, 24.0),
            ),
        };
        Some((t, at))
    } else {
        None
    };
    match tip {
        Some((t, at)) => {
            ctx.data_mut(|d| d.insert_temp(key, (t.clone(), at)));
            bubble(resp, 0, Some(at), |ui| {
                text(ui, &t);
            });
        }
        None => ctx.data_mut(|d| d.remove::<(String, Rect)>(key)),
    }
}

/// The explanation of a word, if the glossary has it (for tests and lookups).
#[allow(dead_code)]
pub fn lookup(word: &str) -> Option<&'static str> {
    find_terms(word, &[])
        .first()
        .filter(|(s, e, _)| *s == 0 && *e == word.len())
        .map(|(_, _, k)| TERMS[*k].text)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn names(text: &str) -> Vec<&str> {
        find_terms(text, &[])
            .into_iter()
            .map(|(s, e, _)| &text[s..e])
            .collect()
    }

    #[test]
    fn finds_whole_words_once() {
        assert_eq!(
            names("BEN is set from N, Z, P; BEN again"),
            vec!["BEN", "N, Z, P"]
        );
        assert_eq!(
            names("LD.BEN and GatePC-1 onto the bus."),
            vec!["LD.BEN", "GatePC-1", "bus"]
        );
        assert_eq!(
            names("SR2MUX picks; TableMUX too"),
            vec!["SR2MUX", "TableMUX"]
        );
        assert!(names("BENT ABUS").is_empty());
        assert_eq!(names("PC + 1"), vec!["PC"]);
    }

    #[test]
    fn every_term_is_found() {
        assert!(lookup("BEN").is_some());
        for t in TERMS {
            if let Words(ws) = &t.m {
                for w in *ws {
                    assert!(lookup(w).is_some(), "{w}");
                }
            }
        }
    }

    #[test]
    fn explanations_dont_link_themselves() {
        for (k, t) in TERMS.iter().enumerate() {
            assert!(find_terms(t.text, &[k]).iter().all(|(_, _, j)| *j != k));
        }
    }
}
