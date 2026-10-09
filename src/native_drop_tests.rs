//! Native message integration test. No cursor, Explorer, or injected egui input.
use crate::{App, core};
use eframe::egui;
use raw_window_handle::{HasWindowHandle, RawWindowHandle};
use std::{
    path::PathBuf,
    ptr,
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};
use windows_sys::Win32::{
    Foundation::{GlobalFree, HWND},
    System::Memory::{GMEM_MOVEABLE, GMEM_ZEROINIT, GlobalAlloc, GlobalLock, GlobalUnlock},
    UI::{
        Shell::{DROPFILES, DragFinish},
        WindowsAndMessaging::{
            GWL_EXSTYLE, GetWindowLongPtrW, PostMessageW, WM_DROPFILES, WS_EX_ACCEPTFILES,
        },
    },
};
use winit::platform::windows::EventLoopBuilderExtWindows;

struct Probe {
    app: App,
    hwnd: HWND,
    files: Vec<PathBuf>,
    phase: u8,
    received: bool,
    deadline: Instant,
    result: Arc<Mutex<Option<Result<(), String>>>>,
    screenshot: Option<PathBuf>,
}

fn post_drop(hwnd: HWND, paths: &[PathBuf]) {
    use std::os::windows::ffi::OsStrExt;
    let mut names = Vec::new();
    for path in paths {
        names.extend(path.as_os_str().encode_wide());
        names.push(0);
    }
    names.push(0);
    // The Shell owns this HGLOBAL after a successful PostMessage. The recipient
    // must read it with DragQueryFileW and release it with DragFinish.
    unsafe {
        let header_size = std::mem::size_of::<DROPFILES>();
        let memory = GlobalAlloc(GMEM_MOVEABLE | GMEM_ZEROINIT, header_size + names.len() * 2);
        assert!(!memory.is_null());
        let bytes = GlobalLock(memory).cast::<u8>();
        if bytes.is_null() {
            GlobalFree(memory);
            panic!("GlobalLock failed");
        }
        ptr::write(
            bytes.cast::<DROPFILES>(),
            DROPFILES {
                pFiles: header_size as u32,
                fWide: 1,
                ..Default::default()
            },
        );
        ptr::copy_nonoverlapping(
            names.as_ptr(),
            bytes.add(header_size).cast::<u16>(),
            names.len(),
        );
        GlobalUnlock(memory);
        if PostMessageW(hwnd, WM_DROPFILES, memory as usize, 0) == 0 {
            DragFinish(memory);
            panic!("PostMessageW failed");
        }
    }
}

impl eframe::App for Probe {
    fn raw_input_hook(&mut self, ctx: &egui::Context, input: &mut egui::RawInput) {
        eframe::App::raw_input_hook(&mut self.app, ctx, input);
        self.received |= self.app.drops_pending();
    }
    fn update(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
        eframe::App::update(&mut self.app, ctx, frame);
        if self.phase == 3 {
            return;
        }
        if self.phase == 0 {
            assert_ne!(
                unsafe { GetWindowLongPtrW(self.hwnd, GWL_EXSTYLE) } & WS_EX_ACCEPTFILES as isize,
                0
            );
            post_drop(self.hwnd, &self.files);
            self.phase = 1;
        }
        let done = self.received
            && !self.app.busy
            && !self.app.drops_pending()
            && self.app.docs.len() == 3;
        if done || Instant::now() >= self.deadline {
            let check = (|| {
                if !done {
                    return Err(format!(
                        "native drop timed out: {} tabs, status {}",
                        self.app.docs.len(),
                        self.app.message
                    ));
                }
                if self.app.docs[0].content.text != "keep unsaved 中文🙂"
                    || !self.app.docs[0].dirty()
                {
                    return Err("original dirty tab changed".into());
                }
                if self.phase == 1 {
                    for path in &self.files[..2] {
                        let canonical = path.canonicalize().unwrap();
                        let doc = self
                            .app
                            .docs
                            .iter()
                            .find(|doc| doc.path.as_ref() == Some(&canonical))
                            .ok_or_else(|| format!("missing tab: {}", path.display()))?;
                        if doc.content != core::read_file(path).unwrap() {
                            return Err("native Unicode path/content mismatch".into());
                        }
                    }
                    if self.app.drop_state.errors.len() != 3 {
                        return Err(format!(
                            "expected 3 visible errors, got {}",
                            self.app.drop_state.errors.len()
                        ));
                    }
                } else if !self.app.drop_state.errors.is_empty()
                    || self.app.docs[1].content.text != "中文🙂\n尾\ndirty"
                    || !self.app.docs[1].dirty()
                    || self.app.message != "拖入完成：開啟 0、已開啟 2、失敗 0、略過 1"
                {
                    return Err(format!(
                        "duplicate native drop lost edits or opened another tab: {}",
                        self.app.message
                    ));
                }
                Ok(())
            })();
            if self.phase == 1 && check.is_ok() {
                assert_eq!(
                    std::fs::read(&self.files[0]).unwrap(),
                    "\u{feff}中文🙂\r\n尾\r\n".as_bytes()
                );
                self.app.docs[1].content.text.push_str("dirty");
                std::fs::write(&self.files[0], b"changed\0disk").unwrap();
                post_drop(
                    self.hwnd,
                    &[
                        self.files[0].clone(),
                        self.files[0].clone(),
                        self.files[1].clone(),
                    ],
                );
                self.received = false;
                self.phase = 2;
                ctx.request_repaint();
                return;
            }
            let successful = check.is_ok();
            *self.result.lock().unwrap() = Some(check);
            self.phase = 3;
            if successful && self.screenshot.is_some() {
                self.app.capture = self.screenshot.take();
                ctx.request_repaint();
            } else {
                self.app.force_exit = true;
                ctx.send_viewport_cmd(egui::ViewportCommand::Close);
            }
        }
        ctx.request_repaint_after(Duration::from_millis(20));
    }
}

#[test]
#[ignore = "creates its own native window; run explicitly on Windows"]
fn windows_shell_drop_opens_unicode_and_long_paths_in_the_real_app() {
    let dir = tempfile::tempdir().unwrap();
    let one = dir.path().join("拖入 中文🙂.txt");
    let long = dir
        .path()
        .join("長路徑資料夾".repeat(20))
        .join("子目錄".repeat(40));
    std::fs::create_dir_all(&long).unwrap();
    let two = long.join("second🙂.txt");
    use std::os::windows::ffi::OsStrExt;
    assert!(two.as_os_str().encode_wide().count() > 260);
    let binary = dir.path().join("binary.txt");
    let missing = dir.path().join("missing.txt");
    std::fs::write(&one, "\u{feff}中文🙂\r\n尾\r\n").unwrap();
    std::fs::write(&two, "second\n").unwrap();
    std::fs::write(&binary, b"a\0b").unwrap();
    let files = vec![
        one.clone(),
        two.clone(),
        binary,
        missing,
        dir.path().to_owned(),
    ];
    let outcome = Arc::new(Mutex::new(None));
    let result = outcome.clone();
    let options = eframe::NativeOptions {
        event_loop_builder: Some(Box::new(|builder| {
            builder.with_any_thread(true);
        })),
        ..crate::native_options()
    };
    eframe::run_native(
        "InkPage native drop test",
        options,
        Box::new(move |cc| {
            let RawWindowHandle::Win32(handle) = cc.window_handle().unwrap().as_raw() else {
                panic!("expected Win32 window")
            };
            let mut app = App::new(cc)?;
            app.docs[0].content.text = "keep unsaved 中文🙂".into();
            Ok(Box::new(Probe {
                app,
                hwnd: handle.hwnd.get() as HWND,
                files,
                phase: 0,
                received: false,
                deadline: Instant::now() + Duration::from_secs(5),
                result,
                screenshot: std::env::var_os("INKPAGE_NATIVE_DROP_SCREENSHOT").map(PathBuf::from),
            }))
        }),
    )
    .unwrap();
    outcome
        .lock()
        .unwrap()
        .take()
        .expect("native probe ran")
        .unwrap();
    assert_eq!(std::fs::read(one).unwrap(), b"changed\0disk");
    assert_eq!(std::fs::read(two).unwrap(), b"second\n");
}
