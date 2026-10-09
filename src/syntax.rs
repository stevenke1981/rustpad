use eframe::egui::{self, Color32, FontId};
use std::{
    collections::HashMap,
    path::Path,
    sync::{
        Arc,
        atomic::{AtomicU64, Ordering},
        mpsc,
    },
    thread,
    time::Instant,
};
use syntect::{
    easy::HighlightLines,
    highlighting::{Color, StyleModifier, Theme, ThemeItem, ThemeSettings},
    parsing::SyntaxSet,
    util::LinesWithEndings,
};

pub const HIGHLIGHT_LIMIT: usize = 256 * 1024;
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Language {
    Plain,
    Rust,
    Python,
    JavaScript,
    TypeScript,
    Tsx,
    Html,
    Css,
    Json,
    Yaml,
    Toml,
    Markdown,
    C,
    Cpp,
    Shell,
    PowerShell,
}
impl Language {
    pub const ALL: [Self; 16] = [
        Self::Plain,
        Self::Rust,
        Self::Python,
        Self::JavaScript,
        Self::TypeScript,
        Self::Tsx,
        Self::Html,
        Self::Css,
        Self::Json,
        Self::Yaml,
        Self::Toml,
        Self::Markdown,
        Self::C,
        Self::Cpp,
        Self::Shell,
        Self::PowerShell,
    ];
    pub fn label(self) -> &'static str {
        match self {
            Self::Plain => "純文字",
            Self::Rust => "Rust",
            Self::Python => "Python",
            Self::JavaScript => "JavaScript",
            Self::TypeScript => "TypeScript",
            Self::Tsx => "TypeScript JSX",
            Self::Html => "HTML",
            Self::Css => "CSS",
            Self::Json => "JSON",
            Self::Yaml => "YAML",
            Self::Toml => "TOML",
            Self::Markdown => "Markdown",
            Self::C => "C",
            Self::Cpp => "C++",
            Self::Shell => "Shell",
            Self::PowerShell => "PowerShell",
        }
    }
    fn extension(self) -> &'static str {
        match self {
            Self::Plain => "txt",
            Self::Rust => "rs",
            Self::Python => "py",
            Self::JavaScript => "js",
            Self::TypeScript => "ts",
            Self::Tsx => "tsx",
            Self::Html => "html",
            Self::Css => "css",
            Self::Json => "json",
            Self::Yaml => "yaml",
            Self::Toml => "toml",
            Self::Markdown => "md",
            Self::C => "c",
            Self::Cpp => "cpp",
            Self::Shell => "sh",
            Self::PowerShell => "ps1",
        }
    }
}
pub fn resolve(path: Option<&Path>, text: &str, manual: Option<Language>) -> Language {
    if let Some(language) = manual {
        return language;
    }
    if let Some(ext) = path.and_then(Path::extension).and_then(|ext| ext.to_str()) {
        match ext.to_ascii_lowercase().as_str() {
            "rs" => return Language::Rust,
            "py" | "pyw" => return Language::Python,
            "js" | "mjs" | "cjs" => return Language::JavaScript,
            "ts" | "mts" | "cts" => return Language::TypeScript,
            "tsx" => return Language::Tsx,
            "html" | "htm" => return Language::Html,
            "css" => return Language::Css,
            "json" => return Language::Json,
            "yaml" | "yml" => return Language::Yaml,
            "toml" => return Language::Toml,
            "md" | "markdown" => return Language::Markdown,
            "c" | "h" => return Language::C,
            "cc" | "cpp" | "cxx" | "hpp" | "hxx" => return Language::Cpp,
            "sh" | "bash" | "zsh" => return Language::Shell,
            "ps1" | "psm1" | "psd1" => return Language::PowerShell,
            _ => {}
        }
    }
    let first = text
        .lines()
        .next()
        .unwrap_or("")
        .trim_start_matches('\u{feff}');
    if let Some(shebang) = first.strip_prefix("#!") {
        let words: Vec<_> = shebang.split_whitespace().collect();
        for word in words {
            let name = word.rsplit('/').next().unwrap_or(word);
            if name.starts_with("python") {
                return Language::Python;
            }
            if ["bash", "sh", "zsh", "dash"].contains(&name) {
                return Language::Shell;
            }
            if ["node", "nodejs"].contains(&name) {
                return Language::JavaScript;
            }
            if ["pwsh", "powershell"].contains(&name) {
                return Language::PowerShell;
            }
        }
    }
    Language::Plain
}
pub fn normal(dark: bool) -> Color32 {
    if dark {
        Color32::from_rgb(215, 221, 232)
    } else {
        Color32::from_rgb(30, 38, 48)
    }
}
fn rgb(color: Color32) -> Color {
    let [r, g, b, a] = color.to_array();
    Color { r, g, b, a }
}
fn theme(dark: bool) -> Theme {
    let colors = if dark {
        [
            (135, 159, 128),
            (226, 169, 126),
            (126, 178, 255),
            (205, 145, 224),
            (94, 196, 203),
            (214, 153, 198),
            (107, 190, 224),
        ]
    } else {
        [
            (80, 112, 85),
            (145, 63, 32),
            (26, 77, 172),
            (131, 54, 160),
            (0, 108, 117),
            (135, 53, 110),
            (0, 101, 148),
        ]
    };
    let scopes = [
        "comment",
        "string",
        "keyword, storage",
        "constant.numeric, constant.language",
        "entity.name, support.type",
        "entity.name.tag, meta.tag",
        "markup.heading, markup.bold, markup.underline.link",
    ];
    Theme {
        settings: ThemeSettings {
            foreground: Some(rgb(normal(dark))),
            ..Default::default()
        },
        scopes: scopes
            .iter()
            .zip(colors)
            .map(|(scope, (r, g, b))| ThemeItem {
                scope: scope.parse().expect("fixed scope selector"),
                style: StyleModifier {
                    foreground: Some(Color { r, g, b, a: 255 }),
                    ..Default::default()
                },
            })
            .collect(),
        ..Default::default()
    }
}
#[derive(Clone, Debug)]
pub struct Span {
    pub start: usize,
    pub end: usize,
    pub color: Color32,
}
pub struct Engine {
    syntaxes: SyntaxSet,
    light: Theme,
    dark: Theme,
}
impl Engine {
    pub fn code_mask(&self, text: &str, language: Language) -> Result<Vec<bool>, String> {
        use syntect::parsing::{ParseState, Scope, ScopeStack};
        let mut mask = vec![true; text.len()];
        if language == Language::Plain {
            return Ok(mask);
        }
        let grammar = self
            .syntaxes
            .find_syntax_by_extension(language.extension())
            .ok_or("找不到語法規則")?;
        let mut parser = ParseState::new(grammar);
        let mut stack = ScopeStack::new();
        let string = Scope::new("string").map_err(|e| e.to_string())?;
        let comment = Scope::new("comment").map_err(|e| e.to_string())?;
        let mut base = 0;
        for line in LinesWithEndings::from(text) {
            let mut previous = 0;
            for (position, operation) in parser
                .parse_line(line, &self.syntaxes)
                .map_err(|e| e.to_string())?
            {
                let code = !stack
                    .scopes
                    .iter()
                    .any(|scope| string.is_prefix_of(*scope) || comment.is_prefix_of(*scope));
                mask[base + previous..base + position].fill(code);
                stack.apply(&operation).map_err(|e| e.to_string())?;
                previous = position;
            }
            let code = !stack
                .scopes
                .iter()
                .any(|scope| string.is_prefix_of(*scope) || comment.is_prefix_of(*scope));
            mask[base + previous..base + line.len()].fill(code);
            base += line.len();
        }
        Ok(mask)
    }
    pub fn new() -> Self {
        Self {
            syntaxes: two_face::syntax::extra_newlines(),
            light: theme(false),
            dark: theme(true),
        }
    }
    pub fn supported(&self, language: Language) -> bool {
        language == Language::Plain
            || self
                .syntaxes
                .find_syntax_by_extension(language.extension())
                .is_some()
    }
    pub fn spans(
        &self,
        text: &str,
        language: Language,
        dark: bool,
        cancel: impl Fn() -> bool,
    ) -> Result<Vec<Span>, String> {
        if !self.supported(language) {
            return Err(format!("{} grammar 未提供，改為純文字", language.label()));
        }
        if language == Language::Plain || text.len() > HIGHLIGHT_LIMIT {
            return Ok(vec![Span {
                start: 0,
                end: text.len(),
                color: normal(dark),
            }]);
        }
        let grammar = self
            .syntaxes
            .find_syntax_by_extension(language.extension())
            .ok_or_else(|| format!("{} grammar 未提供，改為純文字", language.label()))?;
        let mut parser = HighlightLines::new(grammar, if dark { &self.dark } else { &self.light });
        let mut spans = Vec::<Span>::new();
        let mut offset = 0;
        for line in LinesWithEndings::from(text) {
            if cancel() {
                return Err("已被較新版本取代".into());
            }
            for (style, part) in parser
                .highlight_line(line, &self.syntaxes)
                .map_err(|error| error.to_string())?
            {
                let end = offset + part.len();
                let color = Color32::from_rgba_unmultiplied(
                    style.foreground.r,
                    style.foreground.g,
                    style.foreground.b,
                    style.foreground.a,
                );
                if let Some(previous) = spans.last_mut()
                    && previous.end == offset
                    && previous.color == color
                {
                    previous.end = end;
                } else {
                    spans.push(Span {
                        start: offset,
                        end,
                        color,
                    });
                }
                offset = end;
            }
        }
        if offset != text.len() {
            return Err("grammar span 未涵蓋完整原文".into());
        }
        Ok(spans)
    }
}
#[derive(Clone, PartialEq, Eq)]
struct RequestKey {
    doc: u64,
    text: String,
    language: Language,
    dark: bool,
}
struct Request {
    ticket: u64,
    key: RequestKey,
}
struct Parsed {
    ticket: u64,
    key: RequestKey,
    spans: Vec<Span>,
    error: Option<String>,
}
#[derive(Default)]
pub struct LayoutCache {
    key: Option<LayoutKey>,
    job: egui::text::LayoutJob,
}
#[derive(PartialEq, Eq)]
struct LayoutKey {
    text: String,
    language: Language,
    dark: bool,
    tabs: usize,
    height: u32,
    dpi: u32,
    space: u32,
    parsed: u64,
}
pub struct Service {
    tx: mpsc::Sender<Request>,
    rx: mpsc::Receiver<Parsed>,
    ticket: Arc<AtomicU64>,
    last: Option<RequestKey>,
    results: HashMap<u64, Parsed>,
}
impl Service {
    pub fn new(ctx: egui::Context) -> Self {
        let (tx, requests) = mpsc::channel::<Request>();
        let (results, rx) = mpsc::channel();
        let ticket = Arc::new(AtomicU64::new(0));
        let worker_ticket = ticket.clone();
        thread::spawn(move || {
            let engine = Engine::new();
            while let Ok(mut request) = requests.recv() {
                while let Ok(newer) = requests.try_recv() {
                    request = newer;
                }
                let started = Instant::now();
                let spans = engine.spans(
                    &request.key.text,
                    request.key.language,
                    request.key.dark,
                    || worker_ticket.load(Ordering::Relaxed) != request.ticket,
                );
                if worker_ticket.load(Ordering::Relaxed) != request.ticket {
                    continue;
                }
                let (spans, error) = match spans {
                    Ok(spans) => (spans, None),
                    Err(error) => (
                        vec![Span {
                            start: 0,
                            end: request.key.text.len(),
                            color: normal(request.key.dark),
                        }],
                        Some(error),
                    ),
                };
                let _ = started;
                let _ = results.send(Parsed {
                    ticket: request.ticket,
                    key: request.key,
                    spans,
                    error,
                });
                ctx.request_repaint();
            }
        });
        Self {
            tx,
            rx,
            ticket,
            last: None,
            results: HashMap::new(),
        }
    }
    pub fn poll(&mut self) {
        while let Ok(result) = self.rx.try_recv() {
            if result.ticket == self.ticket.load(Ordering::Relaxed) {
                self.results.insert(result.key.doc, result);
            }
        }
    }
    pub fn retain(&mut self, ids: &[u64]) {
        self.results.retain(|id, _| ids.contains(id));
    }
    pub fn status(&self, id: u64, text: &str, language: Language, dark: bool) -> &'static str {
        if text.len() > HIGHLIGHT_LIMIT {
            return "（>256 KiB 純文）";
        }
        if language == Language::Plain {
            return "";
        }
        if let Some(result) = self.results.get(&id)
            && result.key.text == text
            && result.key.language == language
            && result.key.dark == dark
        {
            if result.error.is_some() {
                "（grammar 不支援／錯誤：純文）"
            } else {
                ""
            }
        } else {
            "（背景配色中）"
        }
    }
    #[allow(clippy::too_many_arguments)]
    pub fn layout(
        &mut self,
        id: u64,
        text: &str,
        language: Language,
        dark: bool,
        tabs: usize,
        height: f32,
        dpi: f32,
        space: f32,
        cache: &mut LayoutCache,
    ) -> egui::text::LayoutJob {
        let ready = self.results.get(&id).filter(|result| {
            result.key.text == text && result.key.language == language && result.key.dark == dark
        });
        let parsed = ready.map_or(0, |result| result.ticket);
        let same = cache.key.as_ref().is_some_and(|key| {
            key.text == text
                && key.language == language
                && key.dark == dark
                && key.tabs == tabs
                && key.height == height.to_bits()
                && key.dpi == dpi.to_bits()
                && key.space == space.to_bits()
                && key.parsed == parsed
        });
        if !same {
            let fallback = [Span {
                start: 0,
                end: text.len(),
                color: normal(dark),
            }];
            cache.job = layout(
                text,
                ready.map_or(&fallback[..], |result| result.spans.as_slice()),
                tabs,
                height,
                space,
            );
            cache.key = Some(LayoutKey {
                text: text.to_owned(),
                language,
                dark,
                tabs,
                height: height.to_bits(),
                dpi: dpi.to_bits(),
                space: space.to_bits(),
                parsed,
            });
        }
        if ready.is_none()
            && language != Language::Plain
            && text.len() <= HIGHLIGHT_LIMIT
            && !self.last.as_ref().is_some_and(|key| {
                key.doc == id && key.text == text && key.language == language && key.dark == dark
            })
        {
            let key = RequestKey {
                doc: id,
                text: text.to_owned(),
                language,
                dark,
            };
            let ticket = self.ticket.fetch_add(1, Ordering::Relaxed) + 1;
            let _ = self.tx.send(Request {
                ticket,
                key: key.clone(),
            });
            self.last = Some(key);
        }
        cache.job.clone()
    }
}
pub fn layout(
    text: &str,
    spans: &[Span],
    tabs: usize,
    height: f32,
    space: f32,
) -> egui::text::LayoutJob {
    let mut job = egui::text::LayoutJob::default();
    let mut column = 0;
    for span in spans {
        let part = &text[span.start..span.end];
        let mut start = 0;
        for (index, ch) in part.char_indices() {
            match ch {
                '\t' => {
                    if start < index {
                        job.append(
                            &part[start..index],
                            0.,
                            egui::TextFormat {
                                font_id: FontId::monospace(15.),
                                line_height: Some(height),
                                color: span.color,
                                ..Default::default()
                            },
                        );
                    }
                    let width = crate::core::tab_advance(column, tabs);
                    job.append(
                        "\t",
                        0.,
                        egui::TextFormat {
                            font_id: FontId::monospace(15. * width as f32 / 4.),
                            line_height: Some(height),
                            color: span.color,
                            ..Default::default()
                        },
                    );
                    column += width;
                    start = index + 1;
                }
                '\n' => column = 0,
                _ => column += unicode_width::UnicodeWidthChar::width(ch).unwrap_or(0),
            }
        }
        if start < part.len() {
            job.append(
                &part[start..],
                0.,
                egui::TextFormat {
                    font_id: FontId::monospace(15.),
                    line_height: Some(height),
                    color: span.color,
                    ..Default::default()
                },
            );
        }
    }
    let _ = space;
    job.wrap.max_width = f32::INFINITY;
    debug_assert_eq!(job.text, text);
    job
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn detect_extension_shebang_and_manual() {
        assert_eq!(
            resolve(Some(Path::new("a.TS")), "", None),
            Language::TypeScript
        );
        assert_eq!(
            resolve(
                Some(Path::new("script")),
                "#!/usr/bin/env -S python3 -u\n",
                None
            ),
            Language::Python
        );
        assert_eq!(resolve(None, "#!/bin/bash\n", None), Language::Shell);
        assert_eq!(
            resolve(Some(Path::new("a.rs")), "", Some(Language::Plain)),
            Language::Plain
        );
        assert_eq!(
            resolve(Some(Path::new("a.unknown")), "", None),
            Language::Plain
        );
    }
    #[test]
    fn grammars_and_multiline_unicode_are_real() {
        let engine = Engine::new();
        for language in Language::ALL {
            assert!(engine.supported(language), "missing {}", language.label());
        }
        for (language, text) in [
            (
                Language::Rust,
                "/* 中文🙂\n still comment */\nlet n=42;\nlet s=\"字\";",
            ),
            (Language::Python, "\"\"\"中文\n字🙂\"\"\"\n# comment\nx=42"),
            (Language::JavaScript, "/* 中文\ncomment */\nconst n=42;"),
            (Language::TypeScript, "const name: string = '中文';\n"),
            (
                Language::Html,
                "<!--中文\ncomment-->\n<p title=\"🙂\">字</p>",
            ),
            (Language::Css, "/*中文\ncomment*/\n.box {color: #123abc;}"),
            (Language::Json, "{\"字\":42,\"ok\":true}"),
            (Language::Yaml, "# 字\nvalue: 42\n"),
            (Language::Toml, "# 字\n[section]\nvalue=42"),
            (Language::Markdown, "# 中文\n**strong** `字`"),
            (Language::C, "/*字\ncomment*/\nint n=42;"),
            (Language::Cpp, "//字\nstd::string s=\"中文\";"),
            (Language::Shell, "#字\necho \"中文\"\n"),
            (Language::PowerShell, "#字\n$value = \"中文\"\n"),
        ] {
            for dark in [false, true] {
                let spans = engine.spans(text, language, dark, || false).unwrap();
                assert!(spans.iter().all(
                    |span| text.is_char_boundary(span.start) && text.is_char_boundary(span.end)
                ));
                let job = layout(text, &spans, 4, 18., 9.);
                assert_eq!(job.text.as_bytes(), text.as_bytes());
                assert!(
                    spans.iter().any(|span| span.color != normal(dark)),
                    "no real color {}",
                    language.label()
                );
            }
        }
    }
    #[test]
    fn multiline_scope_state_is_not_reset() {
        let engine = Engine::new();
        let text = "/* start\n中文🙂 comment\n*/\nlet n=42;";
        let spans = engine.spans(text, Language::Rust, false, || false).unwrap();
        let color_at = |byte| {
            spans
                .iter()
                .find(|s| s.start <= byte && byte < s.end)
                .unwrap()
                .color
        };
        assert_eq!(
            color_at(text.find("start").unwrap()),
            color_at(text.find("中文").unwrap())
        );
        assert_ne!(
            color_at(text.find("let").unwrap()),
            color_at(text.find("中文").unwrap())
        );
    }
    #[test]
    fn edits_propagate_multiline_state_and_limit_falls_back() {
        let engine = Engine::new();
        let before = "/* outer /* nested */\n中文 comment */\nlet n=42;";
        let after = before.replace("comment */", "comment");
        let color_at = |text: &str, spans: &[Span], word: &str| {
            let byte = text.rfind(word).unwrap();
            spans
                .iter()
                .find(|span| span.start <= byte && byte < span.end)
                .unwrap()
                .color
        };
        let old = engine
            .spans(before, Language::Rust, false, || false)
            .unwrap();
        let new = engine
            .spans(&after, Language::Rust, false, || false)
            .unwrap();
        assert_ne!(color_at(before, &old, "let"), color_at(&after, &new, "let"));
        let large = "x".repeat(HIGHLIGHT_LIMIT + 1);
        let spans = engine
            .spans(&large, Language::Rust, false, || false)
            .unwrap();
        assert_eq!(spans.len(), 1);
        assert_eq!(spans[0].color, normal(false));
        assert_eq!(layout(&large, &spans, 4, 18., 9.).text, large);
    }
    #[test]
    fn layout_invalidation_language_theme_tabs_font_and_text() {
        let ctx = egui::Context::default();
        let mut service = Service::new(ctx);
        let mut cache = LayoutCache::default();
        let text = "let n=42;\t\n";
        service.layout(1, text, Language::Plain, false, 4, 18., 1., 9., &mut cache);
        let first = cache.key.take().unwrap();
        service.layout(1, text, Language::Rust, false, 4, 18., 1., 9., &mut cache);
        assert!(first != *cache.key.as_ref().unwrap());
        let rust = cache.key.take().unwrap();
        service.layout(1, text, Language::Rust, true, 8, 20., 1.25, 10., &mut cache);
        assert!(rust != *cache.key.as_ref().unwrap());
        service.layout(
            2,
            text,
            Language::Python,
            true,
            8,
            20.,
            1.25,
            10.,
            &mut cache,
        );
        assert_eq!(cache.job.text, text);
    }
}
