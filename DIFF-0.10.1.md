# InkPage 0.10.1

修正 Windows 檔案拖入入口。使用者回報 0.10 放開檔案後沒有成功開啟。檢查 0.10 實作與本地 winit 0.30.13 原始碼後，確認當時只有 OLE → winit → egui 的事件管線，沒有接收 Windows Shell 的 WM_DROPFILES。原生視窗回歸探針把真實 HDROP 投遞至主視窗，舊版逾時仍只有一個空白分頁；接上新入口後同一訊息流程成功開啟。這證實 Shell 訊息入口的缺口；沒有實際觀察使用者那一次拖曳所走的通道，不能斷言其全部原因。

參考使用者提供的 [PhotoCraft 開檔流程](https://github.com/stevenke1981/photocraft/blob/bb3108cfd28a1cc84a9ae3b83531891f36a736fd/crates/ui-egui/src/file_open.rs)（commit bb3108cfd28a1cc84a9ae3b83531891f36a736fd）：檔案拖入路徑交給正常開檔管線、逐檔回報失敗。該版本使用 eframe/egui 0.36 與 dropped file handles；InkPage 保持鎖定的 eframe 0.33.0，獨立實作相容的 Windows 接收器，未複製 PhotoCraft 程式碼或升級整套 GUI。

native_drop.rs 在主視窗建立時以 SetWindowSubclass 安装接收器，再呼叫 DragAcceptFiles。WM_DROPFILES 使用 DragQueryFileW 動態讀取 UTF-16 路徑，支援中文、emoji 和超過 260 字元的路徑；讀取後 DragFinish 釋放原生資源，送入佇列並喚醒畫面。WM_NCDESTROY 清除視窗 handle、移除回呼；App 先結束時也移除回呼。註冊失敗會令啟動回報錯誤，避免顯示可用但實際不接收的視窗。僅影響本程式的視窗，未修改 RDP、系統權限、登錄或 Windows 訊息篩選。

Windows 停用 winit 的 OLE 接收器，由 Shell 檔案接收器處理本機檔案；其他平台保留既有 winit 拖入設定。Windows Shell 入口沒有 hovered_files 提示，放開後顯示開啟中與結果。此版不提供虛擬附件、瀏覽器內容或選取文字的跨程式拖入；檔案格式限制、32 項佇列與 32 個分頁上限仍沿用既有規格。

所有檔案事件移到 App::raw_input_hook，在每次原始輸入更新時收取一次，再由現有背景讀檔與逐檔錯誤處理開啟。忙碌、工作階段載入與關閉確認期間保留佇列；重複路徑保留 dirty 分頁與歷史。四語系、分頁排序、圖示與工作階段格式保留。

Cargo.toml 明列原鎖檔已有的 raw-window-handle 0.6.2、windows-sys 0.61.2（Windows 平台），原生測試使用已有 winit 0.30.13 與 Windows 記憶體 API。未加入新套件版本，既有依賴授權清單已涵蓋它們。Windows 執行檔版本資源更新為 0.10.1，原始碼 binary 名仍為 rustpad，可攜版為 InkPage.exe。

Windows Shell API 行為依據：[DragAcceptFiles](https://learn.microsoft.com/en-us/windows/win32/api/shellapi/nf-shellapi-dragacceptfiles)、[WM_DROPFILES](https://learn.microsoft.com/en-us/windows/win32/shell/wm-dropfiles)、[SetWindowSubclass](https://learn.microsoft.com/en-us/windows/win32/api/commctrl/nf-commctrl-setwindowsubclass)。驗證範圍見 QA-0.10.1.md。
