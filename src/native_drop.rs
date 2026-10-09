//! Windows Shell file drops, delivered directly to the shared opening queue.
use eframe::egui;
use raw_window_handle::{HasWindowHandle, RawWindowHandle};
use std::{cell::Cell, ffi::OsString, io, os::windows::ffi::OsStringExt, ptr, sync::mpsc};
use windows_sys::Win32::{
    Foundation::{HWND, LPARAM, LRESULT, WPARAM},
    UI::{
        Shell::{
            DefSubclassProc, DragAcceptFiles, DragFinish, DragQueryFileW, HDROP,
            RemoveWindowSubclass, SetWindowSubclass,
        },
        WindowsAndMessaging::{WM_DROPFILES, WM_NCDESTROY},
    },
};

const SUBCLASS_ID: usize = 0x494e4b50;

struct State {
    hwnd: Cell<HWND>,
    tx: mpsc::Sender<Vec<egui::DroppedFile>>,
    ctx: egui::Context,
}

/// The boxed callback state stays at one address until its subclass is removed.
/// The HWND and Cell also keep this owner on the window's UI thread.
pub struct Bridge {
    state: Box<State>,
    rx: mpsc::Receiver<Vec<egui::DroppedFile>>,
}

impl Bridge {
    pub fn new(cc: &eframe::CreationContext<'_>) -> io::Result<Self> {
        let handle = cc.window_handle().map_err(io::Error::other)?;
        let RawWindowHandle::Win32(handle) = handle.as_raw() else {
            return Err(io::Error::other(
                "InkPage file drop requires a Win32 window",
            ));
        };
        let hwnd = handle.hwnd.get() as HWND;
        let (tx, rx) = mpsc::channel();
        let state = Box::new(State {
            hwnd: Cell::new(hwnd),
            tx,
            ctx: cc.egui_ctx.clone(),
        });
        // Safety: called on the window's creating thread. State stays boxed,
        // and Drop / WM_NCDESTROY remove the callback before it can be freed.
        unsafe {
            if SetWindowSubclass(
                hwnd,
                Some(window_proc),
                SUBCLASS_ID,
                &*state as *const State as usize,
            ) == 0
            {
                return Err(io::Error::other(
                    "Could not register the InkPage file drop receiver",
                ));
            }
            DragAcceptFiles(hwnd, 1);
        }
        Ok(Self { state, rx })
    }

    pub fn take_files(&self) -> Vec<egui::DroppedFile> {
        self.rx.try_iter().flatten().collect()
    }
}

impl Drop for Bridge {
    fn drop(&mut self) {
        let hwnd = self.state.hwnd.get();
        if !hwnd.is_null() {
            // Safety: the owner remains on the UI thread. A destroyed window
            // clears hwnd in WM_NCDESTROY, preventing reuse of a stale handle.
            unsafe {
                DragAcceptFiles(hwnd, 0);
                RemoveWindowSubclass(hwnd, Some(window_proc), SUBCLASS_ID);
            }
        }
    }
}

unsafe extern "system" fn window_proc(
    hwnd: HWND,
    message: u32,
    wparam: WPARAM,
    lparam: LPARAM,
    id: usize,
    reference: usize,
) -> LRESULT {
    // Safety: Bridge owns this stable allocation for the callback's lifetime.
    let state = unsafe { &*(reference as *const State) };
    if message == WM_DROPFILES {
        let hdrop = wparam as HDROP;
        // Safety: Windows transfers ownership of HDROP to the recipient.
        let count = unsafe { DragQueryFileW(hdrop, u32::MAX, ptr::null_mut(), 0) };
        let mut files = Vec::new();
        for index in 0..count {
            let length = unsafe { DragQueryFileW(hdrop, index, ptr::null_mut(), 0) };
            if length == 0 {
                continue;
            }
            let mut name = vec![0u16; length as usize + 1];
            let copied =
                unsafe { DragQueryFileW(hdrop, index, name.as_mut_ptr(), name.len() as u32) };
            if copied > 0 {
                files.push(egui::DroppedFile {
                    path: Some(OsString::from_wide(&name[..copied as usize]).into()),
                    ..Default::default()
                });
            }
        }
        unsafe { DragFinish(hdrop) };
        if !files.is_empty() && state.tx.send(files).is_ok() {
            state.ctx.request_repaint();
        }
        return 0;
    }
    if message == WM_NCDESTROY {
        state.hwnd.set(ptr::null_mut());
        unsafe { RemoveWindowSubclass(hwnd, Some(window_proc), id) };
    }
    // Safety: all other messages continue through eframe/winit's window proc.
    unsafe { DefSubclassProc(hwnd, message, wparam, lparam) }
}
