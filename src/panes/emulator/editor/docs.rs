//! Short reference docs for every mnemonic, used by completion and hover cards.

pub struct InstrDoc {
    /// Mnemonic as written in completion lists
    pub name: &'static str,
    /// Operand signature, e.g. `DR, SR1, SR2`
    pub operands: &'static str,
    /// One sentence describing the effect
    pub summary: &'static str,
    /// Register transfer description
    pub rtl: &'static str,
    /// Encoding fields from bit 15 down, as (bits, label)
    pub encoding: &'static [(&'static str, &'static str)],
    /// Does it set N/Z/P
    pub sets_cc: bool,
    /// Snippet inserted on completion (`$` marks where the caret ends up)
    pub snippet: &'static str,
}

pub const INSTRUCTIONS: &[InstrDoc] = &[
    InstrDoc {
        name: "ADD",
        operands: "DR, SR1, SR2 | imm5",
        summary: "Add two registers, or a register and a 5-bit signed immediate.",
        rtl: "DR ← SR1 + SR2   or   DR ← SR1 + SEXT(imm5)",
        encoding: &[
            ("0001", "op"),
            ("DR", "3"),
            ("SR1", "3"),
            ("0|1", "mode"),
            ("SR2|imm5", "5"),
        ],
        sets_cc: true,
        snippet: "ADD $",
    },
    InstrDoc {
        name: "AND",
        operands: "DR, SR1, SR2 | imm5",
        summary: "Bitwise AND. `AND R0, R0, #0` is the usual way to clear a register.",
        rtl: "DR ← SR1 & SR2   or   DR ← SR1 & SEXT(imm5)",
        encoding: &[
            ("0101", "op"),
            ("DR", "3"),
            ("SR1", "3"),
            ("0|1", "mode"),
            ("SR2|imm5", "5"),
        ],
        sets_cc: true,
        snippet: "AND $",
    },
    InstrDoc {
        name: "NOT",
        operands: "DR, SR",
        summary: "Bitwise complement. Negate with NOT then ADD #1.",
        rtl: "DR ← NOT(SR)",
        encoding: &[("1001", "op"), ("DR", "3"), ("SR", "3"), ("111111", "")],
        sets_cc: true,
        snippet: "NOT $",
    },
    InstrDoc {
        name: "BR",
        operands: "[n][z][p] LABEL",
        summary: "Branch if any of the named condition codes is set. Plain BR always branches.",
        rtl: "if (n&N | z&Z | p&P)  PC ← PC + SEXT(PCoffset9)",
        encoding: &[("0000", "op"), ("n z p", "3"), ("PCoffset9", "9")],
        sets_cc: false,
        snippet: "BR $",
    },
    InstrDoc {
        name: "JMP",
        operands: "BaseR",
        summary: "Jump to the address held in a register.",
        rtl: "PC ← BaseR",
        encoding: &[("1100", "op"), ("000", ""), ("BaseR", "3"), ("000000", "")],
        sets_cc: false,
        snippet: "JMP $",
    },
    InstrDoc {
        name: "RET",
        operands: "",
        summary: "Return from a subroutine (JMP R7).",
        rtl: "PC ← R7",
        encoding: &[("1100", "op"), ("000", ""), ("111", "R7"), ("000000", "")],
        sets_cc: false,
        snippet: "RET$",
    },
    InstrDoc {
        name: "JSR",
        operands: "LABEL",
        summary: "Call a subroutine. The return address is saved in R7.",
        rtl: "R7 ← PC;  PC ← PC + SEXT(PCoffset11)",
        encoding: &[("0100", "op"), ("1", ""), ("PCoffset11", "11")],
        sets_cc: false,
        snippet: "JSR $",
    },
    InstrDoc {
        name: "JSRR",
        operands: "BaseR",
        summary: "Call a subroutine whose address is in a register.",
        rtl: "R7 ← PC;  PC ← BaseR",
        encoding: &[
            ("0100", "op"),
            ("0", ""),
            ("00", ""),
            ("BaseR", "3"),
            ("000000", ""),
        ],
        sets_cc: false,
        snippet: "JSRR $",
    },
    InstrDoc {
        name: "LD",
        operands: "DR, LABEL",
        summary: "Load a word from a PC-relative address.",
        rtl: "DR ← mem[PC + SEXT(PCoffset9)]",
        encoding: &[("0010", "op"), ("DR", "3"), ("PCoffset9", "9")],
        sets_cc: true,
        snippet: "LD $",
    },
    InstrDoc {
        name: "LDI",
        operands: "DR, LABEL",
        summary: "Load indirect: the word at LABEL is the address to load from.",
        rtl: "DR ← mem[mem[PC + SEXT(PCoffset9)]]",
        encoding: &[("1010", "op"), ("DR", "3"), ("PCoffset9", "9")],
        sets_cc: true,
        snippet: "LDI $",
    },
    InstrDoc {
        name: "LDR",
        operands: "DR, BaseR, offset6",
        summary: "Load from a base register plus a 6-bit signed offset.",
        rtl: "DR ← mem[BaseR + SEXT(offset6)]",
        encoding: &[
            ("0110", "op"),
            ("DR", "3"),
            ("BaseR", "3"),
            ("offset6", "6"),
        ],
        sets_cc: true,
        snippet: "LDR $",
    },
    InstrDoc {
        name: "LEA",
        operands: "DR, LABEL",
        summary: "Load the address of LABEL (does not touch memory).",
        rtl: "DR ← PC + SEXT(PCoffset9)",
        encoding: &[("1110", "op"), ("DR", "3"), ("PCoffset9", "9")],
        sets_cc: false,
        snippet: "LEA $",
    },
    InstrDoc {
        name: "ST",
        operands: "SR, LABEL",
        summary: "Store a register to a PC-relative address.",
        rtl: "mem[PC + SEXT(PCoffset9)] ← SR",
        encoding: &[("0011", "op"), ("SR", "3"), ("PCoffset9", "9")],
        sets_cc: false,
        snippet: "ST $",
    },
    InstrDoc {
        name: "STI",
        operands: "SR, LABEL",
        summary: "Store indirect: the word at LABEL is the address to store to.",
        rtl: "mem[mem[PC + SEXT(PCoffset9)]] ← SR",
        encoding: &[("1011", "op"), ("SR", "3"), ("PCoffset9", "9")],
        sets_cc: false,
        snippet: "STI $",
    },
    InstrDoc {
        name: "STR",
        operands: "SR, BaseR, offset6",
        summary: "Store to a base register plus a 6-bit signed offset.",
        rtl: "mem[BaseR + SEXT(offset6)] ← SR",
        encoding: &[
            ("0111", "op"),
            ("SR", "3"),
            ("BaseR", "3"),
            ("offset6", "6"),
        ],
        sets_cc: false,
        snippet: "STR $",
    },
    InstrDoc {
        name: "TRAP",
        operands: "trapvect8",
        summary: "Call an OS service routine through the trap vector table.",
        rtl: "R7 ← PC;  PC ← mem[ZEXT(trapvect8)]",
        encoding: &[("1111", "op"), ("0000", ""), ("trapvect8", "8")],
        sets_cc: false,
        snippet: "TRAP x$",
    },
    InstrDoc {
        name: "RTI",
        operands: "",
        summary: "Return from interrupt or trap: restores PC and PSR from the supervisor stack.",
        rtl: "PC ← mem[R6]; PSR ← mem[R6+1]; R6 ← R6 + 2",
        encoding: &[("1000", "op"), ("000000000000", "")],
        sets_cc: false,
        snippet: "RTI$",
    },
    InstrDoc {
        name: "GETC",
        operands: "",
        summary: "TRAP x20: read one character from the keyboard into R0 (no echo).",
        rtl: "R0 ← keyboard char",
        encoding: &[("1111", "op"), ("0000", ""), ("x20", "")],
        sets_cc: false,
        snippet: "GETC$",
    },
    InstrDoc {
        name: "OUT",
        operands: "",
        summary: "TRAP x21: write the character in R0[7:0] to the display.",
        rtl: "display ← R0[7:0]",
        encoding: &[("1111", "op"), ("0000", ""), ("x21", "")],
        sets_cc: false,
        snippet: "OUT$",
    },
    InstrDoc {
        name: "PUTS",
        operands: "",
        summary: "TRAP x22: print the null-terminated string whose address is in R0.",
        rtl: "display ← mem[R0], mem[R0+1], … until x0000",
        encoding: &[("1111", "op"), ("0000", ""), ("x22", "")],
        sets_cc: false,
        snippet: "PUTS$",
    },
    InstrDoc {
        name: "IN",
        operands: "",
        summary: "TRAP x23: prompt, read one character into R0 and echo it.",
        rtl: "R0 ← keyboard char (echoed)",
        encoding: &[("1111", "op"), ("0000", ""), ("x23", "")],
        sets_cc: false,
        snippet: "IN$",
    },
    InstrDoc {
        name: "PUTSP",
        operands: "",
        summary: "TRAP x24: print a string packed two characters per word, address in R0.",
        rtl: "display ← packed string at R0",
        encoding: &[("1111", "op"), ("0000", ""), ("x24", "")],
        sets_cc: false,
        snippet: "PUTSP$",
    },
    InstrDoc {
        name: "HALT",
        operands: "",
        summary: "TRAP x25: stop the machine.",
        rtl: "MCR[15] ← 0",
        encoding: &[("1111", "op"), ("0000", ""), ("x25", "")],
        sets_cc: false,
        snippet: "HALT$",
    },
];

pub struct DirectiveDoc {
    pub name: &'static str,
    pub operands: &'static str,
    pub summary: &'static str,
    pub snippet: &'static str,
}

pub const DIRECTIVES: &[DirectiveDoc] = &[
    DirectiveDoc {
        name: ".ORIG",
        operands: "address",
        summary: "Start address for the following code, e.g. .ORIG x3000.",
        snippet: ".ORIG x3000$",
    },
    DirectiveDoc {
        name: ".FILL",
        operands: "value | LABEL",
        summary: "Reserve one word and set it to a value or a label's address.",
        snippet: ".FILL $",
    },
    DirectiveDoc {
        name: ".BLKW",
        operands: "count",
        summary: "Reserve a block of count words, set to zero.",
        snippet: ".BLKW $",
    },
    DirectiveDoc {
        name: ".STRINGZ",
        operands: "\"text\"",
        summary: "Store a string, one character per word, followed by a x0000 terminator.",
        snippet: ".STRINGZ \"$\"",
    },
    DirectiveDoc {
        name: ".END",
        operands: "",
        summary: "End of the source file.",
        snippet: ".END$",
    },
];

/// Look up docs for a mnemonic, accepting BR variants such as `BRnz`.
pub fn instruction(word: &str) -> Option<&'static InstrDoc> {
    let upper = word.to_ascii_uppercase();
    let key = if upper.starts_with("BR")
        && upper.len() <= 5
        && upper[2..].chars().all(|c| matches!(c, 'N' | 'Z' | 'P'))
    {
        "BR"
    } else {
        upper.as_str()
    };
    INSTRUCTIONS.iter().find(|d| d.name == key)
}

pub fn directive(word: &str) -> Option<&'static DirectiveDoc> {
    DIRECTIVES
        .iter()
        .find(|d| d.name.eq_ignore_ascii_case(word))
}
