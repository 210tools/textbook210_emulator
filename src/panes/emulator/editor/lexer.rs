//! A forgiving, line based lexer used by the editor for highlighting, hovers and completion.
//!
//! Unlike the assembler's lexer this never fails: every byte of the line ends up in exactly one
//! token so the highlighter can colour half typed code.

use std::ops::Range;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TokKind {
    Whitespace,
    Comment,
    Opcode,
    /// A trap alias such as PUTS or HALT
    Trap,
    Register,
    Directive,
    Number,
    String,
    /// A label in the first column / before a colon, i.e. a definition
    LabelDef,
    /// Any other identifier, most likely a reference to a label
    Ident,
    Punct,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Tok {
    pub kind: TokKind,
    /// Byte range in the line
    pub range: Range<usize>,
}

pub const OPCODES: &[&str] = &[
    "ADD", "AND", "BR", "BRN", "BRZ", "BRP", "BRNZ", "BRNP", "BRZP", "BRNZP", "JMP", "JSR", "JSRR",
    "LD", "LDI", "LDR", "LEA", "NOT", "RET", "RTI", "ST", "STI", "STR", "TRAP",
];
pub const TRAPS: &[&str] = &["GETC", "OUT", "PUTS", "IN", "PUTSP", "HALT"];
pub const DIRECTIVES: &[&str] = &[".ORIG", ".FILL", ".BLKW", ".STRINGZ", ".END"];

pub fn is_opcode(word: &str) -> bool {
    OPCODES.iter().any(|o| o.eq_ignore_ascii_case(word))
}
pub fn is_trap(word: &str) -> bool {
    TRAPS.iter().any(|o| o.eq_ignore_ascii_case(word))
}
pub fn is_register(word: &str) -> bool {
    let b = word.as_bytes();
    b.len() == 2 && (b[0] == b'R' || b[0] == b'r') && (b'0'..=b'7').contains(&b[1])
}

/// Parse an LC-3 numeric literal (`#12`, `#-3`, `x3000`, `-5`, `12`, `b1010`).
pub fn parse_number(word: &str) -> Option<i32> {
    let w = word.trim();
    let (neg, rest) = match w.strip_prefix('#') {
        Some(r) => match r.strip_prefix('-') {
            Some(r) => (true, r),
            None => (false, r),
        },
        None => match w.strip_prefix('-') {
            Some(r) => (true, r),
            None => (false, w),
        },
    };
    let v = if let Some(h) = rest.strip_prefix('x').or_else(|| rest.strip_prefix('X')) {
        if h.is_empty() {
            return None;
        }
        i32::from_str_radix(h.trim_start_matches('-'), 16)
            .ok()
            .map(|v| if h.starts_with('-') { -v } else { v })?
    } else if let Some(b) = rest.strip_prefix('b').or_else(|| rest.strip_prefix('B')) {
        if b.is_empty() || !w.starts_with(['b', 'B', '#', '-']) {
            return None;
        }
        i32::from_str_radix(b, 2).ok()?
    } else {
        if rest.is_empty() || !rest.bytes().all(|c| c.is_ascii_digit()) {
            return None;
        }
        rest.parse::<i32>().ok()?
    };
    Some(if neg { -v } else { v })
}

fn is_ident_byte(b: u8) -> bool {
    b.is_ascii_alphanumeric() || b == b'_' || b == b'.' || b == b'#' || b == b'-'
}

/// Tokenize one line of source.
pub fn lex_line(line: &str) -> Vec<Tok> {
    let bytes = line.as_bytes();
    let mut toks = Vec::new();
    let mut i = 0;
    let mut seen_word = false;
    while i < bytes.len() {
        let start = i;
        let c = bytes[i];
        let kind = if c == b';' {
            i = bytes.len();
            TokKind::Comment
        } else if c.is_ascii_whitespace() {
            while i < bytes.len() && bytes[i].is_ascii_whitespace() {
                i += 1;
            }
            TokKind::Whitespace
        } else if c == b'"' {
            i += 1;
            while i < bytes.len() {
                if bytes[i] == b'\\' {
                    i += 2;
                    continue;
                }
                if bytes[i] == b'"' {
                    i += 1;
                    break;
                }
                i += 1;
            }
            i = i.min(bytes.len());
            TokKind::String
        } else if c == b',' || c == b':' {
            i += 1;
            TokKind::Punct
        } else if is_ident_byte(c) {
            while i < bytes.len() && is_ident_byte(bytes[i]) {
                i += 1;
            }
            let word = &line[start..i];
            let first_word = !seen_word;
            seen_word = true;
            if word.starts_with('.') {
                TokKind::Directive
            } else if is_register(word) {
                TokKind::Register
            } else if is_opcode(word) {
                TokKind::Opcode
            } else if is_trap(word) {
                TokKind::Trap
            } else if parse_number(word).is_some() {
                TokKind::Number
            } else if first_word {
                TokKind::LabelDef
            } else {
                TokKind::Ident
            }
        } else {
            // Unknown char, take a whole UTF-8 char
            i += line[i..].chars().next().map(char::len_utf8).unwrap_or(1);
            TokKind::Punct
        };
        toks.push(Tok {
            kind,
            range: start..i,
        });
    }
    toks
}

/// Find the token (and its line-relative byte range) that contains the byte column `col`.
pub fn token_at(line: &str, col: usize) -> Option<Tok> {
    lex_line(line)
        .into_iter()
        .find(|t| t.range.start <= col && col < t.range.end.max(t.range.start + 1))
}

/// All label definitions in a document, as (name, 0-based line).
pub fn label_definitions(text: &str) -> Vec<(String, usize)> {
    text.lines()
        .enumerate()
        .filter_map(|(n, l)| {
            lex_line(l)
                .into_iter()
                .find(|t| t.kind != TokKind::Whitespace)
                .filter(|t| t.kind == TokKind::LabelDef)
                .map(|t| (l[t.range].to_string(), n))
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn kinds(line: &str) -> Vec<(TokKind, String)> {
        lex_line(line)
            .into_iter()
            .filter(|t| t.kind != TokKind::Whitespace)
            .map(|t| (t.kind, line[t.range].to_string()))
            .collect()
    }

    #[test]
    fn lexes_instruction_line() {
        assert_eq!(
            kinds("LOOP: ADD R1, R1, #-1 ; dec"),
            vec![
                (TokKind::LabelDef, "LOOP".into()),
                (TokKind::Punct, ":".into()),
                (TokKind::Opcode, "ADD".into()),
                (TokKind::Register, "R1".into()),
                (TokKind::Punct, ",".into()),
                (TokKind::Register, "R1".into()),
                (TokKind::Punct, ",".into()),
                (TokKind::Number, "#-1".into()),
                (TokKind::Comment, "; dec".into()),
            ]
        );
    }

    #[test]
    fn lexes_strings_and_directives() {
        assert_eq!(
            kinds(r#"MSG .STRINGZ "a; \"b\"" "#),
            vec![
                (TokKind::LabelDef, "MSG".into()),
                (TokKind::Directive, ".STRINGZ".into()),
                (TokKind::String, r#""a; \"b\"""#.into()),
            ]
        );
        assert_eq!(kinds("  brz DONE")[1], (TokKind::Ident, "DONE".into()));
        assert_eq!(kinds("  PUTS")[0].0, TokKind::Trap);
    }

    #[test]
    fn numbers() {
        assert_eq!(parse_number("#10"), Some(10));
        assert_eq!(parse_number("#-10"), Some(-10));
        assert_eq!(parse_number("x3000"), Some(0x3000));
        assert_eq!(parse_number("xFFFF"), Some(0xFFFF));
        assert_eq!(parse_number("-5"), Some(-5));
        assert_eq!(parse_number("b101"), Some(5));
        assert_eq!(parse_number("x"), None);
        assert_eq!(parse_number("LOOP"), None);
        assert_eq!(parse_number("xLABEL"), None);
    }

    #[test]
    fn finds_labels() {
        let src = ".ORIG x3000\nLOOP ADD R0,R0,#1\n  BR LOOP\nEND: HALT\n";
        assert_eq!(
            label_definitions(src),
            vec![("LOOP".to_string(), 1), ("END".to_string(), 3)]
        );
    }
}
