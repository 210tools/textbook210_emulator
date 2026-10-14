//! The code editor widget: an egui `TextEdit` with a gutter, syntax highlighting, completion,
//! hover cards, find/replace and the usual editing shortcuts layered on top.

use crate::panes::glossary::Explain;
use std::collections::{BTreeSet, HashMap};
use std::sync::Arc;

use egui::text::{CCursor, CCursorRange, LayoutJob, TextFormat};
use egui::{
    Align2, Color32, CornerRadius, Event, FontId, Galley, Id, Key, Modifiers, Pos2, Rect, RichText,
    Sense, Shape, Stroke, TextBuffer, Ui, Vec2,
};

use super::docs;
use super::edit_ops::{self, line_col, line_start_char, Edit, Sel};
use super::lexer::{self, lex_line, TokKind};
use crate::emulator::Emulator;
use crate::theme::ThemeSettings;

/// A problem found by assembling the document in the background.
#[derive(Debug, Clone, PartialEq)]
pub struct Diagnostic {
    /// 0-based line
    pub line: usize,
    pub message: String,
}

/// Information from the rest of the app the editor needs to draw itself.
pub struct EditorEnv<'a> {
    pub theme: &'a ThemeSettings,
    pub emulator: &'a Emulator,
    pub diagnostics: &'a [Diagnostic],
    /// 0-based line that the PC points at, if this document is the loaded program
    pub pc_line: Option<usize>,
    /// 0-based line -> address, if this document is the loaded program
    pub line_addresses: Option<HashMap<usize, usize>>,
    pub show_addresses: bool,
    /// Show the text for inspection only (the OS source)
    pub read_only: bool,
    /// Lines to mark because a highlighted register was last written (true) or read there
    pub register_marks: Vec<RegisterMark>,
    /// Draw arrows from branches and calls to their targets
    pub show_arrows: bool,
    /// Small notes drawn after the end of lines (values, inputs, register history)
    pub inlays: Vec<Inlay>,
}

/// A note drawn after the end of a line.
#[derive(Debug, Clone)]
pub struct Inlay {
    pub line: usize,
    pub text: String,
    pub kind: InlayKind,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum InlayKind {
    /// What the instruction at the PC is about to read
    Input,
    /// The current value of a register this line last wrote
    Value,
    /// Hovered register: last written here
    Written,
    /// Hovered register: last read here
    Read,
}

/// A line where a register was most recently read or written.
#[derive(Debug, Clone)]
pub struct RegisterMark {
    pub line: usize,
    pub write: bool,
    pub label: String,
}

/// A branch or call from one line to the line its label is defined on.
#[derive(Debug, Clone, PartialEq)]
struct Arrow {
    from: usize,
    to: usize,
    lane: usize,
    call: bool,
}

/// Find branch/call arrows and give each a lane so overlapping ones don't share a column.
fn branch_arrows(text: &str) -> Vec<Arrow> {
    let defs: HashMap<String, usize> = lexer::label_definitions(text)
        .into_iter()
        .map(|(l, n)| (l.to_ascii_uppercase(), n))
        .collect();
    let mut arrows: Vec<Arrow> = Vec::new();
    for (n, line) in text.split('\n').enumerate() {
        let toks: Vec<_> = lex_line(line)
            .into_iter()
            .filter(|t| {
                !matches!(
                    t.kind,
                    TokKind::Whitespace | TokKind::Punct | TokKind::Comment
                )
            })
            .collect();
        let Some(op_i) = toks.iter().position(|t| t.kind == TokKind::Opcode) else {
            continue;
        };
        let op = line[toks[op_i].range.clone()].to_ascii_uppercase();
        let is_branch = op.starts_with("BR");
        let is_call = op == "JSR";
        if !is_branch && !is_call {
            continue;
        }
        let Some(target) = toks
            .get(op_i + 1)
            .filter(|t| matches!(t.kind, TokKind::Ident | TokKind::LabelDef))
        else {
            continue;
        };
        if let Some(&to) = defs.get(&line[target.range.clone()].to_ascii_uppercase()) {
            if to != n {
                arrows.push(Arrow {
                    from: n,
                    to,
                    lane: 0,
                    call: is_call,
                });
            }
        }
    }
    // shortest spans get the innermost lanes
    arrows.sort_by_key(|a| a.from.abs_diff(a.to));
    let mut lanes: Vec<Vec<(usize, usize)>> = Vec::new();
    for a in &mut arrows {
        let (lo, hi) = (a.from.min(a.to), a.from.max(a.to));
        let lane = lanes
            .iter()
            .position(|spans| spans.iter().all(|(l, h)| hi < *l || lo > *h))
            .unwrap_or_else(|| {
                lanes.push(Vec::new());
                lanes.len() - 1
            });
        lanes[lane].push((lo, hi));
        a.lane = lane;
    }
    arrows
}

/// What happened in the editor this frame.
#[derive(Default)]
pub struct EditorOutput {
    pub changed: bool,
    /// 0-based lines whose breakpoint was toggled by clicking the gutter
    pub toggled_breakpoints: Vec<usize>,
    /// (line, column), both 1-based, for the status bar
    pub cursor: (usize, usize),
    /// First and last selected line, 0-based
    pub sel_lines: (usize, usize),
    pub focused: bool,
}

#[derive(Default, Clone)]
struct FindState {
    open: bool,
    show_replace: bool,
    query: String,
    replacement: String,
    case_sensitive: bool,
    focus_query: bool,
    current: usize,
}

#[derive(Clone)]
struct CompletionItem {
    label: String,
    insert: String,
    detail: String,
    doc: String,
    kind: CompletionKind,
}

#[derive(Clone, Copy, PartialEq)]
enum CompletionKind {
    Instruction,
    Directive,
    Register,
    Label,
}

#[derive(Clone)]
struct Completion {
    items: Vec<CompletionItem>,
    selected: usize,
    /// char index where the word being completed starts
    word_start: usize,
    /// The user moved through the list with the arrow keys
    navigated: bool,
}

/// Editor state that only lives for the session (not saved with the document).
#[derive(Default, Clone)]
pub struct EditorUiState {
    find: FindState,
    goto_line: Option<String>,
    completion: Option<Completion>,
    /// Scroll so this char index is visible next frame
    scroll_to: Option<usize>,
    /// Center the scroll target (for jumps) rather than just bringing it into view
    center_scroll: bool,
    /// Request keyboard focus for the text area next frame
    focus_text: bool,
    last_pc_line: Option<usize>,
}

impl EditorUiState {
    pub fn open_find(&mut self, replace: bool) {
        self.find.open = true;
        self.find.show_replace = replace;
        self.find.focus_query = true;
        self.goto_line = None;
    }
    pub fn open_goto(&mut self) {
        self.goto_line = Some(String::new());
        self.find.open = false;
    }
    pub fn focus(&mut self) {
        self.focus_text = true;
    }
    pub fn reveal_line(&mut self, text: &str, line: usize) {
        self.scroll_to = Some(line_start_char(text, line));
        self.center_scroll = true;
    }
}

/// Commands that change the text, triggered by shortcuts or menus.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum TextCommand {
    Indent,
    Outdent,
    ToggleComment,
    Duplicate,
    MoveUp,
    MoveDown,
    DeleteLine,
    Newline,
    Format,
}

fn text_edit_id(doc_id: u64) -> Id {
    Id::new(("lc3_code_editor", doc_id))
}

fn load_sel(ctx: &egui::Context, id: Id) -> Option<Sel> {
    let state = egui::TextEdit::load_state(ctx, id)?;
    let r = state.cursor.char_range()?;
    Some(Sel::new(r.secondary.index, r.primary.index))
}

fn store_sel(ctx: &egui::Context, id: Id, sel: Sel) {
    let mut state = egui::TextEdit::load_state(ctx, id).unwrap_or_default();
    state.cursor.set_char_range(Some(CCursorRange::two(
        CCursor::new(sel.anchor),
        CCursor::new(sel.head),
    )));
    state.store(ctx, id);
}

/// Put the caret at the start of a 0-based line.
pub fn select_line(ctx: &egui::Context, doc_id: u64, text: &str, line: usize) {
    store_sel(
        ctx,
        text_edit_id(doc_id),
        Sel::caret(line_start_char(text, line)),
    );
}

/// Apply a text command to `text` using the editor's current selection.
pub fn apply_command(ctx: &egui::Context, doc_id: u64, text: &mut String, cmd: TextCommand) {
    let id = text_edit_id(doc_id);
    let sel = load_sel(ctx, id).unwrap_or(Sel::caret(0));
    let edit: Edit = match cmd {
        TextCommand::Indent => edit_ops::indent(text, sel),
        TextCommand::Outdent => edit_ops::outdent(text, sel),
        TextCommand::ToggleComment => edit_ops::toggle_comment(text, sel),
        TextCommand::Duplicate => edit_ops::duplicate(text, sel),
        TextCommand::MoveUp => edit_ops::move_lines(text, sel, true),
        TextCommand::MoveDown => edit_ops::move_lines(text, sel, false),
        TextCommand::DeleteLine => edit_ops::delete_lines(text, sel),
        TextCommand::Newline => edit_ops::newline(text, sel),
        TextCommand::Format => {
            let (line, col) = line_col(text, sel.head);
            let t = edit_ops::format_document(text);
            let head = line_start_char(&t, line) + col;
            Edit {
                sel: Sel::caret(head.min(t.chars().count())),
                text: t,
            }
        }
    };
    *text = edit.text;
    store_sel(ctx, id, edit.sel);
}

fn token_color(theme: &ThemeSettings, kind: TokKind) -> Color32 {
    match kind {
        TokKind::Whitespace | TokKind::Punct => theme.secondary_text_color,
        TokKind::Comment => theme.editor_comment_color,
        TokKind::Opcode => theme.editor_opcode_color,
        TokKind::Trap => theme.editor_opcode_color,
        TokKind::Register => theme.editor_register_color,
        TokKind::Directive => theme.editor_directive_color,
        TokKind::Number => theme.editor_literal_color,
        TokKind::String => theme.editor_string_color,
        TokKind::LabelDef => theme.editor_label_color,
        TokKind::Ident => theme.editor_label_color.gamma_multiply(0.85),
    }
}

fn highlight(text: &str, font: &FontId, theme: &ThemeSettings) -> LayoutJob {
    let mut job = LayoutJob::default();
    job.wrap.max_width = f32::INFINITY;
    let mut first = true;
    for line in text.split('\n') {
        if !first {
            job.append(
                "\n",
                0.0,
                TextFormat::simple(font.clone(), theme.primary_text_color),
            );
        }
        first = false;
        for tok in lex_line(line) {
            let mut fmt = TextFormat::simple(font.clone(), token_color(theme, tok.kind));
            if tok.kind == TokKind::Comment {
                fmt.italics = true;
            }
            job.append(&line[tok.range], 0.0, fmt);
        }
    }
    job
}

/// Screen rect of the chars `[a, b)` on one row of the galley.
fn span_rect(galley: &Galley, origin: Pos2, a: usize, b: usize) -> Rect {
    let ra = galley.pos_from_cursor(CCursor::new(a));
    let rb = galley.pos_from_cursor(CCursor::new(b));
    Rect::from_min_max(ra.min, Pos2::new(rb.max.x.max(ra.min.x + 2.0), ra.max.y))
        .translate(origin.to_vec2())
}

/// The galley row (= source line, as nothing wraps) at screen height `y`.
fn line_at_y(galley: &Galley, origin: Pos2, y: f32) -> Option<usize> {
    let rel = y - origin.y;
    if rel < 0.0 {
        return None;
    }
    galley.rows.iter().position(|r| rel < r.rect().max.y)
}

fn line_rect(galley: &Galley, origin: Pos2, line: usize, width: f32) -> Option<Rect> {
    let row = galley.rows.get(line)?;
    let r = row.rect().translate(origin.to_vec2());
    Some(Rect::from_min_size(r.min, Vec2::new(width, r.height())))
}

/// All matches of `query` as char ranges.
fn find_matches(text: &str, query: &str, case_sensitive: bool) -> Vec<(usize, usize)> {
    if query.is_empty() {
        return Vec::new();
    }
    let (hay, needle) = if case_sensitive {
        (text.to_string(), query.to_string())
    } else {
        (text.to_lowercase(), query.to_lowercase())
    };
    // lowercase can change byte lengths for non ascii text, fall back to exact search then
    let (hay, needle) = if hay.len() == text.len() {
        (hay, needle)
    } else {
        (text.to_string(), query.to_string())
    };
    let mut out = Vec::new();
    let mut from = 0;
    while let Some(pos) = hay[from..].find(&needle) {
        let b = from + pos;
        let a_c = edit_ops::char_of(text, b);
        out.push((a_c, a_c + needle.chars().count()));
        from = b + needle.len().max(1);
        if out.len() > 10_000 {
            break;
        }
    }
    out
}

/// The char column of the glyph under `x` (relative to the galley) on a line.
fn col_at_x(galley: &Galley, line: usize, x: f32) -> Option<usize> {
    let placed = galley.rows.get(line)?;
    let x = x - placed.pos.x;
    placed
        .row
        .glyphs
        .iter()
        .position(|g| x >= g.pos.x && x < g.pos.x + g.advance_width)
}

/// The x of a char column's left edge on a line, relative to the galley.
fn x_of_col(galley: &Galley, line: usize, col: usize) -> f32 {
    let Some(placed) = galley.rows.get(line) else {
        return 0.0;
    };
    let glyphs = &placed.row.glyphs;
    placed.pos.x
        + match glyphs.get(col) {
            Some(g) => g.pos.x,
            None => glyphs.last().map_or(0.0, |g| g.pos.x + g.advance_width),
        }
}

/// Whole-word occurrences of `word` as char ranges.
fn word_occurrences(text: &str, word: &str) -> Vec<(usize, usize)> {
    let mut out = Vec::new();
    let mut char_base = 0;
    for line in text.split('\n') {
        for tok in lex_line(line) {
            if matches!(
                tok.kind,
                TokKind::LabelDef | TokKind::Ident | TokKind::Register
            ) && line[tok.range.clone()].eq_ignore_ascii_case(word)
            {
                let a = char_base + line[..tok.range.start].chars().count();
                out.push((a, a + word.chars().count()));
            }
        }
        char_base += line.chars().count() + 1;
    }
    out
}

/// The identifier ending at char index `ci` (for completion): (start char, word).
fn word_before(text: &str, ci: usize) -> (usize, String) {
    let chars: Vec<char> = text.chars().collect();
    let mut s = ci.min(chars.len());
    while s > 0
        && (chars[s - 1].is_ascii_alphanumeric() || chars[s - 1] == '_' || chars[s - 1] == '.')
    {
        s -= 1;
    }
    (s, chars[s..ci.min(chars.len())].iter().collect())
}

fn build_completion(text: &str, caret: usize) -> Option<Completion> {
    let (start, prefix) = word_before(text, caret);
    let (line, _) = line_col(text, caret);
    let line_text = text.split('\n').nth(line).unwrap_or("");
    let line_start = line_start_char(text, line);
    // which word of the line are we in?
    let before: String = line_text.chars().take(start - line_start).collect();
    let toks: Vec<_> = lex_line(&before)
        .into_iter()
        .filter(|t| t.kind != TokKind::Whitespace && t.kind != TokKind::Punct)
        .collect();
    if toks
        .iter()
        .any(|t| t.kind == TokKind::Comment || t.kind == TokKind::String)
    {
        return None;
    }
    let has_op = toks
        .iter()
        .any(|t| matches!(t.kind, TokKind::Opcode | TokKind::Trap | TokKind::Directive));
    let only_label_before = toks.len() == 1 && toks[0].kind == TokKind::LabelDef;
    let mnemonic_position = toks.is_empty() || only_label_before;

    let p = prefix.to_ascii_uppercase();
    let mut items = Vec::new();
    if mnemonic_position {
        for d in docs::INSTRUCTIONS {
            if d.name.starts_with(&p) {
                items.push(CompletionItem {
                    label: d.name.to_string(),
                    insert: d.snippet.to_string(),
                    detail: d.operands.to_string(),
                    doc: d.summary.to_string(),
                    kind: CompletionKind::Instruction,
                });
            }
        }
        for d in docs::DIRECTIVES {
            if d.name.starts_with(&p) || d.name[1..].starts_with(&p) {
                items.push(CompletionItem {
                    label: d.name.to_string(),
                    insert: d.snippet.to_string(),
                    detail: d.operands.to_string(),
                    doc: d.summary.to_string(),
                    kind: CompletionKind::Directive,
                });
            }
        }
    }
    if has_op && !prefix.is_empty() {
        let last_op = toks
            .iter()
            .find(|t| matches!(t.kind, TokKind::Opcode | TokKind::Trap | TokKind::Directive))
            .map(|t| before[t.range.clone()].to_ascii_uppercase())
            .unwrap_or_default();
        let wants_registers = !matches!(
            last_op.as_str(),
            ".ORIG" | ".STRINGZ" | ".BLKW" | ".FILL" | "TRAP"
        );
        if wants_registers {
            for r in 0..8 {
                let name = format!("R{r}");
                if name.starts_with(&p) && name != p {
                    items.push(CompletionItem {
                        insert: format!("{name}$"),
                        label: name,
                        detail: "register".into(),
                        doc: String::new(),
                        kind: CompletionKind::Register,
                    });
                }
            }
        }
    }
    if has_op || !mnemonic_position || !prefix.is_empty() {
        for (label, line_no) in lexer::label_definitions(text) {
            if label.to_ascii_uppercase().starts_with(&p)
                && label.len() > prefix.len()
                && (has_op || mnemonic_position)
            {
                items.push(CompletionItem {
                    insert: format!("{label}$"),
                    label,
                    detail: format!("line {}", line_no + 1),
                    doc: String::new(),
                    kind: CompletionKind::Label,
                });
            }
        }
    }
    // the exact word already typed comes first, so Enter on a complete word keeps it
    items.sort_by_key(|i| !i.label.eq_ignore_ascii_case(&prefix));
    // nothing to offer if the only suggestion is the word as typed
    if let [only] = items.as_slice() {
        if only.label.eq_ignore_ascii_case(&prefix)
            && only.insert.replace('$', "").eq_ignore_ascii_case(&prefix)
        {
            return None;
        }
    }
    if items.is_empty() || prefix.is_empty() && !has_op && !mnemonic_position {
        return None;
    }
    items.truncate(40);
    Some(Completion {
        items,
        selected: 0,
        word_start: start,
        navigated: false,
    })
}

fn accept_completion(
    ctx: &egui::Context,
    id: Id,
    text: &mut String,
    comp: &Completion,
    caret: usize,
) {
    let item = &comp.items[comp.selected.min(comp.items.len() - 1)];
    let (insert, caret_off) = match item.insert.find('$') {
        Some(i) => (
            item.insert.replacen('$', "", 1),
            item.insert[..i].chars().count(),
        ),
        None => (item.insert.clone(), item.insert.chars().count()),
    };
    let a = edit_ops::byte_of(text, comp.word_start);
    let b = edit_ops::byte_of(text, caret);
    text.replace_range(a..b, &insert);
    store_sel(ctx, id, Sel::caret(comp.word_start + caret_off));
}

#[allow(clippy::too_many_arguments)]
fn hover_card(
    ui: &mut Ui,
    env: &EditorEnv<'_>,
    text: &str,
    line: usize,
    col_byte: usize,
    line_addr: Option<usize>,
) -> bool {
    let line_text = text.split('\n').nth(line).unwrap_or("");
    let Some(tok) = lexer::token_at(line_text, col_byte) else {
        return false;
    };
    let word = &line_text[tok.range.clone()];
    let theme = env.theme;
    let mono = |s: String| RichText::new(s).monospace();
    match tok.kind {
        TokKind::Opcode | TokKind::Trap => {
            let Some(d) = docs::instruction(word) else {
                return false;
            };
            ui.set_max_width(380.0);
            ui.label(
                mono(format!("{} {}", d.name, d.operands))
                    .strong()
                    .color(theme.editor_opcode_color),
            );
            ui.label(d.summary);
            ui.label(mono(d.rtl.to_string()).color(theme.secondary_text_color));
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = 2.0;
                let colors = [
                    theme.editor_opcode_color,
                    theme.editor_register_color,
                    theme.editor_literal_color,
                    theme.editor_label_color,
                    theme.editor_directive_color,
                ];
                for (i, (bits, _)) in d.encoding.iter().enumerate() {
                    let c = colors[i % colors.len()];
                    egui::Frame::new()
                        .fill(c.gamma_multiply(0.18))
                        .corner_radius(CornerRadius::same(2))
                        .inner_margin(egui::Margin::symmetric(4, 1))
                        .show(ui, |ui| {
                            ui.label(mono(bits.to_string()).color(c).size(11.5));
                        });
                }
            });
            if d.sets_cc {
                ui.label(
                    RichText::new("Sets condition codes N Z P")
                        .small()
                        .color(theme.secondary_text_color),
                );
            }
            if let Some((addr, v)) =
                line_addr.and_then(|a| Some((a, env.emulator.memory.get(a)?.get())))
            {
                ui.label(
                    RichText::new(format!("This line: x{v:04X} at x{addr:04X}"))
                        .small()
                        .color(theme.secondary_text_color),
                );
            }
        }
        TokKind::Directive => {
            let Some(d) = docs::directive(word) else {
                return false;
            };
            ui.label(
                mono(format!("{} {}", d.name, d.operands))
                    .strong()
                    .color(theme.editor_directive_color),
            );
            ui.label(d.summary);
        }
        TokKind::Register => {
            let n = (word.as_bytes()[1] - b'0') as usize;
            super::hover_register(n);
            let v = env.emulator.r[n].get();
            ui.label(
                mono(format!("R{n}"))
                    .strong()
                    .color(theme.editor_register_color),
            );
            ui.label(mono(format!("x{v:04X}   {}   {}", v, v as i16)));
            ui.label(
                mono(format!(
                    "{:04b} {:04b} {:04b} {:04b}",
                    v >> 12,
                    (v >> 8) & 15,
                    (v >> 4) & 15,
                    v & 15
                ))
                .color(theme.secondary_text_color),
            );
            register_history(ui, env.emulator, n, theme);
        }
        TokKind::Number => {
            let Some(v) = lexer::parse_number(word) else {
                return false;
            };
            let u = v as u16;
            ui.label(
                mono(format!("{v}"))
                    .strong()
                    .color(theme.editor_literal_color),
            );
            ui.label(mono(format!("x{u:04X}   #{}   unsigned {}", u as i16, u)));
            ui.label(
                mono(format!(
                    "{:04b} {:04b} {:04b} {:04b}",
                    u >> 12,
                    (u >> 8) & 15,
                    (u >> 4) & 15,
                    u & 15
                ))
                .color(theme.secondary_text_color),
            );
            if (32..127).contains(&v) {
                ui.label(
                    mono(format!("ASCII '{}'", v as u8 as char)).color(theme.secondary_text_color),
                );
            }
        }
        TokKind::LabelDef | TokKind::Ident => {
            let defs = lexer::label_definitions(text);
            let def = defs.iter().find(|(l, _)| l.eq_ignore_ascii_case(word));
            ui.label(
                mono(word.to_string())
                    .strong()
                    .color(theme.editor_label_color),
            );
            match def {
                Some((_, l)) => {
                    let src = text.split('\n').nth(*l).unwrap_or("").trim();
                    ui.label(
                        RichText::new(format!("Defined on line {}", l + 1))
                            .color(theme.secondary_text_color),
                    );
                    ui.label(mono(src.to_string()).size(12.0));
                }
                None => {
                    ui.label(RichText::new("Not defined in this file").color(theme.error_fg_color));
                }
            }
            let label = env
                .emulator
                .metadata
                .labels
                .get(word)
                .or_else(|| env.emulator.metadata.labels.get(&word.to_ascii_uppercase()));
            // (a label after the last word of memory has no word)
            if let Some((addr, v)) =
                label.and_then(|a| Some((a, env.emulator.memory.get(*a)?.get())))
            {
                ui.label(
                    mono(format!(
                        "Address x{addr:04X}   value x{v:04X} ({})",
                        v as i16
                    ))
                    .color(theme.secondary_text_color),
                );
            }
            if def.is_some() && tok.kind == TokKind::Ident {
                ui.label(
                    RichText::new("Ctrl+click to go to definition")
                        .small()
                        .color(theme.secondary_text_color),
                );
            }
        }
        _ => return false,
    }
    true
}

/// "Last written by …" / "last read by …" lines for a register.
pub fn register_history(ui: &mut Ui, emulator: &Emulator, r: usize, theme: &ThemeSettings) {
    for (what, acc, color) in [
        (
            "written",
            emulator.reg_writes[r],
            theme.accent_color_primary,
        ),
        (
            "read",
            emulator.reg_reads[r].get(),
            theme.accent_color_secondary,
        ),
    ] {
        let Some(a) = acc else {
            ui.label(
                RichText::new(format!("Not {what} yet"))
                    .small()
                    .color(theme.secondary_text_color),
            );
            continue;
        };
        let ago = emulator.instructions.saturating_sub(a.instruction);
        let place = match emulator.source_line(a.addr) {
            Some((file, line, src)) => format!(
                "{file} line {}: {}",
                line + 1,
                src.split(';')
                    .next()
                    .unwrap_or("")
                    .split_whitespace()
                    .collect::<Vec<_>>()
                    .join(" ")
            ),
            None => format!("x{:04X}", a.addr),
        };
        ui.label(
            RichText::new(format!("Last {what} {ago} instr. ago · {place}"))
                .small()
                .color(color),
        );
    }
}

/// Draw the editor for one document.
#[allow(clippy::too_many_lines)]
pub fn show(
    ui: &mut Ui,
    doc_id: u64,
    text: &mut String,
    breakpoints: &mut BTreeSet<usize>,
    state: &mut EditorUiState,
    env: &EditorEnv<'_>,
) -> EditorOutput {
    let mut out = EditorOutput::default();
    let theme = env.theme;
    let id = text_edit_id(doc_id);
    let ctx = ui.ctx().clone();
    let font = egui::TextStyle::Monospace.resolve(ui.style());
    let row_h = ui.fonts_mut(|f| f.row_height(&font));
    let char_w = ui.fonts_mut(|f| f.glyph_width(&font, '0'));

    // ---------- find / go to line bar ----------
    if state.find.open || state.goto_line.is_some() {
        find_bar(ui, id, text, state, theme, env.read_only);
    }

    // ---------- keyboard handling before the TextEdit sees the events ----------
    let focused = ui.memory(|m| m.has_focus(id));
    let mut consumed: Vec<Event> = Vec::new();
    let mut commands: Vec<TextCommand> = Vec::new();
    let mut open_completion = false;
    let mut completion_moved = false;
    if focused {
        let events = ui.input(|i| i.events.clone());
        for ev in &events {
            if let Event::Key {
                key,
                pressed: true,
                modifiers,
                ..
            } = ev
            {
                let m = *modifiers;
                let cmd = m.command;
                let plain = m.is_none();
                let mut take = true;
                if let Some(comp) = state.completion.as_mut() {
                    match key {
                        Key::ArrowDown if plain => {
                            comp.selected = (comp.selected + 1) % comp.items.len();
                            comp.navigated = true;
                            completion_moved = true;
                            consumed.push(ev.clone());
                            continue;
                        }
                        Key::ArrowUp if plain => {
                            comp.selected =
                                (comp.selected + comp.items.len() - 1) % comp.items.len();
                            comp.navigated = true;
                            completion_moved = true;
                            consumed.push(ev.clone());
                            continue;
                        }
                        // Enter on a suggestion that would insert nothing new (PUTS typed, PUTS
                        // highlighted) starts a new line instead
                        Key::Enter
                            if plain && {
                                let caret = load_sel(&ctx, id).map(|s| s.head).unwrap_or(0);
                                let (_, word) = word_before(text, caret);
                                let item = &comp.items[comp.selected];
                                item.label.eq_ignore_ascii_case(&word)
                                    && item.insert.replace('$', "").eq_ignore_ascii_case(&word)
                            } =>
                        {
                            state.completion = None;
                        }
                        Key::Enter | Key::Tab if plain => {
                            let caret = load_sel(&ctx, id).map(|s| s.head).unwrap_or(0);
                            let comp = comp.clone();
                            accept_completion(&ctx, id, text, &comp, caret);
                            state.completion = None;
                            out.changed = true;
                            consumed.push(ev.clone());
                            continue;
                        }
                        Key::Escape => {
                            state.completion = None;
                            consumed.push(ev.clone());
                            continue;
                        }
                        _ => {}
                    }
                }
                match key {
                    Key::Tab if plain => commands.push(TextCommand::Indent),
                    Key::Tab if m.shift && !cmd => commands.push(TextCommand::Outdent),
                    Key::Enter if plain => commands.push(TextCommand::Newline),
                    Key::Slash if cmd => commands.push(TextCommand::ToggleComment),
                    Key::D if cmd && !m.shift => commands.push(TextCommand::Duplicate),
                    Key::ArrowUp if m.alt && !cmd => commands.push(TextCommand::MoveUp),
                    Key::ArrowDown if m.alt && !cmd => commands.push(TextCommand::MoveDown),
                    Key::K if cmd && m.shift => commands.push(TextCommand::DeleteLine),
                    Key::F if m.alt && m.shift => commands.push(TextCommand::Format),
                    Key::Space if cmd => open_completion = true,
                    Key::F if cmd && !m.shift => state.open_find(false),
                    Key::H if cmd => state.open_find(true),
                    Key::G if cmd => state.open_goto(),
                    Key::Escape => {
                        // Escape closes popups; it never kicks focus out of the editor
                        state.find.open = false;
                        state.goto_line = None;
                    }
                    _ => take = false,
                }
                if take {
                    consumed.push(ev.clone());
                }
            }
        }
        if !consumed.is_empty() {
            ui.input_mut(|i| i.events.retain(|e| !consumed.contains(e)));
        }
        if env.read_only {
            // only navigation works in a read-only document
            commands.clear();
            open_completion = false;
            state.completion = None;
        }
        for c in &commands {
            apply_command(&ctx, doc_id, text, *c);
            out.changed = true;
        }
        if !commands.is_empty() {
            state.completion = None;
            if let Some(s) = load_sel(&ctx, id) {
                state.scroll_to = Some(s.head);
            }
        }
    }
    let typed_ident = focused
        && ui.input(|i| {
            i.events.iter().any(|e| {
                matches!(e, Event::Text(t) if t.chars().all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '.'))
            })
        });
    let typed_other = focused
        && ui.input(|i| {
            i.events.iter().any(|e| {
                matches!(e, Event::Text(_))
                    || matches!(
                        e,
                        Event::Key {
                            key: Key::ArrowLeft | Key::ArrowRight | Key::Home | Key::End,
                            pressed: true,
                            ..
                        }
                    )
            })
        });

    let old_line_count = text.split('\n').count();
    let old_caret_line = load_sel(&ctx, id)
        .map(|s| line_col(text, s.head).0)
        .unwrap_or(0);

    // ---------- layout numbers ----------
    let n_lines = text.split('\n').count();
    let digits = n_lines.to_string().len().max(2);
    let bp_w = 18.0;
    let num_w = digits as f32 * char_w + 14.0;
    let addr_w = if env.show_addresses && env.line_addresses.is_some() {
        6.0 * char_w + 6.0
    } else {
        0.0
    };
    let arrows = if env.show_arrows {
        branch_arrows(text)
    } else {
        Vec::new()
    };
    const LANE_W: f32 = 7.0;
    const MAX_LANES: usize = 8;
    let lanes = arrows
        .iter()
        .map(|a| a.lane + 1)
        .max()
        .unwrap_or(0)
        .min(MAX_LANES);
    let arrows_w = if lanes > 0 {
        lanes as f32 * LANE_W + 10.0
    } else {
        0.0
    };
    let gutter_w = bp_w + num_w + addr_w + arrows_w + 8.0;
    let longest = text
        .split('\n')
        .map(|l| l.chars().count())
        .max()
        .unwrap_or(0);
    let view = ui.available_size();

    let pc_line = env.pc_line;
    if pc_line != state.last_pc_line {
        if let Some(l) = pc_line {
            state.scroll_to = Some(line_start_char(text, l));
        }
        state.last_pc_line = pc_line;
    }
    let mut goto_definition: Option<usize> = None;

    egui::ScrollArea::both()
        .id_salt(("editor_scroll", doc_id))
        .auto_shrink([false, false])
        .show(ui, |ui| {
            ui.horizontal_top(|ui| {
                ui.spacing_mut().item_spacing = Vec2::ZERO;
                let content_h = (n_lines as f32 * row_h + row_h * 6.0).max(view.y);
                let (gutter_rect, _) =
                    ui.allocate_exact_size(Vec2::new(gutter_w, content_h), Sense::hover());

                let bg_idx = ui.painter().add(Shape::Noop);
                let text_w = (longest as f32 * char_w + 80.0).max(view.x - gutter_w);
                let theme_for_layout = theme.clone();
                let font_for_layout = font.clone();
                let mut layouter = |ui: &Ui, buf: &dyn TextBuffer, _wrap: f32| -> Arc<Galley> {
                    let job = highlight(buf.as_str(), &font_for_layout, &theme_for_layout);
                    ui.fonts_mut(|f| f.layout_job(job))
                };
                if state.focus_text {
                    ui.memory_mut(|m| m.request_focus(id));
                    state.focus_text = false;
                }
                // a &str buffer can't be edited
                let mut ro_text: &str;
                let buffer: &mut dyn TextBuffer = if env.read_only {
                    ro_text = text.as_str();
                    &mut ro_text
                } else {
                    text
                };
                let output = egui::TextEdit::multiline(buffer)
                    .id(id)
                    .font(font.clone())
                    .frame(false)
                    .lock_focus(true)
                    .margin(egui::Margin::symmetric(6, 0))
                    .desired_width(text_w)
                    .min_size(Vec2::new(text_w, content_h))
                    .layouter(&mut layouter)
                    .show(ui);
                let galley = output.galley.clone();
                let origin = output.galley_pos;
                let resp = output.response.clone();
                // the gutter runs the text's full height: laid-out rows can be a little taller than
                // the font's row height (they snap to pixels), which adds up over long files
                let gutter_rect = Rect::from_min_max(
                    gutter_rect.min,
                    Pos2::new(gutter_rect.max.x, gutter_rect.max.y.max(resp.rect.max.y)),
                );
                let gutter_resp = ui.interact(gutter_rect, id.with("gutter"), Sense::click());
                out.focused = resp.has_focus();
                if resp.changed() {
                    out.changed = true;
                }
                let full_w = text_w + 40.0;

                // ---------- cursor & selection ----------
                let sel = output
                    .cursor_range
                    .map(|r| Sel::new(r.secondary.index, r.primary.index));
                let caret = sel.map(|s| s.head).unwrap_or(0);
                let (cur_line, cur_col) = line_col(text, caret);
                out.cursor = (cur_line + 1, cur_col + 1);
                if let Some(s) = sel {
                    let a = line_col(text, s.min()).0;
                    let mut b = line_col(text, s.max()).0;
                    // a selection ending at the start of a line doesn't include that line
                    if b > a && line_col(text, s.max()).1 == 0 {
                        b -= 1;
                    }
                    out.sel_lines = (a, b);
                } else {
                    out.sel_lines = (cur_line, cur_line);
                }

                // ---------- background decorations ----------
                let mut bg: Vec<Shape> = Vec::new();
                if out.focused {
                    if let Some(r) = line_rect(&galley, origin, cur_line, full_w) {
                        bg.push(Shape::rect_filled(r, 0.0, theme.code_bg_color));
                    }
                }
                if let Some(l) = pc_line {
                    if let Some(r) = line_rect(&galley, origin, l, full_w) {
                        bg.push(Shape::rect_filled(r, 0.0, theme.pc_line_bg));
                        bg.push(Shape::rect_filled(
                            Rect::from_min_size(r.min, Vec2::new(2.0, r.height())),
                            0.0,
                            theme.accent_color_primary,
                        ));
                    }
                }
                for d in env.diagnostics {
                    if let Some(r) = line_rect(&galley, origin, d.line, full_w) {
                        bg.push(Shape::rect_filled(
                            r,
                            0.0,
                            theme.error_bg_color.gamma_multiply(0.6),
                        ));
                    }
                }
                // occurrences of the word under the caret
                if sel.is_some_and(|s| s.is_empty()) {
                    let line_text = text.split('\n').nth(cur_line).unwrap_or("");
                    let col_b = edit_ops::byte_of(line_text, cur_col);
                    let tok = lexer::token_at(line_text, col_b)
                        .or_else(|| lexer::token_at(line_text, col_b.saturating_sub(1)));
                    if let Some(tok) = tok.filter(|t| {
                        matches!(
                            t.kind,
                            TokKind::LabelDef | TokKind::Ident | TokKind::Register
                        )
                    }) {
                        let word = &line_text[tok.range];
                        let occ = word_occurrences(text, word);
                        if occ.len() > 1 {
                            for (a, b) in occ {
                                bg.push(Shape::rect_filled(
                                    span_rect(&galley, origin, a, b),
                                    CornerRadius::same(2),
                                    theme.info_bg_color,
                                ));
                            }
                        }
                    }
                }
                // lines where the highlighted register was last written / read
                for m in &env.register_marks {
                    if let Some(r) = line_rect(&galley, origin, m.line, full_w) {
                        let c = if m.write {
                            theme.accent_color_primary
                        } else {
                            theme.accent_color_secondary
                        };
                        bg.push(Shape::rect_filled(r, 0.0, c.gamma_multiply(0.16)));
                    }
                }
                // the target of the branch on the caret line
                if out.focused {
                    for a in arrows.iter().filter(|a| a.from == cur_line) {
                        if let Some(r) = line_rect(&galley, origin, a.to, full_w) {
                            bg.push(Shape::rect_filled(
                                r,
                                0.0,
                                theme.info_bg_color.gamma_multiply(0.8),
                            ));
                        }
                    }
                }
                // find matches
                if state.find.open {
                    let matches = find_matches(text, &state.find.query, state.find.case_sensitive);
                    for (i, (a, b)) in matches.iter().enumerate() {
                        let r = span_rect(&galley, origin, *a, *b);
                        let current = i == state.find.current % matches.len().max(1);
                        bg.push(Shape::rect_filled(
                            r,
                            CornerRadius::same(2),
                            if current {
                                theme.warn_bg_color.gamma_multiply(1.8)
                            } else {
                                theme.warn_bg_color
                            },
                        ));
                        if current {
                            bg.push(Shape::rect_stroke(
                                r,
                                CornerRadius::same(2),
                                Stroke::new(1.0_f32, theme.warn_fg_color),
                                egui::StrokeKind::Inside,
                            ));
                        }
                    }
                }
                ui.painter().set(bg_idx, Shape::Vec(bg));

                // inline notes after the end of lines
                {
                    let mut next_x: HashMap<usize, f32> = HashMap::new();
                    let inlay_font = FontId::proportional(font.size * 0.82);
                    for inlay in &env.inlays {
                        let Some(row) = galley.rows.get(inlay.line) else {
                            continue;
                        };
                        let r = row.rect().translate(origin.to_vec2());
                        let x = *next_x
                            .entry(inlay.line)
                            .or_insert(r.min.x + row.rect().width() + 3.0 * char_w);
                        let (fg, bg) = match inlay.kind {
                            InlayKind::Input => (theme.accent_color_primary, theme.pc_line_bg),
                            InlayKind::Value => (theme.secondary_text_color, theme.code_bg_color),
                            InlayKind::Written => (theme.accent_color_primary, theme.code_bg_color),
                            InlayKind::Read => (theme.accent_color_secondary, theme.code_bg_color),
                        };
                        let g =
                            ui.painter()
                                .layout_no_wrap(inlay.text.clone(), inlay_font.clone(), fg);
                        let chip = Rect::from_min_size(
                            Pos2::new(x, r.center().y - g.size().y / 2.0 - 1.0),
                            g.size() + Vec2::new(10.0, 2.0),
                        );
                        ui.painter().rect_filled(chip, CornerRadius::same(3), bg);
                        ui.painter().galley(chip.min + Vec2::new(5.0, 1.0), g, fg);
                        next_x.insert(inlay.line, chip.max.x + 6.0);
                    }
                }

                // ---------- squiggles and inline error messages ----------
                for d in env.diagnostics {
                    if let Some(row) = galley.rows.get(d.line) {
                        let r = row.rect().translate(origin.to_vec2());
                        let line_text = text.split('\n').nth(d.line).unwrap_or("");
                        let code_end = line_text.find(';').unwrap_or(line_text.len());
                        let start_x = r.min.x
                            + (line_text.len() - line_text.trim_start().len()) as f32 * char_w;
                        let end_x = (r.min.x
                            + line_text[..code_end].trim_end().chars().count() as f32 * char_w)
                            .max(start_x + char_w * 2.0);
                        let y = r.max.y - 1.5;
                        let mut pts = Vec::new();
                        let mut x = start_x;
                        let mut up = true;
                        while x < end_x {
                            pts.push(Pos2::new(x, if up { y - 1.5 } else { y + 0.5 }));
                            x += 3.0;
                            up = !up;
                        }
                        if pts.len() > 1 {
                            ui.painter().add(Shape::line(
                                pts,
                                Stroke::new(1.2_f32, theme.editor_error_underline_color),
                            ));
                        }
                        ui.painter().text(
                            Pos2::new(r.min.x + row.rect().width() + 3.0 * char_w, r.center().y),
                            Align2::LEFT_CENTER,
                            format!("● {}", d.message),
                            FontId::proportional(font.size * 0.88),
                            theme.error_fg_color,
                        );
                    }
                }

                // ---------- gutter ----------
                let painter = ui.painter_at(gutter_rect);
                painter.rect_filled(gutter_rect, 0.0, theme.panel_background);
                let addresses = env.line_addresses.as_ref();
                let hover_line = gutter_resp
                    .hover_pos()
                    .and_then(|p| line_at_y(&galley, origin, p.y))
                    .map(|l| l as isize);
                for (i, row) in galley.rows.iter().enumerate() {
                    let r = row.rect().translate(origin.to_vec2());
                    if r.max.y < ui.clip_rect().min.y || r.min.y > ui.clip_rect().max.y {
                        continue;
                    }
                    let cy = r.center().y;
                    let is_cur = i == cur_line && out.focused;
                    let is_pc = pc_line == Some(i);
                    let has_err = env.diagnostics.iter().any(|d| d.line == i);
                    // breakpoint
                    let bp_center = Pos2::new(gutter_rect.min.x + bp_w * 0.5 + 2.0, cy);
                    if breakpoints.contains(&i) {
                        painter.circle_filled(bp_center, 4.5, theme.accent_color_negative);
                    } else if hover_line == Some(i as isize) {
                        painter.circle_filled(
                            bp_center,
                            4.5,
                            theme.accent_color_negative.gamma_multiply(0.35),
                        );
                    }
                    if is_pc {
                        let x = gutter_rect.min.x + 2.0;
                        let tri = vec![
                            Pos2::new(x + 3.0, cy - 5.0),
                            Pos2::new(x + 11.0, cy),
                            Pos2::new(x + 3.0, cy + 5.0),
                        ];
                        painter.add(Shape::convex_polygon(
                            tri,
                            theme.accent_color_primary,
                            Stroke::NONE,
                        ));
                    }
                    // line number
                    let num_color = if has_err {
                        theme.error_fg_color
                    } else if is_pc {
                        theme.accent_color_primary
                    } else if is_cur {
                        theme.editor_current_line_number_color
                    } else {
                        theme.editor_line_number_color
                    };
                    painter.text(
                        Pos2::new(gutter_rect.min.x + bp_w + num_w - 8.0, cy),
                        Align2::RIGHT_CENTER,
                        (i + 1).to_string(),
                        font.clone(),
                        num_color,
                    );
                    if let Some(a) = addresses.and_then(|m| m.get(&i)) {
                        painter.text(
                            Pos2::new(gutter_rect.min.x + bp_w + num_w, cy),
                            Align2::LEFT_CENTER,
                            format!("x{a:04X}"),
                            FontId::monospace(font.size * 0.85),
                            theme.editor_line_number_color.gamma_multiply(0.85),
                        );
                    }
                }
                // branch and call arrows
                if !arrows.is_empty() {
                    let right = gutter_rect.max.x - 3.0;
                    let pc_cc = env.emulator.get_nzp();
                    for a in &arrows {
                        let (Some(rf), Some(rt)) = (galley.rows.get(a.from), galley.rows.get(a.to))
                        else {
                            continue;
                        };
                        let yf = rf.rect().center().y + origin.y;
                        let yt = rt.rect().center().y + origin.y;
                        let x = right - 6.0 - a.lane.min(MAX_LANES - 1) as f32 * LANE_W;
                        // emphasise arrows touching the caret line, and the PC's branch if it will be taken
                        let on_caret = out.focused && (a.from == cur_line || a.to == cur_line);
                        let from_pc = pc_line == Some(a.from);
                        let taken = from_pc && {
                            let line_text = text.split('\n').nth(a.from).unwrap_or("");
                            let op = lex_line(line_text)
                                .into_iter()
                                .find(|t| t.kind == TokKind::Opcode)
                                .map(|t| line_text[t.range].to_ascii_uppercase())
                                .unwrap_or_default();
                            if a.call || op == "BR" || op == "BRNZP" {
                                true
                            } else {
                                let f = &op[2..];
                                (f.contains('N') && pc_cc.0)
                                    || (f.contains('Z') && pc_cc.1)
                                    || (f.contains('P') && pc_cc.2)
                            }
                        };
                        let (color, width) = if from_pc && taken {
                            (theme.accent_color_primary, 1.8_f32)
                        } else if on_caret {
                            (theme.accent_color_secondary, 1.6_f32)
                        } else if a.call {
                            (theme.editor_label_color.gamma_multiply(0.55), 1.2_f32)
                        } else {
                            (theme.secondary_text_color.gamma_multiply(0.75), 1.2_f32)
                        };
                        let stroke = Stroke::new(width, color);
                        painter.line_segment([Pos2::new(right, yf), Pos2::new(x, yf)], stroke);
                        painter.line_segment([Pos2::new(x, yf), Pos2::new(x, yt)], stroke);
                        painter.line_segment([Pos2::new(x, yt), Pos2::new(right, yt)], stroke);
                        painter.add(Shape::convex_polygon(
                            vec![
                                Pos2::new(right + 1.0, yt),
                                Pos2::new(right - 4.0, yt - 3.0),
                                Pos2::new(right - 4.0, yt + 3.0),
                            ],
                            color,
                            Stroke::NONE,
                        ));
                        painter.circle_filled(Pos2::new(right - 1.0, yf), 1.6, color);
                    }
                }
                painter.vline(
                    gutter_rect.max.x - 1.0,
                    gutter_rect.y_range(),
                    Stroke::new(1.0_f32, theme.separator_color),
                );
                if gutter_resp.clicked() {
                    if let Some(p) = gutter_resp.interact_pointer_pos() {
                        if let Some(line) = line_at_y(&galley, origin, p.y).filter(|l| *l < n_lines)
                        {
                            out.toggled_breakpoints.push(line);
                        }
                    }
                }
                gutter_resp
                    .on_hover_cursor(egui::CursorIcon::PointingHand)
                    .on_hover_text_at_pointer("Click to toggle a breakpoint");

                // ---------- hover cards and ctrl+click ----------
                if let Some(p) = resp.hover_pos() {
                    let rel = p - origin;
                    if let Some(line) = line_at_y(&galley, origin, p.y).filter(|l| *l < n_lines) {
                        // the glyph under the pointer, from the laid-out text: glyphs snap to the
                        // pixel grid, so column * glyph width drifts at fractional scales
                        let col_chars = col_at_x(&galley, line, rel.x).unwrap_or(usize::MAX);
                        let row_bottom = galley.rows[line].rect().max.y;
                        let line_text = text.split('\n').nth(line).unwrap_or("");
                        if col_chars < line_text.chars().count() {
                            let col_b = edit_ops::byte_of(line_text, col_chars);
                            let cmd_down = ui.input(|i| i.modifiers.command);
                            if cmd_down {
                                if let Some(tok) = lexer::token_at(line_text, col_b).filter(|t| {
                                    matches!(
                                        t.kind,
                                        TokKind::Ident | TokKind::Opcode | TokKind::Trap
                                    )
                                }) {
                                    let x0 = x_of_col(
                                        &galley,
                                        line,
                                        line_text[..tok.range.start].chars().count(),
                                    );
                                    let x1 = x_of_col(
                                        &galley,
                                        line,
                                        line_text[..tok.range.end].chars().count(),
                                    );
                                    let r = Rect::from_min_max(
                                        origin + Vec2::new(x0, row_bottom - 2.0),
                                        origin + Vec2::new(x1, row_bottom - 1.0),
                                    );
                                    ui.painter().rect_filled(r, 0.0, theme.editor_label_color);
                                    ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
                                    if resp.clicked() {
                                        let word = &line_text[tok.range.clone()];
                                        if tok.kind == TokKind::Ident {
                                            if let Some((_, l)) = lexer::label_definitions(text)
                                                .into_iter()
                                                .find(|(l, _)| l.eq_ignore_ascii_case(word))
                                            {
                                                goto_definition = Some(l);
                                            }
                                        } else {
                                            // open the instruction in Help, with this line's operands
                                            let code = line_text.split(';').next().unwrap_or("");
                                            let statement = code[tok.range.start.min(code.len())..]
                                                .trim()
                                                .to_string();
                                            *crate::panes::emulator::help::SHOW_INSTRUCTION
                                                .lock()
                                                .unwrap() = Some(
                                                crate::panes::emulator::help::ShowInstruction {
                                                    mnemonic: word.to_string(),
                                                    address: addresses
                                                        .and_then(|m| m.get(&line))
                                                        .copied(),
                                                    line: statement,
                                                },
                                            );
                                        }
                                    }
                                }
                            } else if state.completion.is_none() {
                                let line_addr = addresses.and_then(|m| m.get(&line)).copied();
                                let has = lexer::token_at(line_text, col_b).is_some_and(|t| {
                                    !matches!(
                                        t.kind,
                                        TokKind::Whitespace
                                            | TokKind::Punct
                                            | TokKind::Comment
                                            | TokKind::String
                                    )
                                });
                                if has {
                                    resp.clone().on_hover_ui_at_pointer(|ui| {
                                        hover_card(ui, env, text, line, col_b, line_addr);
                                    });
                                }
                            }
                        }
                    }
                }

                // ---------- completion popup ----------
                if env.read_only {
                    state.completion = None;
                } else if out.changed || open_completion || typed_other {
                    if (typed_ident && out.focused)
                        || open_completion
                        || (state.completion.is_some() && out.changed)
                    {
                        state.completion = build_completion(
                            text,
                            load_sel(&ctx, id).map(|s| s.head).unwrap_or(caret),
                        );
                    } else if typed_other {
                        state.completion = None;
                    }
                }
                if !out.focused && !completion_moved {
                    state.completion = None;
                }
                if let Some(comp) = &state.completion {
                    let caret_rect = galley
                        .pos_from_cursor(CCursor::new(comp.word_start))
                        .translate(origin.to_vec2());
                    completion_popup(
                        ui,
                        comp,
                        Pos2::new(caret_rect.min.x - 4.0, caret_rect.max.y + 2.0),
                        theme,
                    );
                }

                // ---------- programmatic scrolling ----------
                if let Some(c) = state.scroll_to.take() {
                    let r = galley
                        .pos_from_cursor(CCursor::new(c))
                        .translate(origin.to_vec2());
                    let align = state.center_scroll.then_some(egui::Align::Center);
                    state.center_scroll = false;
                    ui.scroll_to_rect(r.expand2(Vec2::new(40.0, row_h * 3.0)), align);
                }
            });
        });

    if let Some(l) = goto_definition {
        let c = line_start_char(text, l);
        store_sel(&ctx, id, Sel::caret(c));
        state.scroll_to = Some(c);
        state.center_scroll = true;
    }

    // keep breakpoints attached to their lines when lines are inserted or removed
    let new_line_count = text.split('\n').count();
    if new_line_count != old_line_count {
        let delta = new_line_count as isize - old_line_count as isize;
        let moved: BTreeSet<usize> = breakpoints
            .iter()
            .filter_map(|&l| {
                if l <= old_caret_line {
                    Some(l)
                } else {
                    let nl = l as isize + delta;
                    (nl > old_caret_line as isize - if delta < 0 { 0 } else { 1 })
                        .then_some(nl.max(0) as usize)
                }
            })
            .filter(|l| *l < new_line_count)
            .collect();
        *breakpoints = moved;
    }
    out
}

fn completion_popup(ui: &Ui, comp: &Completion, pos: Pos2, theme: &ThemeSettings) {
    egui::Area::new(Id::new("lc3_completion"))
        .order(egui::Order::Foreground)
        .fixed_pos(pos)
        .interactable(false)
        .show(ui.ctx(), |ui| {
            egui::Frame::popup(ui.style())
                .inner_margin(egui::Margin::same(4))
                .show(ui, |ui| {
                    ui.set_width(320.0);
                    ui.spacing_mut().item_spacing.y = 0.0;
                    let first = comp.selected.saturating_sub(7);
                    for (i, item) in comp.items.iter().enumerate().skip(first).take(8) {
                        let selected = i == comp.selected;
                        let (rect, _) = ui.allocate_exact_size(
                            Vec2::new(ui.available_width(), 22.0),
                            Sense::hover(),
                        );
                        if selected {
                            ui.painter().rect_filled(
                                rect,
                                CornerRadius::same(3),
                                theme.info_bg_color,
                            );
                        }
                        let (badge, color) = match item.kind {
                            CompletionKind::Instruction => ("op", theme.editor_opcode_color),
                            CompletionKind::Directive => ("dir", theme.editor_directive_color),
                            CompletionKind::Register => ("reg", theme.editor_register_color),
                            CompletionKind::Label => ("lbl", theme.editor_label_color),
                        };
                        let badge_rect = Rect::from_min_size(
                            rect.min + Vec2::new(4.0, 4.0),
                            Vec2::new(26.0, 14.0),
                        );
                        ui.painter().rect_filled(
                            badge_rect,
                            CornerRadius::same(3),
                            color.gamma_multiply(0.22),
                        );
                        ui.painter().text(
                            badge_rect.center(),
                            Align2::CENTER_CENTER,
                            badge,
                            FontId::monospace(9.5),
                            color,
                        );
                        ui.painter().text(
                            rect.min + Vec2::new(38.0, 11.0),
                            Align2::LEFT_CENTER,
                            &item.label,
                            FontId::monospace(13.0),
                            if selected {
                                theme.strong_text_color
                            } else {
                                theme.primary_text_color
                            },
                        );
                        ui.painter().text(
                            rect.right_center() - Vec2::new(6.0, 0.0),
                            Align2::RIGHT_CENTER,
                            &item.detail,
                            FontId::monospace(11.0),
                            theme.secondary_text_color,
                        );
                    }
                    if let Some(item) = comp.items.get(comp.selected) {
                        if !item.doc.is_empty() {
                            ui.add_space(4.0);
                            ui.separator();
                            ui.label(
                                RichText::new(&item.doc)
                                    .size(12.0)
                                    .color(theme.secondary_text_color),
                            );
                        }
                    }
                    ui.label(
                        RichText::new("↑↓ choose · Enter/Tab insert · Esc close")
                            .size(10.5)
                            .color(theme.editor_line_number_color),
                    );
                });
        });
}

fn find_bar(
    ui: &mut Ui,
    text_id: Id,
    text: &mut String,
    state: &mut EditorUiState,
    theme: &ThemeSettings,
    read_only: bool,
) {
    let ctx = ui.ctx().clone();
    egui::Frame::new()
        .fill(theme.panel_background)
        .inner_margin(egui::Margin::symmetric(8, 5))
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                if let Some(goto) = state.goto_line.as_mut() {
                    ui.label("Go to line");
                    let r = ui.add(
                        egui::TextEdit::singleline(goto)
                            .desired_width(70.0)
                            .hint_text("line"),
                    );
                    if !r.has_focus() && goto.is_empty() {
                        r.request_focus();
                    }
                    let n_lines = text.split('\n').count();
                    ui.label(
                        RichText::new(format!("of {n_lines}")).color(theme.secondary_text_color),
                    );
                    if r.has_focus() && ui.input(|i| i.key_pressed(Key::Enter))
                        || r.lost_focus() && ui.input(|i| i.key_pressed(Key::Enter))
                    {
                        if let Ok(n) = goto.trim().parse::<usize>() {
                            let l = n.clamp(1, n_lines) - 1;
                            let c = line_start_char(text, l);
                            store_sel(&ctx, text_id, Sel::caret(c));
                            state.scroll_to = Some(c);
                            state.center_scroll = true;
                        }
                        state.goto_line = None;
                        state.focus_text = true;
                        // don't let the editor also see this Enter
                        ui.input_mut(|i| i.consume_key(Modifiers::NONE, Key::Enter));
                    }
                    if ui.input(|i| i.key_pressed(Key::Escape)) {
                        ui.input_mut(|i| i.consume_key(Modifiers::NONE, Key::Escape));
                        state.goto_line = None;
                        state.focus_text = true;
                    }
                    return;
                }

                let matches = find_matches(text, &state.find.query, state.find.case_sensitive);
                let toggle_label = if state.find.show_replace {
                    "⏷"
                } else {
                    "⏵"
                };
                if ui
                    .small_button(toggle_label)
                    .explain("Toggle replace (Ctrl+H)")
                    .clicked()
                {
                    state.find.show_replace = !state.find.show_replace;
                }
                let q = ui.add(
                    egui::TextEdit::singleline(&mut state.find.query)
                        .desired_width(180.0)
                        .hint_text("Find")
                        .font(egui::TextStyle::Monospace),
                );
                if state.find.focus_query {
                    q.request_focus();
                    // pre-fill with the selected text
                    if let Some(s) = load_sel(&ctx, text_id) {
                        if !s.is_empty() {
                            let a = edit_ops::byte_of(text, s.min());
                            let b = edit_ops::byte_of(text, s.max());
                            if !text[a..b].contains('\n') {
                                state.find.query = text[a..b].to_string();
                            }
                        }
                    }
                    state.find.focus_query = false;
                }
                if q.changed() {
                    state.find.current = 0;
                }
                let count = if matches.is_empty() {
                    if state.find.query.is_empty() {
                        String::new()
                    } else {
                        "No results".to_string()
                    }
                } else {
                    format!(
                        "{} of {}",
                        state.find.current % matches.len() + 1,
                        matches.len()
                    )
                };
                ui.label(RichText::new(count).color(
                    if matches.is_empty() && !state.find.query.is_empty() {
                        theme.error_fg_color
                    } else {
                        theme.secondary_text_color
                    },
                ));
                ui.toggle_value(
                    &mut state.find.case_sensitive,
                    RichText::new("Aa").monospace(),
                )
                .explain("Match case");
                let go = |state: &mut EditorUiState, delta: isize| {
                    if matches.is_empty() {
                        return;
                    }
                    let n = matches.len() as isize;
                    state.find.current =
                        ((state.find.current as isize + delta).rem_euclid(n)) as usize;
                    let (a, b) = matches[state.find.current];
                    store_sel(&ctx, text_id, Sel::new(a, b));
                    state.scroll_to = Some(a);
                    state.center_scroll = true;
                };
                let enter = q.lost_focus() && ui.input(|i| i.key_pressed(Key::Enter));
                let shift = ui.input(|i| i.modifiers.shift);
                if enter {
                    ui.input_mut(|i| {
                        i.consume_key(Modifiers::NONE, Key::Enter);
                        i.consume_key(Modifiers::SHIFT, Key::Enter);
                    });
                }
                if ui
                    .small_button("↑")
                    .explain("Previous (Shift+Enter)")
                    .clicked()
                    || enter && shift
                {
                    go(state, -1);
                    q.request_focus();
                }
                if ui.small_button("↓").explain("Next (Enter)").clicked()
                    || enter && !shift
                    || ui.input(|i| i.key_pressed(Key::F3))
                {
                    go(state, 1);
                    q.request_focus();
                }
                if state.find.show_replace && !read_only {
                    ui.add_space(8.0);
                    ui.add(
                        egui::TextEdit::singleline(&mut state.find.replacement)
                            .desired_width(140.0)
                            .hint_text("Replace")
                            .font(egui::TextStyle::Monospace),
                    );
                    if ui.small_button("Replace").clicked() && !matches.is_empty() {
                        let (a, b) = matches[state.find.current % matches.len()];
                        let ba = edit_ops::byte_of(text, a);
                        let bb = edit_ops::byte_of(text, b);
                        text.replace_range(ba..bb, &state.find.replacement);
                    }
                    if ui.small_button("All").explain("Replace all").clicked() {
                        for (a, b) in matches.iter().rev() {
                            let ba = edit_ops::byte_of(text, *a);
                            let bb = edit_ops::byte_of(text, *b);
                            text.replace_range(ba..bb, &state.find.replacement);
                        }
                    }
                }
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.small_button("✕").explain("Close (Esc)").clicked()
                        || ui.input(|i| i.key_pressed(Key::Escape))
                    {
                        state.find.open = false;
                        state.focus_text = true;
                    }
                });
            });
        });
    let _ = Modifiers::NONE;
}
