//! Pure text transformations behind the editor's keyboard commands.
//!
//! Every operation takes the document and a selection as *character* indices (which is what egui
//! cursors use) and returns the new document plus the selection to apply afterwards.

use super::lexer::{lex_line, TokKind};

pub const INDENT: &str = "    ";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Sel {
    /// Fixed end of the selection (where shift-selection started)
    pub anchor: usize,
    /// Moving end of the selection (where the caret is drawn)
    pub head: usize,
}

impl Sel {
    pub fn caret(at: usize) -> Self {
        Self {
            anchor: at,
            head: at,
        }
    }
    pub fn new(anchor: usize, head: usize) -> Self {
        Self { anchor, head }
    }
    pub fn min(&self) -> usize {
        self.anchor.min(self.head)
    }
    pub fn max(&self) -> usize {
        self.anchor.max(self.head)
    }
    pub fn is_empty(&self) -> bool {
        self.anchor == self.head
    }
}

pub struct Edit {
    pub text: String,
    pub sel: Sel,
}

/// Byte offset of a char index.
pub fn byte_of(text: &str, ci: usize) -> usize {
    text.char_indices()
        .nth(ci)
        .map(|(b, _)| b)
        .unwrap_or(text.len())
}

/// Char index of a byte offset.
pub fn char_of(text: &str, bi: usize) -> usize {
    text[..bi.min(text.len())].chars().count()
}

/// Byte ranges of each line (without the trailing newline).
pub fn line_spans(text: &str) -> Vec<(usize, usize)> {
    let mut spans = Vec::new();
    let mut start = 0;
    for (i, b) in text.bytes().enumerate() {
        if b == b'\n' {
            spans.push((start, i));
            start = i + 1;
        }
    }
    spans.push((start, text.len()));
    spans
}

/// 0-based line containing a byte offset.
pub fn line_of_byte(text: &str, bi: usize) -> usize {
    text.as_bytes()[..bi.min(text.len())]
        .iter()
        .filter(|b| **b == b'\n')
        .count()
}

/// (line, column) of a char index, both 0-based, column in chars.
pub fn line_col(text: &str, ci: usize) -> (usize, usize) {
    let bi = byte_of(text, ci);
    let line = line_of_byte(text, bi);
    let line_start = text[..bi].rfind('\n').map(|p| p + 1).unwrap_or(0);
    (line, text[line_start..bi].chars().count())
}

/// Char index of the start of a 0-based line.
pub fn line_start_char(text: &str, line: usize) -> usize {
    let spans = line_spans(text);
    let (s, _) = spans[line.min(spans.len() - 1)];
    char_of(text, s)
}

/// Lines covered by a selection. A selection ending at column 0 of a line does not include it.
fn selected_lines(text: &str, sel: Sel) -> (usize, usize) {
    let a = byte_of(text, sel.min());
    let b = byte_of(text, sel.max());
    let first = line_of_byte(text, a);
    let mut last = line_of_byte(text, b);
    if b > a && last > first && text[..b].ends_with('\n') {
        last -= 1;
    }
    (first, last)
}

fn leading_ws(line: &str) -> &str {
    &line[..line.len() - line.trim_start().len()]
}

/// Rebuild the text replacing lines `first..=last` with `new_lines`.
fn replace_lines(text: &str, first: usize, last: usize, new_lines: &[String]) -> String {
    let lines: Vec<&str> = text.split('\n').collect();
    let mut out: Vec<String> = Vec::with_capacity(lines.len());
    out.extend(lines[..first].iter().map(|s| s.to_string()));
    out.extend(new_lines.iter().cloned());
    out.extend(lines[last + 1..].iter().map(|s| s.to_string()));
    out.join("\n")
}

/// Tab: indent selected lines, or insert spaces up to the next tab stop.
pub fn indent(text: &str, sel: Sel) -> Edit {
    let (first, last) = selected_lines(text, sel);
    if sel.is_empty() || first == last && !spans_whole_line(text, sel, first) {
        let (_, col) = line_col(text, sel.min());
        let pad = INDENT.len() - col % INDENT.len();
        let a = byte_of(text, sel.min());
        let b = byte_of(text, sel.max());
        let mut t = text.to_string();
        t.replace_range(a..b, &" ".repeat(pad));
        return Edit {
            text: t,
            sel: Sel::caret(sel.min() + pad),
        };
    }
    let lines: Vec<&str> = text.split('\n').collect();
    let new: Vec<String> = lines[first..=last]
        .iter()
        .map(|l| {
            if l.trim().is_empty() {
                l.to_string()
            } else {
                format!("{INDENT}{l}")
            }
        })
        .collect();
    let t = replace_lines(text, first, last, &new);
    whole_line_selection(t, first, last)
}

fn spans_whole_line(text: &str, sel: Sel, line: usize) -> bool {
    let spans = line_spans(text);
    let (s, e) = spans[line];
    byte_of(text, sel.min()) <= s && byte_of(text, sel.max()) >= e && e > s
}

fn whole_line_selection(text: String, first: usize, last: usize) -> Edit {
    let spans = line_spans(&text);
    let a = char_of(&text, spans[first].0);
    let b = char_of(&text, spans[last].1);
    Edit {
        sel: Sel::new(a, b),
        text,
    }
}

/// Shift+Tab: remove one indent level from the selected lines.
pub fn outdent(text: &str, sel: Sel) -> Edit {
    let (first, last) = selected_lines(text, sel);
    let lines: Vec<&str> = text.split('\n').collect();
    let mut removed_first = 0;
    let new: Vec<String> = lines[first..=last]
        .iter()
        .enumerate()
        .map(|(i, l)| {
            let n = if l.starts_with('\t') {
                1
            } else {
                l.bytes()
                    .take(INDENT.len())
                    .take_while(|b| *b == b' ')
                    .count()
            };
            if i == 0 {
                removed_first = n;
            }
            l[n..].to_string()
        })
        .collect();
    let t = replace_lines(text, first, last, &new);
    if sel.is_empty() {
        let (_, col) = line_col(text, sel.head);
        let start = line_start_char(&t, first);
        return Edit {
            sel: Sel::caret(start + col.saturating_sub(removed_first)),
            text: t,
        };
    }
    whole_line_selection(t, first, last)
}

/// Ctrl+/: comment or uncomment the selected lines.
pub fn toggle_comment(text: &str, sel: Sel) -> Edit {
    let (first, last) = selected_lines(text, sel);
    let lines: Vec<&str> = text.split('\n').collect();
    let block = &lines[first..=last];
    let non_empty: Vec<&&str> = block.iter().filter(|l| !l.trim().is_empty()).collect();
    let all_commented =
        !non_empty.is_empty() && non_empty.iter().all(|l| l.trim_start().starts_with(';'));
    let min_indent = non_empty
        .iter()
        .map(|l| leading_ws(l).len())
        .min()
        .unwrap_or(0);
    let mut delta_first: isize = 0;
    let new: Vec<String> = block
        .iter()
        .enumerate()
        .map(|(i, l)| {
            let (out, d) = if l.trim().is_empty() {
                (l.to_string(), 0)
            } else if all_commented {
                let ws = leading_ws(l);
                let rest = &l[ws.len() + 1..];
                let rest2 = rest.strip_prefix(' ').unwrap_or(rest);
                let d = (l.len() - ws.len() - rest2.len()) as isize;
                (format!("{ws}{rest2}"), -d)
            } else {
                (format!("{}; {}", &l[..min_indent], &l[min_indent..]), 2)
            };
            if i == 0 {
                delta_first = d;
            }
            out
        })
        .collect();
    let t = replace_lines(text, first, last, &new);
    if sel.is_empty() {
        let new_head = (sel.head as isize + delta_first).max(line_start_char(&t, first) as isize);
        return Edit {
            text: t,
            sel: Sel::caret(new_head as usize),
        };
    }
    whole_line_selection(t, first, last)
}

/// Ctrl+D: duplicate the current line, or the selected text.
pub fn duplicate(text: &str, sel: Sel) -> Edit {
    if !sel.is_empty() {
        let a = byte_of(text, sel.min());
        let b = byte_of(text, sel.max());
        let chunk = &text[a..b];
        let mut t = text.to_string();
        t.insert_str(b, chunk);
        let n = chunk.chars().count();
        return Edit {
            text: t,
            sel: Sel::new(sel.max(), sel.max() + n),
        };
    }
    let (first, last) = selected_lines(text, sel);
    let lines: Vec<&str> = text.split('\n').collect();
    let mut new: Vec<String> = lines[first..=last].iter().map(|s| s.to_string()).collect();
    new.extend(lines[first..=last].iter().map(|s| s.to_string()));
    let t = replace_lines(text, first, last, &new);
    let (_, col) = line_col(text, sel.head);
    let start = line_start_char(&t, last + 1);
    Edit {
        text: t,
        sel: Sel::caret(start + col),
    }
}

/// Alt+Up / Alt+Down: move the selected lines.
pub fn move_lines(text: &str, sel: Sel, up: bool) -> Edit {
    let (first, last) = selected_lines(text, sel);
    let mut lines: Vec<String> = text.split('\n').map(str::to_string).collect();
    if (up && first == 0) || (!up && last + 1 >= lines.len()) {
        return Edit {
            text: text.to_string(),
            sel,
        };
    }
    let (head_line, head_col) = line_col(text, sel.head);
    let (anchor_line, anchor_col) = line_col(text, sel.anchor);
    if up {
        let moved = lines.remove(first - 1);
        lines.insert(last, moved);
    } else {
        let moved = lines.remove(last + 1);
        lines.insert(first, moved);
    }
    let t = lines.join("\n");
    let shift = |l: usize| if up { l - 1 } else { l + 1 };
    let head = line_start_char(&t, shift(head_line)) + head_col;
    let anchor = line_start_char(&t, shift(anchor_line)) + anchor_col;
    Edit {
        text: t,
        sel: Sel::new(anchor, head),
    }
}

/// Ctrl+Shift+K: delete the selected lines.
pub fn delete_lines(text: &str, sel: Sel) -> Edit {
    let (first, last) = selected_lines(text, sel);
    let mut lines: Vec<&str> = text.split('\n').collect();
    lines.drain(first..=last);
    if lines.is_empty() {
        lines.push("");
    }
    let t = lines.join("\n");
    let line = first.min(lines.len() - 1);
    Edit {
        sel: Sel::caret(line_start_char(&t, line)),
        text: t,
    }
}

/// Enter: newline that keeps the indentation of the current line. After a line that starts with a
/// label, the new line is indented to the instruction column.
pub fn newline(text: &str, sel: Sel) -> Edit {
    let a = byte_of(text, sel.min());
    let b = byte_of(text, sel.max());
    let line_start = text[..a].rfind('\n').map(|p| p + 1).unwrap_or(0);
    let line = &text[line_start..a];
    let mut indent = leading_ws(line).to_string();
    if indent.is_empty() {
        let toks = lex_line(line);
        if let Some(first) = toks.first() {
            if first.kind == TokKind::LabelDef {
                // indent to the first token after the label (and its colon), or one level
                let after = toks.iter().skip(1).find(|t| {
                    t.kind != TokKind::Whitespace
                        && !(t.kind == TokKind::Punct && &line[t.range.clone()] == ":")
                });
                indent = match after {
                    Some(t) if t.kind != TokKind::Comment => " ".repeat(t.range.start),
                    _ => INDENT.to_string(),
                };
            }
        }
    }
    let insert = format!("\n{indent}");
    let mut t = text.to_string();
    t.replace_range(a..b, &insert);
    Edit {
        sel: Sel::caret(sel.min() + insert.chars().count()),
        text: t,
    }
}

/// Align labels, mnemonics, operands and comments into columns.
pub fn format_document(text: &str) -> String {
    struct Parts {
        label: String,
        op: String,
        operands: Vec<String>,
        comment: Option<String>,
        indent_comment: Option<String>,
    }
    let mut parsed = Vec::new();
    for line in text.split('\n') {
        let toks = lex_line(line);
        let mut p = Parts {
            label: String::new(),
            op: String::new(),
            operands: Vec::new(),
            comment: None,
            indent_comment: None,
        };
        let mut words = toks
            .iter()
            .filter(|t| t.kind != TokKind::Whitespace)
            .peekable();
        if let Some(t) = words.peek() {
            if t.kind == TokKind::Comment {
                // A comment on its own line keeps its own indentation
                p.indent_comment = Some(line.trim_end().to_string());
                parsed.push(p);
                continue;
            }
            if t.kind == TokKind::LabelDef {
                p.label = line[t.range.clone()].to_string();
                words.next();
                if let Some(c) = words.peek() {
                    if c.kind == TokKind::Punct && &line[c.range.clone()] == ":" {
                        p.label.push(':');
                        words.next();
                    }
                }
            }
        }
        for t in words {
            let s = &line[t.range.clone()];
            match t.kind {
                TokKind::Comment => p.comment = Some(s.trim_end().to_string()),
                TokKind::Punct if s == "," => {}
                _ if p.op.is_empty() => p.op = s.to_string(),
                _ => p.operands.push(s.to_string()),
            }
        }
        parsed.push(p);
    }

    let label_w = parsed
        .iter()
        .map(|p| p.label.len())
        .max()
        .unwrap_or(0)
        .clamp(7, 20)
        + 1;
    let code_w = parsed
        .iter()
        .filter(|p| p.comment.is_some() && !p.op.is_empty())
        .map(|p| 9 + p.operands.join(", ").len())
        .max()
        .unwrap_or(0)
        .min(32);

    parsed
        .into_iter()
        .map(|p| {
            if let Some(c) = p.indent_comment {
                return c;
            }
            let mut out = String::new();
            if p.op.is_empty() {
                out.push_str(&p.label);
            } else {
                out.push_str(&format!("{:<label_w$}", p.label));
                let code = if p.operands.is_empty() {
                    p.op.clone()
                } else {
                    format!("{:<8} {}", p.op, p.operands.join(", "))
                };
                out.push_str(&code);
            }
            if let Some(c) = p.comment {
                if out.trim().is_empty() {
                    out = c;
                } else {
                    let pad = (label_w + code_w + 1).saturating_sub(out.len()).max(1);
                    out.push_str(&" ".repeat(pad));
                    out.push_str(&c);
                }
            }
            out.trim_end().to_string()
        })
        .collect::<Vec<_>>()
        .join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn indent_and_outdent_lines() {
        let t = "A\nB\nC";
        let e = indent(t, Sel::new(0, 3));
        assert_eq!(e.text, "    A\n    B\nC");
        let e2 = outdent(&e.text, e.sel);
        assert_eq!(e2.text, t);
    }

    #[test]
    fn tab_inserts_to_tab_stop() {
        let e = indent("AB", Sel::caret(2));
        assert_eq!(e.text, "AB  ");
        assert_eq!(e.sel, Sel::caret(4));
    }

    #[test]
    fn comments_toggle() {
        let t = "    ADD R0, R0, #1\n    HALT";
        let e = toggle_comment(t, Sel::new(0, t.chars().count()));
        assert_eq!(e.text, "    ; ADD R0, R0, #1\n    ; HALT");
        let e2 = toggle_comment(&e.text, e.sel);
        assert_eq!(e2.text, t);
    }

    #[test]
    fn duplicate_and_move() {
        let t = "A\nB";
        let e = duplicate(t, Sel::caret(0));
        assert_eq!(e.text, "A\nA\nB");
        assert_eq!(e.sel, Sel::caret(2));
        let m = move_lines("A\nB\nC", Sel::caret(2), true);
        assert_eq!(m.text, "B\nA\nC");
        assert_eq!(m.sel, Sel::caret(0));
        let m2 = move_lines("A\nB\nC", Sel::caret(2), false);
        assert_eq!(m2.text, "A\nC\nB");
    }

    #[test]
    fn delete_line() {
        let e = delete_lines("A\nB\nC", Sel::caret(2));
        assert_eq!(e.text, "A\nC");
    }

    #[test]
    fn newline_indents_after_label() {
        let t = "LOOP ADD R0, R0, #1";
        let e = newline(t, Sel::caret(t.len()));
        assert_eq!(e.text, "LOOP ADD R0, R0, #1\n     ");
        let t2 = "    ADD R0, R0, #1";
        let e2 = newline(t2, Sel::caret(t2.len()));
        assert_eq!(e2.text, "    ADD R0, R0, #1\n    ");
        let t3 = "DONE:";
        assert_eq!(newline(t3, Sel::caret(5)).text, "DONE:\n    ");
    }

    #[test]
    fn formats_columns_and_still_assembles() {
        let src = ".ORIG x3000\nLOOP: ADD R1,R1,#1 ; inc\n  BRp LOOP\n; note\nMSG .STRINGZ \"a, b\"\nHALT\n.END";
        let out = format_document(src);
        assert!(out.contains("LOOP:   ADD      R1, R1, #1"), "{out}");
        assert!(out.contains("\"a, b\""));
        assert!(out.contains("\n; note\n"));
        let a = crate::emulator::Emulator::parse_program(src, None).expect("src ok");
        let b = crate::emulator::Emulator::parse_program(&out, None).expect("formatted ok");
        assert_eq!(a.machine_code, b.machine_code);
    }
}
