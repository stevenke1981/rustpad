use crate::core::Content;
#[derive(Clone, Copy)]
pub enum Task {
    Find { from: usize, backward: bool },
    Count { range: Option<(usize, usize)> },
    Replace { range: Option<(usize, usize)> },
    ReplaceOne { range: (usize, usize) },
}
pub enum Outcome {
    Error(String),
    Found(Option<(usize, usize)>),
    Count(usize),
    Replaced(Result<(Content, usize), String>),
}
pub fn execute(
    content: &Content,
    query: &str,
    replacement: &str,
    options: SearchOptions,
    task: Task,
) -> Outcome {
    if options.regex
        && let Err(error) = crate::pattern::ranges(&content.text, query, options)
    {
        return Outcome::Error(error);
    }
    match task {
        Task::Find { from, backward } => {
            Outcome::Found(next_match(&content.text, query, from, options, backward))
        }
        Task::Count { range } => Outcome::Count(
            matches(&content.text, query, options)
                .iter()
                .filter(|r| range.is_none_or(|scope| r.0 >= scope.0 && r.1 <= scope.1))
                .count(),
        ),
        Task::Replace { range } => {
            let mut result = content.clone();
            Outcome::Replaced(
                replace_matches(&mut result, query, replacement, options, range)
                    .map(|count| (result, count)),
            )
        }
        Task::ReplaceOne { range } => {
            let mut result = content.clone();
            Outcome::Replaced(
                replace_current(&mut result, query, replacement, options, range)
                    .map(|count| (result, count)),
            )
        }
    }
}
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct SearchOptions {
    pub regex: bool,
    pub match_case: bool,
    pub whole_word: bool,
    pub wrap: bool,
}
impl Default for SearchOptions {
    fn default() -> Self {
        Self {
            regex: false,
            match_case: true,
            whole_word: false,
            wrap: true,
        }
    }
}
pub(crate) fn word(ch: char) -> bool {
    ch.is_alphanumeric()
        || ch == '_'
        || (!ch.is_control() && unicode_width::UnicodeWidthChar::width(ch) == Some(0))
}
#[derive(Clone, Copy)]
pub enum Edit {
    DuplicateLine,
    DeleteLine,
    Uppercase,
    Lowercase,
}
pub fn edit(
    content: &mut Content,
    selection: (usize, usize),
    cursor: usize,
    action: Edit,
) -> Result<(usize, usize), String> {
    let count = content.text.chars().count();
    if selection.0 > selection.1 || selection.1 > count {
        return Err("無效選取範圍".into());
    }
    let mut candidate = content.clone();
    let result = match action {
        Edit::Uppercase | Edit::Lowercase => {
            if selection.0 == selection.1 {
                return Err("請先選取文字".into());
            }
            let selected: String = content
                .text
                .chars()
                .skip(selection.0)
                .take(selection.1 - selection.0)
                .collect();
            let replacement = match action {
                Edit::Uppercase => selected.to_uppercase(),
                _ => selected.to_lowercase(),
            };
            crate::core::replace_range(&mut candidate.text, selection, &replacement);
            (selection.0, selection.0 + replacement.chars().count())
        }
        Edit::DuplicateLine | Edit::DeleteLine => {
            let byte = content
                .text
                .char_indices()
                .nth(cursor.min(count))
                .map_or(content.text.len(), |p| p.0);
            let start = content.text[..byte].rfind('\n').map_or(0, |p| p + 1);
            let end = content.text[byte..]
                .find('\n')
                .map_or(content.text.len(), |p| byte + p + 1);
            let line = &content.text[start..end];
            match action {
                Edit::DuplicateLine => {
                    let replacement = if line.ends_with('\n') {
                        line.to_owned()
                    } else {
                        format!("\n{line}")
                    };
                    candidate.text.insert_str(end, &replacement);
                    let index =
                        content.text[..end].chars().count() + usize::from(!line.ends_with('\n'));
                    (index, index + line.trim_end_matches('\n').chars().count())
                }
                _ => {
                    let start = if start == end && start > 0 {
                        start - 1
                    } else {
                        start
                    };
                    let index = content.text[..start].chars().count();
                    candidate.text.replace_range(start..end, "");
                    (index, index)
                }
            }
        }
    };
    crate::core::encode(&candidate)?;
    *content = candidate;
    Ok(result)
}
struct Prepared<'a> {
    chars: Vec<char>,
    boundaries: Vec<(usize, usize)>,
    haystack: std::borrow::Cow<'a, str>,
    needle: String,
    options: SearchOptions,
}
impl<'a> Prepared<'a> {
    fn new(text: &'a str, query: &str, options: SearchOptions) -> Self {
        let chars: Vec<_> = text.chars().collect();
        let mut boundaries = Vec::with_capacity(chars.len() + 1);
        let haystack = if options.match_case {
            boundaries.extend(
                text.char_indices()
                    .enumerate()
                    .map(|(index, (byte, _))| (byte, index)),
            );
            boundaries.push((text.len(), chars.len()));
            std::borrow::Cow::Borrowed(text)
        } else {
            let mut folded = String::new();
            for (index, ch) in chars.iter().enumerate() {
                boundaries.push((folded.len(), index));
                folded.extend(ch.to_lowercase());
            }
            boundaries.push((folded.len(), chars.len()));
            std::borrow::Cow::Owned(folded)
        };
        let needle = if options.match_case {
            query.to_owned()
        } else {
            query.chars().flat_map(char::to_lowercase).collect()
        };
        Self {
            chars,
            boundaries,
            haystack,
            needle,
            options,
        }
    }
    fn range(&self, byte: usize) -> Option<(usize, usize)> {
        let start = self
            .boundaries
            .binary_search_by_key(&byte, |p| p.0)
            .ok()
            .map(|i| self.boundaries[i].1)?;
        let end = self
            .boundaries
            .binary_search_by_key(&(byte + self.needle.len()), |p| p.0)
            .ok()
            .map(|i| self.boundaries[i].1)?;
        if self.options.whole_word
            && ((start > 0 && word(self.chars[start - 1]))
                || (end < self.chars.len() && word(self.chars[end])))
        {
            None
        } else {
            Some((start, end))
        }
    }
    fn scan(&self, mut start: usize, mut end: usize, backward: bool) -> Option<(usize, usize)> {
        if self.needle.is_empty() {
            return None;
        }
        while start <= end {
            let region = &self.haystack[start..end];
            let relative = if backward {
                region.rfind(&self.needle)
            } else {
                region.find(&self.needle)
            }?;
            let byte = start + relative;
            if let Some(range) = self.range(byte) {
                return Some(range);
            }
            if backward {
                let last = self.haystack[byte..byte + self.needle.len()]
                    .chars()
                    .next_back()?;
                end = byte + self.needle.len() - last.len_utf8();
            } else {
                start = byte + self.haystack[byte..].chars().next()?.len_utf8();
            }
        }
        None
    }
}
pub fn matches(text: &str, query: &str, options: SearchOptions) -> Vec<(usize, usize)> {
    if options.regex {
        return crate::pattern::ranges(text, query, options).unwrap_or_default();
    }
    if query.is_empty() {
        return vec![];
    }
    let prepared = Prepared::new(text, query, options);
    prepared
        .haystack
        .match_indices(&prepared.needle)
        .filter_map(|(byte, _)| prepared.range(byte))
        .collect()
}
pub fn next_match(
    text: &str,
    query: &str,
    from: usize,
    options: SearchOptions,
    backward: bool,
) -> Option<(usize, usize)> {
    if options.regex {
        let ranges = crate::pattern::ranges(text, query, options).ok()?;
        return if backward {
            ranges
                .iter()
                .rev()
                .find(|r| r.1 <= from && r.0 < from)
                .copied()
                .or_else(|| options.wrap.then(|| ranges.last().copied()).flatten())
        } else {
            ranges
                .iter()
                .find(|r| r.0 >= from)
                .copied()
                .or_else(|| options.wrap.then(|| ranges.first().copied()).flatten())
        };
    }
    if query.is_empty() {
        return None;
    }
    let prepared = Prepared::new(text, query, options);
    let anchor = prepared.boundaries[from.min(prepared.chars.len())].0;
    let size = prepared.haystack.len();
    if backward {
        prepared.scan(0, anchor, true).or_else(|| {
            options
                .wrap
                .then(|| prepared.scan(anchor, size, true))
                .flatten()
        })
    } else {
        prepared.scan(anchor, size, false).or_else(|| {
            options
                .wrap
                .then(|| prepared.scan(0, anchor, false))
                .flatten()
        })
    }
}
pub fn replace_current(
    content: &mut Content,
    query: &str,
    replacement: &str,
    options: SearchOptions,
    range: (usize, usize),
) -> Result<usize, String> {
    if options.regex {
        return crate::pattern::replace(content, query, replacement, options, Some(range), true);
    }
    if next_match(
        &content.text,
        query,
        range.0,
        SearchOptions {
            wrap: false,
            ..options
        },
        false,
    ) != Some(range)
    {
        return Ok(0);
    }
    let mut candidate = content.clone();
    crate::core::replace_range(&mut candidate.text, range, replacement);
    crate::core::encode(&candidate)?;
    *content = candidate;
    Ok(1)
}
pub fn replace_matches(
    content: &mut Content,
    query: &str,
    replacement: &str,
    options: SearchOptions,
    range: Option<(usize, usize)>,
) -> Result<usize, String> {
    if options.regex {
        return crate::pattern::replace(content, query, replacement, options, range, false);
    }
    let chars = content.text.chars().count();
    let range = range.unwrap_or((0, chars));
    if range.0 > range.1 || range.1 > chars {
        return Err("無效選取範圍".into());
    }
    let matches: Vec<_> = matches(&content.text, query, options)
        .into_iter()
        .filter(|r| r.0 >= range.0 && r.1 <= range.1)
        .collect();
    if matches.is_empty() {
        return Ok(0);
    }
    let offsets: Vec<_> = content
        .text
        .char_indices()
        .map(|p| p.0)
        .chain(std::iter::once(content.text.len()))
        .collect();
    let removed: usize = matches.iter().map(|r| offsets[r.1] - offsets[r.0]).sum();
    let added = matches
        .len()
        .checked_mul(replacement.len())
        .ok_or("取代結果超限")?;
    let size = content
        .text
        .len()
        .saturating_sub(removed)
        .checked_add(added)
        .ok_or("取代結果超限")?;
    if size > crate::core::MAX_BYTES {
        return Err("取代結果超過 2 MiB 上限".into());
    }
    let mut output = String::with_capacity(size);
    let mut byte = 0;
    for r in &matches {
        output.push_str(&content.text[byte..offsets[r.0]]);
        output.push_str(replacement);
        byte = offsets[r.1];
    }
    output.push_str(&content.text[byte..]);
    let mut candidate = content.clone();
    candidate.text = output;
    crate::core::encode(&candidate)?;
    *content = candidate;
    Ok(matches.len())
}
pub fn goto_line(text: &str, line: usize) -> Option<usize> {
    if line == 0 {
        return None;
    }
    if line == 1 {
        return Some(0);
    }
    text.chars()
        .enumerate()
        .filter(|(_, ch)| *ch == '\n')
        .nth(line - 2)
        .map(|(index, _)| index + 1)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::{History, Newline};
    #[test]
    fn case_mapping_uses_the_same_rules_on_query_and_source() {
        let options = SearchOptions {
            match_case: false,
            ..Default::default()
        };
        assert_eq!(matches("ΟΣ", "ΟΣ", options), vec![(0, 2)]);
        assert_eq!(matches("İ", "İ", options), vec![(0, 1)]);
    }
    #[test]
    fn cursor_inside_a_match_preserves_next_previous_boundaries() {
        let options = SearchOptions {
            wrap: false,
            ..Default::default()
        };
        assert_eq!(next_match("🙂aaaa", "aa", 2, options, false), Some((2, 4)));
        assert_eq!(next_match("🙂aaaa", "aa", 4, options, true), Some((2, 4)));
        assert_eq!(matches("aaaa", "aa", options), vec![(0, 2), (2, 4)]);
    }
    #[test]
    fn line_edit_eof_and_failure_preserves_original() {
        let mut c = Content {
            text: "中文🙂\n尾".into(),
            bom: true,
            newline: Newline::Crlf,
        };
        edit(&mut c, (0, 0), 4, Edit::DuplicateLine).unwrap();
        assert_eq!(c.text, "中文🙂\n尾\n尾");
        edit(&mut c, (0, 0), 6, Edit::DeleteLine).unwrap();
        assert_eq!(c.text, "中文🙂\n尾\n");
        edit(&mut c, (0, 0), 6, Edit::DeleteLine).unwrap();
        assert_eq!(c.text, "中文🙂\n尾");
        assert!(c.bom);
        assert_eq!(c.newline, Newline::Crlf);
        c.text = "x".repeat(16_384);
        let before = c.clone();
        assert!(edit(&mut c, (0, 0), 0, Edit::DuplicateLine).is_ok());
        c = before;
        c.text = "x\n".repeat(19_999) + "x";
        let before = c.clone();
        assert!(edit(&mut c, (0, 0), 0, Edit::DuplicateLine).is_err());
        assert_eq!(c, before);
        c.text.clear();
        edit(&mut c, (0, 0), 0, Edit::DeleteLine).unwrap();
        assert_eq!(c.text, "");
    }
    #[test]
    fn unicode_case_expansion_and_no_selection_is_safe() {
        let mut c = Content {
            text: "🙂Straße中文 END".into(),
            ..Default::default()
        };
        assert_eq!(edit(&mut c, (1, 7), 0, Edit::Uppercase).unwrap(), (1, 8));
        assert_eq!(c.text, "🙂STRASSE中文 END");
        edit(&mut c, (11, 14), 0, Edit::Lowercase).unwrap();
        assert_eq!(c.text, "🙂STRASSE中文 end");
        let before = c.clone();
        assert!(edit(&mut c, (1, 1), 0, Edit::Uppercase).is_err());
        assert_eq!(c, before);
    }
    #[test]
    fn unicode_search_options_preserve_original_ranges() {
        let options = SearchOptions {
            match_case: false,
            whole_word: true,
            ..Default::default()
        };
        let text = "🙂 Rust rusty RUST 中文Rust Rust_ e\u{301} é İ";
        assert_eq!(matches(text, "rust", options), vec![(2, 6), (13, 17)]);
        assert!(matches(text, "e", options).is_empty());
        assert!(matches(text, "i", options).is_empty());
        assert_eq!(matches("İ", "i\u{307}", options), vec![(0, 1)]);
        assert!(matches(text, "", options).is_empty());
    }
    #[test]
    fn previous_next_and_no_wrap_boundaries() {
        let text = "🙂a a 中文a";
        let options = SearchOptions::default();
        assert_eq!(next_match(text, "a", 4, options, true), Some((3, 4)));
        assert_eq!(next_match(text, "a", 0, options, true), Some((7, 8)));
        assert_eq!(next_match(text, "a", 4, options, false), Some((7, 8)));
        assert_eq!(
            next_match(
                text,
                "a",
                8,
                SearchOptions {
                    wrap: false,
                    ..options
                },
                false
            ),
            None
        );
        assert_eq!(
            next_match(
                text,
                "a",
                0,
                SearchOptions {
                    wrap: false,
                    ..options
                },
                true
            ),
            None
        );
    }
    #[test]
    fn selection_replace_is_atomic_unicode_and_one_undo() {
        let mut c = Content {
            text: "🙂 Rust RUST rusty Rust".into(),
            bom: true,
            newline: Newline::Crlf,
        };
        let before = c.clone();
        let options = SearchOptions {
            match_case: false,
            whole_word: true,
            ..Default::default()
        };
        assert_eq!(
            replace_matches(&mut c, "rust", "中文", options, Some((2, 11))).unwrap(),
            2
        );
        assert_eq!(c.text, "🙂 中文 中文 rusty Rust");
        assert!(c.bom);
        assert_eq!(c.newline, Newline::Crlf);
        let mut history = History::default();
        history.record(before.clone());
        history.undo(&mut c);
        assert_eq!(c, before);
        assert!(replace_matches(&mut c, "Rust", "\0", options, None).is_err());
        assert_eq!(c, before);
        assert!(
            replace_matches(
                &mut c,
                "Rust",
                &"x".repeat(crate::core::MAX_BYTES),
                options,
                None
            )
            .is_err()
        );
        assert_eq!(c, before);
        assert!(replace_matches(&mut c, "Rust", "x", options, Some((9, 2))).is_err());
        assert_eq!(c, before);
    }
    #[test]
    fn line_navigation_handles_unicode_eof_and_invalid_input() {
        assert_eq!(goto_line("中文🙂\n\n最後\n", 3), Some(5));
        assert_eq!(goto_line("中文🙂\n\n最後\n", 4), Some(8));
        assert_eq!(goto_line("", 1), Some(0));
        assert_eq!(goto_line("x", 0), None);
        assert_eq!(goto_line("x", 2), None);
    }
}
