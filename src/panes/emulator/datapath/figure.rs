//! Patt & Patel (3rd edition) Figure C.8, the LC-3 datapath with the structures for interrupt
//! and exception control, drawn with the egui painter. Coordinates follow the printed figure so
//! it reads like the textbook. As in the book, the parts Figure C.8 adds to Figure C.3 are tinted.

use crate::emulator::disasm;
use crate::emulator::trace::{Activity, Dp, MicroRecord, Sig, Snapshot};
use crate::theme::ThemeSettings;
use egui::epaint::TextShape;
use egui::{vec2, Align2, Color32, CornerRadius, FontId, Pos2, Rect, Shape, Stroke};

/// The figure's extent in its own units.
pub const X0: f32 = 58.0;
pub const Y0: f32 = 108.0;
pub const W: f32 = 845.0;
pub const H: f32 = 945.0;

type Pts = &'static [(f32, f32)];

/// Wires that carry data, from source to destination. `true` marks Figure C.8's additions.
const WIRES: &[(Dp, bool, Pts)] = &[
    // PC and its neighbours
    (Dp::GatePc, false, &[(459.0, 230.0), (459.0, 119.0)]),
    (
        Dp::PcInc,
        false,
        &[
            (459.0, 230.0),
            (459.0, 216.0),
            (524.0, 216.0),
            (524.0, 265.0),
        ],
    ),
    (
        Dp::PcInc,
        false,
        &[
            (524.0, 290.0),
            (524.0, 315.0),
            (484.0, 315.0),
            (484.0, 296.0),
        ],
    ),
    (Dp::PcMux, false, &[(459.0, 270.0), (459.0, 250.0)]),
    (
        Dp::BusToPcMux,
        false,
        &[
            (367.0, 119.0),
            (367.0, 315.0),
            (431.0, 315.0),
            (431.0, 296.0),
        ],
    ),
    (
        Dp::PcToAddr1,
        false,
        &[
            (459.0, 230.0),
            (459.0, 216.0),
            (550.0, 216.0),
            (550.0, 452.0),
            (432.0, 452.0),
            (432.0, 440.0),
        ],
    ),
    (
        Dp::PcDec,
        true,
        &[
            (459.0, 230.0),
            (459.0, 216.0),
            (524.0, 216.0),
            (524.0, 197.0),
        ],
    ),
    (Dp::GatePcDec, true, &[(524.0, 172.0), (524.0, 119.0)]),
    // address adder
    (
        Dp::AdderToMarMux,
        false,
        &[
            (354.0, 352.0),
            (354.0, 327.0),
            (273.0, 327.0),
            (273.0, 262.0),
        ],
    ),
    (
        Dp::AdderToPcMux,
        false,
        &[
            (354.0, 352.0),
            (354.0, 327.0),
            (459.0, 327.0),
            (459.0, 296.0),
        ],
    ),
    (
        Dp::Addr2Mux,
        false,
        &[
            (280.0, 413.0),
            (280.0, 403.0),
            (318.0, 403.0),
            (318.0, 383.0),
        ],
    ),
    (
        Dp::Addr1Mux,
        false,
        &[
            (412.0, 414.0),
            (412.0, 403.0),
            (385.0, 403.0),
            (385.0, 383.0),
        ],
    ),
    (Dp::ZeroToAddr2, false, &[(322.0, 466.0), (322.0, 440.0)]),
    // MARMUX and ZEXT
    (Dp::GateMarMux, false, &[(255.0, 236.0), (255.0, 119.0)]),
    (Dp::Zext, false, &[(104.0, 639.0), (104.0, 373.0)]),
    (
        Dp::ZextToMarMux,
        false,
        &[
            (104.0, 353.0),
            (104.0, 308.0),
            (235.0, 308.0),
            (235.0, 262.0),
        ],
    ),
    // IR fields into the sign extenders
    (
        Dp::Sext11,
        false,
        &[(104.0, 639.0), (104.0, 465.0), (189.0, 465.0)],
    ),
    (
        Dp::Sext11,
        false,
        &[(232.0, 465.0), (241.0, 465.0), (241.0, 440.0)],
    ),
    (
        Dp::Sext9,
        false,
        &[(104.0, 639.0), (104.0, 496.0), (189.0, 496.0)],
    ),
    (
        Dp::Sext9,
        false,
        &[(232.0, 496.0), (268.0, 496.0), (268.0, 440.0)],
    ),
    (
        Dp::Sext6,
        false,
        &[(104.0, 639.0), (104.0, 528.0), (189.0, 528.0)],
    ),
    (
        Dp::Sext6,
        false,
        &[(232.0, 528.0), (296.0, 528.0), (296.0, 440.0)],
    ),
    (
        Dp::Sext5,
        false,
        &[(104.0, 639.0), (104.0, 557.0), (189.0, 557.0)],
    ),
    (
        Dp::Sext5,
        false,
        &[
            (232.0, 557.0),
            (320.0, 557.0),
            (320.0, 490.0),
            (655.0, 490.0),
            (655.0, 508.0),
        ],
    ),
    // register file and ALU
    (Dp::BusToReg, false, &[(711.0, 119.0), (711.0, 242.0)]),
    (Dp::Sr2Out, false, &[(685.0, 367.0), (685.0, 508.0)]),
    (Dp::Sr1Out, false, &[(737.0, 367.0), (737.0, 560.0)]),
    (
        Dp::RegToAddr1,
        false,
        &[
            (737.0, 367.0),
            (737.0, 462.0),
            (393.0, 462.0),
            (393.0, 440.0),
        ],
    ),
    (
        Dp::Sr2Mux,
        false,
        &[
            (104.0, 639.0),
            (104.0, 581.0),
            (365.0, 581.0),
            (365.0, 507.0),
            (610.0, 507.0),
            (610.0, 521.0),
            (637.0, 521.0),
        ],
    ),
    (Dp::Sr2Mux, false, &[(670.0, 533.0), (670.0, 560.0)]),
    (Dp::GateAlu, false, &[(708.0, 597.0), (708.0, 775.0)]),
    // IR, control, condition codes
    (Dp::BusToIr, false, &[(104.0, 775.0), (104.0, 659.0)]),
    (
        Dp::IrToControl,
        false,
        &[
            (104.0, 639.0),
            (104.0, 594.0),
            (425.0, 594.0),
            (425.0, 573.0),
        ],
    ),
    (Dp::BusToLogic, false, &[(614.0, 775.0), (614.0, 742.0)]),
    (Dp::Logic, false, &[(614.0, 717.0), (614.0, 697.0)]),
    (Dp::NzpMux, true, &[(628.0, 677.0), (628.0, 650.0)]),
    (
        Dp::Nzp,
        false,
        &[
            (628.0, 632.0),
            (628.0, 610.0),
            (568.0, 610.0),
            (568.0, 573.0),
        ],
    ),
    // the PSR: privilege, priority and condition codes
    (
        Dp::GatePsr,
        true,
        &[
            (628.0, 632.0),
            (628.0, 610.0),
            (687.0, 610.0),
            (687.0, 775.0),
        ],
    ),
    (
        Dp::GatePsr,
        true,
        &[(265.0, 700.0), (300.0, 700.0), (300.0, 775.0)],
    ),
    (
        Dp::GatePsr,
        true,
        &[
            (505.0, 682.0),
            (505.0, 674.0),
            (545.0, 674.0),
            (545.0, 775.0),
        ],
    ),
    (Dp::BusToPsr, true, &[(642.0, 775.0), (642.0, 697.0)]),
    (Dp::BusToPsr, true, &[(272.0, 775.0), (272.0, 745.0)]),
    (Dp::BusToPsr, true, &[(530.0, 775.0), (530.0, 741.0)]),
    (Dp::PrivMux, true, &[(252.0, 725.0), (252.0, 710.0)]),
    (
        Dp::SetPriv,
        true,
        &[(197.0, 763.0), (237.0, 763.0), (237.0, 745.0)],
    ),
    (Dp::PriorityMux, true, &[(505.0, 721.0), (505.0, 702.0)]),
    // stack pointer logic
    (
        Dp::UspIn,
        true,
        &[
            (737.0, 367.0),
            (737.0, 462.0),
            (779.0, 462.0),
            (779.0, 478.0),
        ],
    ),
    (
        Dp::SspIn,
        true,
        &[
            (737.0, 367.0),
            (737.0, 462.0),
            (858.0, 462.0),
            (858.0, 478.0),
        ],
    ),
    (
        Dp::SpIncPath,
        true,
        &[
            (737.0, 367.0),
            (737.0, 462.0),
            (806.0, 462.0),
            (806.0, 535.0),
        ],
    ),
    (Dp::SpIncPath, true, &[(806.0, 555.0), (806.0, 612.0)]),
    (
        Dp::SpDecPath,
        true,
        &[
            (737.0, 367.0),
            (737.0, 462.0),
            (832.0, 462.0),
            (832.0, 535.0),
        ],
    ),
    (Dp::SpDecPath, true, &[(832.0, 555.0), (832.0, 612.0)]),
    (Dp::UspOut, true, &[(779.0, 555.0), (779.0, 612.0)]),
    (Dp::SspOut, true, &[(858.0, 555.0), (858.0, 612.0)]),
    (Dp::GateSp, true, &[(819.0, 640.0), (819.0, 775.0)]),
    // memory side
    (Dp::GateMdr, false, &[(144.0, 821.0), (144.0, 775.0)]),
    (
        Dp::BusToMdr,
        false,
        &[
            (110.0, 775.0),
            (110.0, 912.0),
            (133.0, 912.0),
            (133.0, 884.0),
        ],
    ),
    (Dp::BusToMdr, false, &[(144.0, 864.0), (144.0, 840.0)]),
    (
        Dp::MemToMdr,
        false,
        &[
            (221.0, 968.0),
            (175.0, 968.0),
            (175.0, 1031.0),
            (387.0, 1031.0),
            (387.0, 1025.0),
            (370.0, 1025.0),
        ],
    ),
    (
        Dp::MemToMdr,
        false,
        &[(350.0, 998.0), (155.0, 998.0), (155.0, 884.0)],
    ),
    (Dp::MemToMdr, false, &[(144.0, 864.0), (144.0, 840.0)]),
    (
        Dp::MdrToMem,
        false,
        &[
            (144.0, 821.0),
            (144.0, 810.0),
            (245.0, 810.0),
            (245.0, 894.0),
        ],
    ),
    (Dp::BusToMar, false, &[(281.0, 775.0), (281.0, 821.0)]),
    (Dp::MarToMem, false, &[(281.0, 840.0), (281.0, 894.0)]),
    (
        Dp::MarToMem,
        false,
        &[(281.0, 865.0), (375.0, 865.0), (375.0, 902.0)],
    ),
    // Table and Vector
    (Dp::GateVector, true, &[(688.0, 827.0), (688.0, 775.0)]),
    (Dp::GateVector, true, &[(779.0, 827.0), (779.0, 775.0)]),
    (Dp::TabX00, true, &[(552.0, 811.0), (576.0, 811.0)]),
    (Dp::TabX01, true, &[(552.0, 835.0), (576.0, 835.0)]),
    (
        Dp::TableMux,
        true,
        &[
            (590.0, 822.0),
            (630.0, 822.0),
            (630.0, 836.0),
            (667.0, 836.0),
        ],
    ),
    (Dp::VectorSel, true, &[(777.0, 870.0), (777.0, 846.0)]),
    (
        Dp::IrToVector,
        true,
        &[
            (882.0, 775.0),
            (882.0, 895.0),
            (789.0, 895.0),
            (789.0, 882.0),
        ],
    ),
    (Dp::VectorMux, true, &[(765.0, 900.0), (765.0, 882.0)]),
    (
        Dp::VecIntv,
        true,
        &[(807.0, 947.0), (776.0, 947.0), (776.0, 919.0)],
    ),
    (
        Dp::VecX00,
        true,
        &[(807.0, 972.0), (755.0, 972.0), (755.0, 919.0)],
    ),
    (
        Dp::VecX01,
        true,
        &[(807.0, 997.0), (737.0, 997.0), (737.0, 919.0)],
    ),
    (
        Dp::VecX02,
        true,
        &[(807.0, 1023.0), (717.0, 1023.0), (717.0, 919.0)],
    ),
    // interrupt priority: into the comparator (INT) and PSRMUX
    (Dp::IntPriorityIn, true, &[(445.0, 646.0), (488.0, 646.0)]),
    (
        Dp::IntPriorityIn,
        true,
        &[
            (440.0, 646.0),
            (440.0, 755.0),
            (488.0, 755.0),
            (488.0, 741.0),
        ],
    ),
    (Dp::IntSignal, true, &[(505.0, 635.0), (505.0, 573.0)]),
];

/// Wires the emulator does not trace: memory-mapped I/O, ACV and interrupt priority.
const IDLE_WIRES: &[(bool, Pts)] = &[
    (false, &[(245.0, 854.0), (676.0, 854.0), (676.0, 916.0)]),
    (false, &[(540.0, 854.0), (540.0, 916.0)]),
    (false, &[(612.0, 854.0), (612.0, 887.0)]),
    (false, &[(420.0, 925.0), (517.0, 925.0)]),
    (
        false,
        &[
            (400.0, 945.0),
            (400.0, 962.0),
            (570.0, 962.0),
            (570.0, 897.0),
            (589.0, 897.0),
        ],
    ),
    (
        false,
        &[
            (388.0, 945.0),
            (388.0, 970.0),
            (636.0, 970.0),
            (636.0, 926.0),
            (647.0, 926.0),
        ],
    ),
    (false, &[(491.0, 904.0), (491.0, 981.0), (370.0, 981.0)]),
    (false, &[(540.0, 936.0), (540.0, 994.0), (370.0, 994.0)]),
    (false, &[(676.0, 936.0), (676.0, 1008.0), (370.0, 1008.0)]),
    (true, &[(300.0, 700.0), (352.0, 700.0)]),
    (
        true,
        &[
            (300.0, 700.0),
            (300.0, 600.0),
            (447.0, 600.0),
            (447.0, 573.0),
        ],
    ),
    (true, &[(372.0, 775.0), (372.0, 716.0)]),
    (true, &[(372.0, 691.0), (372.0, 669.0)]),
    (
        true,
        &[
            (370.0, 651.0),
            (370.0, 630.0),
            (481.0, 630.0),
            (481.0, 573.0),
        ],
    ),
    (true, &[(505.0, 682.0), (505.0, 652.0)]),
];

const DOTS: &[(f32, f32)] = &[
    (459.0, 216.0),
    (524.0, 216.0),
    (354.0, 327.0),
    (104.0, 465.0),
    (104.0, 496.0),
    (104.0, 528.0),
    (104.0, 557.0),
    (104.0, 581.0),
    (104.0, 594.0),
    (737.0, 462.0),
    (779.0, 462.0),
    (806.0, 462.0),
    (832.0, 462.0),
    (300.0, 700.0),
    (505.0, 674.0),
    (628.0, 610.0),
    (144.0, 810.0),
    (245.0, 854.0),
    (281.0, 865.0),
    (540.0, 854.0),
    (612.0, 854.0),
    (336.0, 887.0),
    (144.0, 864.0),
];

#[derive(Clone, Copy, PartialEq)]
enum Lit {
    Idle,
    /// Used earlier in this instruction
    Trail,
    Active,
}

pub struct Frame<'a> {
    pub rec: Option<&'a MicroRecord>,
    pub trail: Activity,
    pub snap: &'a Snapshot,
    pub prev: Option<&'a Snapshot>,
    /// Extra values for tooltips: (Saved_SSP, Saved_USP, PSR, Table, Vector)
    pub extra: [u16; 5],
    /// Animate the active wires
    pub flow_time: Option<f64>,
    /// Where the pointer is, to highlight and describe what is under it
    pub hover: Option<Pos2>,
    /// The true state: every live wire lit, with its value
    pub live: Option<&'a super::live::Live>,
}

/// Draw the figure into `area`; returns the tooltip for what is under the pointer.
pub fn draw(ui: &egui::Ui, area: Rect, theme: &ThemeSettings, f: &Frame<'_>) -> Option<String> {
    let act = f.rec.map(|r| r.activity).unwrap_or_default();
    let trail = f.trail;
    let snap = f.snap;
    let [saved_ssp, saved_usp, psr, table, vector] = f.extra;
    let lit = |p: Dp| {
        if act.has(p) || f.live.is_some_and(|l| l.has(p)) {
            Lit::Active
        } else if trail.has(p) {
            Lit::Trail
        } else {
            Lit::Idle
        }
    };
    let lit_any = |ps: &[Dp]| {
        ps.iter()
            .map(|p| lit(*p))
            .fold(Lit::Idle, |a, b| match (a, b) {
                (Lit::Active, _) | (_, Lit::Active) => Lit::Active,
                (Lit::Trail, _) | (_, Lit::Trail) => Lit::Trail,
                _ => Lit::Idle,
            })
    };

    let avail = area.shrink2(vec2(6.0, 4.0));
    let scale = (avail.width() / W).min(avail.height() / H).max(0.05);
    let origin = Pos2::new(avail.center().x - W * scale / 2.0, avail.min.y);
    let t = |x: f32, y: f32| -> Pos2 { origin + vec2(x - X0, y - Y0) * scale };
    let fs = (scale * 1.3).clamp(0.8, 1.7);
    let p = ui.painter_at(area);
    let hits: std::cell::RefCell<Vec<(Rect, String)>> = Default::default();
    let hit = |r: Rect, s: String| hits.borrow_mut().push((r, s));

    let accent = theme.accent_color_primary;
    let ink = theme.secondary_text_color;
    // the book draws Figure C.8's additions in magenta
    let added = Color32::from_rgb(196, 92, 168);
    let idle_wire = theme.editor_line_number_color;
    let idle_added = idle_wire.lerp_to_gamma(added, 0.45);
    let box_fill = theme.panel_background;
    let bg = theme.window_background;
    let stroke_for = |l: Lit, add: bool| match l {
        Lit::Active => Stroke::new(1.8_f32, accent),
        Lit::Trail => Stroke::new(1.1_f32, accent.gamma_multiply(0.3)),
        Lit::Idle => Stroke::new(
            1.0_f32,
            if add {
                idle_added
            } else {
                ink.gamma_multiply(0.8)
            },
        ),
    };
    let fill_for = |l: Lit| match l {
        Lit::Active => box_fill.blend(accent.gamma_multiply(0.16)),
        _ => box_fill,
    };
    let text_col = |l: Lit, add: bool| match l {
        Lit::Active => theme.strong_text_color,
        Lit::Trail => theme.primary_text_color,
        Lit::Idle if add => theme
            .primary_text_color
            .lerp_to_gamma(added, 0.35)
            .gamma_multiply(0.85),
        Lit::Idle => theme.primary_text_color.gamma_multiply(0.85),
    };
    let wire_w = 1.1 * scale.max(0.7);
    let mono = |size: f32| FontId::monospace(size * fs);
    let prop = |size: f32| FontId::proportional(size * fs);

    // ── the bus: along the top, down the right, back along the bottom
    {
        let l = lit(Dp::Bus);
        let col = match l {
            // the whole bus carries the value, but the route below is what matters
            Lit::Active => idle_wire
                .gamma_multiply(0.8)
                .blend(accent.gamma_multiply(0.25)),
            Lit::Trail | Lit::Idle => idle_wire.gamma_multiply(0.8),
        };
        let w = 9.0 * scale;
        let pts = vec![
            t(86.0, 119.0),
            t(886.0, 119.0),
            t(886.0, 775.0),
            t(86.0, 775.0),
        ];
        p.add(Shape::line(pts, Stroke::new(w, col)));
        for y in [119.0, 775.0] {
            p.add(Shape::convex_polygon(
                vec![t(64.0, y), t(88.0, y - 11.0), t(88.0, y + 11.0)],
                col,
                Stroke::NONE,
            ));
            hit(
                Rect::from_min_max(t(64.0, y - 8.0), t(890.0, y + 8.0)),
                "The bus: one 16-bit value per clock cycle, driven by exactly one gate".into(),
            );
        }
        hit(
            Rect::from_min_max(t(880.0, 119.0), t(892.0, 775.0)),
            "The bus".into(),
        );
    }

    // ── wires: idle first, then the trail, then the active ones on top
    for (add, pts) in IDLE_WIRES {
        let pts: Vec<Pos2> = pts.iter().map(|(x, y)| t(*x, *y)).collect();
        let col = if *add { idle_added } else { idle_wire }.gamma_multiply(0.75);
        p.add(Shape::line(pts.clone(), Stroke::new(wire_w, col)));
        arrow_head(&p, &pts, 5.0 * fs, col);
    }
    // (the true state's live wires go under the ones this state uses)
    for (pass, used) in [
        (Lit::Idle, false),
        (Lit::Trail, false),
        (Lit::Active, false),
        (Lit::Active, true),
    ] {
        if pass == Lit::Trail {
            for (x, y) in DOTS {
                p.circle_filled(t(*x, *y), 2.4 * scale.max(0.7), idle_wire);
            }
        }
        for (dp, add, pts) in WIRES {
            if lit(*dp) != pass || (pass == Lit::Active && act.has(*dp) != used) {
                continue;
            }
            let pts: Vec<Pos2> = pts.iter().map(|(x, y)| t(*x, *y)).collect();
            match pass {
                Lit::Idle => {
                    let col = if *add { idle_added } else { idle_wire };
                    p.add(Shape::line(pts.clone(), Stroke::new(wire_w, col)));
                    arrow_head(&p, &pts, 5.0 * fs, col);
                }
                Lit::Trail => {
                    p.add(Shape::line(
                        pts.clone(),
                        Stroke::new(wire_w * 1.2, accent.gamma_multiply(0.22)),
                    ));
                    arrow_head(&p, &pts, 5.5 * fs, accent.gamma_multiply(0.25));
                }
                // live in the true state, but not what this state uses: lit, without the glow
                Lit::Active if !act.has(*dp) => {
                    p.add(Shape::line(
                        pts.clone(),
                        Stroke::new(1.7 * fs, accent.gamma_multiply(0.8)),
                    ));
                    arrow_head(&p, &pts, 6.5 * fs, accent.gamma_multiply(0.8));
                }
                Lit::Active => {
                    p.add(Shape::line(
                        pts.clone(),
                        Stroke::new(7.0 * fs, accent.gamma_multiply(0.15)),
                    ));
                    p.add(Shape::line(pts.clone(), Stroke::new(2.3 * fs, accent)));
                    if let Some(time) = f.flow_time.filter(|_| act.has(*dp)) {
                        flow_dashes(
                            &p,
                            &pts,
                            5.0 * fs,
                            9.0 * fs,
                            (time * 30.0) as f32 * fs,
                            Stroke::new(2.3 * fs, Color32::WHITE.gamma_multiply(0.75)),
                        );
                    }
                    arrow_head(&p, &pts, 8.0 * fs, accent);
                }
            }
        }
    }

    // ── the true state: a wire into a gate that is off is live up to the gate, and stops there
    if let Some(l) = f.live {
        // where each gate's input side is
        const GATE_INPUTS: [(Dp, f32, f32); 11] = [
            (Dp::GatePc, 459.0, 151.0),
            (Dp::GatePcDec, 524.0, 151.0),
            (Dp::GateMarMux, 255.0, 151.0),
            (Dp::GateAlu, 708.0, 738.0),
            (Dp::GateSp, 819.0, 738.0),
            (Dp::GateMdr, 144.0, 797.0),
            (Dp::GatePsr, 300.0, 748.0),
            (Dp::GatePsr, 545.0, 750.0),
            (Dp::GatePsr, 687.0, 655.0),
            (Dp::GateVector, 688.0, 800.0),
            (Dp::GateVector, 779.0, 800.0),
        ];
        for (dp, _, pts) in WIRES.iter().filter(|(d, _, _)| l.stops_at_gate(*d)) {
            // how far along the wire its gate is
            let mut at = 0.0;
            let mut stop = None;
            for w in pts.windows(2) {
                let (a, b) = (Pos2::new(w[0].0, w[0].1), Pos2::new(w[1].0, w[1].1));
                for (_, gx, gy) in GATE_INPUTS.iter().filter(|(d, _, _)| d == dp) {
                    let g = Pos2::new(*gx, *gy);
                    if segment_distance(g, a, b) < 2.0 {
                        stop = Some(at + a.distance(g));
                    }
                }
                if stop.is_some() {
                    break;
                }
                at += a.distance(b);
            }
            let Some(stop) = stop else { continue };
            let model: Vec<Pos2> = pts.iter().map(|(x, y)| Pos2::new(*x, *y)).collect();
            let part: Vec<Pos2> = path_span(&model, 0.0, stop)
                .into_iter()
                .map(|q| t(q.x, q.y))
                .collect();
            if part.len() >= 2 {
                p.add(Shape::line(
                    part,
                    Stroke::new(2.0 * fs, accent.gamma_multiply(0.9)),
                ));
            }
        }
    }

    // ── the bus route: from the gate driving the bus, along it, to each part loading from it
    if act.has(Dp::Bus) {
        let bus = [
            t(86.0, 119.0),
            t(886.0, 119.0),
            t(886.0, 775.0),
            t(86.0, 775.0),
        ];
        // distance along the bus (top, right side, then bottom) of a tap
        let along = |x: f32, y: f32| -> f32 {
            if y <= 120.0 {
                x - 86.0
            } else if y >= 774.0 {
                800.0 + 656.0 + (886.0 - x)
            } else {
                800.0 + (y - 119.0)
            }
        };
        const SOURCES: [(Dp, f32, f32); 8] = [
            (Dp::GatePc, 459.0, 119.0),
            (Dp::GatePcDec, 524.0, 119.0),
            (Dp::GateMarMux, 255.0, 119.0),
            (Dp::GateAlu, 708.0, 775.0),
            (Dp::GateMdr, 144.0, 775.0),
            (Dp::GateSp, 819.0, 775.0),
            (Dp::GateVector, 733.0, 775.0),
            (Dp::GatePsr, 545.0, 775.0),
        ];
        const DESTINATIONS: [(Dp, f32, f32); 10] = [
            (Dp::BusToMar, 281.0, 775.0),
            (Dp::BusToMdr, 110.0, 775.0),
            (Dp::BusToIr, 104.0, 775.0),
            (Dp::BusToReg, 711.0, 119.0),
            (Dp::BusToPcMux, 367.0, 119.0),
            (Dp::BusToLogic, 614.0, 775.0),
            (Dp::BusToPsr, 272.0, 775.0),
            (Dp::BusToPsr, 530.0, 775.0),
            (Dp::BusToPsr, 642.0, 775.0),
            (Dp::BusToPcMux, 367.0, 119.0),
        ];
        // scale distances: the bus points are in screen space
        let k = scale;
        if let Some((_, sx, sy)) = SOURCES.iter().find(|(dp, _, _)| act.has(*dp)) {
            let from = along(*sx, *sy) * k;
            let w = 9.0 * scale;
            for (_, dx, dy) in DESTINATIONS.iter().filter(|(dp, _, _)| act.has(*dp)) {
                let to = along(*dx, *dy) * k;
                let mut route = path_span(&bus, from.min(to), from.max(to));
                if to < from {
                    route.reverse();
                }
                if route.len() < 2 {
                    continue;
                }
                p.add(Shape::line(
                    route.clone(),
                    Stroke::new(w + 6.0 * fs, accent.gamma_multiply(0.18)),
                ));
                p.add(Shape::line(route.clone(), Stroke::new(w, accent)));
                if let Some(time) = f.flow_time {
                    flow_dashes(
                        &p,
                        &route,
                        6.0 * fs,
                        10.0 * fs,
                        (time * 30.0) as f32 * fs,
                        Stroke::new(2.5 * fs, Color32::WHITE.gamma_multiply(0.8)),
                    );
                }
            }
        }
    }

    // ── control inputs: labels with hollow arrows, lit while their signal is asserted
    let on = |sigs: &[Sig]| sigs.iter().any(|s| act.signal(*s));
    let ctl_arrow = |pts: &[(f32, f32)], hot: bool, add: bool| {
        let pts: Vec<Pos2> = pts.iter().map(|(x, y)| t(*x, *y)).collect();
        let col = if hot {
            accent
        } else if add {
            idle_added
        } else {
            ink
        };
        p.add(Shape::line(
            pts.clone(),
            Stroke::new(if hot { 1.8 * fs } else { wire_w }, col),
        ));
        if let [.., a, b] = pts.as_slice() {
            let dir = (*b - *a).normalized();
            let side = vec2(-dir.y, dir.x);
            let s = 5.5 * fs;
            let base = *b - dir * s;
            p.add(Shape::convex_polygon(
                vec![*b, base + side * s * 0.45, base - side * s * 0.45],
                bg,
                Stroke::new(1.0_f32, col),
            ));
        }
    };
    let label = |x: f32, y: f32, align: Align2, s: &str, hot: bool, add: bool| {
        let col = if hot {
            accent
        } else if add {
            ink.lerp_to_gamma(added, 0.5)
        } else {
            ink
        };
        p.text(t(x, y), align, s, prop(8.0), col);
    };
    let ctl = |sigs: &[Sig],
               add: bool,
               text: &str,
               tx: f32,
               ty: f32,
               align: Align2,
               arrow: &[(f32, f32)],
               help: &str| {
        let hot = on(sigs);
        ctl_arrow(arrow, hot, add);
        if !text.is_empty() {
            label(tx, ty, align, text, hot, add);
            hit(
                align.anchor_size(t(tx, ty), vec2(text.len() as f32 * 6.0, 12.0) * scale),
                help.to_string(),
            );
        }
    };
    use Align2 as A;
    let pc_mux = [Sig::PcMuxInc, Sig::PcMuxBus, Sig::PcMuxAdder];
    let addr2 = [
        Sig::Addr2Zero,
        Sig::Addr2Off6,
        Sig::Addr2Off9,
        Sig::Addr2Off11,
    ];
    let aluk = [Sig::AlukAdd, Sig::AlukAnd, Sig::AlukNot, Sig::AlukPassA];
    let sp_mux = [Sig::SpMuxInc, Sig::SpMuxDec, Sig::SpMuxUsp, Sig::SpMuxSsp];
    ctl(
        &[Sig::LdPc],
        false,
        "LD.PC",
        400.0,
        240.0,
        A::RIGHT_CENTER,
        &[(402.0, 240.0), (425.0, 240.0)],
        "LD.PC: the PC loads PCMUX's output",
    );
    ctl(
        &pc_mux,
        false,
        "PCMUX",
        393.0,
        282.0,
        A::RIGHT_CENTER,
        &[(395.0, 282.0), (418.0, 282.0)],
        "PCMUX select: PC + 1, the bus, or the address adder",
    );
    ctl(
        &[Sig::MarMuxAdder, Sig::MarMuxZext],
        false,
        "",
        0.0,
        0.0,
        A::LEFT_TOP,
        &[(198.0, 249.0), (219.0, 249.0)],
        "",
    );
    ctl(
        &addr2,
        false,
        "ADDR2MUX",
        196.0,
        437.0,
        A::RIGHT_CENTER,
        &[(200.0, 425.0), (233.0, 425.0)],
        "ADDR2MUX select: 0, offset6, PCoffset9 or PCoffset11",
    );
    ctl(
        &[Sig::Addr1Pc, Sig::Addr1Base],
        false,
        "ADDR1MUX",
        464.0,
        424.0,
        A::LEFT_CENTER,
        &[(462.0, 424.0), (444.0, 424.0)],
        "ADDR1MUX select: PC or BaseR",
    );
    ctl(
        &[Sig::LdReg],
        false,
        "LD.REG",
        633.0,
        288.0,
        A::RIGHT_CENTER,
        &[(635.0, 288.0), (665.0, 288.0)],
        "LD.REG: the register file writes DR",
    );
    ctl(
        &[Sig::LdReg],
        false,
        "DR",
        633.0,
        319.0,
        A::RIGHT_CENTER,
        &[(635.0, 319.0), (665.0, 319.0)],
        "DR: which register is written",
    );
    let sr2 = act.has(Dp::Sr2Out);
    ctl_arrow(&[(635.0, 353.0), (665.0, 353.0)], sr2, false);
    label(633.0, 353.0, A::RIGHT_CENTER, "SR2", sr2, false);
    let sr1 = act.has(Dp::Sr1Out) || act.has(Dp::RegToAddr1) || act.has(Dp::Sr1ToSp);
    ctl_arrow(&[(780.0, 353.0), (757.0, 353.0)], sr1, false);
    label(782.0, 353.0, A::LEFT_CENTER, "SR1", sr1, false);
    ctl(
        &[Sig::LdIr],
        false,
        "LD.IR",
        164.0,
        649.0,
        A::LEFT_CENTER,
        &[(162.0, 649.0), (137.0, 649.0)],
        "LD.IR: IR loads the bus",
    );
    ctl(
        &[Sig::LdCc],
        false,
        "LD.CC",
        576.0,
        641.0,
        A::RIGHT_CENTER,
        &[(578.0, 641.0), (605.0, 641.0)],
        "LD.CC: N, Z and P load PSRMUX's output",
    );
    ctl(
        &aluk,
        false,
        "ALUK",
        594.0,
        570.0,
        A::LEFT_CENTER,
        &[
            (560.0, 534.0),
            (560.0, 522.0),
            (588.0, 522.0),
            (588.0, 578.0),
            (655.0, 578.0),
        ],
        "ALUK: ADD, AND, NOT or PASSA",
    );
    ctl(
        &[Sig::LdMdr],
        false,
        "LD.MDR",
        197.0,
        831.0,
        A::LEFT_CENTER,
        &[(195.0, 831.0), (171.0, 831.0)],
        "LD.MDR: MDR loads the MIO.EN mux's output",
    );
    ctl(
        &[Sig::LdMar],
        false,
        "LD.MAR",
        329.0,
        831.0,
        A::LEFT_CENTER,
        &[(327.0, 831.0), (303.0, 831.0)],
        "LD.MAR: MAR loads the bus",
    );
    ctl(
        &[Sig::MioEn],
        false,
        "MIO.EN",
        192.0,
        874.0,
        A::LEFT_CENTER,
        &[(190.0, 874.0), (166.0, 874.0)],
        "MIO.EN: MDR takes memory or I/O rather than the bus",
    );
    ctl(
        &[Sig::MioEn],
        false,
        "MIO.EN",
        410.0,
        878.0,
        A::CENTER_BOTTOM,
        &[(410.0, 880.0), (410.0, 902.0)],
        "MIO.EN: enable memory or I/O",
    );
    ctl(
        &[Sig::RwRead, Sig::RwWrite],
        false,
        "R.W",
        336.0,
        878.0,
        A::CENTER_BOTTOM,
        &[(336.0, 880.0), (336.0, 903.0)],
        "R.W: read or write",
    );
    ctl(
        &[Sig::RwRead, Sig::RwWrite],
        false,
        "",
        0.0,
        0.0,
        A::LEFT_TOP,
        &[
            (336.0, 887.0),
            (313.0, 887.0),
            (313.0, 920.0),
            (295.0, 920.0),
        ],
        "",
    );
    ctl(
        &[Sig::MioEn],
        false,
        "",
        0.0,
        0.0,
        A::LEFT_TOP,
        &[(332.0, 945.0), (332.0, 966.0), (295.0, 966.0)],
        "",
    );
    ctl(
        &[Sig::RwRead],
        false,
        "",
        0.0,
        0.0,
        A::LEFT_TOP,
        &[(355.0, 945.0), (355.0, 972.0)],
        "",
    );
    ctl(
        &[],
        false,
        "",
        0.0,
        0.0,
        A::LEFT_TOP,
        &[(540.0, 590.0), (540.0, 573.0)],
        "",
    );
    for x in [428.0, 449.0, 468.0] {
        ctl_arrow(&[(x, 534.0), (x, 520.0)], false, false);
    }
    // Figure C.8's control inputs
    ctl(
        &[Sig::LdPriv],
        true,
        "LD.Priv",
        211.0,
        697.0,
        A::RIGHT_CENTER,
        &[(213.0, 697.0), (231.0, 697.0)],
        "LD.Priv: PSR[15] loads PSRMUX's output",
    );
    ctl(
        &[Sig::SetPriv, Sig::LdPriv],
        true,
        "PSRMUX",
        206.0,
        735.0,
        A::RIGHT_CENTER,
        &[(208.0, 735.0), (224.0, 735.0)],
        "PSRMUX: the bus or Set.Priv",
    );
    ctl(
        &[],
        true,
        "LD.ACV",
        322.0,
        680.0,
        A::CENTER_TOP,
        &[(322.0, 676.0), (322.0, 660.0), (352.0, 660.0)],
        "LD.ACV: load the access control violation flag",
    );
    ctl(
        &[Sig::LdPriority],
        true,
        "LD.Priority",
        468.0,
        692.0,
        A::RIGHT_CENTER,
        &[(470.0, 692.0), (488.0, 692.0)],
        "LD.Priority: PSR[10:8] loads PSRMUX's output",
    );
    ctl(
        &[Sig::LdPriority],
        true,
        "PSRMUX",
        460.0,
        731.0,
        A::RIGHT_CENTER,
        &[(462.0, 731.0), (478.0, 731.0)],
        "PSRMUX: the bus or the interrupt's priority",
    );
    ctl(
        &[Sig::LdCc],
        true,
        "PSRMUX",
        586.0,
        690.0,
        A::RIGHT_CENTER,
        &[(588.0, 687.0), (606.0, 687.0)],
        "PSRMUX: the bus or LOGIC",
    );
    ctl(
        &[Sig::LdSavedUsp],
        true,
        "",
        0.0,
        0.0,
        A::LEFT_TOP,
        &[(763.0, 548.0), (773.0, 548.0)],
        "",
    );
    ctl(
        &[Sig::LdSavedSsp],
        true,
        "",
        0.0,
        0.0,
        A::LEFT_TOP,
        &[(876.0, 548.0), (866.0, 548.0)],
        "",
    );
    ctl(
        &sp_mux,
        true,
        "SPMUX",
        745.0,
        650.0,
        A::CENTER_CENTER,
        &[(740.0, 625.0), (768.0, 625.0)],
        "SPMUX select: R6 + 1, R6 - 1, Saved.SSP or Saved.USP",
    );
    ctl(
        &[Sig::TableMux],
        true,
        "TableMUX",
        602.0,
        792.0,
        A::LEFT_CENTER,
        &[(600.0, 792.0), (583.0, 792.0), (583.0, 805.0)],
        "TableMUX: x00 (traps) or x01 (interrupts and exceptions)",
    );
    ctl(
        &[Sig::LdVector],
        true,
        "TableMUX",
        814.0,
        876.0,
        A::LEFT_CENTER,
        &[(812.0, 876.0), (797.0, 876.0)],
        "Picks IR[7:0] (TRAP) or VectorMUX for Vector (labelled TableMUX in the book)",
    );
    ctl(
        &[Sig::VectorMux],
        true,
        "VectorMUX",
        792.0,
        909.0,
        A::LEFT_CENTER,
        &[(790.0, 909.0), (781.0, 909.0)],
        "VectorMUX: INTV, x00, x01 or x02",
    );
    ctl(
        &[Sig::LdVector],
        true,
        "LD.Vector",
        814.0,
        837.0,
        A::LEFT_CENTER,
        &[(812.0, 837.0), (802.0, 837.0)],
        "LD.Vector: Vector loads its mux's output",
    );
    for (x, sig, name) in [
        (757.0, Sig::LdSavedUsp, "LD.SavedUSP"),
        (875.0, Sig::LdSavedSsp, "LD.SavedSSP"),
    ] {
        let hot = act.signal(sig);
        let col = if hot {
            accent
        } else {
            ink.lerp_to_gamma(added, 0.5)
        };
        let galley = p.layout_no_wrap(name.to_string(), prop(7.5), col);
        let size = galley.size();
        let pos = t(x, 552.0) + vec2(-size.y / 2.0, 0.0);
        p.add(TextShape::new(pos, galley, col).with_angle(-std::f32::consts::FRAC_PI_2));
        hit(
            Rect::from_min_max(t(x - 6.0, 470.0), t(x + 6.0, 555.0)),
            format!("{name}: save R6"),
        );
    }

    // gates onto the bus, with their names
    for (dp, sig, x, y, up, name, tx, align, add) in [
        (
            Dp::GateMarMux,
            Sig::GateMarMux,
            255.0,
            144.0,
            true,
            "GateMARMUX",
            243.0,
            A::RIGHT_CENTER,
            false,
        ),
        (
            Dp::GatePc,
            Sig::GatePc,
            459.0,
            144.0,
            true,
            "GatePC",
            447.0,
            A::RIGHT_CENTER,
            false,
        ),
        (
            Dp::GatePcDec,
            Sig::GatePcMinus1,
            524.0,
            144.0,
            true,
            "GatePC-1",
            536.0,
            A::LEFT_CENTER,
            true,
        ),
        (
            Dp::GateAlu,
            Sig::GateAlu,
            708.0,
            745.0,
            false,
            "GateALU",
            720.0,
            A::LEFT_CENTER,
            false,
        ),
        (
            Dp::GateSp,
            Sig::GateSp,
            819.0,
            745.0,
            false,
            "GateSP",
            831.0,
            A::LEFT_CENTER,
            true,
        ),
        (
            Dp::GateMdr,
            Sig::GateMdr,
            144.0,
            790.0,
            true,
            "GateMDR",
            156.0,
            A::LEFT_CENTER,
            false,
        ),
        (
            Dp::GatePsr,
            Sig::GatePsr,
            300.0,
            755.0,
            false,
            "GatePSR",
            312.0,
            A::LEFT_CENTER,
            true,
        ),
        (
            Dp::GatePsr,
            Sig::GatePsr,
            545.0,
            757.0,
            false,
            "GatePSR",
            557.0,
            A::LEFT_CENTER,
            true,
        ),
        (
            Dp::GatePsr,
            Sig::GatePsr,
            687.0,
            662.0,
            false,
            "GatePSR",
            675.0,
            A::RIGHT_CENTER,
            true,
        ),
        (
            Dp::GateVector,
            Sig::GateVector,
            688.0,
            793.0,
            true,
            "",
            0.0,
            A::LEFT_CENTER,
            true,
        ),
        (
            Dp::GateVector,
            Sig::GateVector,
            779.0,
            793.0,
            true,
            "GateVector",
            733.0,
            A::CENTER_CENTER,
            true,
        ),
    ] {
        let l = lit(dp);
        let pts = if up {
            vec![t(x - 7.0, y + 7.0), t(x + 7.0, y + 7.0), t(x, y - 7.0)]
        } else {
            vec![t(x - 7.0, y - 7.0), t(x + 7.0, y - 7.0), t(x, y + 7.0)]
        };
        p.add(Shape::convex_polygon(
            pts,
            if l == Lit::Active { accent } else { bg },
            stroke_for(l, add),
        ));
        if !name.is_empty() {
            label(tx, y, align, name, act.signal(sig), add);
        }
        hit(
            Rect::from_center_size(t(x, y), vec2(18.0, 16.0) * scale),
            format!(
                "{}: a tri-state buffer that puts this value on the bus",
                if name.is_empty() { "GateVector" } else { name }
            ),
        );
    }

    // ── boxes
    let boxed = |l: Lit,
                 add: bool,
                 x0: f32,
                 y0: f32,
                 x1: f32,
                 y1: f32,
                 name: &str,
                 size: f32,
                 help: String| {
        let r = Rect::from_min_max(t(x0, y0), t(x1, y1));
        p.rect(
            r,
            CornerRadius::same(2),
            fill_for(l),
            stroke_for(l, add),
            egui::StrokeKind::Inside,
        );
        if !name.is_empty() {
            let size = fit(size * fs, name, r.width() - 3.0);
            p.text(
                r.center(),
                Align2::CENTER_CENTER,
                name,
                FontId::proportional(size),
                text_col(l, add),
            );
        }
        hit(r, help);
        r
    };
    boxed(
        lit(Dp::Pc),
        false,
        425.0,
        230.0,
        491.0,
        250.0,
        "PC",
        11.0,
        format!(
            "PC: address of the next instruction\n{}",
            disasm::value_text(snap.pc)
        ),
    );
    boxed(
        lit(Dp::PcInc),
        false,
        512.0,
        265.0,
        537.0,
        290.0,
        "+1",
        10.0,
        "Incrementer: PC + 1".into(),
    );
    boxed(lit(Dp::PcDec), true, 512.0, 172.0, 537.0, 197.0, "-1", 10.0, "Decrementer: PC - 1, pushed by interrupts and exceptions so RTI returns to the right place".into());
    boxed(
        lit(Dp::Zext),
        false,
        82.0,
        353.0,
        126.0,
        373.0,
        "ZEXT",
        9.0,
        "ZEXT: zero-extends IR[7:0]".into(),
    );
    boxed(
        lit(Dp::Sext11),
        false,
        189.0,
        456.0,
        232.0,
        475.0,
        "SEXT",
        9.0,
        "SEXT [10:0]: sign-extends PCoffset11 (JSR)".into(),
    );
    boxed(
        lit(Dp::Sext9),
        false,
        189.0,
        486.0,
        232.0,
        506.0,
        "SEXT",
        9.0,
        "SEXT [8:0]: sign-extends PCoffset9 (LD, ST, LDI, STI, LEA, BR)".into(),
    );
    boxed(
        lit(Dp::Sext6),
        false,
        189.0,
        518.0,
        232.0,
        537.0,
        "SEXT",
        9.0,
        "SEXT [5:0]: sign-extends offset6 (LDR, STR)".into(),
    );
    boxed(
        lit(Dp::Sext5),
        false,
        189.0,
        547.0,
        232.0,
        566.0,
        "SEXT",
        9.0,
        "SEXT [4:0]: sign-extends imm5 (ADD, AND)".into(),
    );
    boxed(
        lit(Dp::Ir),
        false,
        71.0,
        639.0,
        137.0,
        659.0,
        "IR",
        11.0,
        format!(
            "IR: the instruction being executed\n{}",
            disasm::value_text(snap.ir)
        ),
    );
    boxed(
        lit(Dp::Control),
        false,
        410.0,
        534.0,
        578.0,
        573.0,
        "CONTROL LOGIC",
        10.5,
        "Control logic: the state machine that asserts the control signals each clock cycle".into(),
    );
    boxed(
        lit(Dp::Logic),
        false,
        595.0,
        717.0,
        632.0,
        742.0,
        "LOGIC",
        8.5,
        "LOGIC: works out N, Z or P from the value on the bus".into(),
    );
    boxed(
        lit(Dp::Mdr),
        false,
        117.0,
        821.0,
        171.0,
        840.0,
        "MDR",
        10.0,
        format!(
            "MDR: Memory Data Register\n{}",
            disasm::value_text(snap.mdr)
        ),
    );
    boxed(
        lit(Dp::Mar),
        false,
        257.0,
        821.0,
        303.0,
        840.0,
        "MAR",
        10.0,
        format!(
            "MAR: Memory Address Register\n{}",
            disasm::value_text(snap.mar)
        ),
    );
    let mem = lit(Dp::Memory);
    boxed(
        mem,
        false,
        221.0,
        894.0,
        295.0,
        979.0,
        "",
        10.0,
        "MEMORY: reads into MDR or writes from MDR at the address in MAR".into(),
    );
    p.text(
        t(258.0, 933.0),
        A::CENTER_CENTER,
        "MEMORY",
        prop(9.0),
        text_col(mem, false),
    );
    p.text(t(258.0, 966.0), A::CENTER_CENTER, "MEM.EN", prop(7.5), ink);
    let ctl_logic = lit_any(&[Dp::MarToMem]);
    boxed(
        ctl_logic,
        false,
        327.0,
        902.0,
        420.0,
        945.0,
        "",
        9.0,
        "ADDR. CTL. LOGIC: decides whether MAR names memory or a device register".into(),
    );
    p.text(
        t(373.0, 916.0),
        A::CENTER_CENTER,
        "ADDR. CTL.",
        prop(8.0),
        text_col(ctl_logic, false),
    );
    p.text(
        t(373.0, 930.0),
        A::CENTER_CENTER,
        "LOGIC",
        prop(8.0),
        text_col(ctl_logic, false),
    );
    for (x0, y0, x1, y1, name, help) in [
        (
            468.0,
            886.0,
            514.0,
            904.0,
            "KBDR",
            "KBDR: keyboard data (xFE02)",
        ),
        (
            517.0,
            916.0,
            564.0,
            936.0,
            "KBSR",
            "KBSR: keyboard status (xFE00)",
        ),
        (
            589.0,
            887.0,
            646.0,
            907.0,
            "DDR",
            "DDR: display data (xFE06)",
        ),
        (
            647.0,
            916.0,
            702.0,
            936.0,
            "DSR",
            "DSR: display status (xFE04)",
        ),
    ] {
        boxed(Lit::Idle, false, x0, y0, x1, y1, name, 8.5, help.into());
    }
    let dash = Stroke::new(1.0_f32, ink.gamma_multiply(0.6));
    for (a, b) in [
        ((452.0, 864.0), (572.0, 955.0)),
        ((572.0, 864.0), (706.0, 955.0)),
    ] {
        let pts = [
            t(a.0, a.1),
            t(b.0, a.1),
            t(b.0, b.1),
            t(a.0, b.1),
            t(a.0, a.1),
        ];
        for w in pts.windows(2) {
            p.extend(Shape::dashed_line(&[w[0], w[1]], dash, 4.0 * fs, 3.0 * fs));
        }
    }
    p.text(t(458.0, 873.0), A::LEFT_CENTER, "INPUT", prop(7.5), ink);
    p.text(t(578.0, 873.0), A::LEFT_CENTER, "OUTPUT", prop(7.5), ink);

    // Figure C.8's boxes
    boxed(
        Lit::Idle,
        true,
        352.0,
        651.0,
        387.0,
        669.0,
        "ACV",
        8.5,
        "ACV: set when User mode accesses privileged memory".into(),
    );
    boxed(
        Lit::Idle,
        true,
        352.0,
        691.0,
        397.0,
        716.0,
        "LOGIC",
        8.0,
        "Works out ACV from the address on the bus and PSR[15]".into(),
    );
    boxed(
        Lit::Idle,
        true,
        488.0,
        635.0,
        520.0,
        652.0,
        "A>B",
        8.0,
        "Compares an interrupt's priority with the running program's (INT)".into(),
    );
    boxed(
        lit(Dp::Priv),
        true,
        231.0,
        684.0,
        265.0,
        710.0,
        "Priv",
        9.0,
        format!("PSR[15]: 1 = User mode, 0 = Supervisor mode\nPSR = x{psr:04X}"),
    );
    boxed(
        lit(Dp::Priority),
        true,
        488.0,
        682.0,
        539.0,
        702.0,
        "Priority",
        8.5,
        format!("PSR[10:8]: the running program's priority\nPSR = x{psr:04X}"),
    );
    boxed(lit(Dp::Table), true, 667.0, 827.0, 709.0, 846.0, "Table", 9.0, format!("Table: x00 for the trap vector table, x01 for the interrupt vector table\nx{table:02X}"));
    boxed(
        lit(Dp::Vector),
        true,
        752.0,
        827.0,
        802.0,
        846.0,
        "Vector",
        9.0,
        format!("Vector: the entry in the table\nx{vector:02X}"),
    );
    for (y, name, part) in [(800.0, "x00", Dp::TabX00), (826.0, "x01", Dp::TabX01)] {
        boxed(
            lit(part),
            true,
            500.0,
            y,
            552.0,
            y + 19.0,
            name,
            8.5,
            "A constant for Table".into(),
        );
    }
    for (y, name, part) in [
        (938.0, "INTV", Dp::VecIntv),
        (963.0, "x00", Dp::VecX00),
        (988.0, "x01", Dp::VecX01),
        (1014.0, "x02", Dp::VecX02),
    ] {
        boxed(lit(part), true, 807.0, y, 860.0, y + 18.0, name, 8.5, "Vector constants: INTV from the interrupting device, x00 privilege, x01 illegal opcode, x02 ACV".into());
    }
    for (dp, x0, y0, x1, name, help) in [
        (
            Dp::SavedUsp,
            773.0,
            478.0,
            786.0,
            "Saved.USP",
            format!("Saved.USP: the user stack pointer while in Supervisor mode\nx{saved_usp:04X}"),
        ),
        (
            Dp::SavedSsp,
            852.0,
            478.0,
            866.0,
            "Saved.SSP",
            format!("Saved.SSP: the supervisor stack pointer while in User mode\nx{saved_ssp:04X}"),
        ),
    ] {
        let l = lit(dp);
        let r = boxed(l, true, x0, y0, x1, 555.0, "", 8.0, help);
        let galley = p.layout_no_wrap(name.to_string(), prop(7.5), text_col(l, true));
        let size = galley.size();
        let pos = r.center() + vec2(-size.y / 2.0, size.x / 2.0);
        p.add(
            TextShape::new(pos, galley, text_col(l, true)).with_angle(-std::f32::consts::FRAC_PI_2),
        );
    }
    boxed(
        lit(Dp::SpIncPath),
        true,
        799.0,
        535.0,
        813.0,
        555.0,
        "+1",
        8.0,
        "R6 + 1".into(),
    );
    boxed(
        lit(Dp::SpDecPath),
        true,
        825.0,
        535.0,
        839.0,
        555.0,
        "-1",
        8.0,
        "R6 - 1".into(),
    );

    // register file, with the registers' values
    {
        let l = lit(Dp::RegFile);
        boxed(l, false, 665.0, 242.0, 757.0, 367.0, "", 10.0, "REG FILE: R0–R7. Reads SR1 and SR2, writes DR when LD.REG is asserted. R6 is the stack pointer (SP)".into());
        p.text(
            t(711.0, 255.0),
            A::CENTER_CENTER,
            "REG FILE",
            prop(9.0),
            text_col(l, false),
        );
        let read = f.rec.map_or(0, |r| r.regs_read);
        let written = f.rec.and_then(|r| r.reg_written);
        for i in 0..8usize {
            let (x, y) = (668.0 + (i / 4) as f32 * 44.5, 265.0 + (i % 4) as f32 * 17.0);
            let cell = Rect::from_min_size(t(x, y), vec2(42.0, 15.0) * scale);
            let is_w = written == Some(i as u8);
            let is_r = read & (1 << i) != 0;
            if is_w {
                p.rect_filled(cell, CornerRadius::same(2), accent.gamma_multiply(0.4));
            } else if is_r {
                p.rect_stroke(
                    cell,
                    CornerRadius::same(2),
                    Stroke::new(1.2_f32, theme.accent_color_secondary),
                    egui::StrokeKind::Inside,
                );
            }
            let changed = f.prev.is_some_and(|q| q.r[i] != snap.r[i]);
            let text = format!("R{i} {:04X}", snap.r[i]);
            p.text(
                cell.center(),
                A::CENTER_CENTER,
                &text,
                FontId::monospace(fit(9.0 * fs, &text, cell.width() - 2.0)),
                if is_w || changed {
                    theme.strong_text_color
                } else {
                    theme.primary_text_color.gamma_multiply(0.85)
                },
            );
            hit(cell, format!("R{i} = {}", disasm::value_text(snap.r[i])));
        }
        for (x, s) in [(685.0, "SR2"), (737.0, "SR1")] {
            p.text(t(x, 344.0), A::CENTER_CENTER, s, prop(7.0), ink);
            p.text(t(x, 355.0), A::CENTER_CENTER, "OUT", prop(7.0), ink);
        }
    }

    // N Z P
    {
        let l = lit(Dp::Nzp);
        let r = boxed(
            l,
            false,
            605.0,
            632.0,
            650.0,
            650.0,
            "",
            9.0,
            "Condition codes N, Z, P (PSR[2:0])".into(),
        );
        for (k, c) in ["N", "Z", "P"].iter().enumerate() {
            let x = 605.0 + k as f32 * 15.0;
            if k > 0 {
                p.vline(t(x, 0.0).x, r.y_range(), stroke_for(l, false));
            }
            let set = snap.nzp & (4 >> k) != 0;
            let cell = Rect::from_min_size(t(x, 632.0), vec2(15.0, 18.0) * scale).shrink(1.5);
            if set {
                p.rect_filled(cell, CornerRadius::same(2), accent.gamma_multiply(0.45));
            }
            p.text(
                cell.center(),
                A::CENTER_CENTER,
                *c,
                mono(8.0),
                if set { theme.strong_text_color } else { ink },
            );
        }
    }

    // control's other outputs and inputs
    for x in [482.0, 497.0, 513.0] {
        p.circle_filled(t(x, 527.0), 1.8 * fs, ink);
    }
    p.text(t(540.0, 600.0), A::CENTER_CENTER, "R", prop(8.0), ink);

    // muxes: (top left, top right, top y, bottom left, bottom right, bottom y)
    let mux = |l: Lit, add: bool, q: [f32; 6], name: &str, help: &str| {
        let [a, b, ty, c, d, by] = q;
        let pts = vec![t(a, ty), t(b, ty), t(d, by), t(c, by)];
        p.add(Shape::convex_polygon(pts, fill_for(l), stroke_for(l, add)));
        let r = Rect::from_min_max(t(a.min(c), ty), t(b.max(d), by));
        if !name.is_empty() {
            let size = fit(8.5 * fs, name, r.width() * 0.78);
            p.text(
                r.center(),
                A::CENTER_CENTER,
                name,
                FontId::proportional(size),
                text_col(l, add),
            );
        }
        hit(r, help.to_string());
    };
    mux(
        lit(Dp::MarMux),
        false,
        [227.0, 284.0, 236.0, 215.0, 295.0, 262.0],
        "MARMUX",
        "MARMUX: the address adder or ZEXT(IR[7:0])",
    );
    mux(
        lit(Dp::PcMux),
        false,
        [432.0, 490.0, 270.0, 418.0, 500.0, 296.0],
        "PCMUX",
        "PCMUX: the PC's next value is PC + 1, the bus, or the address adder",
    );
    mux(
        lit(Dp::Addr2Mux),
        false,
        [241.0, 325.0, 413.0, 228.0, 333.0, 439.0],
        "",
        "ADDR2MUX: the adder's offset",
    );
    mux(
        lit(Dp::Addr1Mux),
        false,
        [387.0, 440.0, 414.0, 380.0, 447.0, 440.0],
        "",
        "ADDR1MUX: the adder's base, PC or SR1",
    );
    mux(
        lit(Dp::Sr2Mux),
        false,
        [637.0, 700.0, 508.0, 645.0, 694.0, 533.0],
        "SR2MUX",
        "SR2MUX: the ALU's B input is SR2 or SEXT(imm5), chosen by IR[5]",
    );
    mux(
        lit_any(&[Dp::BusToMdr, Dp::MemToMdr]),
        false,
        [135.0, 155.0, 864.0, 117.0, 172.0, 884.0],
        "",
        "MIO.EN mux: MDR loads from the bus, or from memory/IO",
    );
    mux(
        lit(Dp::SpMux),
        true,
        [765.0, 872.0, 612.0, 772.0, 865.0, 640.0],
        "",
        "SPMUX: picks the new stack pointer",
    );
    mux(
        lit(Dp::PrivMux),
        true,
        [227.0, 277.0, 725.0, 217.0, 285.0, 745.0],
        "",
        "PSRMUX for PSR[15]: the bus or Set.Priv",
    );
    mux(
        lit(Dp::PriorityMux),
        true,
        [485.0, 522.0, 721.0, 472.0, 537.0, 741.0],
        "",
        "PSRMUX for PSR[10:8]: the bus or the interrupt's priority",
    );
    mux(
        lit(Dp::NzpMux),
        true,
        [612.0, 655.0, 677.0, 600.0, 662.0, 697.0],
        "",
        "PSRMUX for N, Z, P: the bus or LOGIC",
    );
    mux(
        lit(Dp::VectorSel),
        true,
        [765.0, 790.0, 870.0, 754.0, 800.0, 882.0],
        "",
        "Vector's mux: IR[7:0] or VectorMUX",
    );
    mux(
        lit(Dp::VectorMux),
        true,
        [727.0, 770.0, 900.0, 710.0, 787.0, 919.0],
        "",
        "VectorMUX",
    );
    {
        let l = lit(Dp::TableMux);
        p.add(Shape::convex_polygon(
            vec![
                t(576.0, 800.0),
                t(590.0, 808.0),
                t(590.0, 836.0),
                t(576.0, 844.0),
            ],
            fill_for(l),
            stroke_for(l, true),
        ));
        let l = lit(Dp::MemToMdr);
        p.add(Shape::convex_polygon(
            vec![
                t(350.0, 985.0),
                t(370.0, 972.0),
                t(370.0, 1030.0),
                t(350.0, 1018.0),
            ],
            fill_for(l),
            stroke_for(l, false),
        ));
        p.text(t(345.0, 1024.0), A::RIGHT_CENTER, "INMUX", prop(8.0), ink);
        hit(
            Rect::from_min_max(t(350.0, 972.0), t(370.0, 1030.0)),
            "INMUX: memory, KBDR, KBSR or DSR into MDR".into(),
        );
        p.text(t(212.0, 933.0), A::RIGHT_CENTER, "R", prop(8.0), ink);
        ctl_arrow(&[(221.0, 933.0), (214.0, 933.0)], false, false);
    }

    // address adder and ALU (notched trapezoids)
    {
        let l = lit(Dp::Adder);
        p.add(Shape::convex_polygon(
            vec![
                t(322.0, 352.0),
                t(386.0, 352.0),
                t(406.0, 383.0),
                t(302.0, 383.0),
            ],
            fill_for(l),
            stroke_for(l, false),
        ));
        p.add(Shape::convex_polygon(
            vec![t(343.0, 384.0), t(354.0, 371.0), t(365.0, 384.0)],
            bg,
            Stroke::NONE,
        ));
        p.add(Shape::line(
            vec![t(343.0, 383.0), t(354.0, 371.0), t(365.0, 383.0)],
            stroke_for(l, false),
        ));
        p.text(
            t(354.0, 361.0),
            A::CENTER_CENTER,
            "+",
            mono(12.0),
            text_col(l, false),
        );
        hit(
            Rect::from_min_max(t(302.0, 352.0), t(406.0, 383.0)),
            "The address adder: ADDR1MUX + ADDR2MUX".into(),
        );

        let l = lit(Dp::Alu);
        p.add(Shape::convex_polygon(
            vec![
                t(652.0, 560.0),
                t(765.0, 560.0),
                t(745.0, 597.0),
                t(675.0, 597.0),
            ],
            fill_for(l),
            stroke_for(l, false),
        ));
        p.add(Shape::convex_polygon(
            vec![t(697.0, 559.0), t(708.0, 572.0), t(719.0, 559.0)],
            bg,
            Stroke::NONE,
        ));
        p.add(Shape::line(
            vec![t(697.0, 560.0), t(708.0, 572.0), t(719.0, 560.0)],
            stroke_for(l, false),
        ));
        p.text(t(672.0, 568.0), A::CENTER_CENTER, "B", prop(7.5), ink);
        p.text(t(742.0, 568.0), A::CENTER_CENTER, "A", prop(7.5), ink);
        let op = alu_op(&act);
        p.text(
            t(708.0, 585.0),
            A::CENTER_CENTER,
            if l == Lit::Active && !op.is_empty() {
                op
            } else {
                "ALU"
            },
            prop(10.0),
            if l == Lit::Active {
                accent
            } else {
                text_col(l, false)
            },
        );
        hit(
            Rect::from_min_max(t(652.0, 560.0), t(765.0, 597.0)),
            "The ALU: ADD, AND, NOT, or pass A through (PASSA)".into(),
        );
    }

    // static field labels
    // the IR fields the sign extenders, ZEXT and SR2MUX take: the bits themselves while in use
    for (x, y, dp, hi, lo) in [
        (110.0, 395.0, Dp::Zext, 7, 0),
        (107.0, 450.0, Dp::Sext11, 10, 0),
        (107.0, 481.0, Dp::Sext9, 8, 0),
        (107.0, 513.0, Dp::Sext6, 5, 0),
        (107.0, 542.0, Dp::Sext5, 4, 0),
        (107.0, 572.0, Dp::Sr2Mux, 5, 5),
    ] {
        let used = lit(dp) == Lit::Active;
        let name = if hi == lo {
            format!("[{hi}]")
        } else {
            format!("[{hi}:{lo}]")
        };
        if used || f.live.is_some() {
            let bits: String = (lo..=hi)
                .rev()
                .map(|b| if snap.ir >> b & 1 == 1 { '1' } else { '0' })
                .collect();
            let text = format!("{name} {bits}");
            p.text(
                t(x, y),
                A::LEFT_CENTER,
                &text,
                mono(7.0),
                if used { accent } else { ink },
            );
            hit(
                A::LEFT_CENTER.anchor_size(t(x, y), vec2(text.len() as f32 * 4.6, 10.0) * fs),
                format!("IR{name} = {bits}: the bits of IR this wire takes"),
            );
        } else {
            label(x, y, A::LEFT_CENTER, &name, false, false);
        }
    }
    for (x, y, s, add) in [
        (305.0, 720.0, "[15]", true),
        (551.0, 742.0, "[10:8]", true),
        (662.0, 627.0, "[2:0]", true),
        (648.0, 760.0, "[2:0]", true),
        (695.0, 810.0, "[15:8]", true),
        (786.0, 810.0, "[7:0]", true),
        (865.0, 790.0, "[7:0]", true),
        (392.0, 641.0, "Interrupt", true),
        (392.0, 651.0, "Priority", true),
        (512.0, 615.0, "INT", true),
        (468.0, 655.0, "A", true),
        (493.0, 666.0, "B", true),
        (170.0, 763.0, "Set.Priv", true),
    ] {
        let hot = add && s == "Set.Priv" && act.signal(Sig::SetPriv);
        label(x, y, A::LEFT_CENTER, s, hot, add);
    }
    label(
        322.0,
        476.0,
        A::CENTER_CENTER,
        "0",
        act.has(Dp::ZeroToAddr2),
        false,
    );

    // ── values
    let placed: std::cell::RefCell<Vec<Rect>> = Default::default();
    let chip = |pos: Pos2, align: Align2, text: String, hot: bool| -> Rect {
        let col = if hot { bg } else { theme.memory_value_color };
        let galley = p.layout_no_wrap(text, mono(9.0), col);
        let r = align.anchor_size(pos, galley.size() + vec2(6.0, 2.0) * fs);
        p.rect_filled(
            r,
            CornerRadius::same(3),
            if hot { accent } else { theme.code_bg_color },
        );
        p.galley(r.center() - galley.size() / 2.0, galley, col);
        placed.borrow_mut().push(r);
        r
    };
    let changed = |g: fn(&Snapshot) -> u16| f.prev.is_some_and(|q| g(q) != g(snap));
    chip(
        t(422.0, 256.0),
        A::RIGHT_CENTER,
        format!("x{:04X}", snap.pc),
        act.signal(Sig::LdPc) || changed(|s| s.pc),
    );
    chip(
        t(140.0, 663.0),
        A::LEFT_TOP,
        format!("x{:04X}", snap.ir),
        act.signal(Sig::LdIr),
    );
    chip(
        t(175.0, 843.0),
        A::LEFT_TOP,
        format!("x{:04X}", snap.mdr),
        act.signal(Sig::LdMdr) || changed(|s| s.mdr),
    );
    chip(
        t(306.0, 818.0),
        A::LEFT_BOTTOM,
        format!("x{:04X}", snap.mar),
        act.signal(Sig::LdMar),
    );

    // ── what each wire carries: the ones in use, or all of them
    {
        let mut shown: Vec<Dp> = Vec::new();
        // the last polyline of a connection is its output end
        for (dp, _, pts) in WIRES.iter().rev() {
            if shown.contains(dp) || BUS_TAPS.contains(dp) {
                continue;
            }
            let active = act.has(*dp);
            let value = if active {
                f.rec.and_then(|r| r.wire(*dp))
            } else {
                None
            }
            .map(|v| (v, true))
            .or_else(|| f.live.and_then(|l| l.value(*dp)).map(|v| (v, false)));
            let Some((v, hot)) = value else { continue };
            shown.push(*dp);
            let pts: Vec<Pos2> = pts.iter().map(|(x, y)| t(*x, *y)).collect();
            let text = wire_value_text(*dp, v);
            let font = mono(if hot { 8.0 } else { 7.0 });
            let galley = p.layout_no_wrap(text, font, if hot { bg } else { ink });
            let size = galley.size() + vec2(5.0, 1.0) * fs;
            // the wire's own end first (its first segments may be a trunk shared with other
            // wires), skipping segments too short for the chip, then the rest, avoiding chips
            // already there
            let segs: Vec<(Pos2, Pos2)> = pts.windows(2).map(|w| (w[0], w[1])).rev().collect();
            let long = |(a, b): &(Pos2, Pos2)| a.distance(*b) >= size.x.min(size.y * 3.0);
            let segs: Vec<(Pos2, Pos2)> = segs
                .iter()
                .copied()
                .filter(long)
                .chain(segs.iter().copied().filter(|s| !long(s)))
                .collect();
            let spots: Vec<Pos2> = segs
                .iter()
                .flat_map(|(a, b)| [0.5_f32, 0.25, 0.75].map(|k| a.lerp(*b, k)))
                .collect();
            let free = |c: Pos2| {
                let r = Rect::from_center_size(c, size);
                !placed.borrow().iter().any(|q| q.expand(1.0).intersects(r))
            };
            let Some(at) = spots
                .iter()
                .copied()
                .find(|c| free(*c))
                .or(spots.first().copied())
            else {
                continue;
            };
            let r = Rect::from_center_size(at, size);
            if hot {
                p.rect_filled(r, CornerRadius::same(3), accent);
            } else {
                p.rect(
                    r,
                    CornerRadius::same(3),
                    theme.code_bg_color,
                    Stroke::new(1.0_f32, idle_wire),
                    egui::StrokeKind::Inside,
                );
            }
            p.galley(
                r.center() - galley.size() / 2.0,
                galley,
                if hot { bg } else { ink },
            );
            placed.borrow_mut().push(r);
        }
    }

    if let Some(r) = f.rec {
        if let Some(v) = r.value {
            let text = format!("x{v:04X}");
            let a = r.activity;
            if a.has(Dp::Bus) {
                let at = [
                    (Dp::GatePc, 459.0, 119.0),
                    (Dp::GatePcDec, 524.0, 119.0),
                    (Dp::GateMarMux, 255.0, 119.0),
                    (Dp::GateAlu, 708.0, 775.0),
                    (Dp::GateMdr, 144.0, 775.0),
                    (Dp::GateSp, 819.0, 775.0),
                    (Dp::GateVector, 735.0, 775.0),
                    (Dp::GatePsr, 300.0, 775.0),
                    (Dp::BusToLogic, 614.0, 775.0),
                ]
                .iter()
                .find(|(dp, _, _)| a.has(*dp))
                .map(|(_, x, y)| t(*x, *y));
                if let Some(at) = at {
                    chip(
                        at + vec2(10.0 * fs, 0.0),
                        A::LEFT_CENTER,
                        text.clone(),
                        true,
                    );
                }
            } else if a.has(Dp::Adder) && !a.has(Dp::PcMux) {
                chip(t(410.0, 368.0), A::LEFT_CENTER, text.clone(), true);
            } else if a.has(Dp::Alu) {
                chip(t(750.0, 603.0), A::LEFT_TOP, text.clone(), true);
            }
            if a.has(Dp::Memory) {
                let what = if a.signal(Sig::RwWrite) {
                    "write"
                } else {
                    "read"
                };
                p.text(
                    t(420.0, 1040.0),
                    A::LEFT_CENTER,
                    format!("memory {what}: M[x{:04X}] = x{v:04X}", r.snap.mar),
                    mono(8.5),
                    accent,
                );
            }
        }
    }

    // ── what the pointer is over: the smallest part within a few pixels, else the nearest wire
    let pos = f.hover?;
    let hits = hits.into_inner();
    let pad = 4.0;
    let part = hits
        .iter()
        .filter(|(r, _)| r.expand(pad).contains(pos))
        .min_by(|a, b| a.0.area().total_cmp(&b.0.area()));
    let wire = WIRES
        .iter()
        .filter(|(dp, _, _)| !wire_text(*dp).is_empty())
        .map(|(dp, _, pts)| {
            let pts: Vec<Pos2> = pts.iter().map(|(x, y)| t(*x, *y)).collect();
            let d = pts
                .windows(2)
                .map(|w| segment_distance(pos, w[0], w[1]))
                .fold(f32::INFINITY, f32::min);
            (*dp, d)
        })
        .filter(|(_, d)| *d <= 6.0)
        .min_by(|a, b| a.1.total_cmp(&b.1));
    // a wire wins unless the pointer is right inside a part
    let over_part = part.filter(|(r, _)| r.contains(pos) || wire.is_none());
    let highlight = theme.strong_text_color;
    if let Some((r, text)) = over_part {
        p.rect_stroke(
            r.expand(2.0),
            CornerRadius::same(3),
            Stroke::new(1.5_f32, highlight),
            egui::StrokeKind::Outside,
        );
        return Some(text.clone());
    }
    let (dp, _) = wire?;
    // highlight every segment of this connection
    for (_, _, pts) in WIRES.iter().filter(|(d, _, _)| *d == dp) {
        let pts: Vec<Pos2> = pts.iter().map(|(x, y)| t(*x, *y)).collect();
        p.add(Shape::line(
            pts.clone(),
            Stroke::new(3.0 * fs, highlight.gamma_multiply(0.85)),
        ));
        arrow_head(&p, &pts, 8.0 * fs, highlight);
    }
    let status = match lit(dp) {
        Lit::Active => "Carries data in this state",
        Lit::Trail => "Used earlier in this instruction",
        Lit::Idle => "Not used by this instruction",
    };
    let mut text = format!("{}\n{status}", wire_text(dp));
    let carried = if lit(dp) == Lit::Active {
        f.rec.and_then(|r| r.wire(dp))
    } else {
        None
    };
    if let Some(v) = carried {
        text.push_str(&format!(": {}", wire_value_long(dp, v)));
    } else if let Some(v) = combinational(dp, snap, &f.extra) {
        text.push_str(&format!("\nOn the wire now: {}", wire_value_long(dp, v)));
    }
    Some(text)
}

/// Short connections between the bus and a gate or a register: the bus's own value chip shows
/// what they carry.
const BUS_TAPS: [Dp; 15] = [
    Dp::GatePc,
    Dp::GatePcDec,
    Dp::GateMarMux,
    Dp::GateAlu,
    Dp::GateMdr,
    Dp::GateSp,
    Dp::GatePsr,
    Dp::GateVector,
    Dp::BusToMar,
    Dp::BusToMdr,
    Dp::BusToIr,
    Dp::BusToReg,
    Dp::BusToPcMux,
    Dp::BusToLogic,
    Dp::BusToPsr,
];

pub(super) fn sext(v: u16, bits: u32) -> u16 {
    let shift = 16 - bits;
    (((v << shift) as i16) >> shift) as u16
}

/// What a wire carries whatever the control signals say: the parts feeding it always drive it
/// (the IR's fields, the registers SR1 and SR2 name, PC + 1...). None for wires whose value
/// depends on a mux or an operation.
pub(super) fn combinational(dp: Dp, s: &Snapshot, extra: &[u16; 5]) -> Option<u16> {
    let [saved_ssp, saved_usp, psr, table, vector] = *extra;
    let ir = s.ir;
    let reg = |n: u16| s.r[(n & 7) as usize];
    let sr1 = reg(ir >> 6);
    Some(match dp {
        Dp::PcInc => s.pc.wrapping_add(1),
        Dp::PcDec => s.pc,
        Dp::GatePcDec => s.pc.wrapping_sub(1),
        Dp::PcToAddr1 => s.pc,
        Dp::Zext | Dp::ZextToMarMux | Dp::IrToVector => ir & 0xFF,
        Dp::Sext11 => sext(ir & 0x7FF, 11),
        Dp::Sext9 => sext(ir & 0x1FF, 9),
        Dp::Sext6 => sext(ir & 0x3F, 6),
        Dp::Sext5 => sext(ir & 0x1F, 5),
        Dp::Sr2Out => reg(ir),
        Dp::Sr1Out | Dp::RegToAddr1 => sr1,
        Dp::Sr2Mux => {
            if ir & 0x20 != 0 {
                sext(ir & 0x1F, 5)
            } else {
                reg(ir)
            }
        }
        Dp::IrToControl => ir,
        Dp::ZeroToAddr2 => 0,
        Dp::MarToMem => s.mar,
        Dp::MdrToMem => s.mdr,
        Dp::Nzp => s.nzp as u16,
        Dp::UspIn | Dp::SspIn => s.r[6],
        Dp::SpIncPath => s.r[6].wrapping_add(1),
        Dp::SpDecPath => s.r[6].wrapping_sub(1),
        Dp::UspOut => saved_usp,
        Dp::SspOut => saved_ssp,
        Dp::GatePsr => psr,
        Dp::GateVector => (table & 0xFF) << 8 | (vector & 0xFF),
        Dp::TabX00 => 0,
        Dp::TabX01 => 1,
        Dp::VecIntv => crate::emulator::KEYBOARD_INTV,
        Dp::VecX00 => 0,
        Dp::VecX01 => 1,
        Dp::VecX02 => 2,
        Dp::IntPriorityIn => crate::emulator::KEYBOARD_PRIORITY,
        _ => return None,
    })
}

/// A wire's value, as wide as the wire.
pub(super) fn wire_value_text(dp: Dp, v: u16) -> String {
    match dp {
        Dp::IntSignal => format!("{}", v & 1),
        Dp::IntPriorityIn | Dp::PriorityMux => format!("PL{}", v & 7),
        Dp::Nzp | Dp::NzpMux | Dp::Logic => {
            let f = |b: u16, c: char| {
                if v & b != 0 {
                    c.to_ascii_uppercase()
                } else {
                    c
                }
            };
            format!("{}{}{}", f(4, 'n'), f(2, 'z'), f(1, 'p'))
        }
        Dp::TabX00
        | Dp::TabX01
        | Dp::TableMux
        | Dp::VecIntv
        | Dp::VecX00
        | Dp::VecX01
        | Dp::VecX02
        | Dp::VectorMux
        | Dp::VectorSel
        | Dp::IrToVector => {
            format!("x{:02X}", v & 0xFF)
        }
        Dp::PrivMux => format!("{}", v & 1),
        _ => format!("x{v:04X}"),
    }
}

/// A wire's value for its tooltip: also in decimal (and signed) for 16-bit wires.
pub(super) fn wire_value_long(dp: Dp, v: u16) -> String {
    let short = wire_value_text(dp, v);
    if short.len() == 5 && short.starts_with('x') {
        disasm::value_text(v)
    } else {
        short
    }
}

pub(super) fn segment_distance(p: Pos2, a: Pos2, b: Pos2) -> f32 {
    let ab = b - a;
    let len2 = ab.length_sq();
    let t = if len2 == 0.0 {
        0.0
    } else {
        ((p - a).dot(ab) / len2).clamp(0.0, 1.0)
    };
    (a + ab * t).distance(p)
}

/// What a connection carries, for its tooltip.
fn wire_text(dp: Dp) -> &'static str {
    match dp {
        Dp::GatePc => "PC → GatePC → bus",
        Dp::PcInc => "PC → +1 → PCMUX: the next instruction's address",
        Dp::PcMux => "PCMUX → PC",
        Dp::BusToPcMux => "Bus → PCMUX (e.g. PC <- MDR in TRAP and RTI)",
        Dp::PcToAddr1 => "PC → ADDR1MUX: PC as the address adder's base",
        Dp::PcDec => "PC → -1",
        Dp::GatePcDec => "PC - 1 → GatePC-1 → bus (the return address interrupts push)",
        Dp::AdderToMarMux => "Address adder → MARMUX",
        Dp::AdderToPcMux => "Address adder → PCMUX: a branch or jump target",
        Dp::Addr2Mux => "ADDR2MUX → address adder: the offset",
        Dp::Addr1Mux => "ADDR1MUX → address adder: the base",
        Dp::ZeroToAddr2 => "The constant 0 → ADDR2MUX (JMP, JSRR: no offset)",
        Dp::GateMarMux => "MARMUX → GateMARMUX → bus",
        Dp::Zext => "IR[7:0] → ZEXT",
        Dp::ZextToMarMux => "ZEXT(IR[7:0]) → MARMUX",
        Dp::Sext11 => "IR[10:0] → SEXT → ADDR2MUX: PCoffset11",
        Dp::Sext9 => "IR[8:0] → SEXT → ADDR2MUX: PCoffset9",
        Dp::Sext6 => "IR[5:0] → SEXT → ADDR2MUX: offset6",
        Dp::Sext5 => "IR[4:0] → SEXT → SR2MUX: imm5",
        Dp::BusToReg => "Bus → REG FILE: written to DR when LD.REG is asserted",
        Dp::Sr2Out => "SR2 OUT → SR2MUX",
        Dp::Sr1Out => "SR1 OUT → ALU input A",
        Dp::RegToAddr1 => "SR1 OUT → ADDR1MUX: a base register",
        Dp::Sr2Mux => "IR[5] picks SR2MUX's input; SR2MUX → ALU input B",
        Dp::GateAlu => "ALU → GateALU → bus",
        Dp::BusToIr => "Bus → IR",
        Dp::IrToControl => "IR → control logic: the opcode and BEN's condition bits",
        Dp::BusToLogic => "Bus → LOGIC: N, Z or P of the value being written",
        Dp::Logic => "LOGIC → PSRMUX → N Z P",
        Dp::NzpMux => "PSRMUX → N Z P",
        Dp::Nzp => "N Z P → control logic (BEN)",
        Dp::GatePsr => "PSR → GatePSR → bus (MDR <- PSR)",
        Dp::BusToPsr => "Bus → PSRMUX → PSR (PSR <- MDR in RTI)",
        Dp::PrivMux => "PSRMUX → Priv (PSR[15])",
        Dp::SetPriv => "Set.Priv: the constant for PSR[15] <- 0",
        Dp::PriorityMux => "PSRMUX → Priority (PSR[10:8])",
        Dp::Sr1ToSp => "SR1 OUT (R6, the stack pointer) → stack pointer logic",
        Dp::UspIn => "R6 → Saved.USP",
        Dp::SspIn => "R6 → Saved.SSP",
        Dp::SpIncPath => "R6 → +1 → SPMUX (pop)",
        Dp::SpDecPath => "R6 → -1 → SPMUX (push)",
        Dp::UspOut => "Saved.USP → SPMUX",
        Dp::SspOut => "Saved.SSP → SPMUX",
        Dp::GateSp => "SPMUX → GateSP → bus",
        Dp::GateMdr => "MDR → GateMDR → bus",
        Dp::BusToMdr => "Bus → MIO.EN mux → MDR",
        Dp::MemToMdr => "Memory → INMUX → MIO.EN mux → MDR",
        Dp::MdrToMem => "MDR → memory: the data written",
        Dp::BusToMar => "Bus → MAR",
        Dp::MarToMem => "MAR → memory and ADDR. CTL. LOGIC: the address",
        Dp::GateVector => "Table'Vector → GateVector → bus: a vector table entry's address",
        Dp::TableMux => "x00 or x01 → TableMUX → Table",
        Dp::VectorSel => "Mux → Vector",
        Dp::IrToVector => "IR[7:0] → Vector: the trap vector",
        Dp::VectorMux => "INTV, x00, x01 or x02 → VectorMUX: an interrupt or exception vector",
        Dp::TabX00 => "x00 → TableMUX: the trap vector table, x0000",
        Dp::TabX01 => "x01 → TableMUX: the interrupt vector table, x0100",
        Dp::VecIntv => "INTV → VectorMUX: the interrupting device's vector (keyboard x80)",
        Dp::VecX00 => "x00 → VectorMUX: privilege mode violation",
        Dp::VecX01 => "x01 → VectorMUX: illegal opcode",
        Dp::VecX02 => "x02 → VectorMUX: access control violation",
        Dp::IntPriorityIn => {
            "Interrupt priority (keyboard PL4): compared with PSR[10:8], and into PSRMUX"
        }
        Dp::IntSignal => "INT: a device with higher priority than the program wants to interrupt",
        _ => "",
    }
}

pub(super) fn alu_op(a: &Activity) -> &'static str {
    if a.signal(Sig::AlukAdd) {
        "ADD"
    } else if a.signal(Sig::AlukAnd) {
        "AND"
    } else if a.signal(Sig::AlukNot) {
        "NOT"
    } else if a.signal(Sig::AlukPassA) {
        "PASSA"
    } else {
        ""
    }
}

/// A font size no bigger than `size` that fits `text` in `width` pixels.
pub fn fit(size: f32, text: &str, width: f32) -> f32 {
    let chars = text.chars().count().max(1) as f32;
    size.min(width / (chars * 0.62)).max(5.0)
}

/// The part of a path between two distances along it, corners included.
pub(super) fn path_span(pts: &[Pos2], a: f32, b: f32) -> Vec<Pos2> {
    let mut out = Vec::new();
    let mut at = 0.0;
    for w in pts.windows(2) {
        let len = w[0].distance(w[1]);
        let (s, e) = (at, at + len);
        if e >= a && s <= b && len > 0.0 {
            let point = |d: f32| w[0] + (w[1] - w[0]) * ((d - s) / len);
            if out.is_empty() {
                out.push(point(a.max(s)));
            }
            out.push(point(b.min(e)));
        }
        at = e;
    }
    out
}

/// Dashes moving along a wire from its source to its destination, bending round its corners.
/// `travel` is how far the pattern has moved along the wire.
pub(super) fn flow_dashes(
    p: &egui::Painter,
    pts: &[Pos2],
    dash: f32,
    gap: f32,
    travel: f32,
    stroke: Stroke,
) {
    let lengths: Vec<f32> = pts.windows(2).map(|w| w[0].distance(w[1])).collect();
    let total: f32 = lengths.iter().sum();
    let period = dash + gap;
    if total <= 0.0 || period <= 0.0 {
        return;
    }
    let span = |a: f32, b: f32| path_span(pts, a, b);
    let mut start = travel.rem_euclid(period) - period;
    while start < total {
        let (a, b) = (start.max(0.0), (start + dash).min(total));
        if b > a {
            let points = span(a, b);
            if points.len() >= 2 {
                p.add(Shape::line(points, stroke));
            }
        }
        start += period;
    }
}

/// An arrow head at the end of a wire, pointing along its last segment.
pub(super) fn arrow_head(p: &egui::Painter, pts: &[Pos2], size: f32, color: Color32) {
    if let [.., a, b] = pts {
        let dir = (*b - *a).normalized();
        let side = vec2(-dir.y, dir.x);
        let base = *b - dir * size;
        p.add(Shape::convex_polygon(
            vec![*b, base + side * size * 0.45, base - side * size * 0.45],
            color,
            Stroke::NONE,
        ));
    }
}
