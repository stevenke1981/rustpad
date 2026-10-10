//! Validated, atomic line commands. All ranges use Unicode scalar positions.
use crate::{
    core::{self, Content},
    syntax::Language,
};
use std::{collections::HashSet, ops::Range};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum LineCommand {
    SortAscending,
    SortDescending,
    Reverse,
    Unique,
    UniqueConsecutive,
}
impl LineCommand {
    pub const ALL: [Self; 5] = [
        Self::SortAscending,
        Self::SortDescending,
        Self::Reverse,
        Self::Unique,
        Self::UniqueConsecutive,
    ];
    pub fn label(self) -> &'static str {
        match self {
            Self::SortAscending => "依字元排序（升冪）",
            Self::SortDescending => "依字元排序（降冪）",
            Self::Reverse => "反轉行順序",
            Self::Unique => "移除重複行",
            Self::UniqueConsecutive => "移除連續重複行",
        }
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum CommentCommand {
    Toggle,
    Add,
    Remove,
}
impl CommentCommand {
    pub const ALL: [Self; 3] = [Self::Toggle, Self::Add, Self::Remove];
    pub fn label(self) -> &'static str {
        match self {
            Self::Toggle => "切換行註解    Ctrl+Q",
            Self::Add => "加入行註解",
            Self::Remove => "移除行註解",
        }
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Command {
    Lines(LineCommand),
    Comment(CommentCommand),
}
#[derive(Clone, Copy)]
enum CommentSyntax {
    Line(&'static str),
    Block(&'static str, &'static str),
}

fn comment_syntax(language: Language) -> Option<CommentSyntax> {
    use Language::*;
    match language {
        Rust | JavaScript | TypeScript | Tsx | C | Cpp => Some(CommentSyntax::Line("//")),
        Python | Yaml | Toml | Shell | PowerShell => Some(CommentSyntax::Line("#")),
        Html | Markdown => Some(CommentSyntax::Block("<!--", "-->")),
        Css => Some(CommentSyntax::Block("/*", "*/")),
        Plain | Json => None,
    }
}
pub(super) fn supports_comments(language: Language) -> bool {
    comment_syntax(language).is_some()
}

pub(super) struct Edit {
    pub content: Content,
    pub selection: (usize, usize),
}

pub(super) fn byte_position(text: &str, index: usize) -> Result<usize, String> {
    text.char_indices()
        .map(|(offset, _)| offset)
        .chain(std::iter::once(text.len()))
        .nth(index)
        .ok_or_else(|| "無效選取範圍".into())
}

/// A selection ending exactly at the next line start excludes that next line.
pub(super) fn line_range(
    text: &str,
    selection: (usize, usize),
    whole_if_empty: bool,
) -> Result<Range<usize>, String> {
    if selection.0 > selection.1 {
        return Err("無效選取範圍".into());
    }
    let start = byte_position(text, selection.0)?;
    let end = byte_position(text, selection.1)?;
    if start == end && whole_if_empty {
        return Ok(0..text.len());
    }
    let first = text[..start].rfind('\n').map_or(0, |offset| offset + 1);
    let last = if end > start && text[..end].ends_with('\n') {
        end
    } else {
        text[end..]
            .find('\n')
            .map_or(text.len(), |offset| end + offset + 1)
    };
    Ok(first..last)
}

fn is_commented(body: &str, syntax: CommentSyntax) -> bool {
    match syntax {
        CommentSyntax::Line(prefix) => body.starts_with(prefix),
        CommentSyntax::Block(open, close) => body
            .strip_prefix(open)
            .is_some_and(|rest| rest.ends_with(close)),
    }
}

fn comments(
    lines: &[&str],
    syntax: CommentSyntax,
    command: CommentCommand,
) -> Result<String, String> {
    let adding = match command {
        CommentCommand::Add => true,
        CommentCommand::Remove => false,
        CommentCommand::Toggle => !lines
            .iter()
            .map(|line| line.trim_start_matches([' ', '\t']))
            .filter(|body| !body.is_empty())
            .all(|body| is_commented(body, syntax)),
    };
    let mut result = String::new();
    for (index, line) in lines.iter().enumerate() {
        if index > 0 {
            result.push('\n');
        }
        let body = line.trim_start_matches([' ', '\t']);
        let indentation = &line[..line.len() - body.len()];
        result.push_str(indentation);
        if body.is_empty() {
            continue;
        }
        let commented = is_commented(body, syntax);
        match (adding, commented, syntax) {
            (true, false, CommentSyntax::Line(prefix)) => {
                result.push_str(prefix);
                result.push(' ');
                result.push_str(body);
            }
            (true, false, CommentSyntax::Block(open, close)) => {
                if body.contains(open) || body.contains(close) {
                    return Err("選取內容包含區塊註解界符，未修改".into());
                }
                result.push_str(open);
                result.push(' ');
                result.push_str(body);
                result.push(' ');
                result.push_str(close);
            }
            (false, true, CommentSyntax::Line(prefix)) => {
                let rest = &body[prefix.len()..];
                result.push_str(rest.strip_prefix(' ').unwrap_or(rest));
            }
            (false, true, CommentSyntax::Block(open, close)) => {
                let rest = &body[open.len()..body.len() - close.len()];
                let rest = rest.strip_prefix(' ').unwrap_or(rest);
                result.push_str(rest.strip_suffix(' ').unwrap_or(rest));
            }
            _ => result.push_str(body),
        }
    }
    Ok(result)
}

/// Build and validate the complete candidate before the caller changes a document.
/// Invalid selections, unsupported languages and oversized output return an error.
pub(super) fn apply(
    content: &Content,
    selection: (usize, usize),
    command: Command,
    language: Language,
) -> Result<Edit, String> {
    let range = line_range(
        &content.text,
        selection,
        matches!(command, Command::Lines(_)),
    )?;
    let selected = &content.text[range.clone()];
    let trailing_newline = selected.ends_with('\n');
    let body = selected.strip_suffix('\n').unwrap_or(selected);
    let mut lines: Vec<&str> = body.split('\n').collect();
    let mut replacement = match command {
        Command::Lines(operation) => {
            match operation {
                LineCommand::SortAscending => lines.sort(),
                LineCommand::SortDescending => lines.sort_by(|a, b| b.cmp(a)),
                LineCommand::Reverse => lines.reverse(),
                LineCommand::Unique => {
                    let mut seen = HashSet::with_capacity(lines.len());
                    lines.retain(|line| seen.insert(*line));
                }
                LineCommand::UniqueConsecutive => lines.dedup(),
            }
            lines.join("\n")
        }
        Command::Comment(operation) => comments(
            &lines,
            comment_syntax(language).ok_or_else(|| "目前語言未定義註解".to_owned())?,
            operation,
        )?,
    };
    if trailing_newline {
        replacement.push('\n');
    }
    let start = content.text[..range.start].chars().count();
    let selection = (start, start + replacement.chars().count());
    let mut candidate = content.clone();
    candidate.text.replace_range(range, &replacement);
    core::encode(&candidate)?;
    Ok(Edit {
        content: candidate,
        selection,
    })
}

impl crate::App {
    pub(super) fn text_command(&mut self, command: Command) {
        if self.busy
            || self.pending_close.is_some()
            || self.exit
            || self.docs[self.active].composing
        {
            return;
        }
        let d = &mut self.docs[self.active];
        let language = crate::syntax::resolve(d.path.as_deref(), &d.content.text, d.language);
        match apply(&d.content, d.selected, command, language) {
            Ok(edit) => {
                if edit.content != d.content {
                    d.history
                        .record(std::mem::replace(&mut d.content, edit.content));
                    if matches!(command, Command::Lines(_)) {
                        // Reordering can invalidate a bookmark even when the line count is unchanged.
                        d.bookmarks.clear();
                    } else {
                        d.bookmarks.sync(&d.content.text);
                    }
                    d.folds.sync(&d.content.text);
                }
                d.selected = edit.selection;
                d.cursor = edit.selection.1;
                self.selection = Some(edit.selection);
                self.match_range = None;
                self.match_signature = None;
                self.message = "文字處理完成；Ctrl+Z 可復原".into();
            }
            Err(error) => self.message = format!("文字處理失敗：{error}"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn rejected_and_blocked_app_commands_preserve_selection_history_and_other_tabs() {
        let mut app = crate::app_tests::app();
        app.docs[0].content = Content {
            text: "a\n/* unfinished\n".into(),
            bom: true,
            newline: core::Newline::Crlf,
        };
        app.docs[0].saved = app.docs[0].content.clone();
        app.docs[0].language = Some(Language::Css);
        let end = app.docs[0].content.text.chars().count();
        app.docs[0].selected = (0, end);
        app.docs[0].cursor = end;
        let before = app.docs[0].content.clone();
        let command = Command::Comment(CommentCommand::Add);
        app.text_command(command);
        assert_eq!(app.docs[0].content, before);
        assert_eq!(app.docs[0].selected, (0, end));
        assert_eq!(app.docs[0].cursor, end);
        assert!(!app.docs[0].history.can_undo());
        assert!(app.message.contains("區塊註解界符"));
        app.docs[0].language = Some(Language::Rust);
        for blocked in 0..4 {
            app.busy = blocked == 0;
            app.docs[0].composing = blocked == 1;
            app.exit = blocked == 2;
            app.pending_close = (blocked == 3).then_some(app.docs[0].id);
            app.text_command(command);
            assert_eq!(app.docs[0].content, before);
            assert_eq!(app.docs[0].selected, (0, end));
            assert!(!app.docs[0].history.can_undo());
        }
        app.pending_close = None;
        app.new_doc();
        app.docs[1].content.text = "b\na\nb\n".into();
        app.text_command(Command::Lines(LineCommand::Unique));
        assert_eq!(app.docs[1].content.text, "b\na\n");
        assert_eq!(app.docs[0].content, before);
        let d = &mut app.docs[1];
        d.history.undo(&mut d.content);
        assert_eq!(d.content.text, "b\na\nb\n");
        assert!(!d.history.can_undo());
        d.history.redo(&mut d.content);
        assert_eq!(d.content.text, "b\na\n");
    }
    #[test]
    fn sorting_selected_unicode_lines_preserves_outside_and_bom_crlf() {
        let content = Content {
            text: "外\n中🙂\nA\n中🙂\n0尾".into(),
            bom: true,
            newline: core::Newline::Crlf,
        };
        let edit = apply(
            &content,
            (2, 10),
            Command::Lines(LineCommand::SortAscending),
            Language::Plain,
        )
        .unwrap();
        assert_eq!(edit.content.text, "外\nA\n中🙂\n中🙂\n0尾");
        assert_eq!(edit.selection, (2, 10));
        assert_eq!(
            core::encode(&edit.content).unwrap(),
            "\u{feff}外\r\nA\r\n中🙂\r\n中🙂\r\n0尾".as_bytes()
        );
    }
    #[test]
    fn unique_lines_keep_first_occurrence_and_terminal_newline() {
        let content = Content {
            text: "中\n中\nA\n中\n\n\n".into(),
            ..Default::default()
        };
        let run = |command| {
            apply(&content, (0, 0), Command::Lines(command), Language::Plain)
                .unwrap()
                .content
                .text
        };
        assert_eq!(run(LineCommand::Unique), "中\nA\n\n");
        assert_eq!(run(LineCommand::UniqueConsecutive), "中\nA\n中\n\n");
        assert_eq!(run(LineCommand::SortDescending), "中\n中\n中\nA\n\n\n");
        assert_eq!(run(LineCommand::Reverse), "\n\n中\nA\n中\n中\n");
        assert_eq!(content.text, "中\n中\nA\n中\n\n\n");
    }
    #[test]
    fn comments_toggle_indented_unicode_and_skip_blank_lines_without_touching_next_line() {
        let content = Content {
            text: "  let 中 = \"🙂\";\n\tprintln!();\n  \n尾\n".into(),
            bom: true,
            newline: core::Newline::Cr,
        };
        let end = content.text[..content.text.find("尾").unwrap()]
            .chars()
            .count();
        let commented = apply(
            &content,
            (1, end),
            Command::Comment(CommentCommand::Toggle),
            Language::Rust,
        )
        .unwrap();
        assert_eq!(
            commented.content.text,
            "  // let 中 = \"🙂\";\n\t// println!();\n  \n尾\n"
        );
        let restored = apply(
            &commented.content,
            commented.selection,
            Command::Comment(CommentCommand::Toggle),
            Language::Rust,
        )
        .unwrap();
        assert_eq!(restored.content, content);
        let added = apply(
            &commented.content,
            commented.selection,
            Command::Comment(CommentCommand::Add),
            Language::Rust,
        )
        .unwrap();
        assert_eq!(added.content, commented.content);
    }
    #[test]
    fn language_rules_and_invalid_block_delimiters_are_atomic() {
        for language in [
            Language::Python,
            Language::Yaml,
            Language::Toml,
            Language::Shell,
            Language::PowerShell,
        ] {
            let content = Content {
                text: "\t中文🙂\n".into(),
                ..Default::default()
            };
            assert_eq!(
                apply(
                    &content,
                    (1, 1),
                    Command::Comment(CommentCommand::Add),
                    language
                )
                .unwrap()
                .content
                .text,
                "\t# 中文🙂\n"
            );
        }
        for (language, expected) in [
            (Language::Html, "<!-- <p>中🙂</p> -->\n"),
            (Language::Css, "/* <p>中🙂</p> */\n"),
        ] {
            let content = Content {
                text: "<p>中🙂</p>\n".into(),
                ..Default::default()
            };
            let edit = apply(
                &content,
                (0, 0),
                Command::Comment(CommentCommand::Toggle),
                language,
            )
            .unwrap();
            assert_eq!(edit.content.text, expected);
            assert_eq!(
                apply(
                    &edit.content,
                    edit.selection,
                    Command::Comment(CommentCommand::Remove),
                    language
                )
                .unwrap()
                .content,
                content
            );
        }
        let invalid = Content {
            text: "valid\n/* unfinished\n".into(),
            ..Default::default()
        };
        assert!(
            apply(
                &invalid,
                (0, invalid.text.chars().count()),
                Command::Comment(CommentCommand::Add),
                Language::Css
            )
            .is_err()
        );
        assert!(
            apply(
                &invalid,
                (0, 0),
                Command::Comment(CommentCommand::Add),
                Language::Json
            )
            .is_err()
        );
        assert!(
            apply(
                &invalid,
                (4, 1),
                Command::Lines(LineCommand::Unique),
                Language::Plain
            )
            .is_err()
        );
        assert!(
            apply(
                &invalid,
                (0, 999),
                Command::Lines(LineCommand::Unique),
                Language::Plain
            )
            .is_err()
        );
        let huge = Content {
            text: "x".repeat(16_383),
            ..Default::default()
        };
        assert!(
            apply(
                &huge,
                (0, 0),
                Command::Comment(CommentCommand::Add),
                Language::Rust
            )
            .is_err()
        );
    }
}
