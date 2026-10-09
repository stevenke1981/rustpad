#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
mod actions;
mod core;
mod perf;
mod syntax;
use core::{Content, History, Newline, WhitespaceEdit};
use eframe::egui::{
    self, Color32, FontId, Key, Modifiers,
    text::{CCursor, CCursorRange},
};
use std::{path::PathBuf, sync::mpsc, time::Duration};
fn history_shortcuts(input: &mut egui::InputState, editor_focused: bool) -> (bool, bool) {
    if !editor_focused {
        return (false, false);
    }
    let undo = input.consume_key(Modifiers::CTRL, Key::Z);
    let redo = input.consume_key(Modifiers::CTRL, Key::Y)
        || input.consume_key(Modifiers::CTRL | Modifiers::SHIFT, Key::Z);
    (undo, redo)
}

struct Document {
    id: u64,
    path: Option<PathBuf>,
    content: Content,
    saved: Content,
    history: History,
    cursor: usize,
    selected: (usize, usize),
    highlight_cache: syntax::LayoutCache,
    language: Option<syntax::Language>,
}
impl Document {
    fn new(id: u64, path: Option<PathBuf>, content: Content) -> Self {
        Self {
            id,
            path,
            saved: content.clone(),
            content,
            history: History::default(),
            cursor: 0,
            selected: (0, 0),
            highlight_cache: Default::default(),
            language: None,
        }
    }
    fn dirty(&self) -> bool {
        self.content != self.saved
    }
    fn title(&self) -> String {
        format!(
            "{}{}",
            if self.dirty() { "● " } else { "" },
            self.path
                .as_ref()
                .and_then(|p| p.file_name())
                .map(|n| n.to_string_lossy().into_owned())
                .unwrap_or_else(|| format!("未命名 {}", self.id))
        )
    }
}
enum ResultEvent {
    Open(Result<Option<(PathBuf, Content)>, String>),
    Save(u64, Content, Result<Option<PathBuf>, String>, bool),
    Action(
        u64,
        Content,
        String,
        String,
        actions::SearchOptions,
        actions::Task,
        actions::Outcome,
    ),
}
struct App {
    docs: Vec<Document>,
    active: usize,
    next_id: u64,
    tx: mpsc::Sender<ResultEvent>,
    rx: mpsc::Receiver<ResultEvent>,
    busy: bool,
    message: String,
    dark: bool,
    search: bool,
    query: String,
    replacement: String,
    selection: Option<(usize, usize)>,
    pending_close: Option<u64>,
    exit: bool,
    force_exit: bool,
    match_range: Option<(u64, (usize, usize))>,
    match_signature: Option<(String, actions::SearchOptions)>,
    capture: Option<PathBuf>,
    capture_frame: usize,
    show_spaces: bool,
    show_eol: bool,
    tab_width: usize,
    insert_spaces: bool,
    syntax: syntax::Service,
    startup_report: Option<PathBuf>,
    search_options: actions::SearchOptions,
    in_selection: bool,
    goto_open: bool,
    goto_input: String,
    repaint: egui::Context,
}
impl App {
    fn new(cc: &eframe::CreationContext<'_>) -> Self {
        let mut fonts = egui::FontDefinitions::default();
        for path in [
            "C:/Windows/Fonts/msjh.ttc",
            "/usr/share/fonts/opentype/noto/NotoSansCJK-Regular.ttc",
            "/usr/share/fonts/truetype/wqy/wqy-zenhei.ttc",
        ] {
            if let Ok(bytes) = std::fs::read(path) {
                fonts
                    .font_data
                    .insert("cjk".into(), egui::FontData::from_owned(bytes).into());
                for family in [egui::FontFamily::Proportional, egui::FontFamily::Monospace] {
                    fonts.families.entry(family).or_default().push("cjk".into());
                }
                break;
            }
        }
        if let Ok(bytes) = std::fs::read("C:/Windows/Fonts/seguiemj.ttf") {
            fonts
                .font_data
                .insert("emoji".into(), egui::FontData::from_owned(bytes).into());
            for family in [egui::FontFamily::Proportional, egui::FontFamily::Monospace] {
                fonts
                    .families
                    .entry(family)
                    .or_default()
                    .push("emoji".into());
            }
        }
        cc.egui_ctx.set_fonts(fonts);
        cc.egui_ctx.set_visuals(egui::Visuals::light());
        let (tx, rx) = mpsc::channel();
        Self {
            docs: vec![Document::new(1, None, Content::default())],
            active: 0,
            next_id: 2,
            tx,
            rx,
            busy: false,
            message: "就緒 · UTF-8 編輯器 · 2 MiB 上限".into(),
            dark: false,
            search: false,
            query: String::new(),
            replacement: String::new(),
            selection: None,
            pending_close: None,
            exit: false,
            force_exit: false,
            match_range: None,
            match_signature: None,
            capture: None,
            capture_frame: 0,
            show_spaces: false,
            show_eol: false,
            tab_width: 4,
            insert_spaces: false,
            syntax: syntax::Service::new(cc.egui_ctx.clone()),
            startup_report: None,
            search_options: Default::default(),
            in_selection: false,
            goto_open: false,
            goto_input: "1".into(),
            repaint: cc.egui_ctx.clone(),
        }
    }
    fn new_doc(&mut self) {
        self.docs
            .push(Document::new(self.next_id, None, Content::default()));
        self.next_id += 1;
        self.activate(self.docs.len() - 1);
    }
    fn open(&mut self, ctx: &egui::Context) {
        if self.busy {
            return;
        }
        self.busy = true;
        let tx = self.tx.clone();
        let ctx = ctx.clone();
        std::thread::spawn(move || {
            let result = rfd::FileDialog::new()
                .pick_file()
                .map(|p| core::read_file(&p).map(|c| (p, c)))
                .transpose();
            let _ = tx.send(ResultEvent::Open(result));
            ctx.request_repaint();
        });
    }
    fn save(&mut self, ctx: &egui::Context, save_as: bool, close: bool) {
        if self.busy {
            return;
        }
        let d = &self.docs[self.active];
        let id = d.id;
        let content = d.content.clone();
        let path = if save_as { None } else { d.path.clone() };
        let expected = d.saved.clone();
        let original_path = path.clone();
        let tx = self.tx.clone();
        let ctx = ctx.clone();
        self.busy = true;
        std::thread::spawn(move || {
            let result = (|| {
                let p = path.or_else(|| {
                    rfd::FileDialog::new()
                        .set_file_name("untitled.txt")
                        .save_file()
                });
                if let Some(p) = p {
                    if original_path.as_ref() == Some(&p)
                        && core::encode(&core::read_file(&p)?)? != core::encode(&expected)?
                    {
                        return Err("磁碟內容已被外部修改；請另存新檔避免覆寫".into());
                    }
                    core::atomic_save(&p, &content)?;
                    Ok(Some(p))
                } else {
                    Ok(None)
                }
            })();
            let _ = tx.send(ResultEvent::Save(id, content, result, close));
            ctx.request_repaint();
        });
    }
    fn request_close(&mut self, id: u64) {
        if self.busy {
            self.message = "請等待檔案操作完成再關閉分頁".into();
            return;
        }
        if let Some(d) = self.docs.iter().find(|d| d.id == id) {
            if core::may_close(&d.content, &d.saved, false) {
                self.remove(id)
            } else {
                self.pending_close = Some(id)
            }
        }
    }
    fn remove(&mut self, id: u64) {
        self.docs.retain(|d| d.id != id);
        if self.docs.is_empty() {
            self.new_doc()
        }
        self.active = self.active.min(self.docs.len() - 1);
        self.pending_close = None;
    }
    fn activate(&mut self, index: usize) {
        if index < self.docs.len() {
            self.active = index;
            self.selection = None;
            self.match_range = None;
            self.in_selection = false;
        }
    }
    fn find_next(&mut self) {
        self.find_direction(false);
    }
    fn maybe_background(&mut self, task: actions::Task) -> bool {
        if self.busy {
            self.message = "請等待目前背景工作完成".into();
            return true;
        }
        if self.query.len() > 16_384 {
            self.message = "搜尋字串超過 16 KiB 上限".into();
            self.match_range = None;
            return true;
        }
        let d = &self.docs[self.active];
        if d.content.text.len() <= 256 * 1024 && self.query.len() <= 128 {
            return false;
        }
        let (id, before, query, replacement, options) = (
            d.id,
            d.content.clone(),
            self.query.clone(),
            self.replacement.clone(),
            self.search_options,
        );
        let tx = self.tx.clone();
        let ctx = self.repaint.clone();
        self.busy = true;
        self.match_range = None;
        self.match_signature = None;
        self.message = "背景搜尋／取代中；可繼續編輯，過期結果會捨棄".into();
        std::thread::spawn(move || {
            let outcome = actions::execute(&before, &query, &replacement, options, task);
            let _ = tx.send(ResultEvent::Action(
                id,
                before,
                query,
                replacement,
                options,
                task,
                outcome,
            ));
            ctx.request_repaint();
        });
        true
    }
    fn find_direction(&mut self, backward: bool) {
        let d = &self.docs[self.active];
        let from = if backward {
            d.selected.0.min(d.cursor)
        } else {
            d.cursor
        };
        if self.maybe_background(actions::Task::Find { from, backward }) {
            return;
        }
        let d = &self.docs[self.active];
        self.selection = actions::next_match(
            &d.content.text,
            &self.query,
            from,
            self.search_options,
            backward,
        );
        self.match_range = self.selection.map(|range| (d.id, range));
        self.match_signature = self
            .selection
            .map(|_| (self.query.clone(), self.search_options));
        if let Some(range) = self.selection {
            self.docs[self.active].cursor = range.1;
            self.docs[self.active].selected = range;
        }
        self.message = if self.selection.is_some() {
            "已選取符合文字"
        } else {
            "找不到符合文字"
        }
        .into();
    }
    fn replace_one(&mut self) {
        if self.busy {
            self.message = "請等待目前背景工作完成".into();
            return;
        }
        if let Some((id, range)) = self.match_range
            && id == self.docs[self.active].id
            && self
                .match_signature
                .as_ref()
                .is_some_and(|(q, o)| q == &self.query && o == &self.search_options)
            && (self.docs[self.active].content.text.len() > 256 * 1024 || self.query.len() > 128)
        {
            self.maybe_background(actions::Task::ReplaceOne { range });
            return;
        }
        let signature = self.match_signature.take();
        let d = &mut self.docs[self.active];
        if let Some((id, range)) = self.match_range.take()
            && id == d.id
            && signature.is_some_and(|(query, options)| {
                query == self.query && options == self.search_options
            })
            && actions::next_match(
                &d.content.text,
                &self.query,
                range.0,
                actions::SearchOptions {
                    wrap: false,
                    ..self.search_options
                },
                false,
            ) == Some(range)
        {
            let before = d.content.clone();
            match actions::replace_current(
                &mut d.content,
                &self.query,
                &self.replacement,
                self.search_options,
                range,
            ) {
                Ok(_) => {
                    if before != d.content {
                        d.history.record(before);
                    }
                    d.cursor = range.0 + self.replacement.chars().count();
                    self.selection = Some((d.cursor, d.cursor));
                    self.message = "已取代此項".into();
                }
                Err(error) => self.message = error,
            }
        }
    }
    fn replace_all(&mut self) {
        let d = &mut self.docs[self.active];
        let range = self.in_selection.then_some(d.selected);
        if range.is_some_and(|r| r.0 == r.1) {
            self.message = "請先選取文字；未取代".into();
            return;
        }
        if self.maybe_background(actions::Task::Replace { range }) {
            return;
        }
        let d = &mut self.docs[self.active];
        let before = d.content.clone();
        match actions::replace_matches(
            &mut d.content,
            &self.query,
            &self.replacement,
            self.search_options,
            range,
        ) {
            Ok(count) => {
                if before != d.content {
                    d.history.record(before);
                }
                d.cursor = d.cursor.min(d.content.text.chars().count());
                self.selection = Some((d.cursor, d.cursor));
                self.match_range = None;
                self.message = format!("已取代 {count} 處");
            }
            Err(error) => self.message = error,
        }
    }
    fn count_matches(&mut self) {
        let range = self.in_selection.then_some(self.docs[self.active].selected);
        if self.maybe_background(actions::Task::Count { range }) {
            return;
        }
        let d = &self.docs[self.active];
        let ranges = actions::matches(&d.content.text, &self.query, self.search_options);
        let count = ranges
            .iter()
            .filter(|r| !self.in_selection || (r.0 >= d.selected.0 && r.1 <= d.selected.1))
            .count();
        self.message = format!(
            "{}共有 {count} 處",
            if self.in_selection {
                "選取範圍"
            } else {
                "文件"
            }
        );
    }
    fn go_to_line(&mut self) {
        let d = &mut self.docs[self.active];
        if let Ok(line) = self.goto_input.trim().parse::<usize>()
            && let Some(index) = actions::goto_line(&d.content.text, line)
        {
            d.cursor = index;
            d.selected = (index, index);
            self.selection = Some((index, index));
            self.match_range = None;
            self.goto_open = false;
            self.message = format!("已跳至第 {line} 行");
        } else {
            self.message = "行號超出範圍，游標未移動".into();
        }
    }
    fn whitespace_edit(&mut self, action: WhitespaceEdit, selected: bool) {
        let d = &mut self.docs[self.active];
        let before = d.content.clone();
        let range = selected.then_some(d.selected);
        if range.is_some_and(|(start, end)| start == end) {
            self.message = "請先選取文字".into();
            return;
        }
        match core::edit_whitespace(&mut d.content, range, action) {
            Ok(true) => {
                d.history.record(before);
                d.cursor = d.cursor.min(d.content.text.chars().count());
                self.selection = Some((d.cursor, d.cursor));
                self.match_range = None;
                self.message = "空白處理已完成；Ctrl+Z 可復原".into();
            }
            Ok(false) => self.message = "內容無需變更".into(),
            Err(error) => self.message = error,
        }
    }
    fn edit_action(&mut self, action: actions::Edit) {
        let d = &mut self.docs[self.active];
        let before = d.content.clone();
        match actions::edit(&mut d.content, d.selected, d.cursor, action) {
            Ok(range) => {
                if before != d.content {
                    d.history.record(before);
                }
                d.cursor = range.1;
                d.selected = range;
                self.selection = Some(range);
                self.match_range = None;
                self.message = "編輯完成；Ctrl+Z 可復原".into();
            }
            Err(error) => self.message = error,
        }
    }
    fn poll(&mut self) {
        while let Ok(event) = self.rx.try_recv() {
            self.busy = false;
            match event {
                ResultEvent::Action(id, before, query, replacement, options, task, outcome) => {
                    let d = &mut self.docs[self.active];
                    if d.id != id
                        || d.content != before
                        || self.query != query
                        || self.search_options != options
                        || (matches!(outcome, actions::Outcome::Replaced(_))
                            && self.replacement != replacement)
                    {
                        self.message = "背景結果已過期，文件未修改；請重新操作".into();
                        continue;
                    }
                    match outcome {
                        actions::Outcome::Found(range) => {
                            self.selection = range;
                            self.match_range = range.map(|r| (id, r));
                            self.match_signature = range.map(|_| (query, options));
                            if let Some(r) = range {
                                d.cursor = r.1;
                                d.selected = r;
                            }
                            self.message = if range.is_some() {
                                "已選取符合文字"
                            } else {
                                "找不到符合文字"
                            }
                            .into();
                        }
                        actions::Outcome::Count(count) => {
                            self.message = format!("背景計數：{count} 處")
                        }
                        actions::Outcome::Replaced(result) => match result {
                            Ok((after, count)) => {
                                if before != after {
                                    d.history.record(before);
                                    d.content = after;
                                }
                                d.cursor = if let actions::Task::ReplaceOne { range } = task {
                                    range.0 + replacement.chars().count()
                                } else {
                                    d.cursor.min(d.content.text.chars().count())
                                };
                                self.selection = Some((d.cursor, d.cursor));
                                self.match_range = None;
                                self.message = format!("背景取代完成：{count} 處");
                            }
                            Err(error) => self.message = error,
                        },
                    }
                }
                ResultEvent::Open(Ok(Some((path, c)))) => {
                    if let Some(index) = self
                        .docs
                        .iter()
                        .position(|d| d.path.as_ref() == Some(&path))
                    {
                        self.active = index
                    } else {
                        self.docs.push(Document::new(self.next_id, Some(path), c));
                        self.next_id += 1;
                        self.active = self.docs.len() - 1;
                    }
                    self.message = "已開啟".into();
                }
                ResultEvent::Open(Ok(None)) => {}
                ResultEvent::Open(Err(e)) => self.message = format!("開啟失敗：{e}"),
                ResultEvent::Save(id, c, Ok(Some(path)), close) => {
                    if let Some(d) = self.docs.iter_mut().find(|d| d.id == id) {
                        d.path = Some(path);
                        d.saved = c;
                        let clean = !d.dirty();
                        if close && clean {
                            self.remove(id);
                        }
                    }
                    self.message = "已安全儲存".into();
                }
                ResultEvent::Save(_, _, Ok(None), _) => self.message = "已取消儲存".into(),
                ResultEvent::Save(_, _, Err(e), _) => {
                    self.message = format!("儲存失敗（保留原檔）：{e}")
                }
            }
        }
    }
}
impl eframe::App for App {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        ctx.set_visuals(if self.dark {
            egui::Visuals::dark()
        } else {
            egui::Visuals::light()
        });
        if let Some(path) = self.capture.clone() {
            self.capture_frame += 1;
            ctx.request_repaint();
            if self.capture_frame == if self.startup_report.is_some() { 1 } else { 4 } {
                let doc = &self.docs[self.active];
                let language =
                    syntax::resolve(doc.path.as_deref(), &doc.content.text, doc.language);
                if self.startup_report.is_some()
                    || !self
                        .syntax
                        .status(doc.id, &doc.content.text, language, self.dark)
                        .contains("背景")
                {
                    ctx.send_viewport_cmd(egui::ViewportCommand::Screenshot(Default::default()));
                } else {
                    self.capture_frame = 3;
                }
            }
            let screenshot = ctx.input(|input| {
                input.events.iter().find_map(|event| {
                    if let egui::Event::Screenshot { image, .. } = event {
                        Some(image.clone())
                    } else {
                        None
                    }
                })
            });
            if let Some(image) = screenshot {
                if let Some(report) = &self.startup_report {
                    perf::report_startup(report);
                }
                let result = (|| -> Result<(), Box<dyn std::error::Error>> {
                    let file = std::fs::File::create(path)?;
                    let mut encoder =
                        png::Encoder::new(file, image.size[0] as u32, image.size[1] as u32);
                    encoder.set_color(png::ColorType::Rgba);
                    encoder.set_depth(png::BitDepth::Eight);
                    let mut writer = encoder.write_header()?;
                    let bytes: Vec<u8> = image
                        .pixels
                        .iter()
                        .flat_map(|pixel| pixel.to_array())
                        .collect();
                    writer.write_image_data(&bytes)?;
                    Ok(())
                })();
                if let Err(error) = result {
                    eprintln!("screenshot failed: {error}");
                }
                self.force_exit = true;
                ctx.send_viewport_cmd(egui::ViewportCommand::Close);
            }
        }
        self.poll();
        self.syntax.poll();
        self.syntax
            .retain(&self.docs.iter().map(|doc| doc.id).collect::<Vec<_>>());
        if self.busy {
            ctx.request_repaint_after(Duration::from_millis(100));
        }
        if ctx.input(|i| i.viewport().close_requested())
            && !self.force_exit
            && (self.busy || self.docs.iter().any(Document::dirty))
        {
            ctx.send_viewport_cmd(egui::ViewportCommand::CancelClose);
            self.exit = true;
        }
        let mut new = false;
        let mut open = false;
        let mut save = false;
        let mut save_as = false;
        let mut close = false;
        let mut undo = false;
        let mut redo = false;
        let mut find = false;
        let mut previous = false;
        let mut edit_action = None;
        let mut goto_requested = false;
        let editor_focused = ctx.memory(|memory| {
            memory.has_focus(egui::Id::new(("editor", self.docs[self.active].id)))
        });
        ctx.input_mut(|i| {
            new = i.consume_key(Modifiers::CTRL, Key::N);
            open = i.consume_key(Modifiers::CTRL, Key::O);
            save_as = i.consume_key(Modifiers::CTRL | Modifiers::SHIFT, Key::S);
            save = i.consume_key(Modifiers::CTRL, Key::S);
            close = i.consume_key(Modifiers::CTRL, Key::W);
            (undo, redo) = history_shortcuts(i, editor_focused);
            if i.consume_key(Modifiers::CTRL, Key::F) || i.consume_key(Modifiers::CTRL, Key::H) {
                self.search = true;
            }
            previous = i.consume_key(Modifiers::SHIFT, Key::F3);
            find = i.consume_key(Modifiers::NONE, Key::F3);
            if i.consume_key(Modifiers::CTRL, Key::G) {
                self.goto_open = true;
                goto_requested = true;
            }
            if editor_focused {
                if i.consume_key(Modifiers::CTRL, Key::D) {
                    edit_action = Some(actions::Edit::DuplicateLine);
                }
                if i.consume_key(Modifiers::CTRL | Modifiers::SHIFT, Key::K) {
                    edit_action = Some(actions::Edit::DeleteLine);
                }
                if i.consume_key(Modifiers::CTRL | Modifiers::SHIFT, Key::U) {
                    edit_action = Some(actions::Edit::Uppercase);
                }
                if i.consume_key(Modifiers::CTRL, Key::U) {
                    edit_action = Some(actions::Edit::Lowercase);
                }
            }
            if editor_focused && self.insert_spaces && i.consume_key(Modifiers::NONE, Key::Tab) {
                let doc = &self.docs[self.active];
                let column = core::column_at(&doc.content.text, doc.selected.0, self.tab_width);
                i.events.push(egui::Event::Text(
                    " ".repeat(core::tab_advance(column, self.tab_width)),
                ));
            }
        });
        egui::TopBottomPanel::top("chrome").show(ctx, |ui| {
            egui::MenuBar::new().ui(ui, |ui| {
                ui.menu_button("檔案", |ui| {
                    new |= ui.button("新建    Ctrl+N").clicked();
                    open |= ui.button("開啟…    Ctrl+O").clicked();
                    save |= ui.button("儲存    Ctrl+S").clicked();
                    save_as |= ui.button("另存新檔…    Ctrl+Shift+S").clicked();
                    close |= ui.button("關閉分頁    Ctrl+W").clicked();
                });
                ui.menu_button("編輯", |ui| {
                    if ui.button("複製目前行    Ctrl+D").clicked() {
                        edit_action = Some(actions::Edit::DuplicateLine);
                    }
                    if ui.button("刪除目前行    Ctrl+Shift+K").clicked() {
                        edit_action = Some(actions::Edit::DeleteLine);
                    }
                    ui.separator();
                    if ui.button("選取文字轉大寫    Ctrl+Shift+U").clicked() {
                        edit_action = Some(actions::Edit::Uppercase);
                    }
                    if ui.button("選取文字轉小寫    Ctrl+U").clicked() {
                        edit_action = Some(actions::Edit::Lowercase);
                    }
                    ui.separator();
                    undo |= ui.button("復原    Ctrl+Z").clicked();
                    redo |= ui.button("重做    Ctrl+Y").clicked();
                });
                ui.menu_button("搜尋", |ui| {
                    previous |= ui.button("尋找上一個    Shift+F3").clicked();
                    if ui.button("跳至行…    Ctrl+G").clicked() {
                        self.goto_open = true;
                        goto_requested = true;
                    }
                    if ui.button("搜尋／取代    Ctrl+F / Ctrl+H").clicked() {
                        self.search = true;
                    }
                    find |= ui.button("尋找下一個    F3").clicked();
                });
                ui.menu_button("檢視", |ui| {
                    if ui.checkbox(&mut self.dark, "深色主題").changed() {
                        ctx.set_visuals(if self.dark {
                            egui::Visuals::dark()
                        } else {
                            egui::Visuals::light()
                        });
                    }
                    ui.checkbox(&mut self.show_spaces, "顯示空格與 Tab（僅視覺標記）");
                    ui.checkbox(&mut self.show_eol, "顯示真實行尾與 EOF 標記");
                    ui.label("不自動折行；行尾標記代表檔案換行");
                });
                ui.menu_button("格式", |ui| {
                    let d = &mut self.docs[self.active];
                    let before = d.content.clone();
                    ui.checkbox(&mut d.content.bom, "UTF-8 BOM");
                    ui.selectable_value(&mut d.content.newline, Newline::Lf, "LF（Linux）");
                    ui.selectable_value(&mut d.content.newline, Newline::Crlf, "CRLF（Windows）");
                    ui.selectable_value(&mut d.content.newline, Newline::Cr, "CR（舊式格式）");
                    if before != d.content {
                        d.history.record(before);
                    }
                });
                ui.menu_button("空白", |ui| {
                    ui.label("Tab stops（每 N 欄對齊；Unicode 邏輯字寬）");
                    ui.add(egui::Slider::new(&mut self.tab_width, 1..=8).text("Tab 寬度"));
                    ui.checkbox(&mut self.insert_spaces, "按 Tab 插入空格至下一個 Tab stop");
                    ui.separator();
                    let selected =
                        self.docs[self.active].selected.0 != self.docs[self.active].selected.1;
                    if ui
                        .add_enabled(selected, egui::Button::new("選取的 Tab → 空格"))
                        .clicked()
                    {
                        self.whitespace_edit(WhitespaceEdit::TabsToSpaces(self.tab_width), true);
                    }
                    if ui
                        .add_enabled(selected, egui::Button::new("選取的空格 → Tab"))
                        .clicked()
                    {
                        self.whitespace_edit(WhitespaceEdit::SpacesToTabs(self.tab_width), true);
                    }
                    if ui.button("全文件 Tab → 空格").clicked() {
                        self.whitespace_edit(WhitespaceEdit::TabsToSpaces(self.tab_width), false);
                    }
                    if ui.button("全文件空格 → Tab").clicked() {
                        self.whitespace_edit(WhitespaceEdit::SpacesToTabs(self.tab_width), false);
                    }
                    ui.separator();
                    if ui.button("清除行尾空格與 Tab").clicked() {
                        self.whitespace_edit(WhitespaceEdit::TrimTrailing, false);
                    }
                    if ui.button("確保檔尾一個換行（不刪空白行）").clicked() {
                        self.whitespace_edit(WhitespaceEdit::AddFinalNewline, false);
                    }
                    if ui.button("移除最後一個換行").clicked() {
                        self.whitespace_edit(WhitespaceEdit::RemoveFinalNewline, false);
                    }
                    ui.label("上述內容轉換可 Ctrl+Z 復原；預設不自動清理");
                });
                ui.menu_button("語言", |ui| {
                    let doc = &mut self.docs[self.active];
                    ui.selectable_value(&mut doc.language, None, "自動（副檔名／shebang）");
                    ui.separator();
                    for language in syntax::Language::ALL {
                        ui.selectable_value(&mut doc.language, Some(language), language.label());
                    }
                });
                ui.label("RustPad");
            });
            ui.horizontal(|ui| {
                new |= ui.small_button("＋ 新建").clicked();
                open |= ui.small_button("開啟").clicked();
                save |= ui.small_button("儲存").clicked();
                ui.separator();
                undo |= ui.small_button("復原").clicked();
                redo |= ui.small_button("重做").clicked();
                if ui.small_button("搜尋／取代").clicked() {
                    self.search = !self.search;
                }
                if self.busy {
                    ui.spinner();
                    ui.label("處理檔案中…");
                }
            });
            egui::ScrollArea::horizontal()
                .id_salt("tabs")
                .show(ui, |ui| {
                    ui.horizontal(|ui| {
                        let mut closing = None;
                        let mut activating = None;
                        for (index, d) in self.docs.iter().enumerate() {
                            if ui
                                .selectable_label(index == self.active, d.title())
                                .clicked()
                            {
                                activating = Some(index);
                            }
                            if ui.small_button("×").clicked() {
                                closing = Some(d.id);
                            }
                            ui.separator();
                        }
                        if let Some(index) = activating {
                            self.activate(index);
                        }
                        if let Some(id) = closing {
                            self.request_close(id)
                        }
                    });
                });
            if self.search {
                let before = (self.query.clone(), self.search_options);
                ui.horizontal(|ui| {
                    ui.label("尋找");
                    ui.add(egui::TextEdit::singleline(&mut self.query).desired_width(170.));
                    previous |= ui.button("上一個").clicked();
                    find |= ui.button("下一個").clicked();
                    if ui.button("計數").clicked() {
                        self.count_matches();
                    }
                    ui.label("取代");
                    ui.add(egui::TextEdit::singleline(&mut self.replacement).desired_width(170.));
                    if before.0 != self.query {
                        self.match_range = None;
                    }
                    if ui.button("取代此項").clicked() {
                        self.replace_one();
                    }
                    if ui.button("全部取代").clicked() {
                        self.replace_all();
                    }
                    if ui.small_button("×").clicked() {
                        self.search = false;
                    }
                });
                ui.horizontal(|ui| {
                    ui.checkbox(&mut self.search_options.match_case, "符合大小寫");
                    ui.checkbox(&mut self.search_options.whole_word, "全字");
                    ui.checkbox(&mut self.search_options.wrap, "循環搜尋");
                    ui.checkbox(&mut self.in_selection, "計數／全部取代限選取範圍");
                    ui.weak("一般文字；不含 regex");
                });
                if before != (self.query.clone(), self.search_options) {
                    self.match_range = None;
                }
            }
        });
        if goto_requested {
            ctx.memory_mut(|m| m.request_focus(egui::Id::new("goto-line")));
        }
        if new {
            self.new_doc()
        }
        if open {
            self.open(ctx)
        }
        if save || save_as {
            self.save(ctx, save_as, false)
        }
        if close {
            self.request_close(self.docs[self.active].id)
        }
        if undo {
            let d = &mut self.docs[self.active];
            d.history.undo(&mut d.content);
            self.selection = None;
            self.match_range = None;
        }
        if redo {
            let d = &mut self.docs[self.active];
            d.history.redo(&mut d.content);
            self.selection = None;
            self.match_range = None;
        }
        if previous {
            self.find_direction(true);
        }
        if let Some(action) = edit_action {
            self.edit_action(action);
        }
        if find {
            self.find_next();
        }
        if self.goto_open {
            egui::Window::new("跳至行")
                .collapsible(false)
                .resizable(false)
                .anchor(egui::Align2::CENTER_CENTER, [0., 0.])
                .show(ctx, |ui| {
                    let max = self.docs[self.active].content.text.matches('\n').count() + 1;
                    ui.label(format!("行號 1–{max}（Unicode 游標，保留內容）"));
                    let field = ui.add(
                        egui::TextEdit::singleline(&mut self.goto_input)
                            .id(egui::Id::new("goto-line"))
                            .desired_width(180.),
                    );
                    if field.lost_focus() && ui.input(|i| i.key_pressed(Key::Enter)) {
                        self.go_to_line();
                    }
                    ui.horizontal(|ui| {
                        if ui.button("跳至").clicked() {
                            self.go_to_line();
                        }
                        if ui.button("取消").clicked() {
                            self.goto_open = false;
                        }
                    });
                });
        }
        egui::TopBottomPanel::bottom("status").show(ctx, |ui| {
            let d = &self.docs[self.active];
            let language = syntax::resolve(d.path.as_deref(), &d.content.text, d.language);
            let prefix: String = d.content.text.chars().take(d.cursor).collect();
            let line = prefix.matches('\n').count() + 1;
            let col = prefix.rsplit('\n').next().unwrap_or("").chars().count() + 1;
            ui.horizontal(|ui| {
                ui.label(&self.message);
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.label(format!(
                        "{}{} | {} | {} | Tab:{}{} | 末尾:{} | 行 {line}:列 {col} | {} 字",
                        language.label(),
                        self.syntax
                            .status(d.id, &d.content.text, language, self.dark),
                        if d.content.bom { "UTF-8 BOM" } else { "UTF-8" },
                        d.content.newline.label(),
                        self.tab_width,
                        if self.insert_spaces { "空格" } else { "Tab" },
                        if d.content.text.ends_with('\n') {
                            "有"
                        } else {
                            "無"
                        },
                        d.content.text.chars().count()
                    ));
                });
            });
        });
        egui::CentralPanel::default()
            .frame(
                egui::Frame::central_panel(&ctx.style())
                    .fill(ctx.style().visuals.extreme_bg_color)
                    .inner_margin(4.),
            )
            .show(ctx, |ui| {
                let d = &mut self.docs[self.active];
                let before =
                    (!ctx.input(|input| input.events.is_empty())).then(|| d.content.clone());
                let id = egui::Id::new(("editor", d.id));
                let pending_scroll = self.selection.is_some();
                if let Some((start, end)) = self.selection.take() {
                    let mut state = egui::TextEdit::load_state(ctx, id).unwrap_or_default();
                    state.cursor.set_char_range(Some(CCursorRange::two(
                        CCursor::new(start),
                        CCursor::new(end),
                    )));
                    state.store(ctx, id);
                    ctx.memory_mut(|m| m.request_focus(id));
                }
                egui::ScrollArea::both()
                    .animated(false)
                    .id_salt(("scroll", d.id))
                    .show(ui, |ui| {
                        ui.horizontal_top(|ui| {
                            let count = d.content.text.matches('\n').count() + 1;
                            let nums = (1..=count).map(|n| format!("{n:>4}\n")).collect::<String>();
                            ui.add(
                                egui::Label::new(
                                    egui::RichText::new(nums.trim_end())
                                        .font(FontId::monospace(15.))
                                        .color(Color32::GRAY),
                                )
                                .selectable(false),
                            );
                            ui.separator();
                            let dark = self.dark;
                            let tab_width = self.tab_width;
                            let cache = &mut d.highlight_cache;
                            let path = d.path.clone();
                            let manual = d.language;
                            let doc_id = d.id;
                            let service = &mut self.syntax;
                            let mut layouter =
                                move |ui: &egui::Ui, text: &dyn egui::TextBuffer, _width: f32| {
                                    let language =
                                        syntax::resolve(path.as_deref(), text.as_str(), manual);
                                    let (height, space) = ui.fonts_mut(|fonts| {
                                        (
                                            fonts.row_height(&FontId::monospace(15.)),
                                            fonts.glyph_width(&FontId::monospace(15.), ' '),
                                        )
                                    });
                                    let job = service.layout(
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
                                    ui.fonts_mut(|fonts| fonts.layout_job(job))
                                };
                            let output = egui::TextEdit::multiline(&mut d.content.text)
                                .id(id)
                                .font(FontId::monospace(15.))
                                .code_editor()
                                .frame(false)
                                .margin(egui::Vec2::ZERO)
                                .desired_width(ui.available_width().max(600.))
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
                                let rect = output
                                    .galley
                                    .pos_from_cursor(CCursor::new(d.cursor))
                                    .translate(output.galley_pos.to_vec2());
                                ui.scroll_to_rect(rect, Some(egui::Align::Center));
                            }
                            paint_whitespace(
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
            });
        if let Some(id) = self.pending_close {
            egui::Window::new("尚有未儲存變更")
                .collapsible(false)
                .resizable(false)
                .anchor(egui::Align2::CENTER_CENTER, [0., 0.])
                .show(ctx, |ui| {
                    ui.label(
                        self.docs
                            .iter()
                            .find(|d| d.id == id)
                            .map(Document::title)
                            .unwrap_or_default(),
                    );
                    ui.horizontal(|ui| {
                        if ui
                            .add_enabled(!self.busy, egui::Button::new("儲存後關閉"))
                            .clicked()
                            && let Some(i) = self.docs.iter().position(|d| d.id == id)
                        {
                            self.active = i;
                            self.save(ctx, false, true);
                        }
                        if ui
                            .add_enabled(!self.busy, egui::Button::new("放棄變更"))
                            .clicked()
                        {
                            self.remove(id);
                        }
                        if ui.button("取消").clicked() {
                            self.pending_close = None;
                        }
                    });
                });
        }
        if self.exit {
            egui::Window::new("結束 RustPad")
                .collapsible(false)
                .anchor(egui::Align2::CENTER_CENTER, [0., 0.])
                .show(ctx, |ui| {
                    ui.label("仍有未儲存變更或檔案操作。請先儲存需要的分頁。");
                    ui.horizontal(|ui| {
                        if ui
                            .add_enabled(!self.busy, egui::Button::new("放棄全部並結束"))
                            .clicked()
                        {
                            self.force_exit = true;
                            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                        }
                        if ui.button("返回編輯").clicked() {
                            self.exit = false;
                        }
                    });
                });
        }
    }
}
fn paint_whitespace(
    ui: &egui::Ui,
    output: &egui::text_edit::TextEditOutput,
    spaces: bool,
    eol: bool,
    newline: Newline,
) {
    if !spaces && !eol {
        return;
    }
    let color = if ui.visuals().dark_mode {
        Color32::from_rgb(116, 139, 155)
    } else {
        Color32::from_rgb(130, 151, 165)
    };
    for row in &output.galley.rows {
        let origin = output.galley_pos + row.pos.to_vec2();
        let rect = egui::Rect::from_min_size(origin, row.row.size);
        if !ui.clip_rect().intersects(rect) {
            continue;
        }
        let y = origin.y + row.row.size.y * 0.5;
        if spaces {
            for glyph in &row.row.glyphs {
                let x = origin.x + glyph.pos.x;
                match glyph.chr {
                    ' ' => {
                        ui.painter().circle_filled(
                            egui::pos2(x + glyph.advance_width * 0.5, y),
                            1.,
                            color,
                        );
                    }
                    '\t' => {
                        let left = x + 2.;
                        let right = x + glyph.advance_width - 2.;
                        if right > left {
                            let stroke = egui::Stroke::new(1_f32, color);
                            ui.painter()
                                .line_segment([egui::pos2(left, y), egui::pos2(right, y)], stroke);
                            ui.painter().line_segment(
                                [egui::pos2(right - 3., y - 3.), egui::pos2(right, y)],
                                stroke,
                            );
                            ui.painter().line_segment(
                                [egui::pos2(right - 3., y + 3.), egui::pos2(right, y)],
                                stroke,
                            );
                        }
                    }
                    _ => {}
                }
            }
        }
        if eol {
            let label = if row.row.ends_with_newline {
                newline.label()
            } else {
                "EOF"
            };
            ui.painter().text(
                egui::pos2(origin.x + row.row.size.x + 3., y),
                egui::Align2::LEFT_CENTER,
                label,
                FontId::monospace(9.),
                color,
            );
        }
    }
}
fn main() -> eframe::Result {
    perf::mark_start();
    let arguments: Vec<String> = std::env::args().collect();
    if let Some(index) = arguments.iter().position(|arg| arg == "--benchmark")
        && let Some(path) = arguments.get(index + 1)
    {
        let engine = syntax::Engine::new();
        perf::benchmark(PathBuf::from(path).as_path(), |text| {
            let spans = engine
                .spans(text, syntax::Language::Rust, false, || false)
                .expect("benchmark grammar");
            syntax::layout(text, &spans, 4, 18., 9.)
        });
        return Ok(());
    }
    eframe::run_native(
        "RustPad — 文字編輯器",
        eframe::NativeOptions {
            viewport: egui::ViewportBuilder::default()
                .with_inner_size([1080., 720.])
                .with_min_inner_size([760., 420.]),
            ..Default::default()
        },
        Box::new(|cc| {
            let mut app = App::new(cc);
            let args: Vec<String> = std::env::args().collect();
            if let Some(i) = args.iter().position(|s| s == "--screenshot")
                && let Some(path) = args.get(i + 1)
            {
                app.capture = Some(PathBuf::from(path));
                app.docs[0].path = Some(PathBuf::from("welcome.rs"));
                app.docs[0].content.text="// RustPad — 繁體中文與 Unicode 測試\nfn main() {\n    let greeting = \"你好，世界 🙂\";\n    println!(\"{greeting}\");\n}\n\n// Ctrl+F 搜尋 · Ctrl+S 儲存\n".into();
                app.docs[0].saved = app.docs[0].content.clone();
                app.new_doc();
                app.active = 0;
                app.search = true;
                app.query = "世界".into();
                app.replacement = "Rust".into();
                if args.iter().any(|s| s == "--whitespace") {
                    app.show_spaces = true;
                    app.show_eol = true;
                    app.docs[0].content.text="// Windows CRLF / Linux LF（內容格式，不由作業系統偷改）\nfn main() {\n\tlet greeting = \"你好，世界 🙂\";  \n\tprintln!(\"{greeting}\");\t\n}\n \t\n// · = 空格；箭頭 = Tab；標記不會寫入檔案".into();
                    app.docs[0].content.newline = Newline::Crlf;
                    app.docs[0].saved = app.docs[0].content.clone();
                }
                if args.iter().any(|s| s == "--dark") {
                    app.dark = true;
                    cc.egui_ctx.set_visuals(egui::Visuals::dark());
                }
                if let Some(index) = args.iter().position(|s| s == "--sample")
                    && let Some(sample) = args.get(index + 1)
                {
                    let (name, text) = match sample.as_str() {
                        "python" => (
                            "welcome.py",
                            "#!/usr/bin/env python3\n\"\"\"跨行文件字串\n保留中文與 emoji 🙂\n\"\"\"\nimport json\n\n# 字串、註解、關鍵字與數字\ndef greet(name: str) -> str:\n    count = 42\n    return f\"你好，{name} {count}\"\n\nprint(greet(\"世界\"))\n",
                        ),
                        "json" => (
                            "settings.json",
                            "{\n  \"name\": \"RustPad — 中文 🙂\",\n  \"version\": 0.2,\n  \"enabled\": true,\n  \"languages\": [\"Rust\", \"Python\", \"JSON\"],\n  \"empty\": null\n}\n",
                        ),
                        _ => (
                            "welcome.rs",
                            "/* 跨行註解\n   中文與 emoji 🙂 */\nfn main() {\n\tlet greeting = \"你好，世界 🙂\";  \n\tlet count: usize = 42;\n\tprintln!(\"{greeting} {count}\");\n}\n",
                        ),
                    };
                    app.docs[0].path = Some(PathBuf::from(name));
                    app.docs[0].content.text = text.into();
                    app.docs[0].saved = app.docs[0].content.clone();
                }
            }
            if let Some(index) = args.iter().position(|arg| arg == "--startup-probe")
                && let Some(path) = args.get(index + 1)
            {
                let report = PathBuf::from(path);
                app.capture = Some(report.with_extension("png"));
                app.startup_report = Some(report);
                app.docs = vec![Document::new(1, None, Content::default())];
                app.active = 0;
                app.search = false;
            }
            if app.capture.is_some()
                && app.startup_report.is_none()
                && let Some(index) = args.iter().position(|arg| arg == "--qa-flow")
                && let Some(flow) = args.get(index + 1)
            {
                app.docs[0].path = Some(PathBuf::from("qa-demo.txt"));
                match flow.as_str() {
                    "search" => {
                        app.docs[0].content.text =
                            "Rust RUST rusty\n中文 Rust 中文Rust\n🙂 rust_ RUST\n".into();
                        app.docs[0].saved = app.docs[0].content.clone();
                        app.query = "rust".into();
                        app.search_options.match_case = false;
                        app.search_options.whole_word = true;
                        app.find_direction(true);
                        app.count_matches();
                    }
                    "navigation" => {
                        app.docs[0].content.text =
                            (1..=500).map(|n| format!("第 {n} 行 — 中文🙂\n")).collect();
                        app.docs[0].saved = app.docs[0].content.clone();
                        app.goto_input = "350".into();
                        app.go_to_line();
                        app.search = false;
                    }
                    "goto" => {
                        app.goto_open = true;
                        app.goto_input = "3".into();
                    }
                    "close" => {
                        app.docs[0].content.text.push_str("// 未儲存變更\n");
                        app.request_close(app.docs[0].id);
                    }
                    "edits" => {
                        app.docs[0].content.text = "中文🙂\nStraße\n尾".into();
                        app.docs[0].content.newline = Newline::Crlf;
                        app.docs[0].saved = app.docs[0].content.clone();
                        app.docs[0].selected = (4, 10);
                        app.edit_action(actions::Edit::Uppercase);
                        app.edit_action(actions::Edit::DuplicateLine);
                        app.show_eol = true;
                    }
                    _ => {}
                }
            }
            Ok(Box::new(app))
        }),
    )
}

#[cfg(test)]
mod app_tests {
    use super::*;
    #[test]
    fn replace_current_uses_the_match_found_from_inside_an_overlap() {
        let mut a = app();
        a.docs[0].content.text = "🙂aaaa".into();
        a.docs[0].cursor = 2;
        a.query = "aa".into();
        a.replacement = "X".into();
        a.find_next();
        assert_eq!(a.selection, Some((2, 4)));
        a.replace_one();
        assert_eq!(a.docs[0].content.text, "🙂aXa");
        let d = &mut a.docs[0];
        d.history.undo(&mut d.content);
        assert_eq!(d.content.text, "🙂aaaa");
    }
    fn receive_action(a: &mut App) {
        let event =
            a.rx.recv_timeout(Duration::from_secs(10))
                .expect("bounded background result");
        a.tx.send(event).unwrap();
        a.poll();
    }
    #[test]
    fn large_search_and_replace_worker_preserves_edits_and_history() {
        let mut a = app();
        a.docs[0].content.text = "Rust 中文🙂\n".repeat(18_000);
        a.docs[0].saved = a.docs[0].content.clone();
        a.query = "Rust".into();
        a.replacement = "改".into();
        a.count_matches();
        assert!(a.busy);
        receive_action(&mut a);
        assert!(!a.busy);
        assert!(a.message.contains("18000"));
        assert!(!a.docs[0].dirty());
        a.replace_all();
        assert!(a.busy);
        receive_action(&mut a);
        assert_eq!(a.docs[0].content.text, "改 中文🙂\n".repeat(18_000));
        let d = &mut a.docs[0];
        d.history.undo(&mut d.content);
        assert_eq!(d.content, d.saved);
        a.replace_all();
        assert!(a.busy);
        a.docs[0].content.text.push_str("新增修改");
        let current = a.docs[0].content.clone();
        receive_action(&mut a);
        assert_eq!(a.docs[0].content, current);
        assert!(a.message.contains("過期"));
    }
    #[test]
    fn background_result_cannot_mutate_a_different_tab_or_query() {
        let mut a = app();
        a.docs[0].content.text = "Rust 中文🙂\n".repeat(18_000);
        a.query = "Rust".into();
        a.replacement = "改".into();
        a.find_next();
        assert!(a.busy);
        a.query = "different".into();
        receive_action(&mut a);
        assert_eq!(a.selection, None);
        assert!(a.message.contains("過期"));
        a.query = "Rust".into();
        a.replace_all();
        let before = a.docs[0].content.clone();
        a.new_doc();
        receive_action(&mut a);
        assert_eq!(a.docs[0].content, before);
        assert!(!a.docs[1].dirty());
    }
    #[test]
    fn changing_search_conditions_cannot_replace_an_old_match() {
        let mut a = app();
        a.docs[0].content.text = "Rust".into();
        a.query = "Rust".into();
        a.search_options.match_case = false;
        a.replacement = "損毀".into();
        a.find_next();
        a.query = "rust".into();
        a.replace_one();
        assert_eq!(a.docs[0].content.text, "Rust");
        a.query = "Rust".into();
        a.find_next();
        a.search_options.whole_word = true;
        a.replace_one();
        assert_eq!(a.docs[0].content.text, "Rust");
    }
    #[test]
    fn new_tabs_preserve_content_and_clear_old_search_scope() {
        let mut a = app();
        a.docs[0].content.text = "中文🙂".into();
        a.in_selection = true;
        a.selection = Some((0, 1));
        let first = a.docs[0].id;
        a.new_doc();
        assert_eq!(a.active, 1);
        assert!(!a.docs[1].dirty());
        assert!(!a.in_selection);
        assert_eq!(a.selection, None);
        a.activate(0);
        assert_eq!(a.docs[0].content.text, "中文🙂");
        a.request_close(first);
        assert_eq!(a.pending_close, Some(first));
        a.pending_close = None;
        assert_eq!(a.docs.len(), 2);
        a.remove(first);
        assert_eq!(a.docs.len(), 1);
        assert!(!a.docs[0].dirty());
    }
    #[test]
    fn search_count_selection_replace_and_undo_redo_are_consistent() {
        let mut a = app();
        a.docs[0].content.text = "Rust RUST rusty Rust".into();
        a.docs[0].saved = a.docs[0].content.clone();
        a.query = "rust".into();
        a.replacement = "中文🙂".into();
        a.search_options.match_case = false;
        a.search_options.whole_word = true;
        a.count_matches();
        assert!(a.message.contains("3"));
        assert!(!a.docs[0].dirty());
        a.in_selection = true;
        a.docs[0].selected = (0, 9);
        a.count_matches();
        assert!(a.message.contains("2"));
        a.replace_all();
        assert_eq!(a.docs[0].content.text, "中文🙂 中文🙂 rusty Rust");
        assert!(a.docs[0].dirty());
        let d = &mut a.docs[0];
        d.history.undo(&mut d.content);
        assert_eq!(d.content, d.saved);
        d.history.redo(&mut d.content);
        assert_eq!(d.content.text, "中文🙂 中文🙂 rusty Rust");
        a.docs[0].selected = (0, 0);
        let before = a.docs[0].content.clone();
        a.replace_all();
        assert_eq!(a.docs[0].content, before);
    }
    #[test]
    fn navigation_and_edit_actions_update_cursor_without_bad_dirty() {
        let mut a = app();
        a.docs[0].content.text = "中文🙂\nStraße\n尾".into();
        a.docs[0].saved = a.docs[0].content.clone();
        a.goto_input = "2".into();
        a.goto_open = true;
        a.go_to_line();
        assert_eq!(a.selection, Some((4, 4)));
        assert!(!a.goto_open);
        assert!(!a.docs[0].dirty());
        a.goto_input = "0".into();
        a.go_to_line();
        assert_eq!(a.docs[0].cursor, 4);
        a.docs[0].selected = (4, 10);
        a.edit_action(actions::Edit::Uppercase);
        assert_eq!(a.docs[0].content.text, "中文🙂\nSTRASSE\n尾");
        assert_eq!(a.selection, Some((4, 11)));
        let d = &mut a.docs[0];
        d.history.undo(&mut d.content);
        assert_eq!(d.content, d.saved);
        a.docs[0].cursor = 4;
        a.docs[0].selected = (4, 4);
        a.edit_action(actions::Edit::DuplicateLine);
        assert_eq!(a.docs[0].content.text, "中文🙂\nStraße\nStraße\n尾");
    }
    fn app() -> App {
        let (tx, rx) = mpsc::channel();
        App {
            docs: vec![Document::new(1, None, Content::default())],
            active: 0,
            next_id: 2,
            tx,
            rx,
            busy: false,
            message: String::new(),
            dark: false,
            search: false,
            query: String::new(),
            replacement: String::new(),
            selection: None,
            pending_close: None,
            exit: false,
            force_exit: false,
            match_range: None,
            match_signature: None,
            capture: None,
            capture_frame: 0,
            show_spaces: false,
            show_eol: false,
            tab_width: 4,
            insert_spaces: false,
            syntax: syntax::Service::new(egui::Context::default()),
            startup_report: None,
            search_options: Default::default(),
            in_selection: false,
            goto_open: false,
            goto_input: "1".into(),
            repaint: egui::Context::default(),
        }
    }
    #[test]
    fn dirty_close_cancel_and_discard() {
        let mut a = app();
        a.docs[0].content.text = "未儲存🙂".into();
        a.request_close(1);
        assert_eq!(a.pending_close, Some(1));
        a.pending_close = None;
        assert_eq!(a.docs[0].content.text, "未儲存🙂");
        a.request_close(1);
        a.remove(1);
        assert_eq!(a.docs.len(), 1);
        assert!(!a.docs[0].dirty());
        assert_ne!(a.docs[0].id, 1);
    }
    #[test]
    fn async_save_keeps_newer_edits_and_failed_save_dirty() {
        let mut a = app();
        let saved = Content {
            text: "first".into(),
            ..Default::default()
        };
        a.docs[0].content.text = "newer".into();
        a.busy = true;
        a.tx.send(ResultEvent::Save(
            1,
            saved.clone(),
            Ok(Some(PathBuf::from("file.txt"))),
            true,
        ))
        .unwrap();
        a.poll();
        assert_eq!(a.docs[0].saved, saved);
        assert!(a.docs[0].dirty());
        assert_eq!(a.docs[0].id, 1);
        a.tx.send(ResultEvent::Save(
            1,
            a.docs[0].content.clone(),
            Err("disk error".into()),
            true,
        ))
        .unwrap();
        a.poll();
        assert!(a.docs[0].dirty());
        a.tx.send(ResultEvent::Save(
            1,
            a.docs[0].content.clone(),
            Ok(None),
            true,
        ))
        .unwrap();
        a.poll();
        assert!(a.docs[0].dirty());
    }
    #[test]
    fn find_and_replace_survives_widget_selection_application() {
        let mut a = app();
        a.docs[0].content.text = "甲🙂乙🙂".into();
        a.query = "🙂".into();
        a.replacement = "中文".into();
        a.find_next();
        assert_eq!(a.selection, Some((1, 2)));
        a.selection.take();
        a.replace_one();
        assert_eq!(a.docs[0].content.text, "甲中文乙🙂");
        let doc = &mut a.docs[0];
        doc.history.undo(&mut doc.content);
        assert_eq!(a.docs[0].content.text, "甲🙂乙🙂");
    }
    #[test]
    fn search_match_does_not_cross_tabs() {
        let mut a = app();
        a.docs[0].content.text = "abc".into();
        a.query = "a".into();
        a.replacement = "z".into();
        a.find_next();
        a.new_doc();
        a.docs[1].content.text = "abc".into();
        a.replace_one();
        assert_eq!(a.docs[1].content.text, "abc");
    }
    #[test]
    fn tab_layout_keeps_text_and_cursor_mapping() {
        let ctx = egui::Context::default();
        let text = "a\t🙂\n  b";
        let mut dimensions = None;
        let _ = ctx.run(Default::default(), |ctx| {
            dimensions = Some(ctx.fonts_mut(|fonts| {
                let height = fonts.row_height(&FontId::monospace(15.));
                let small = fonts.layout_job(syntax::layout(
                    text,
                    &[syntax::Span {
                        start: 0,
                        end: text.len(),
                        color: syntax::normal(false),
                    }],
                    2,
                    height,
                    9.,
                ));
                let large = fonts.layout_job(syntax::layout(
                    text,
                    &[syntax::Span {
                        start: 0,
                        end: text.len(),
                        color: syntax::normal(false),
                    }],
                    8,
                    height,
                    9.,
                ));
                assert_eq!(small.job.text, text);
                assert_eq!(large.job.text, text);
                assert_eq!(small.rows.len(), large.rows.len());
                assert_eq!(small.size().y, large.size().y);
                (
                    small.pos_from_cursor(CCursor::new(2)).min.x,
                    large.pos_from_cursor(CCursor::new(2)).min.x,
                )
            }));
        });
        let (small, large) = dimensions.unwrap();
        assert!(large > small);
    }
    #[test]
    fn whitespace_ui_action_records_dirty_and_undo() {
        let mut a = app();
        a.docs[0].content.text = "甲\t \n".into();
        a.docs[0].saved = a.docs[0].content.clone();
        a.whitespace_edit(WhitespaceEdit::TrimTrailing, false);
        assert!(a.docs[0].dirty());
        assert_eq!(a.docs[0].content.text, "甲\n");
        let d = &mut a.docs[0];
        d.history.undo(&mut d.content);
        assert!(!d.dirty());
    }
    #[test]
    fn search_focus_does_not_steal_undo_from_editor() {
        let ctx = egui::Context::default();
        let mut a = app();
        let snapshot = a.docs[0].content.clone();
        a.docs[0].history.record(snapshot);
        a.docs[0].content.text = "keep editor change".into();
        let before = a.docs[0].content.clone();
        ctx.memory_mut(|memory| memory.request_focus(egui::Id::new("find-query")));
        let _ = ctx.run(
            egui::RawInput {
                modifiers: Modifiers::CTRL,
                events: vec![egui::Event::Key {
                    key: Key::Z,
                    physical_key: None,
                    pressed: true,
                    repeat: false,
                    modifiers: Modifiers::CTRL,
                }],
                ..Default::default()
            },
            |ctx| {
                let focus = ctx.memory(|memory| memory.has_focus(egui::Id::new(("editor", 1_u64))));
                ctx.input_mut(|input| {
                    let (undo, redo) = history_shortcuts(input, focus);
                    assert!(!undo && !redo);
                    assert!(input.consume_key(Modifiers::CTRL, Key::Z));
                });
            },
        );
        assert_eq!(a.docs[0].content, before);
    }
    #[test]
    fn save_as_same_text_resolves_new_language_without_dirty() {
        let mut a = app();
        let text = Content {
            text: "let n=42;".into(),
            ..Default::default()
        };
        a.docs[0].content = text.clone();
        a.docs[0].saved = text.clone();
        a.docs[0].path = Some(PathBuf::from("same.txt"));
        assert_eq!(
            syntax::resolve(a.docs[0].path.as_deref(), &text.text, None),
            syntax::Language::Plain
        );
        a.tx.send(ResultEvent::Save(
            1,
            text.clone(),
            Ok(Some(PathBuf::from("same.rs"))),
            false,
        ))
        .unwrap();
        a.poll();
        assert_eq!(
            syntax::resolve(a.docs[0].path.as_deref(), &text.text, None),
            syntax::Language::Rust
        );
        a.docs[0].language = Some(syntax::Language::Python);
        a.dark = true;
        a.tab_width = 8;
        assert!(!a.docs[0].dirty());
        let doc = &mut a.docs[0];
        doc.history.undo(&mut doc.content);
        assert_eq!(doc.content, text);
    }
}
