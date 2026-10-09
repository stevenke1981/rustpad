# RustPad 0.1 規格／架構

暫名 RustPad，原創 Rust 桌面文字編輯器。參考 Notepad++ 的緊湊資訊布局；不含其程式、標誌、圖示或素材。Notepad++ 官方 LICENSE 目前為 GPLv3；本專案原創部分 MIT。

## 選型

eframe/egui 0.33，glow 原生 OpenGL，Windows/Linux 共用程式，無 WebView。egui 適合快速建立選單、工具列、分頁與狀態列，MIT OR Apache-2.0。iced 的 text_editor 可提供 IME/highlight，但完整編輯命令與 undo 整合需額外工作；Slint 的授權選項與自訂編輯器成本較高。TextEdit 並非 Scintilla；IME、無障礙與大檔效能尚需專門驗證。

## 功能對照

| 傳統編輯器功能 | 首版 |
|---|---|
| 新建／開啟／儲存／另存 | 背景執行檔案對話框與 I/O |
| 多分頁與未儲存狀態 | 穩定文件 ID；關閉確認；視窗退出確認 |
| Undo/redo | 各文件獨立歷史，最多 64 筆／16 MiB；全部取代為一次操作 |
| 搜尋取代 | 大小寫敏感 literal 搜尋、循環下一個、單項／全部取代，Unicode char 邊界 |
| 編碼／換行 | 嚴格 UTF-8，可切 BOM、LF/CRLF；不猜 Big5/UTF-16 |
| 語法高亮 | 基礎關鍵字；並非完整語言解析 |
| 行號／行列／狀態 | Unicode scalar 計數；不等於顯示寬度或 grapheme |
| 亮暗／繁中 | 可切換；系統 CJK 字型，不散布 Windows 字型 |
| 插件／多游標／矩形選取 | 不支援 |

## 架構及安全

`core.rs`：Content（正規化 LF、BOM、換行）、解編碼、原子儲存、Unicode 搜尋、history。`main.rs`：Document、UI、channel workers。每個文件用穩定 ID 保留 TextEdit 狀態。

首版檔案與輸出上限 2 MiB、20,000 行、單行 16 KiB，開檔最多讀上限+1 bytes。禁止 NUL、無效 UTF-8、混合換行；錯誤不更動原檔。同目錄暫存、write_all、sync_all、persist 原子替換；不先清空目標。失敗保留 dirty，儲存中修改的新內容不會誤標為已儲存。避免大檔效能宣稱。檔案 I/O 與對話框在背景執行；文字布局仍在 UI 執行。

已知待補：完整語法、持久偏好、完整 IME／無障礙 QA、捲動虛擬化、Linux 實機 QA。首版不適合多人同時修改同一檔案。

## 官方參考

新增空白處理：可見標記由 galley glyph 座標覆畫，LayoutJob.text 保持原 bytes／char 數；不改 TextEdit 游標映射，不把符號寫入檔案。Tab 字寬透過單 Tab 字型尺度調整且固定行高，1–8 固定空格寬；暫不提供 tab stops 或視覺自動折行。內容轉換使用 Unicode scalar 範圍，驗證完整候選內容後才提交並記一筆 undo。行尾清理限定 U+0020 與 Tab；檔尾換行操作保留其他空白行。預設任何清理均不自動執行。

- https://github.com/notepad-plus-plus/notepad-plus-plus
- https://github.com/notepad-plus-plus/notepad-plus-plus/blob/master/LICENSE
- https://docs.rs/eframe/0.33.0/eframe/
- https://docs.rs/egui/0.33.0/egui/widgets/text_edit/struct.TextEdit.html
- https://docs.rs/tempfile/3.23.0/tempfile/struct.NamedTempFile.html
