# InkPage 0.11 驗證

2026-10-09；5900X／Windows 11 x64、Rust 1.99.0 MSVC，cargo -j1。保留既有 0.10.1 修改為獨立提交 4ba1e69；備份與舊包未覆蓋。

本機完整測試 91 passed、0 failed、1 ignored（Windows Shell drop 原生探針，本輪未重跑）。cargo fmt --check、cargo clippy --locked -j1 --all-targets -- -D warnings、cargo test --locked -j1、cargo build --release --locked -j1、功能矩陣檢查與 git diff --check 全通過。三個新增回歸先確認 RED：Ctrl+Tab、未存分頁中鍵關閉、Unicode/BOM/CRLF 狀態統計，再修正到 GREEN。另測視窗選單／狀態雙擊跳行；原有格式往返、Unicode 搜尋與替換、undo/redo、取消關閉、儲存失敗保留原檔及 session 測試全部通過。

使用正式 release 程式與自己的合成文件，經 OpenGL framebuffer 截圖並逐張檢視：

| 畫面 | 檢查 |
|---|---|
| [更新前](qa/v11-before.png) | 0.10.1 同尺寸基準 |
| [亮色](qa/v11-light.png)、[暗色](qa/v11-dark.png) | 1350×900 physical；選單、圖示分組、搜尋及狀態資訊 |
| [窄亮色](qa/v11-narrow-light.png)、[窄暗色](qa/v11-narrow-dark.png) | 950×525 physical／760×420 logical；八分頁、長檔名、dirty、選取、BOM／CRLF，狀態無裁切 |
| [工具提示](qa/v11-tooltip.png) | 原創新建圖示的 Ctrl+N tooltip；自己的 raw-input hover，不是 OS 輸入 |
| [英文](qa/v11-narrow-en.png)、[日文](qa/v11-narrow-ja.png) | 窄視窗翻譯及狀態布局 |

亮暗 gutter、圖示與繁中文字體可讀；分頁列右邊露出部分下一分頁是水平捲動提示，可從視窗選單到達所有分頁。字元／列資訊採 Unicode scalar，不宣稱字素或 Tab 視覺欄位。圖示資源驗證見 [JSON](qa/v11-icon-resources.json)：原創九種尺寸，ProductName InkPage、FileVersion／ProductVersion 0.11.0。

release EXE 8,001,024 bytes；同機保留的 0.10.1 EXE 7,910,400 bytes，增加 90,624 bytes（約 1.15%）。這是檔案大小比較，未量測啟動效能，也不宣稱更快。ZIP 僅含執行檔、公開文件、原創素材與授權，不含個人設定／工作階段。

固定核心矩陣仍為 29/36。CUA nativeApps=0、browser surfaces=3、errors=[]，原生 API disabled；沒有繞過限制。七項原生鍵鼠驗收、完整 IME 與 Linux GUI 仍未測；framebuffer 和 egui 測試不能代替這些項目。GitHub Windows／Ubuntu CI 必須核對本次最終提交；結果另存交付目錄，不沿用舊版結果。未建立 Release／tag。
