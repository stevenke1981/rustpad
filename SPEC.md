# RustPad 0.2 規格與架構

完整功能對照與限制：[V02.md](V02.md)。

選用 eframe/egui 0.33 + glow 原生 OpenGL：同碼 Windows/Linux，MIT/Apache-2.0，快速實現傳統桌面工具列與 TextEdit。iced 的 text_editor 也可行，但首版重用現有 egui 檔案/歷史流程；Slint 額外的編輯元件與授權選項增加實作成本。egui TextEdit 並非 Scintilla，IME、無障礙與大型檔案需獨立實測。

`core.rs`：內部 LF 正規化的 Content（BOM/EOL 分開保存）、嚴格解碼、同目錄原子儲存、Unicode 搜尋與替換、Tab stops 轉換、history（最多 64 筆/16 MiB）。不依赖 UI。

`main.rs`：Document 與穩定分頁 ID、UI、檔案對話框/I/O channel workers、dirty 關閉/退出確認。讀寫期間 UI 可繪製，儲存回覆按文件 ID 與已存 snapshot 比對，避免覆蓋新修改的 dirty。

`syntax.rs`：syntect/two-face 語法、原創亮暗色盤、單背景 worker、較新 ticket 淘汰舊任務、各分頁著色结果、LayoutJob cache。每次內容改變重新解析整份 <=256 KiB 文件，跨行 state 不重設；不是增量 lexer。語法 byte ranges 與 TextEdit char cursors 分開；LayoutJob.text 原始 bytes 不變。Tab glyph 按下一個邏輯停止點調整，維持固定行高。

`perf.rs`：明確啟用的啟動/排版探針；`scripts/measure.ps1` 五次取中位數；不作原生鍵鼠或 IME 驗證。

輸入與輸出 2 MiB / 20,000 行 / 單行 16 KiB，開檔最多讀上限+1 byte。NUL、非法 UTF-8、混合 EOL 拒絕，錯誤不改原檔。原子儲存 write_all/sync_all/persist，失敗不先刪舊檔；外部變更比對仍存在檢查至替換間的競態。搜尋/排版/編輯仍需 UI 執行，未承諾巨型檔案流暢。

官方參考：https://github.com/notepad-plus-plus/notepad-plus-plus （GPLv3）；RustPad 原創程式 MIT。語法資料由鎖定官方 crates 提供，授權另列，不使用 Notepad++ 程式碼/圖示/品牌。
