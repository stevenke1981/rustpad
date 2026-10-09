use crate::{App, Content, Document, ResultEvent, core};
use eframe::egui;
use std::{collections::VecDeque, path::PathBuf};

#[derive(Default)]
pub struct State {
    queue: VecDeque<PathBuf>,
    running: bool,
    opened: usize,
    existing: usize,
    skipped: usize,
    pub errors: Vec<(String, String)>,
    pub errors_open: bool,
}
pub struct TabPayload(pub u64);
pub enum OpenOutcome {
    Opened(PathBuf, Content),
    Existing(PathBuf),
}
pub type OpenResult = Result<OpenOutcome, (String, String)>;

impl App {
    pub fn receive_drops(&mut self, files: Vec<egui::DroppedFile>) {
        if files.is_empty() {
            return;
        }
        if !self.drop_state.running {
            self.drop_state = State {
                running: true,
                ..State::default()
            };
        }
        for file in files {
            let name = file
                .path
                .as_ref()
                .map_or(file.name.clone(), |path| path.display().to_string());
            let Some(path) = file.path else {
                self.drop_state
                    .errors
                    .push((name, "不支援沒有本機路徑的拖入項目".into()));
                continue;
            };
            if self.drop_state.queue.contains(&path) {
                self.drop_state.skipped += 1;
                continue;
            }
            if self.drop_state.queue.len() >= 32 {
                self.drop_state
                    .errors
                    .push((name, "拖入佇列已達 32 個檔案上限".into()));
                continue;
            }
            self.drop_state.queue.push_back(path);
        }
    }
    pub fn process_drops(&mut self, ctx: &egui::Context) {
        if !self.drop_state.running
            || self.busy
            || self.exit
            || self.force_exit
            || self.pending_close.is_some()
            || self.session.as_ref().is_some_and(|s| s.loading)
        {
            return;
        }
        let Some(path) = self.drop_state.queue.pop_front() else {
            self.drop_state.running = false;
            self.drop_state.errors_open = !self.drop_state.errors.is_empty();
            self.message = format!(
                "拖入完成：開啟 {}、已開啟 {}、失敗 {}、略過 {}",
                self.drop_state.opened,
                self.drop_state.existing,
                self.drop_state.errors.len(),
                self.drop_state.skipped
            );
            return;
        };
        let existing: Vec<_> = self
            .docs
            .iter()
            .filter_map(|doc| doc.path.clone())
            .collect();
        let tx = self.tx.clone();
        let ctx = ctx.clone();
        self.busy = true;
        self.message = "拖入開啟中…".into();
        std::thread::spawn(move || {
            let result = (|| {
                let canonical = path.canonicalize().map_err(|e| e.to_string())?;
                if !std::fs::metadata(&canonical)
                    .map_err(|e| e.to_string())?
                    .is_file()
                {
                    return Err("請拖入一般檔案；資料夾不會遞迴開啟".into());
                }
                if existing.contains(&canonical) {
                    return Ok(OpenOutcome::Existing(canonical));
                }
                core::read_file(&canonical).map(|content| OpenOutcome::Opened(canonical, content))
            })()
            .map_err(|error| (path.display().to_string(), error));
            let _ = tx.send(ResultEvent::Drop(result));
            ctx.request_repaint();
        });
    }
    pub fn poll_drop(&mut self, result: OpenResult) {
        match result {
            Ok(OpenOutcome::Opened(path, content)) => {
                if let Some(index) = self
                    .docs
                    .iter()
                    .position(|doc| doc.path.as_ref() == Some(&path))
                {
                    self.activate(index);
                    self.drop_state.existing += 1;
                } else if self.docs.len() >= 32 {
                    self.drop_state.errors.push((
                        path.display().to_string(),
                        "拖入檔案最多開啟 32 個分頁".into(),
                    ));
                } else {
                    self.docs
                        .push(Document::new(self.next_id, Some(path), content));
                    self.next_id += 1;
                    self.activate(self.docs.len() - 1);
                    self.drop_state.opened += 1;
                }
            }
            Ok(OpenOutcome::Existing(path)) => {
                if let Some(index) = self
                    .docs
                    .iter()
                    .position(|doc| doc.path.as_ref() == Some(&path))
                {
                    self.activate(index);
                    self.drop_state.existing += 1;
                } else {
                    // The user may have closed the tab while canonicalization ran.
                    // Retry through the same validated reader instead of losing it.
                    self.drop_state.queue.push_front(path);
                }
            }
            Err(error) => self.drop_state.errors.push(error),
        }
    }
    pub fn move_tab(&mut self, source: u64, target: u64, after: bool) {
        if source == target {
            return;
        }
        let Some(from) = self.docs.iter().position(|doc| doc.id == source) else {
            return;
        };
        let Some(to) = self.docs.iter().position(|doc| doc.id == target) else {
            return;
        };
        let active_id = self.docs[self.active].id;
        let slot = to + usize::from(after);
        let destination = slot - usize::from(from < slot);
        if from == destination {
            return;
        }
        let doc = self.docs.remove(from);
        self.docs.insert(destination, doc);
        self.active = self
            .docs
            .iter()
            .position(|doc| doc.id == active_id)
            .unwrap();
        self.message = "分頁順序已更新".into();
    }
    pub fn drops_pending(&self) -> bool {
        self.drop_state.running
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn dropped(path: PathBuf) -> egui::DroppedFile {
        egui::DroppedFile {
            path: Some(path),
            ..Default::default()
        }
    }
    fn drain(app: &mut App) {
        let ctx = egui::Context::default();
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
        while app.drops_pending() || app.busy {
            app.poll();
            app.process_drops(&ctx);
            assert!(
                std::time::Instant::now() < deadline,
                "drop worker timed out"
            );
            std::thread::sleep(std::time::Duration::from_millis(1));
        }
    }
    #[test]
    fn multiple_drops_preserve_dirty_tabs_and_reject_invalid_files() {
        let dir = tempfile::tempdir().unwrap();
        let one = dir.path().join("中文🙂.txt");
        let two = dir.path().join("two.txt");
        let invalid = dir.path().join("binary.txt");
        std::fs::write(&one, "\u{feff}中文🙂\r\n尾\r\n").unwrap();
        std::fs::write(&two, "two\n").unwrap();
        std::fs::write(&invalid, b"a\0b").unwrap();
        let mut app = crate::app_tests::app();
        app.docs[0].content.text = "未儲存🙂".into();
        app.receive_drops(vec![
            dropped(one.clone()),
            dropped(two),
            dropped(invalid),
            dropped(dir.path().to_owned()),
        ]);
        drain(&mut app);
        assert_eq!(app.docs.len(), 3);
        assert_eq!(app.docs[0].content.text, "未儲存🙂");
        assert!(app.docs[1].content.bom);
        assert_eq!(app.docs[1].content.newline, crate::core::Newline::Crlf);
        assert_eq!(app.drop_state.errors.len(), 2);
        app.docs[1].content.text.push_str("dirty");
        std::fs::write(&one, b"changed\0disk").unwrap();
        app.receive_drops(vec![dropped(one.clone()), dropped(one)]);
        drain(&mut app);
        assert_eq!(app.docs.len(), 3);
        assert_eq!(app.active, 1);
        assert!(app.docs[1].content.text.ends_with("dirty"));
        assert_eq!(app.drop_state.existing, 1);
        assert_eq!(app.drop_state.skipped, 1);
        assert!(app.drop_state.errors.is_empty());
    }
    #[test]
    fn busy_operations_defer_drops_and_queue_limits_are_visible() {
        let mut app = crate::app_tests::app();
        app.busy = true;
        app.receive_drops(
            (0..40)
                .map(|n| dropped(PathBuf::from(format!("{n}.txt"))))
                .collect(),
        );
        app.process_drops(&egui::Context::default());
        assert_eq!(app.drop_state.queue.len(), 32);
        assert_eq!(app.drop_state.errors.len(), 8);
        app.busy = false;
        app.pending_close = Some(app.docs[0].id);
        app.process_drops(&egui::Context::default());
        assert_eq!(app.drop_state.queue.len(), 32);
    }
    #[test]
    fn reorder_preserves_active_identity_history_cursor_and_close_target() {
        let mut app = crate::app_tests::app();
        app.new_doc();
        app.new_doc();
        let id = app.docs[1].id;
        app.activate(1);
        let before = app.docs[1].content.clone();
        app.docs[1].history.record(before);
        app.docs[1].content.text = "中文🙂".into();
        app.docs[1].cursor = 3;
        app.docs[1].selected = (1, 3);
        app.match_range = Some((id, (1, 3)));
        app.pending_close = Some(id);
        app.move_tab(id, app.docs[2].id, true);
        assert_eq!(app.active, 2);
        assert_eq!(app.docs[app.active].id, id);
        assert_eq!(app.docs[2].selected, (1, 3));
        assert_eq!(app.docs[2].cursor, 3);
        assert_eq!(app.pending_close, Some(id));
        assert_eq!(app.match_range, Some((id, (1, 3))));
        assert_eq!(app.snapshot().active, 2);
        let doc = &mut app.docs[2];
        doc.history.undo(&mut doc.content);
        assert!(doc.content.text.is_empty());
        app.move_tab(id, app.docs[0].id, false);
        assert_eq!(app.active, 0);
        let ids: Vec<_> = app.docs.iter().map(|doc| doc.id).collect();
        app.move_tab(id, id, true);
        app.move_tab(u64::MAX, id, false);
        assert_eq!(ids, app.docs.iter().map(|doc| doc.id).collect::<Vec<_>>());
    }
}
