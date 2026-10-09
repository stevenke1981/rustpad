use crate::core::{Content, Newline};
use eframe::egui::{self, Stroke};

#[derive(Debug, Default, PartialEq, Eq)]
pub struct Stats {
    pub bytes: usize,
    pub chars: usize,
    pub lines: usize,
    pub line: usize,
    pub column: usize,
    pub selected_chars: usize,
    pub selected_lines: usize,
}
impl Stats {
    pub fn new(content: &Content, cursor: usize, selected: (usize, usize)) -> Self {
        let (start, end) = (selected.0.min(selected.1), selected.0.max(selected.1));
        let mut stats = Self {
            bytes: content.text.len() + usize::from(content.bom) * 3,
            lines: 1,
            line: 1,
            column: 1,
            ..Default::default()
        };
        let mut selected_newlines = 0;
        let mut selected_ends_in_newline = false;
        for (index, ch) in content.text.chars().enumerate() {
            stats.chars += 1;
            if ch == '\n' {
                stats.lines += 1;
                if content.newline == Newline::Crlf {
                    stats.bytes += 1;
                }
            }
            if index < cursor {
                if ch == '\n' {
                    stats.line += 1;
                    stats.column = 1;
                } else {
                    stats.column += 1;
                }
            }
            if (start..end).contains(&index) {
                stats.selected_chars += 1;
                selected_ends_in_newline = ch == '\n';
                selected_newlines += usize::from(selected_ends_in_newline);
            }
        }
        if stats.selected_chars > 0 {
            stats.selected_lines = selected_newlines + usize::from(!selected_ends_in_newline);
        }
        stats
    }
}

#[derive(Clone, Copy)]
pub enum Icon {
    New,
    Open,
    Save,
    SaveAs,
    Close,
    Undo,
    Redo,
    Find,
    Replace,
    GoLine,
}

/// Original geometry, drawn at UI scale. No external icon pack or font glyphs.
pub fn tool(
    ui: &mut egui::Ui,
    icon: Icon,
    label: &str,
    enabled: bool,
    selected: bool,
) -> egui::Response {
    let response = ui
        .add_enabled_ui(enabled, |ui| {
            let (rect, response) =
                ui.allocate_exact_size(egui::vec2(24., 24.), egui::Sense::click());
            if response.hovered() || selected {
                let fill = if selected {
                    ui.visuals().selection.bg_fill
                } else {
                    ui.visuals().widgets.hovered.bg_fill
                };
                ui.painter().rect_filled(rect, 2., fill);
            }
            let color = ui.style().interact(&response).fg_stroke.color;
            let stroke = Stroke::new(1.5_f32, color);
            let origin = rect.center() - egui::vec2(8., 8.);
            let p = |x: f32, y: f32| origin + egui::vec2(x, y);
            let line = |a: (f32, f32), b: (f32, f32)| {
                ui.painter()
                    .line_segment([p(a.0, a.1), p(b.0, b.1)], stroke);
            };
            let path = |points: &[(f32, f32)]| {
                ui.painter().add(egui::Shape::line(
                    points.iter().map(|&(x, y)| p(x, y)).collect(),
                    stroke,
                ));
            };
            match icon {
                Icon::New => {
                    path(&[
                        (3., 1.),
                        (9., 1.),
                        (13., 5.),
                        (13., 15.),
                        (3., 15.),
                        (3., 1.),
                    ]);
                    line((8., 7.), (8., 12.));
                    line((5.5, 9.5), (10.5, 9.5));
                }
                Icon::Open => {
                    path(&[(1., 5.), (1., 3.), (6., 3.), (8., 5.), (14., 5.), (14., 7.)]);
                    path(&[(1., 6.), (3., 13.), (13., 13.), (15., 7.), (3., 7.)]);
                }
                Icon::Save | Icon::SaveAs => {
                    path(&[
                        (2., 2.),
                        (12., 2.),
                        (14., 4.),
                        (14., 14.),
                        (2., 14.),
                        (2., 2.),
                    ]);
                    path(&[(5., 2.), (5., 6.), (11., 6.), (11., 2.)]);
                    path(&[(5., 14.), (5., 9.), (11., 9.), (11., 14.)]);
                    if matches!(icon, Icon::SaveAs) {
                        line((9., 15.), (15., 9.));
                    }
                }
                Icon::Close => {
                    line((4., 4.), (12., 12.));
                    line((4., 12.), (12., 4.));
                }
                Icon::Undo | Icon::Redo => {
                    let flip = |x| {
                        if matches!(icon, Icon::Redo) {
                            16. - x
                        } else {
                            x
                        }
                    };
                    path(&[(flip(5.), 2.), (flip(2.), 5.), (flip(5.), 8.)]);
                    path(&[
                        (flip(2.), 5.),
                        (flip(9.), 5.),
                        (flip(13.), 8.),
                        (flip(13.), 13.),
                    ]);
                }
                Icon::Find => {
                    ui.painter().circle_stroke(p(6.5, 6.5), 4.5, stroke);
                    line((10., 10.), (14.5, 14.5));
                }
                Icon::Replace => {
                    line((2., 4.), (13., 4.));
                    path(&[(10., 1.), (13., 4.), (10., 7.)]);
                    line((14., 12.), (3., 12.));
                    path(&[(6., 9.), (3., 12.), (6., 15.)]);
                }
                Icon::GoLine => {
                    line((12., 2.), (12., 14.));
                    line((2., 8.), (9., 8.));
                    path(&[(6., 5.), (9., 8.), (6., 11.)]);
                }
            }
            response.widget_info(|| {
                egui::WidgetInfo::labeled(egui::WidgetType::Button, enabled, label)
            });
            response
        })
        .inner;
    response.on_hover_text(label).on_disabled_hover_text(label)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::Newline;
    #[test]
    fn status_counts_real_encoded_bytes_unicode_and_selection_line_boundary() {
        let content = Content {
            text: "甲🙂\n乙".into(),
            bom: true,
            newline: Newline::Crlf,
        };
        assert_eq!(
            Stats::new(&content, 4, (3, 0)),
            Stats {
                bytes: 15,
                chars: 4,
                lines: 2,
                line: 2,
                column: 2,
                selected_chars: 3,
                selected_lines: 1,
            }
        );
        assert_eq!(Stats::new(&content, 2, (1, 4)).selected_lines, 2);
        let content = Content {
            text: "e\u{301}🙂\n".into(),
            bom: false,
            newline: Newline::Lf,
        };
        let stats = Stats::new(&content, usize::MAX, (2, usize::MAX));
        assert_eq!(
            (
                stats.bytes,
                stats.chars,
                stats.lines,
                stats.line,
                stats.column
            ),
            (8, 4, 2, 2, 1)
        );
        assert_eq!((stats.selected_chars, stats.selected_lines), (2, 1));
        let stats = Stats::new(&Content::default(), 50, (2, 80));
        assert_eq!(
            (
                stats.line,
                stats.column,
                stats.lines,
                stats.selected_chars,
                stats.selected_lines
            ),
            (1, 1, 1, 0, 0)
        );
    }
}
