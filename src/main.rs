#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
mod core;
use core::{Content, History, Newline, WhitespaceEdit};
use eframe::egui::{
    self, Color32, FontId, Key, Modifiers,
    text::{CCursor, CCursorRange},
};
use std::{path::PathBuf, sync::mpsc, time::Duration};

struct Document {
    id: u64,
    path: Option<PathBuf>,
    content: Content,
    saved: Content,
    history: History,
    cursor: usize,
    selected: (usize, usize),
    highlight_cache: Option<(String, bool, usize, egui::text::LayoutJob)>,
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
            highlight_cache: None,
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
    capture: Option<PathBuf>,
    capture_frame: usize,
    show_spaces: bool,
    show_eol: bool,
    tab_width: usize,
    insert_spaces: bool,
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
            capture: None,
            capture_frame: 0,
            show_spaces: false,
            show_eol: false,
            tab_width: 4,
            insert_spaces: false,
        }
    }
    fn new_doc(&mut self) {
        self.docs
            .push(Document::new(self.next_id, None, Content::default()));
        self.next_id += 1;
        self.active = self.docs.len() - 1;
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
    fn find_next(&mut self) {
        let d = &self.docs[self.active];
        self.selection = core::find(&d.content.text, &self.query, d.cursor);
        self.match_range = self.selection.map(|range| (d.id, range));
        self.message = if self.selection.is_some() {
            "已找到；再次搜尋會往後並循環"
        } else {
            "找不到符合文字"
        }
        .into();
    }
    fn replace_one(&mut self) {
        let d = &mut self.docs[self.active];
        if let Some((id, range)) = self.match_range.take()
            && id == d.id
        {
            let chars: String = d
                .content
                .text
                .chars()
                .skip(range.0)
                .take(range.1 - range.0)
                .collect();
            if chars == self.query && !self.query.is_empty() {
                let before = d.content.clone();
                core::replace_range(&mut d.content.text, range, &self.replacement);
                if let Err(error) = core::validate_text(&d.content.text) {
                    d.content = before;
                    self.message = error;
                } else {
                    d.history.record(before);
                    d.cursor = range.0 + self.replacement.chars().count();
                }
            }
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
    fn poll(&mut self) {
        while let Ok(event) = self.rx.try_recv() {
            self.busy = false;
            match event {
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
            if self.capture_frame == 4 {
                ctx.send_viewport_cmd(egui::ViewportCommand::Screenshot(Default::default()));
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
        let editor_focused = ctx.memory(|memory| {
            memory.has_focus(egui::Id::new(("editor", self.docs[self.active].id)))
        });
        ctx.input_mut(|i| {
            new = i.consume_key(Modifiers::CTRL, Key::N);
            open = i.consume_key(Modifiers::CTRL, Key::O);
            save_as = i.consume_key(Modifiers::CTRL | Modifiers::SHIFT, Key::S);
            save = i.consume_key(Modifiers::CTRL, Key::S);
            close = i.consume_key(Modifiers::CTRL, Key::W);
            undo = i.consume_key(Modifiers::CTRL, Key::Z);
            redo = i.consume_key(Modifiers::CTRL, Key::Y)
                || i.consume_key(Modifiers::CTRL | Modifiers::SHIFT, Key::Z);
            if i.consume_key(Modifiers::CTRL, Key::F) || i.consume_key(Modifiers::CTRL, Key::H) {
                self.search = true;
            }
            find = i.consume_key(Modifiers::NONE, Key::F3);
            if editor_focused && self.insert_spaces && i.consume_key(Modifiers::NONE, Key::Tab) {
                i.events.push(egui::Event::Text(" ".repeat(self.tab_width)));
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
                    undo |= ui.button("復原    Ctrl+Z").clicked();
                    redo |= ui.button("重做    Ctrl+Y").clicked();
                });
                ui.menu_button("搜尋", |ui| {
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
                    ui.label("Tab 固定字寬（內容不隨顯示設定改動）");
                    ui.add(egui::Slider::new(&mut self.tab_width, 1..=8).text("Tab 寬度"));
                    ui.checkbox(&mut self.insert_spaces, "按 Tab 插入等量空格");
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
                        for (index, d) in self.docs.iter().enumerate() {
                            if ui
                                .selectable_label(index == self.active, d.title())
                                .clicked()
                            {
                                self.active = index;
                                self.selection = None;
                            }
                            if ui.small_button("×").clicked() {
                                closing = Some(d.id);
                            }
                            ui.separator();
                        }
                        if let Some(id) = closing {
                            self.request_close(id)
                        }
                    });
                });
            if self.search {
                ui.horizontal(|ui| {
                    ui.label("尋找");
                    ui.add(egui::TextEdit::singleline(&mut self.query).desired_width(180.));
                    find |= ui.button("下一個").clicked();
                    ui.label("取代");
                    ui.add(egui::TextEdit::singleline(&mut self.replacement).desired_width(180.));
                    if ui.button("取代此項").clicked() {
                        self.replace_one();
                    }
                    if ui.button("全部取代").clicked() && !self.query.is_empty() {
                        let d = &mut self.docs[self.active];
                        let before = d.content.clone();
                        let count = d.content.text.matches(&self.query).count();
                        let projected = d.content.text.len().saturating_add(count.saturating_mul(
                            self.replacement.len().saturating_sub(self.query.len()),
                        ));
                        if projected <= core::MAX_BYTES {
                            d.content.text = d.content.text.replace(&self.query, &self.replacement);
                            if let Err(error) = core::validate_text(&d.content.text) {
                                d.content = before;
                                self.message = error;
                            } else {
                                if before != d.content {
                                    d.history.record(before);
                                }
                                self.selection = None;
                                self.match_range = None;
                                self.message = format!("已取代 {count} 處");
                            }
                        } else {
                            self.message = "取代結果超過 2 MiB 上限".into();
                        }
                    }
                    if ui.small_button("×").clicked() {
                        self.search = false;
                    }
                });
            }
        });
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
        }
        if redo {
            let d = &mut self.docs[self.active];
            d.history.redo(&mut d.content);
            self.selection = None;
        }
        if find {
            self.find_next()
        }
        egui::TopBottomPanel::bottom("status").show(ctx, |ui| {
            let d = &self.docs[self.active];
            let prefix: String = d.content.text.chars().take(d.cursor).collect();
            let line = prefix.matches('\n').count() + 1;
            let col = prefix.rsplit('\n').next().unwrap_or("").chars().count() + 1;
            ui.horizontal(|ui| {
                ui.label(&self.message);
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.label(format!(
                        "{} | {} | Tab:{}{} | 末尾換行:{} | 行 {line} : 列 {col} | {} 字元",
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
                let before = d.content.clone();
                let id = egui::Id::new(("editor", d.id));
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
                            let mut layouter =
                                move |ui: &egui::Ui, text: &dyn egui::TextBuffer, _width: f32| {
                                    if !cache.as_ref().is_some_and(|(s, theme, tabs, _)| {
                                        s == text.as_str() && *theme == dark && *tabs == tab_width
                                    }) {
                                        *cache = Some((
                                            text.as_str().to_owned(),
                                            dark,
                                            tab_width,
                                            highlight(
                                                text.as_str(),
                                                dark,
                                                tab_width,
                                                ui.fonts_mut(|fonts| {
                                                    fonts.row_height(&FontId::monospace(15.))
                                                }),
                                            ),
                                        ));
                                    }
                                    ui.fonts_mut(|fonts| {
                                        fonts.layout_job(
                                            cache
                                                .as_ref()
                                                .expect("initialized layout cache")
                                                .3
                                                .clone(),
                                        )
                                    })
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
                            paint_whitespace(
                                ui,
                                &output,
                                self.show_spaces,
                                self.show_eol,
                                d.content.newline,
                            );
                        });
                    });
                if before != d.content {
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
fn append_segment(
    job: &mut egui::text::LayoutJob,
    text: &str,
    color: Color32,
    tab_width: usize,
    line_height: f32,
) {
    let parts: Vec<&str> = text.split('\t').collect();
    for (index, part) in parts.iter().enumerate() {
        if index > 0 {
            job.append(
                "\t",
                0.,
                egui::TextFormat {
                    font_id: FontId::monospace(15. * tab_width as f32 / 4.),
                    line_height: Some(line_height),
                    color,
                    ..Default::default()
                },
            );
        }
        if !part.is_empty() {
            job.append(
                part,
                0.,
                egui::TextFormat {
                    font_id: FontId::monospace(15.),
                    line_height: Some(line_height),
                    color,
                    ..Default::default()
                },
            );
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
fn highlight(text: &str, dark: bool, tab_width: usize, line_height: f32) -> egui::text::LayoutJob {
    let mut job = egui::text::LayoutJob::default();
    let normal = if dark {
        Color32::from_rgb(220, 224, 232)
    } else {
        Color32::from_rgb(30, 38, 48)
    };
    let keyword = if dark {
        Color32::from_rgb(130, 180, 255)
    } else {
        Color32::from_rgb(30, 75, 175)
    };
    if text.len() > 256 * 1024 {
        append_segment(&mut job, text, normal, tab_width, line_height);
        job.wrap.max_width = f32::INFINITY;
        return job;
    }
    for segment in text.split_inclusive(|c: char| !c.is_alphanumeric() && c != '_') {
        let word = segment.trim_end_matches(|c: char| !c.is_alphanumeric() && c != '_');
        let color = if [
            "fn", "let", "mut", "pub", "use", "struct", "enum", "impl", "if", "else", "for",
            "while", "return", "match", "const", "true", "false", "class", "def", "import",
            "function", "var",
        ]
        .contains(&word)
        {
            keyword
        } else {
            normal
        };
        append_segment(&mut job, segment, color, tab_width, line_height);
    }
    job.wrap.max_width = f32::INFINITY;
    job
}
fn main() -> eframe::Result {
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
            }
            Ok(Box::new(app))
        }),
    )
}

#[cfg(test)]
mod app_tests {
    use super::*;
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
            capture: None,
            capture_frame: 0,
            show_spaces: false,
            show_eol: false,
            tab_width: 4,
            insert_spaces: false,
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
                let small = fonts.layout_job(highlight(text, false, 2, height));
                let large = fonts.layout_job(highlight(text, false, 8, height));
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
}
