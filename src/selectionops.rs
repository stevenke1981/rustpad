//! Selection-aware duplication and line navigation; candidates validate before commit.
use crate::{
    core::{self, Content},
    textops::{self, Edit},
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Command {
    Duplicate,
    MoveUp,
    MoveDown,
    SelectLine,
}

impl Command {
    pub const ALL: [Self; 4] = [
        Self::Duplicate,
        Self::MoveUp,
        Self::MoveDown,
        Self::SelectLine,
    ];
    pub fn label(self) -> &'static str {
        match self {
            Self::Duplicate => "複製選取文字／目前行    Ctrl+D",
            Self::MoveUp => "上移選取行／目前行    Ctrl+Shift+↑",
            Self::MoveDown => "下移選取行／目前行    Ctrl+Shift+↓",
            Self::SelectLine => "選取目前行    Ctrl+Alt+L",
        }
    }
}

pub(super) fn apply(
    content: &Content,
    selection: (usize, usize),
    cursor: usize,
    command: Command,
) -> Result<Edit, String> {
    // Validate all scalar indices even for boundary no-ops.
    let range = textops::line_range(&content.text, selection, false)?;
    textops::byte_position(&content.text, cursor)?;
    let mut candidate = content.clone();
    let mut result = selection;
    match command {
        Command::SelectLine => {
            let line = textops::line_range(&content.text, (cursor, cursor), false)?;
            result = (
                content.text[..line.start].chars().count(),
                content.text[..line.end].chars().count(),
            );
        }
        Command::Duplicate if selection.0 != selection.1 => {
            let first = textops::byte_position(&content.text, selection.0)?;
            let last = textops::byte_position(&content.text, selection.1)?;
            candidate.text.insert_str(last, &content.text[first..last]);
            result = (selection.1, selection.1 + selection.1 - selection.0);
        }
        Command::Duplicate => {
            result = crate::actions::edit(
                &mut candidate,
                selection,
                cursor,
                crate::actions::Edit::DuplicateLine,
            )?;
        }
        Command::MoveUp | Command::MoveDown => {
            let terminal_newline = content.text.ends_with('\n');
            let body = content.text.strip_suffix('\n').unwrap_or(&content.text);
            let mut lines: Vec<&str> = body.split('\n').collect();
            let first = content.text[..range.start].matches('\n').count();
            let last = content.text[..range.end].matches('\n').count()
                - usize::from(range.end > range.start && content.text[..range.end].ends_with('\n'));
            // The virtual empty row at a terminal newline is not a movable data line.
            let boundary = first >= lines.len()
                || (command == Command::MoveUp && first == 0)
                || (command == Command::MoveDown && last + 1 >= lines.len());
            if !boundary {
                let new_first = if command == Command::MoveUp {
                    lines[first - 1..=last].rotate_left(1);
                    first - 1
                } else {
                    lines[first..=last + 1].rotate_right(1);
                    first + 1
                };
                let old_start = content.text[..range.start].chars().count();
                let new_start: usize = lines[..new_first]
                    .iter()
                    .map(|line| line.chars().count() + 1)
                    .sum();
                candidate.text = lines.join("\n");
                if terminal_newline {
                    candidate.text.push('\n');
                }
                let count = candidate.text.chars().count();
                result = (
                    (new_start + selection.0 - old_start).min(count),
                    (new_start + selection.1 - old_start).min(count),
                );
            }
        }
    }
    core::encode(&candidate)?;
    Ok(Edit {
        content: candidate,
        selection: result,
    })
}

impl crate::App {
    pub(super) fn selection_input(
        &mut self,
        ctx: &eframe::egui::Context,
        input: &mut eframe::egui::RawInput,
    ) {
        use eframe::egui::{Event, Key, Modifiers};
        if self.busy
            || self.pending_close.is_some()
            || self.exit
            || self.docs[self.active].composing
            || input
                .events
                .iter()
                .any(|event| matches!(event, Event::Ime(_)))
            || !ctx.memory(|memory| {
                memory.has_focus(eframe::egui::Id::new(("editor", self.docs[self.active].id)))
            })
        {
            return;
        }
        let mut commands = Vec::new();
        // Consume before egui computes directional focus navigation for arrow keys.
        input.events.retain(|event| {
            if let Event::Key {
                key,
                modifiers,
                pressed: true,
                ..
            } = event
            {
                let command = match (*modifiers, *key) {
                    (m, Key::D) if m == Modifiers::CTRL => Some(Command::Duplicate),
                    (m, Key::ArrowUp) if m == (Modifiers::CTRL | Modifiers::SHIFT) => {
                        Some(Command::MoveUp)
                    }
                    (m, Key::ArrowDown) if m == (Modifiers::CTRL | Modifiers::SHIFT) => {
                        Some(Command::MoveDown)
                    }
                    (m, Key::L) if m == (Modifiers::CTRL | Modifiers::ALT) => {
                        Some(Command::SelectLine)
                    }
                    _ => None,
                };
                if let Some(command) = command {
                    commands.push(command);
                    return false;
                }
            }
            true
        });
        for command in commands {
            self.selection_command(command);
        }
    }
    pub(super) fn selection_command(&mut self, command: Command) {
        if self.busy
            || self.pending_close.is_some()
            || self.exit
            || self.docs[self.active].composing
        {
            return;
        }
        let d = &self.docs[self.active];
        match apply(&d.content, d.selected, d.cursor, command) {
            Ok(edit) => {
                let d = &mut self.docs[self.active];
                let changed = edit.content != d.content;
                if changed {
                    d.history
                        .record(std::mem::replace(&mut d.content, edit.content));
                    if matches!(command, Command::MoveUp | Command::MoveDown) {
                        d.bookmarks.clear();
                    } else {
                        d.bookmarks.sync(&d.content.text);
                    }
                    d.folds.sync(&d.content.text);
                }
                if changed || command == Command::SelectLine {
                    d.folds.hidden.clear();
                }
                let start_caret = d.cursor == d.selected.0
                    && matches!(command, Command::MoveUp | Command::MoveDown);
                d.selected = edit.selection;
                d.cursor = if start_caret {
                    edit.selection.0
                } else {
                    edit.selection.1
                };
                self.selection = Some(edit.selection);
                self.match_range = None;
                self.match_signature = None;
                self.message = if changed {
                    "文字處理完成；Ctrl+Z 可復原"
                } else if command == Command::SelectLine {
                    "已選取目前行"
                } else {
                    "邊界或內容相同；未修改"
                }
                .into();
            }
            Err(error) => self.message = format!("文字處理失敗：{error}"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::Newline;
    #[test]
    fn moving_unicode_block_keeps_bytes_format_and_next_line_boundary() {
        for newline in [Newline::Lf, Newline::Crlf, Newline::Cr] {
            let content = Content {
                text: "外\n甲🙂\n乙\n尾".into(),
                bom: true,
                newline,
            };
            let moved = apply(&content, (2, 7), 7, Command::MoveUp).unwrap();
            assert_eq!(moved.content.text, "甲🙂\n乙\n外\n尾");
            assert_eq!(moved.selection, (0, 5));
            let restored = apply(&moved.content, moved.selection, 5, Command::MoveDown).unwrap();
            assert_eq!(
                core::encode(&restored.content).unwrap(),
                core::encode(&content).unwrap()
            );
            assert_eq!(restored.selection, (2, 7));
        }
    }
    #[test]
    fn move_preserves_terminal_newline_and_caret_column_at_unterminated_eof() {
        for text in ["外\n甲🙂\n乙", "外\n甲🙂\n乙\n"] {
            let content = Content {
                text: text.into(),
                ..Default::default()
            };
            let edit = apply(&content, (4, 4), 4, Command::MoveDown).unwrap();
            assert_eq!(
                edit.content.text,
                if text.ends_with('\n') {
                    "外\n乙\n甲🙂\n"
                } else {
                    "外\n乙\n甲🙂"
                }
            );
            assert_eq!(edit.selection, (6, 6));
            let restored = apply(&edit.content, edit.selection, 6, Command::MoveUp).unwrap();
            assert_eq!(restored.content, content);
            assert_eq!(restored.selection, (4, 4));
        }
    }
    #[test]
    fn boundary_moves_and_select_empty_eof_do_not_create_history_or_clear_bookmarks() {
        let mut app = crate::app_tests::app();
        app.docs[0].content.text = "甲\n尾\n".into();
        app.docs[0].saved = app.docs[0].content.clone();
        app.docs[0].bookmarks.toggle("甲\n尾\n", 0);
        for (selection, command) in [
            ((0, 0), Command::MoveUp),
            ((2, 2), Command::MoveDown),
            ((4, 4), Command::MoveUp),
            ((4, 4), Command::SelectLine),
        ] {
            app.docs[0].selected = selection;
            app.docs[0].cursor = selection.1;
            app.selection_command(command);
            assert_eq!(app.docs[0].content.text, "甲\n尾\n");
            assert_eq!(app.docs[0].selected, selection);
            assert!(!app.docs[0].history.can_undo());
            assert_eq!(app.docs[0].bookmarks.lines.len(), 1);
        }
        let empty = Content::default();
        for command in [Command::MoveUp, Command::MoveDown, Command::SelectLine] {
            assert_eq!(apply(&empty, (0, 0), 0, command).unwrap().content, empty);
        }
    }
    #[test]
    fn duplicate_selection_keeps_combining_emoji_multiline_and_whole_line_fallback() {
        let content = Content {
            text: "外e\u{301}🙂\n甲尾".into(),
            bom: true,
            newline: Newline::Crlf,
        };
        let edit = apply(&content, (1, 6), 6, Command::Duplicate).unwrap();
        assert_eq!(edit.content.text, "外e\u{301}🙂\n甲e\u{301}🙂\n甲尾");
        assert_eq!(edit.selection, (6, 11));
        assert!(edit.content.bom);
        assert_eq!(edit.content.newline, Newline::Crlf);
        let content = Content {
            text: "甲🙂".into(),
            ..Default::default()
        };
        let edit = apply(&content, (1, 1), 1, Command::Duplicate).unwrap();
        assert_eq!(edit.content.text, "甲🙂\n甲🙂");
        assert_eq!(edit.selection, (3, 5));
    }
    #[test]
    fn rejected_ranges_and_oversize_duplication_do_not_mutate_document_or_history() {
        let content = Content {
            text: "甲🙂\n尾".into(),
            ..Default::default()
        };
        for (selection, cursor) in [((3, 1), 1), ((0, 99), 0), ((0, 0), 99)] {
            for command in Command::ALL {
                assert!(apply(&content, selection, cursor, command).is_err());
            }
        }
        let mut app = crate::app_tests::app();
        app.docs[0].content.text = ("a".repeat(2000) + "\n").repeat(600);
        core::encode(&app.docs[0].content).unwrap();
        let before = app.docs[0].content.clone();
        let end = before.text.chars().count();
        app.docs[0].selected = (0, end);
        app.docs[0].cursor = end;
        app.selection_command(Command::Duplicate);
        assert_eq!(app.docs[0].content, before);
        assert_eq!(app.docs[0].selected, (0, end));
        assert!(!app.docs[0].history.can_undo());
        assert!(app.message.contains("2 MiB"));
    }
    #[test]
    fn blocked_commands_and_other_tabs_are_safe_and_backward_caret_follows_move() {
        let mut app = crate::app_tests::app();
        app.docs[0].content.text = "外\n甲🙂\n尾".into();
        app.docs[0].selected = (2, 5);
        app.docs[0].cursor = 2;
        let before = app.docs[0].content.clone();
        for blocked in 0..4 {
            app.busy = blocked == 0;
            app.docs[0].composing = blocked == 1;
            app.exit = blocked == 2;
            app.pending_close = (blocked == 3).then_some(app.docs[0].id);
            for command in Command::ALL {
                app.selection_command(command);
            }
            assert_eq!(app.docs[0].content, before);
            assert_eq!(app.docs[0].selected, (2, 5));
            assert!(!app.docs[0].history.can_undo());
        }
        app.pending_close = None;
        app.new_doc();
        let other = app.docs[1].content.clone();
        app.activate(0);
        app.selection_command(Command::MoveUp);
        assert_eq!(app.docs[0].selected, (0, 3));
        assert_eq!(app.docs[0].cursor, 0);
        assert_eq!(app.docs[1].content, other);
        let d = &mut app.docs[0];
        d.history.undo(&mut d.content);
        assert_eq!(d.content, before);
        assert!(!d.history.can_undo());
        d.history.redo(&mut d.content);
        assert_eq!(d.content.text, "甲🙂\n外\n尾");
    }
}
