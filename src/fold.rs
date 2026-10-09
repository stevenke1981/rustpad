use eframe::egui;
use std::ops::Range;
pub fn should_expand(events: &[egui::Event]) -> bool {
    events.iter().any(|event| match event {
        egui::Event::Key {
            key,
            modifiers,
            pressed: true,
            ..
        } => {
            !((*key == egui::Key::F || *key == egui::Key::U)
                && *modifiers == (egui::Modifiers::CTRL | egui::Modifiers::ALT))
        }
        egui::Event::Text(_) | egui::Event::Paste(_) | egui::Event::Cut | egui::Event::Ime(_) => {
            true
        }
        _ => false,
    })
}
pub fn block(
    text: &str,
    language: crate::syntax::Language,
    cursor: usize,
) -> Result<Option<Range<usize>>, String> {
    let offsets: Vec<_> = text
        .char_indices()
        .map(|p| p.0)
        .chain(std::iter::once(text.len()))
        .collect();
    let mut ranges = Vec::new();
    for (open, close) in crate::structure::pairs(text, language)? {
        if cursor < open || cursor > close + 1 {
            continue;
        }
        let start = text[offsets[open]..]
            .find('\n')
            .map(|p| offsets[open] + p + 1);
        let end = text[..offsets[close]].rfind('\n').map(|p| p + 1);
        if let (Some(start), Some(end)) = (start, end)
            && start < end
        {
            ranges.push(start..end);
        }
    }
    Ok(ranges.into_iter().min_by_key(|r| r.end - r.start))
}

#[derive(Default)]
pub struct State {
    pub hidden: Vec<Range<usize>>,
    source: String,
}
impl State {
    pub fn sync(&mut self, text: &str) {
        if self.source != text {
            self.hidden.clear();
            self.source = text.into();
        }
    }
    pub fn toggle(&mut self, text: &str, range: Range<usize>) {
        self.sync(text);
        if let Some(index) = self.hidden.iter().position(|r| *r == range) {
            self.hidden.remove(index);
        } else {
            self.hidden.push(range);
            self.hidden.sort_by_key(|r| r.start);
        }
    }
}
pub fn apply(job: &mut egui::text::LayoutJob, hidden: &[Range<usize>]) {
    if hidden.is_empty() {
        return;
    }
    let mut result = Vec::new();
    for section in &job.sections {
        let mut cuts = vec![section.byte_range.start, section.byte_range.end];
        for range in hidden {
            for point in [range.start, range.end] {
                if section.byte_range.contains(&point) && job.text.is_char_boundary(point) {
                    cuts.push(point);
                }
            }
        }
        cuts.sort_unstable();
        cuts.dedup();
        for pair in cuts.windows(2) {
            let mut part = section.clone();
            part.byte_range = pair[0]..pair[1];
            if pair[0] != section.byte_range.start {
                part.leading_space = 0.;
            }
            if hidden.iter().any(|r| r.contains(&pair[0])) {
                part.format.font_id.size = 0.01;
                part.format.line_height = Some(0.);
                part.format.color = egui::Color32::TRANSPARENT;
                part.format.background = egui::Color32::TRANSPARENT;
                part.format.extra_letter_spacing = 0.;
                part.leading_space = 0.;
            }
            result.push(part);
        }
    }
    job.sections = result;
}
pub fn gutter(text: &str, hidden: &[Range<usize>]) -> Vec<usize> {
    let mut byte = 0;
    let mut rows = Vec::new();
    for (line, part) in text.split('\n').enumerate() {
        if !hidden.iter().any(|r| r.contains(&byte)) {
            rows.push(line + 1);
        }
        byte += part.len() + 1;
    }
    rows
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn folded_widget_copy_contains_hidden_unicode_and_edit_expands_first() {
        let text = "中{\n\t中文🙂\n}\n尾";
        let hidden = std::iter::once(text.find('\n').unwrap() + 1..text.find('}').unwrap())
            .collect::<Vec<_>>();
        let ctx = egui::Context::default();
        let id = egui::Id::new("fold-copy");
        let mut content = text.to_owned();
        let render = |ctx: &egui::Context, content: &mut String, hidden: &[Range<usize>]| {
            egui::CentralPanel::default().show(ctx, |ui| {
                let mut layouter = |ui: &egui::Ui, text: &dyn egui::TextBuffer, _: f32| {
                    let mut job = egui::text::LayoutJob::simple(
                        text.as_str().into(),
                        egui::FontId::monospace(15.),
                        egui::Color32::WHITE,
                        f32::INFINITY,
                    );
                    apply(&mut job, hidden);
                    ui.fonts_mut(|fonts| fonts.layout_job(job))
                };
                egui::TextEdit::multiline(content)
                    .id(id)
                    .layouter(&mut layouter)
                    .show(ui);
            });
        };
        let _ = ctx.run(Default::default(), |ctx| render(ctx, &mut content, &hidden));
        let mut state = egui::TextEdit::load_state(&ctx, id).unwrap();
        state
            .cursor
            .set_char_range(Some(egui::text::CCursorRange::two(
                egui::text::CCursor::new(0),
                egui::text::CCursor::new(text.chars().count()),
            )));
        state.store(&ctx, id);
        ctx.memory_mut(|m| m.request_focus(id));
        let output = ctx.run(
            egui::RawInput {
                events: vec![egui::Event::Copy],
                ..Default::default()
            },
            |ctx| render(ctx, &mut content, &hidden),
        );
        assert!(
            output
                .platform_output
                .commands
                .iter()
                .any(|c| matches!(c,egui::OutputCommand::CopyText(copied) if copied==text))
        );
        assert_eq!(content, text);
        assert!(should_expand(&[egui::Event::Paste("新🙂".into())]));
        assert!(!should_expand(&[egui::Event::Copy]));
        let output = ctx.run(
            egui::RawInput {
                events: vec![egui::Event::Paste("新🙂".into())],
                ..Default::default()
            },
            |ctx| render(ctx, &mut content, &[]),
        );
        assert!(output.platform_output.commands.is_empty());
        assert_eq!(content, "新🙂");
    }
    #[test]
    fn block_scopes_nested_unicode_eof_and_changed_content_are_safe() {
        let text = "中{\n // { }\n if true {\n  \"}\";\n }\n}";
        let inner = text.find("if").unwrap();
        let cursor = text[..inner].chars().count() + 8;
        let range = block(text, crate::syntax::Language::Rust, cursor)
            .unwrap()
            .unwrap();
        assert_eq!(&text[range.clone()], "  \"}\";\n");
        let mut state = State::default();
        state.toggle(text, range.clone());
        assert_eq!(state.hidden.as_slice(), std::slice::from_ref(&range));
        state.toggle(text, range);
        assert!(state.hidden.is_empty());
        state.toggle(text, 3..text.rfind('}').unwrap());
        state.sync(&(text.to_owned() + "新"));
        assert!(state.hidden.is_empty());
        assert!(
            block("{\n}", crate::syntax::Language::Rust, 0)
                .unwrap()
                .is_none()
        );
        assert!(
            block("{\n x\n", crate::syntax::Language::Rust, 0)
                .unwrap()
                .is_none()
        );
    }
    #[test]
    fn folding_keeps_real_text_and_scalar_mapping_but_removes_row_height() {
        let text = "中{\n    中文🙂\n    尾\n}\n";
        let start = text.find('\n').unwrap() + 1;
        let end = text.rfind("}\n").unwrap();
        let ctx = egui::Context::default();
        let _ = ctx.run(Default::default(), |ctx| {
            let mut job = egui::text::LayoutJob::simple(
                text.into(),
                egui::FontId::monospace(15.),
                egui::Color32::WHITE,
                f32::INFINITY,
            );
            let expanded = ctx.fonts_mut(|fonts| fonts.layout_job(job.clone()));
            apply(&mut job, std::slice::from_ref(&(start..end)));
            assert_eq!(job.text, text);
            let collapsed = ctx.fonts_mut(|fonts| fonts.layout_job(job));
            assert!(collapsed.size().y < expanded.size().y - 20.);
            assert_eq!(collapsed.rows[1].row.size.y, 0.);
            assert_eq!(collapsed.rows[2].row.size.y, 0.);
            let source_cursor = text[..end].chars().count();
            assert_eq!(
                collapsed
                    .pos_from_cursor(egui::text::CCursor::new(source_cursor))
                    .min
                    .y,
                collapsed.rows[3].pos.y
            );
            assert_eq!(
                gutter(text, std::slice::from_ref(&(start..end))),
                vec![1, 4, 5]
            );
        });
    }
}
