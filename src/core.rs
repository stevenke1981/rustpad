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
    if cr != crlf || (crlf > 0 && lf != crlf) {
        return Err("混合換行或單獨 CR 暫不支援；原檔未變更".into());
    }
    validate_text(text)?;
    Ok(Content {
        text: text.replace("\r\n", "\n"),
        bom,
        newline: if crlf > 0 { Newline::Crlf } else { Newline::Lf },
    })
}
pub fn encode(content: &Content) -> Result<Vec<u8>, String> {
    validate_text(&content.text)?;
    if content.text.contains('\r') || content.text.contains('\0') {
        return Err("內容包含不支援的 CR 或 NUL".into());
    }
    let text = if content.newline == Newline::Crlf {
        content.text.replace('\n', "\r\n")
    } else {
        content.text.clone()
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
        for bytes in [b"\xff".as_slice(), b"a\r\nb\n", b"a\rb", b"a\0b"] {
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
}
