use std::{
    fs,
    io::{self, Read, Write},
    path::Path,
};

pub const MAX_BYTES: usize = 2 * 1024 * 1024;
pub fn validate_text(text: &str) -> Result<(), String> {
    if text.len() > MAX_BYTES {
        return Err("超過 2 MiB 上限".into());
    }
    if text.lines().count() > 20_000 {
        return Err("超過 20,000 行上限".into());
    }
    if text.split('\n').any(|line| line.len() > 16_384) {
        return Err("單行超過 16 KiB 上限".into());
    }
    Ok(())
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Newline {
    Lf,
    Crlf,
    Cr,
}
impl Newline {
    pub fn label(self) -> &'static str {
        match self {
            Self::Lf => "LF",
            Self::Crlf => "CRLF",
            Self::Cr => "CR",
        }
    }
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Content {
    pub text: String,
    pub bom: bool,
    pub newline: Newline,
}
impl Default for Content {
    fn default() -> Self {
        Self {
            text: String::new(),
            bom: false,
            newline: Newline::Lf,
        }
    }
}
pub fn decode(bytes: &[u8]) -> Result<Content, String> {
    if bytes.len() > MAX_BYTES {
        return Err("超過 2 MiB 檔案上限".into());
    }
    let bom = bytes.starts_with(&[0xef, 0xbb, 0xbf]);
    let text = std::str::from_utf8(if bom { &bytes[3..] } else { bytes })
        .map_err(|_| "不是有效 UTF-8；首版不支援 UTF-16、Big5 或其他編碼")?;
    if text.contains('\0') {
        return Err("包含 NUL，可能是二進位檔案".into());
    }
    let crlf = text.matches("\r\n").count();
    let lf = text.matches('\n').count();
    let cr = text.matches('\r').count();
    if (crlf > 0 && (cr != crlf || lf != crlf)) || (crlf == 0 && cr > 0 && lf > 0) {
        return Err("偵測到混合 CRLF／LF／CR；拒絕開啟以避免靜默改寫，原檔未變更".into());
    }
    let newline = if crlf > 0 {
        Newline::Crlf
    } else if cr > 0 {
        Newline::Cr
    } else {
        Newline::Lf
    };
    let text = match newline {
        Newline::Crlf => text.replace("\r\n", "\n"),
        Newline::Cr => text.replace('\r', "\n"),
        Newline::Lf => text.to_owned(),
    };
    validate_text(&text)?;
    Ok(Content { text, bom, newline })
}
pub fn encode(content: &Content) -> Result<Vec<u8>, String> {
    validate_text(&content.text)?;
    if content.text.contains('\r') || content.text.contains('\0') {
        return Err("內容包含不支援的 CR 或 NUL".into());
    }
    let text = match content.newline {
        Newline::Crlf => content.text.replace('\n', "\r\n"),
        Newline::Cr => content.text.replace('\n', "\r"),
        Newline::Lf => content.text.clone(),
    };
    let mut bytes = if content.bom {
        vec![0xef, 0xbb, 0xbf]
    } else {
        Vec::new()
    };
    bytes.extend_from_slice(text.as_bytes());
    if bytes.len() > MAX_BYTES {
        return Err("儲存內容超過 2 MiB 上限".into());
    }
    Ok(bytes)
}
pub fn read_file(path: &Path) -> Result<Content, String> {
    let file = fs::File::open(path).map_err(|e| e.to_string())?;
    let mut bytes = Vec::new();
    file.take((MAX_BYTES + 1) as u64)
        .read_to_end(&mut bytes)
        .map_err(|e| e.to_string())?;
    decode(&bytes)
}
pub fn atomic_save(path: &Path, content: &Content) -> Result<(), String> {
    let bytes = encode(content)?;
    atomic_write(path, &bytes, |file, bytes| file.write_all(bytes)).map_err(|e| e.to_string())
}
fn atomic_write(
    path: &Path,
    bytes: &[u8],
    write: impl FnOnce(&mut fs::File, &[u8]) -> io::Result<()>,
) -> io::Result<()> {
    let parent = path
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or(Path::new("."));
    let mut temp = tempfile::NamedTempFile::new_in(parent)?;
    if let Ok(meta) = fs::metadata(path) {
        if meta.permissions().readonly() {
            return Err(io::Error::new(io::ErrorKind::PermissionDenied, "檔案唯讀"));
        }
        temp.as_file().set_permissions(meta.permissions())?;
    }
    write(temp.as_file_mut(), bytes)?;
    temp.as_file().sync_all()?;
    temp.persist(path).map_err(|e| e.error)?;
    Ok(())
}
pub fn find(text: &str, query: &str, from_char: usize) -> Option<(usize, usize)> {
    if query.is_empty() {
        return None;
    }
    let from = text
        .char_indices()
        .nth(from_char)
        .map_or(text.len(), |(i, _)| i);
    let byte = text[from..]
        .find(query)
        .map(|i| from + i)
        .or_else(|| text[..from].find(query))?;
    let start = text[..byte].chars().count();
    Some((start, start + query.chars().count()))
}
pub fn replace_range(text: &mut String, range: (usize, usize), replacement: &str) {
    let start = text
        .char_indices()
        .nth(range.0)
        .map_or(text.len(), |(i, _)| i);
    let end = text
        .char_indices()
        .nth(range.1)
        .map_or(text.len(), |(i, _)| i);
    text.replace_range(start..end, replacement);
}
#[derive(Default)]
pub struct History {
    undo: Vec<Content>,
    redo: Vec<Content>,
}
impl History {
    pub fn record(&mut self, before: Content) {
        self.redo.clear();
        self.undo.push(before);
        while self.undo.len() > 64
            || self.undo.iter().map(|c| c.text.len()).sum::<usize>() > 16 * 1024 * 1024
        {
            self.undo.remove(0);
        }
    }
    pub fn undo(&mut self, current: &mut Content) {
        if let Some(previous) = self.undo.pop() {
            self.redo.push(std::mem::replace(current, previous));
        }
    }
    pub fn redo(&mut self, current: &mut Content) {
        if let Some(next) = self.redo.pop() {
            self.undo.push(std::mem::replace(current, next));
        }
    }
}
pub fn may_close(current: &Content, saved: &Content, discard: bool) -> bool {
    current == saved || discard
}
#[derive(Clone, Copy)]
pub enum WhitespaceEdit {
    TabsToSpaces(usize),
    SpacesToTabs(usize),
    TrimTrailing,
    AddFinalNewline,
    RemoveFinalNewline,
}
pub fn tab_advance(column: usize, width: usize) -> usize {
    let width = width.clamp(1, 8);
    width - column % width
}
pub fn column_at(text: &str, index: usize, width: usize) -> usize {
    let mut column = 0;
    for ch in text.chars().take(index) {
        column = match ch {
            '\n' => 0,
            '\t' => column + tab_advance(column, width),
            _ => column + unicode_width::UnicodeWidthChar::width(ch).unwrap_or(0),
        }
    }
    column
}
pub fn edit_whitespace(
    content: &mut Content,
    range: Option<(usize, usize)>,
    action: WhitespaceEdit,
) -> Result<bool, String> {
    let mut candidate = content.clone();
    let (start, end) = range.unwrap_or((0, content.text.chars().count()));
    if start > end || end > content.text.chars().count() {
        return Err("無效選取範圍".into());
    }
    let original: String = content.text.chars().skip(start).take(end - start).collect();
    let changed = match action {
        WhitespaceEdit::TabsToSpaces(width) => {
            if !(1..=8).contains(&width) {
                return Err("Tab 寬度需介於 1 至 8".into());
            }
            let mut result = String::new();
            let mut column = column_at(&content.text, start, width);
            for ch in original.chars() {
                match ch {
                    '\t' => {
                        let advance = tab_advance(column, width);
                        result.push_str(&" ".repeat(advance));
                        column += advance;
                    }
                    '\n' => {
                        result.push(ch);
                        column = 0;
                    }
                    _ => {
                        result.push(ch);
                        column += unicode_width::UnicodeWidthChar::width(ch).unwrap_or(0);
                    }
                }
            }
            result
        }
        WhitespaceEdit::SpacesToTabs(width) => {
            if !(1..=8).contains(&width) {
                return Err("Tab 寬度需介於 1 至 8".into());
            }
            let mut result = String::new();
            let mut column = column_at(&content.text, start, width);
            let mut chars = original.chars().peekable();
            while let Some(ch) = chars.next() {
                if ch == ' ' {
                    let mut count = 1;
                    while chars.peek() == Some(&' ') {
                        chars.next();
                        count += 1;
                    }
                    while count > 0 {
                        let advance = tab_advance(column, width);
                        if count >= advance {
                            result.push('\t');
                            column += advance;
                            count -= advance;
                        } else {
                            result.push_str(&" ".repeat(count));
                            column += count;
                            count = 0;
                        }
                    }
                } else {
                    result.push(ch);
                    column = match ch {
                        '\n' => 0,
                        '\t' => column + tab_advance(column, width),
                        _ => column + unicode_width::UnicodeWidthChar::width(ch).unwrap_or(0),
                    };
                }
            }
            result
        }
        WhitespaceEdit::TrimTrailing => original
            .split('\n')
            .map(|line| line.trim_end_matches([' ', '\t']))
            .collect::<Vec<_>>()
            .join("\n"),
        WhitespaceEdit::AddFinalNewline => {
            if original.ends_with('\n') {
                original
            } else {
                original + "\n"
            }
        }
        WhitespaceEdit::RemoveFinalNewline => {
            original.strip_suffix('\n').unwrap_or(&original).to_owned()
        }
    };
    replace_range(&mut candidate.text, (start, end), &changed);
    validate_text(&candidate.text)?;
    let changed = candidate != *content;
    if changed {
        *content = candidate;
    }
    Ok(changed)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn roundtrip() {
        for bytes in [
            b"abc\n".as_slice(),
            b"\xef\xbb\xbfabc\r\n",
            "中文🙂\r\n".as_bytes(),
            b"\xef\xbb\xbf",
        ] {
            assert_eq!(encode(&decode(bytes).unwrap()).unwrap(), bytes);
        }
    }
    #[test]
    fn rejects_unsafe_input() {
        for bytes in [b"\xff".as_slice(), b"a\r\nb\n", b"a\rb\n", b"a\0b"] {
            assert!(decode(bytes).is_err());
        }
        assert!(decode(&vec![b'a'; MAX_BYTES + 1]).is_err());
    }
    #[test]
    fn unicode_search_replace() {
        let mut s = "甲🙂乙🙂".to_owned();
        assert_eq!(find(&s, "🙂", 2), Some((3, 4)));
        assert_eq!(find(&s, "甲", 4), Some((0, 1)));
        assert_eq!(find(&s, "", 0), None);
        replace_range(&mut s, (1, 2), "繁中");
        assert_eq!(s, "甲繁中乙🙂");
        assert_eq!(find(&s, "🙂", 0), Some((4, 5)));
    }
    #[test]
    fn close_cancel_and_history() {
        let saved = Content::default();
        let mut c = saved.clone();
        let mut h = History::default();
        h.record(c.clone());
        c.text = "🙂".into();
        assert!(!may_close(&c, &saved, false));
        assert!(may_close(&c, &saved, true));
        h.undo(&mut c);
        assert!(may_close(&c, &saved, false));
        h.redo(&mut c);
        assert_eq!(c.text, "🙂");
    }
    #[test]
    fn failed_write_preserves_original() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("file.txt");
        fs::write(&path, b"original").unwrap();
        assert!(
            atomic_write(&path, b"new", |f, _| {
                f.write_all(b"partial")?;
                Err(io::Error::other("injected disk failure"))
            })
            .is_err()
        );
        assert_eq!(fs::read(&path).unwrap(), b"original");
        atomic_save(
            &path,
            &Content {
                text: "中文".into(),
                ..Default::default()
            },
        )
        .unwrap();
        assert_eq!(fs::read(&path).unwrap(), "中文".as_bytes());
    }
    #[test]
    fn output_limits_and_invalid_content_preserve_original() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("original.txt");
        fs::write(&path, b"safe").unwrap();
        for text in [
            "\0".to_owned(),
            "\r".to_owned(),
            "x".repeat(16_385),
            "x\n".repeat(20_001),
        ] {
            assert!(
                atomic_save(
                    &path,
                    &Content {
                        text,
                        ..Default::default()
                    }
                )
                .is_err()
            );
            assert_eq!(fs::read(&path).unwrap(), b"safe");
        }
    }
    #[test]
    fn replacement_is_one_undo_transaction() {
        let mut c = Content {
            text: "甲🙂甲🙂".into(),
            ..Default::default()
        };
        let mut h = History::default();
        h.record(c.clone());
        c.text = c.text.replace("甲🙂", "乙");
        assert_eq!(c.text, "乙乙");
        h.undo(&mut c);
        assert_eq!(c.text, "甲🙂甲🙂");
        h.redo(&mut c);
        assert_eq!(c.text, "乙乙");
    }
    #[test]
    fn external_comparison_uses_bytes_for_no_newline_files() {
        for text in ["", "中文🙂"] {
            let original = Content {
                text: text.into(),
                bom: true,
                newline: Newline::Crlf,
            };
            let bytes = encode(&original).unwrap();
            let loaded = decode(&bytes).unwrap();
            assert_eq!(encode(&loaded).unwrap(), encode(&original).unwrap());
        }
    }
    #[test]
    fn legacy_and_platform_eol_roundtrip() {
        for bytes in [b"a\rb\r".as_slice(), b"\xef\xbb\xbfa\r\nb", b"a\nb\n"] {
            let c = decode(bytes).unwrap();
            assert_eq!(encode(&c).unwrap(), bytes);
        }
        let mut c = decode(b"a\r\nb\r\n").unwrap();
        let saved = c.clone();
        let mut h = History::default();
        h.record(c.clone());
        c.newline = Newline::Lf;
        assert_eq!(encode(&c).unwrap(), b"a\nb\n");
        h.undo(&mut c);
        assert_eq!(c, saved);
    }
    #[test]
    fn whitespace_preserves_unicode_blank_lines_and_bom() {
        let mut c = Content {
            text: "甲\t  \u{3000}\u{a0} \t\n \t\n末尾".into(),
            bom: true,
            newline: Newline::Crlf,
        };
        let mut h = History::default();
        let before = c.clone();
        h.record(before.clone());
        edit_whitespace(&mut c, None, WhitespaceEdit::TrimTrailing).unwrap();
        assert_eq!(c.text, "甲\t  \u{3000}\u{a0}\n\n末尾");
        assert!(c.bom);
        assert_eq!(c.newline, Newline::Crlf);
        h.undo(&mut c);
        assert_eq!(c, before);
    }
    #[test]
    fn selection_tab_conversion_and_undo() {
        let mut c = Content {
            text: "前🙂\t後\t".into(),
            ..Default::default()
        };
        let before = c.clone();
        let mut h = History::default();
        h.record(c.clone());
        edit_whitespace(&mut c, Some((2, 3)), WhitespaceEdit::TabsToSpaces(4)).unwrap();
        assert_eq!(c.text, "前🙂    後\t");
        edit_whitespace(&mut c, Some((2, 6)), WhitespaceEdit::SpacesToTabs(4)).unwrap();
        assert_eq!(c, before);
        h.undo(&mut c);
        assert_eq!(c, before);
    }
    #[test]
    fn final_newline_keeps_blank_lines() {
        let mut c = Content {
            text: "a\n\n".into(),
            ..Default::default()
        };
        assert!(!edit_whitespace(&mut c, None, WhitespaceEdit::AddFinalNewline).unwrap());
        edit_whitespace(&mut c, None, WhitespaceEdit::RemoveFinalNewline).unwrap();
        assert_eq!(c.text, "a\n");
        edit_whitespace(&mut c, None, WhitespaceEdit::RemoveFinalNewline).unwrap();
        assert_eq!(c.text, "a");
        edit_whitespace(&mut c, None, WhitespaceEdit::AddFinalNewline).unwrap();
        assert_eq!(c.text, "a\n");
    }
    #[test]
    fn whitespace_conversion_limits_no_partial_mutation() {
        let mut c = Content {
            text: "\t".repeat(4096),
            ..Default::default()
        };
        let before = c.clone();
        assert!(edit_whitespace(&mut c, None, WhitespaceEdit::TabsToSpaces(8)).is_err());
        assert_eq!(c, before);
        assert!(edit_whitespace(&mut c, None, WhitespaceEdit::SpacesToTabs(0)).is_err());
    }
    #[test]
    fn tab_stops_conversion_matches_columns_and_selection() {
        let mut c = Content {
            text: "a\tb\t\n中文\t🙂\t".into(),
            ..Default::default()
        };
        let original = c.clone();
        let columns: Vec<_> = c
            .text
            .lines()
            .map(|line| column_at(line, line.chars().count(), 4))
            .collect();
        edit_whitespace(&mut c, None, WhitespaceEdit::TabsToSpaces(4)).unwrap();
        assert_eq!(c.text, "a   b   \n中文    🙂  ");
        assert_eq!(
            columns,
            c.text
                .lines()
                .map(|line| column_at(line, line.chars().count(), 4))
                .collect::<Vec<_>>()
        );
        edit_whitespace(&mut c, None, WhitespaceEdit::SpacesToTabs(4)).unwrap();
        assert_eq!(c, original);
        let mut c = Content {
            text: "a  b".into(),
            ..Default::default()
        };
        edit_whitespace(&mut c, Some((1, 3)), WhitespaceEdit::SpacesToTabs(4)).unwrap();
        assert_eq!(c.text, "a  b");
    }
}
