# 墨頁 InkPage 0.5 驗證

Windows NT 10.0.22631 / Rust 1.99.0 MSVC / cargo -j1。fmt --check、clippy --all-targets -D warnings、locked tests、release build。Windows 61 tests；Ubuntu CI 另有 Unix symlink loop test，預期 62；最終通過數以本次 GitHub Actions 終態為準。

RED → GREEN：bookmarks stub 的導航循環與修改後映射兩個 tests 失敗；lines stub 的選取合併與 Unicode 字素分割兩個 tests 失敗；最小實作後通過。上限/非法條件 test 初始 stub 已返回 Err，沒有宣稱這項曾 RED。另有 app 分頁隔離/非 dirty/Unicode 游標/編輯-undo 書籤移動，以及行命令一筆 undo/redo/失敗選取不變，這兩項是在實作後補回歸。

既有 54 Windows / 55 Unix tests 不刪除，包含中文/BOM/CRLF/LF、儲存失敗不毀原檔、regex captures/零長度/18,000 行、stale worker、binary/symlink/多檔定位與 dirty 分頁保护。本輪新增 7 tests。字素 case 包含 e+combining acute、🇹🇼、ZWJ family；不以 glyph 外觀判斷 bytes 是否安全。

## 真正 Windows renderer

release exe 的 eframe/glow framebuffer Screenshot 事件，opt-in fixtures 後自動退出；逐張檢視，非 HTML、非純 selftest：

- qa/v05-bookmarks.png：600 行 fixture 的原第 350 行，在前面插入一行后移到第 351 行，gutter `*`、行號及状态對應。
- qa/v05-lines-join.png：選取中文/emoji 行合併，外側行不變、UTF-8 BOM/CRLF、dirty 狀態。
- qa/v05-lines-split.png：2 字素寬度分割，資料中的 combining/flag/family 字素完整，保留空白行/外側行/BOM/CRLF。
- qa/v05-about.png：關於視窗標題／工具列產品名與關於 0.5 畫面。

重現：`rustpad.exe --screenshot <output.png> --qa-flow bookmarks|lines-join|lines-split|about --dark`；可攜版改以 InkPage.exe 執行；about 另以亮色檢查。自動 QA model 動作不等於真人按鍵操作。

**NOT EXECUTED**：原生鍵鼠/Clipboard/系統檔案與資料夾對話框、完整繁中 IME、Linux desktop GUI、Windows junction 實體 fixture、硬即時取消／並行 symlink 攻擊、商標或全面同名清查。Linux build/test 不冒充 Linux GUI。已清除書籤不由內容 undo 恢復；無持久書籤／session restore；摺疊仍缺少。

歷史：QA-0.1..0.4.md；效能與大小見 PERFORMANCE-0.5.md。完成定義仍為既定 data/widget/renderer 等級；固定核心矩陣 25/36，非全產品比例。

已觀察視覺限制：目前 OS 字型／egui renderer 將 ZWJ family 顯示成多個圖形；byte/grapheme 測試確認未插入換行於群集內，但不宣稱完整 emoji shaping。原生 OS 標題列不在 framebuffer 截圖中，名稱設定由 eframe run_native 參數確認。
