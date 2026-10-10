use std::{borrow::Cow, io::Read, path::PathBuf, sync::OnceLock};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Locale {
    #[default]
    Traditional,
    Simplified,
    English,
    Japanese,
}
impl Locale {
    pub const ALL: [Self; 4] = [
        Self::Traditional,
        Self::Simplified,
        Self::English,
        Self::Japanese,
    ];
    pub fn tag(self) -> &'static str {
        ["zh-TW", "zh-CN", "en", "ja"][self as usize]
    }
    pub fn name(self) -> &'static str {
        ["繁體中文", "简体中文", "English", "日本語"][self as usize]
    }
    pub fn parse(tag: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|locale| locale.tag() == tag)
    }
    pub fn text(self, source: &str) -> &str {
        catalog()
            .iter()
            .find(|row| row[0] == source)
            .map_or(source, |row| row[self as usize])
    }
    pub fn format(self, source: &str, values: &[&str]) -> String {
        render(self.text(source), values)
    }
    /// Translate application messages at display time, so an existing notice also
    /// changes language. Captured paths and OS diagnostics remain opaque, except
    /// for the explicitly declared nested application-error wrappers below.
    pub fn message(self, source: &str) -> Cow<'_, str> {
        let translated = self.text(source);
        if translated != source || self == Self::Traditional {
            return Cow::Borrowed(translated);
        }
        let limited = "；已達限制，結果不完整";
        if let Some(base) = source.strip_suffix(limited) {
            return Cow::Owned(format!("{}{}", self.message(base), self.text(limited)));
        }
        for (prefix, key) in [
            ("開啟失敗：", "開啟失敗：{0}"),
            ("儲存失敗（保留原檔）：", "儲存失敗（保留原檔）：{0}"),
            ("工作階段停用：", "工作階段停用：{0}"),
            (
                "快照保存失敗，前次快照保留：",
                "快照保存失敗，前次快照保留：{0}",
            ),
            ("語系設定未保存：", "語系設定未保存：{0}"),
            ("文字處理失敗：", "文字處理失敗：{0}"),
        ] {
            if let Some(error) = source.strip_prefix(prefix) {
                return Cow::Owned(self.format(key, &[&self.message(error)]));
            }
        }
        for (pattern, row) in message_patterns() {
            if let Some(captures) = pattern.captures(source) {
                let values: Vec<&str> = captures
                    .iter()
                    .skip(1)
                    .map(|m| m.unwrap().as_str())
                    .collect();
                return Cow::Owned(render(row[self as usize], &values));
            }
        }
        Cow::Borrowed(source)
    }
}
fn catalog() -> &'static Vec<[&'static str; 4]> {
    static CATALOG: OnceLock<Vec<[&'static str; 4]>> = OnceLock::new();
    CATALOG.get_or_init(|| {
        include_str!("../locales/catalog.tsv")
            .lines()
            .filter(|line| !line.is_empty() && !line.starts_with('#'))
            .map(|line| {
                line.split('\t')
                    .collect::<Vec<_>>()
                    .try_into()
                    .expect("four locale columns")
            })
            .collect()
    })
}
fn message_patterns() -> &'static Vec<(regex::Regex, &'static [&'static str; 4])> {
    static PATTERNS: OnceLock<Vec<(regex::Regex, &'static [&'static str; 4])>> = OnceLock::new();
    PATTERNS.get_or_init(|| {
        catalog()
            .iter()
            .filter(|row| row[0].contains("{0}"))
            .map(|row| {
                let mut expression = "^".to_owned();
                let mut rest = row[0];
                let mut index = 0;
                while let Some((before, after)) = rest.split_once(&format!("{{{index}}}")) {
                    expression.push_str(&regex::escape(before));
                    expression.push_str("(?s:(.*?))");
                    rest = after;
                    index += 1;
                }
                expression.push_str(&regex::escape(rest));
                expression.push('$');
                (
                    regex::Regex::new(&expression).expect("escaped translation template"),
                    row,
                )
            })
            .collect()
    })
}
fn render(template: &str, values: &[&str]) -> String {
    let mut rendered = String::new();
    let mut rest = template;
    while let Some(start) = rest.find('{') {
        rendered.push_str(&rest[..start]);
        let Some(end) = rest[start..].find('}').map(|end| start + end) else {
            rendered.push_str(&rest[start..]);
            return rendered;
        };
        if let Ok(index) = rest[start + 1..end].parse::<usize>()
            && let Some(value) = values.get(index)
        {
            rendered.push_str(value);
        } else {
            rendered.push_str(&rest[start..=end]);
        }
        rest = &rest[end + 1..];
    }
    rendered.push_str(rest);
    rendered
}

#[derive(Default)]
pub struct Preferences {
    path: Option<PathBuf>,
    expected: Option<Vec<u8>>,
    blocked: bool,
}
fn read(path: &std::path::Path) -> Result<Option<Vec<u8>>, String> {
    let file = match std::fs::File::open(path) {
        Ok(file) => file,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(error) => return Err(error.to_string()),
    };
    let mut bytes = Vec::new();
    file.take(513)
        .read_to_end(&mut bytes)
        .map_err(|e| e.to_string())?;
    if bytes.len() > 512 {
        return Err("語系設定格式錯誤，保留原檔".into());
    }
    Ok(Some(bytes))
}
impl Preferences {
    pub fn load(path: Option<PathBuf>) -> (Self, Locale, Option<String>) {
        let mut preferences = Self {
            path,
            ..Self::default()
        };
        let result: Result<Locale, String> = (|| {
            let Some(path) = &preferences.path else {
                return Ok(Locale::default());
            };
            preferences.expected = read(path)?;
            let Some(bytes) = &preferences.expected else {
                return Ok(Locale::default());
            };
            let text = std::str::from_utf8(bytes).map_err(|_| "語系設定格式錯誤，保留原檔")?;
            let tag = text
                .strip_prefix("InkPage.settings\nversion=1\nlocale=")
                .and_then(|tag| tag.strip_suffix('\n'))
                .ok_or("語系設定格式錯誤，保留原檔")?;
            Locale::parse(tag).ok_or("語系設定格式錯誤，保留原檔".into())
        })();
        match result {
            Ok(locale) => (preferences, locale, None),
            Err(error) => {
                preferences.blocked = true;
                (preferences, Locale::default(), Some(error))
            }
        }
    }
    pub fn save(&mut self, locale: Locale) -> Result<(), String> {
        let Some(path) = &self.path else {
            return Ok(());
        };
        if self.blocked {
            return Err("語系設定格式錯誤，保留原檔".into());
        }
        if read(path)? != self.expected {
            self.blocked = true;
            return Err("語系設定已被外部修改，保留原檔".into());
        }
        let bytes = format!("InkPage.settings\nversion=1\nlocale={}\n", locale.tag()).into_bytes();
        crate::core::atomic_bytes(path, &bytes)?;
        self.expected = Some(bytes);
        Ok(())
    }
}

pub fn from_args(args: &[String]) -> (Preferences, Locale, Option<String>) {
    let argument = |key: &str| {
        args.iter()
            .position(|arg| arg == key)
            .and_then(|index| args.get(index + 1))
    };
    let path = if args.iter().any(|arg| arg == "--no-settings") {
        None
    } else if let Some(path) = argument("--settings") {
        Some(PathBuf::from(path))
    } else if args.iter().any(|arg| arg == "--screenshot") {
        None
    } else {
        std::env::current_exe()
            .ok()
            .map(|path| path.with_file_name("InkPage.settings"))
    };
    let (preferences, mut locale, mut error) = Preferences::load(path);
    if let Some(tag) = argument("--lang") {
        if let Some(selected) = Locale::parse(tag) {
            locale = selected;
        } else {
            error = Some(format!("不支援的介面語言：{tag}"));
        }
    }
    (preferences, locale, error)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn catalog_has_all_languages_and_preserves_every_placeholder() {
        let placeholders = regex::Regex::new(r"\{\d+\}").unwrap();
        let mut keys = std::collections::HashSet::new();
        for row in catalog() {
            assert!(keys.insert(row[0]), "duplicate {}", row[0]);
            let mut expected: Vec<_> = placeholders.find_iter(row[0]).map(|m| m.as_str()).collect();
            expected.sort();
            for translation in row {
                assert!(
                    !translation.trim().is_empty(),
                    "empty translation {}",
                    row[0]
                );
                let mut actual: Vec<_> = placeholders
                    .find_iter(translation)
                    .map(|m| m.as_str())
                    .collect();
                actual.sort();
                assert_eq!(expected, actual, "{}", row[0]);
            }
        }
        for label in crate::textops::LineCommand::ALL
            .map(|command| command.label())
            .into_iter()
            .chain(crate::textops::CommentCommand::ALL.map(|command| command.label()))
        {
            assert!(keys.contains(label), "missing command translation: {label}");
        }
        let lookup = regex::Regex::new(r#"locale\.(?:text|format)\(\s*"([^"]+)""#).unwrap();
        for captures in lookup.captures_iter(include_str!("main.rs")) {
            assert!(
                keys.contains(&captures[1]),
                "missing UI translation: {}",
                &captures[1]
            );
        }
    }
    #[test]
    fn preferences_restart_conflict_and_corrupt_file_are_safe() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("InkPage.settings");
        let (mut settings, locale, error) = Preferences::load(Some(path.clone()));
        assert_eq!(locale, Locale::Traditional);
        assert!(error.is_none());
        settings.save(Locale::Japanese).unwrap();
        let (_, restored, error) = Preferences::load(Some(path.clone()));
        assert_eq!(restored, Locale::Japanese);
        assert!(error.is_none());
        std::fs::write(&path, "external").unwrap();
        assert!(settings.save(Locale::English).is_err());
        let (mut corrupt, _, error) = Preferences::load(Some(path.clone()));
        assert!(error.is_some());
        assert!(corrupt.save(Locale::English).is_err());
        assert_eq!(std::fs::read(&path).unwrap(), b"external");
    }
    #[test]
    fn localization_keeps_opaque_values_and_changes_existing_notices() {
        let literal = "日本語{1} / 中文🙂";
        assert_eq!(
            Locale::English.format("未命名 {0}", &[literal]),
            format!("Untitled {literal}")
        );
        assert_eq!(
            Locale::English.message("已取代 42 處"),
            "Replaced 42 matches"
        );
        assert_eq!(
            Locale::Japanese.message("已取代 42 處"),
            "42 件を置換しました"
        );
        assert_eq!(
            Locale::English.message("開啟失敗：包含 NUL，可能是二進位檔案"),
            "Open failed: Contains NUL; possibly a binary file"
        );
        assert_eq!(Locale::English.message(literal), literal);
        assert_eq!(
            Locale::English.message("文字處理失敗：目前語言未定義註解"),
            "Text operation failed: No comments defined for this language"
        );
        assert_eq!(
            Locale::English.message("搜尋已取消：2 處／1 檔，略過 3；已達限制，結果不完整"),
            "Search cancelled: 2 matches / 1 files, 3 skipped; limit reached, incomplete results"
        );
    }
}
