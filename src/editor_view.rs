//! Display-only editor preferences and layout. Text and stored line endings stay intact.
use crate::{App, core, fold, syntax};
use eframe::egui::{
    self, Color32, FontId, Key,
    text::{CCursor, CCursorRange},
};

const DEFAULT_FONT_SIZE: f32 = 15.;

pub(super) struct Options {
    pub wrap: bool,
    pub font_size: f32,
    editor_rect: Option<egui::Rect>,
}
impl Default for Options {
    fn default() -> Self {
        Self {
            wrap: false,
            font_size: DEFAULT_FONT_SIZE,
            editor_rect: None,
        }
    }
}
impl Options {
    pub fn zoom(&mut self, direction: i8) {
        self.font_size = (self.font_size + f32::from(direction)).clamp(9., 32.);
    }
    pub fn reset_zoom(&mut self) {
        self.font_size = DEFAULT_FONT_SIZE;
    }
}

impl App {
    pub(super) fn editor_view_input(&mut self, ctx: &egui::Context, input: &mut egui::RawInput) {
        // Consume editor zoom before egui's global UI zoom plugin sees the events.
        ctx.options_mut(|options| options.zoom_with_keyboard = false);
        let d = &self.docs[self.active];
        if d.composing
            || self.pending_close.is_some()
            || self.exit
            || input
                .events
                .iter()
                .any(|event| matches!(event, egui::Event::Ime(_)))
        {
            return;
        }
        let focused = ctx.memory(|memory| memory.has_focus(egui::Id::new(("editor", d.id))));
        let mut pointer = ctx.input(|input| input.pointer.hover_pos());
        input.events.retain(|event| {
            if let egui::Event::PointerMoved(position) = event {
                pointer = Some(*position);
            }
            if matches!(event, egui::Event::PointerGone) {
                pointer = None;
            }
            match event {
                egui::Event::Key {
                    key,
                    modifiers,
                    pressed,
                    ..
                } if focused && modifiers.ctrl && !modifiers.alt && !modifiers.mac_cmd => {
                    let direction = match key {
                        Key::Plus | Key::Equals => Some(1),
                        Key::Minus => Some(-1),
                        Key::Num0 => Some(0),
                        _ => None,
                    };
                    if let Some(direction) = direction {
                        if *pressed {
                            if direction == 0 {
                                self.view.reset_zoom();
                            } else {
                                self.view.zoom(direction);
                            }
                        }
                        return false;
                    }
                    true
                }
                egui::Event::MouseWheel {
                    delta, modifiers, ..
                } if modifiers.ctrl
                    && !modifiers.alt
                    && !modifiers.mac_cmd
                    && delta.y != 0.
                    && pointer.is_some_and(|position| {
                        self.view
                            .editor_rect
                            .is_some_and(|rect| rect.contains(position))
                    }) =>
                {
                    self.view.zoom(if delta.y > 0. { 1 } else { -1 });
                    false
                }
                _ => true,
            }
        });
    }

    pub(super) fn show_editor(&mut self, ctx: &egui::Context) {
        egui::CentralPanel::default()
            .frame(
                egui::Frame::central_panel(&ctx.style())
                    .fill(ctx.style().visuals.extreme_bg_color)
                    .inner_margin(4.),
            )
            .show(ctx, |ui| {
                self.view.editor_rect = Some(ui.max_rect());
                let font = FontId::monospace(self.view.font_size);
                let wrap = self.view.wrap;
                let d = &mut self.docs[self.active];
                d.bookmarks.sync(&d.content.text);
                d.folds.sync(&d.content.text);
                let before =
                    (!ctx.input(|input| input.events.is_empty())).then(|| d.content.clone());
                let id = egui::Id::new(("editor", d.id));
                let pending_scroll = self.selection.is_some();
                if let Some((start, end)) = self.selection.take() {
                    let mut state = egui::TextEdit::load_state(ctx, id).unwrap_or_default();
                    state.cursor.set_char_range(Some(CCursorRange::two(
                        CCursor::new(if d.cursor == start { end } else { start }),
                        CCursor::new(if d.cursor == start { start } else { end }),
                    )));
                    state.store(ctx, id);
                    ctx.memory_mut(|memory| memory.request_focus(id));
                }
                egui::ScrollArea::new([!wrap, true])
                    .animated(false)
                    .id_salt(("scroll", d.id))
                    .show(ui, |ui| {
                        ui.horizontal_top(|ui| {
                            let lines = d.content.text.matches('\n').count() + 1;
                            let gutter_width = ui.fonts_mut(|fonts| fonts.glyph_width(&font, '0'))
                                * (lines.to_string().len().max(4) + 2) as f32
                                + 4.;
                            let (gutter, _) = ui.allocate_exact_size(
                                egui::vec2(gutter_width, 0.),
                                egui::Sense::hover(),
                            );
                            let dark = self.dark;
                            let tab_width = self.tab_width;
                            let cache = &mut d.highlight_cache;
                            let path = d.path.clone();
                            let manual = d.language;
                            let doc_id = d.id;
                            let service = &mut self.syntax;
                            let hidden = d.folds.hidden.clone();
                            let layout_font = font.clone();
                            let mut layouter =
                                move |ui: &egui::Ui, text: &dyn egui::TextBuffer, width: f32| {
                                    let language =
                                        syntax::resolve(path.as_deref(), text.as_str(), manual);
                                    let (height, space) = ui.fonts_mut(|fonts| {
                                        (
                                            fonts.row_height(&layout_font),
                                            fonts.glyph_width(&layout_font, ' '),
                                        )
                                    });
                                    let mut job = service.layout(
                                        doc_id,
                                        text.as_str(),
                                        language,
                                        dark,
                                        tab_width,
                                        height,
                                        ui.ctx().pixels_per_point(),
                                        space,
                                        cache,
                                    );
                                    for section in &mut job.sections {
                                        section.format.font_id.size *=
                                            layout_font.size / DEFAULT_FONT_SIZE;
                                    }
                                    job.wrap.max_width =
                                        if wrap { width.max(1.) } else { f32::INFINITY };
                                    fold::apply(&mut job, &hidden);
                                    ui.fonts_mut(|fonts| fonts.layout_job(job))
                                };
                            let width = (ui.available_width()
                                - if self.show_eol { 30. } else { 0. })
                            .max(1.);
                            let output = egui::TextEdit::multiline(&mut d.content.text)
                                .id(id)
                                .code_editor()
                                .font(font.clone())
                                .frame(false)
                                .margin(egui::Vec2::ZERO)
                                .desired_width(if wrap { width } else { width.max(600.) })
                                .desired_rows(30)
                                .layouter(&mut layouter)
                                .show(ui);
                            if let Some(range) = output.cursor_range {
                                d.cursor = range.primary.index;
                                d.selected = (
                                    range.primary.index.min(range.secondary.index),
                                    range.primary.index.max(range.secondary.index),
                                );
                            }
                            if pending_scroll {
                                output.response.request_focus();
                                let rect = output
                                    .galley
                                    .pos_from_cursor(CCursor::new(d.cursor))
                                    .translate(output.galley_pos.to_vec2());
                                ui.scroll_to_rect(rect, Some(egui::Align::Center));
                            }
                            paint_gutter(
                                ui,
                                &output,
                                gutter,
                                &font,
                                &d.bookmarks.lines,
                                &d.content.text,
                                &d.folds.hidden,
                            );
                            crate::paint_whitespace(
                                ui,
                                &output,
                                self.show_spaces,
                                self.show_eol,
                                d.content.newline,
                            );
                        });
                    });
                if let Some(before) = before
                    && before != d.content
                {
                    if let Err(error) = core::validate_text(&d.content.text) {
                        d.content = before;
                        self.message = error;
                    } else {
                        d.history.record(before);
                    }
                    self.match_range = None;
                }
                d.bookmarks.sync(&d.content.text);
            });
    }
}

/// Label the first visual row only; soft continuations don't create source lines.
fn paint_gutter(
    ui: &egui::Ui,
    output: &egui::text_edit::TextEditOutput,
    gutter: egui::Rect,
    font: &FontId,
    bookmarks: &std::collections::BTreeSet<usize>,
    text: &str,
    hidden: &[std::ops::Range<usize>],
) {
    let color = if ui.visuals().dark_mode {
        Color32::from_rgb(173, 182, 194)
    } else {
        Color32::from_rgb(94, 102, 115)
    };
    let folded: std::collections::BTreeSet<_> = hidden
        .iter()
        .map(|range| text[..range.start].matches('\n').count())
        .collect();
    let mut line = 1;
    let mut first = true;
    for row in &output.galley.rows {
        if first && row.row.size.y > 0. {
            let y = output.galley_pos.y + row.pos.y + row.row.size.y * 0.5;
            if y >= ui.clip_rect().top() - font.size && y <= ui.clip_rect().bottom() + font.size {
                let marker = if folded.contains(&line) {
                    ">"
                } else if bookmarks.contains(&(line - 1)) {
                    "*"
                } else {
                    " "
                };
                ui.painter().text(
                    egui::pos2(gutter.right() - 7., y),
                    egui::Align2::RIGHT_CENTER,
                    format!("{marker}{line:>4}"),
                    font.clone(),
                    color,
                );
            }
        }
        first = row.row.ends_with_newline;
        if first {
            line += 1;
        }
    }
    let x = gutter.right() - 2.;
    ui.painter().line_segment(
        [
            egui::pos2(x, output.response.rect.top()),
            egui::pos2(x, output.response.rect.bottom()),
        ],
        ui.visuals().widgets.noninteractive.bg_stroke,
    );
}
