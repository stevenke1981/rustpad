# InkPage 0.12 驗證

2026-10-10；Windows x64、Rust 1.99.0 MSVC；原生 host `x86_64-pc-windows-msvc`、既有預設依賴 features、Cargo 單一建置工作。以 0.11 的提交 d899601 為基礎，舊交付包保留；本輪不自動提交、推送或發布。

## Rust 技能與檢查

使用本機 rust-workbench → rust-engineering，另以 rust-quality 完成可重跑檢查。品質技能先產生 [命令計畫](qa/v12-rust-plan.json)，再執行並保存 [完整紀錄](qa/v12-rust-verification.json)，所有適用步驟 exit code 0：

```text
cargo fmt --package rustpad -- --check
cargo check --package rustpad --all-targets --locked --offline
cargo clippy --package rustpad --all-targets --locked --offline -- -D warnings
cargo test --package rustpad --all-targets --locked --offline
```

完整測試 101 passed、0 failed、1 ignored；grammar_licenses example target 0 tests。此 crate 沒有 library target，doctest 明列 not_applicable，不算通過。程式新增的五個文字處理測試及五個 GUI 測試涵蓋 Unicode、BOM／LF／CRLF／CR、選取邊界、排序／反轉／兩種去重、語言註解、錯誤不修改、阻擋狀態、分頁隔離、一次 undo／redo、軟折行行號／EOL、完整複製、貼上復原、摺疊書籤及縮放上下限／焦點／UI scale。既有存檔、衝突、搜尋、拖入、歷史及 session 回歸通過。

新增 GUI 行操作選單測試曾出現「處理後 Ctrl+Z 沒有回到原文」；定位為選單點擊清除焦點，改為 widget 處理輸入後重新要求編輯焦點，原 assertion 通過。新測試提供回歸證據，未宣稱所有新增測試都先在舊版跑過失敗。

```text
cargo build --release --locked -j1
cargo test --locked --offline -j1 windows_shell_drop -- --ignored --nocapture
python scripts/check_alignment.py
git diff --check
```

正式版建置及額外檢查 exit code 0。被一般測試忽略的 Windows Shell 探針另行執行，1 passed、0 failed；[輸出](qa/v12-native-drop.log)、[畫面](qa/v12-native-drop.png)。它建立自己的 App 視窗並送出真實 WM_DROPFILES，驗證 Unicode／emoji 名稱、超過 260 字元的路徑、多檔、無效項目、保留未存內容及重複拖入；未操作 Explorer 或實體游標。固定歷史矩陣仍為 29/36。

## 真實程式畫面

以下均為正式版自己的合成文件，經 OpenGL framebuffer 截圖後逐張檢視；[執行紀錄](qa/v12-capture-verification.json) 保存參數與 exit code。這些畫面與 egui raw-input 測試不等於實體鍵鼠驗收。

| 畫面 | 檢查 |
|---|---|
| [自動換行亮色](qa/v12-wrap-light.png) | 1350×900 physical；長註解、CJK／emoji 字串、超長單字；8 個實際行號、真正 CRLF 與單一 EOF |
| [窄暗色](qa/v12-wrap-dark.png) | 950×525 physical／760×420 logical；換行、書籤、行號與狀態可讀 |
| [22 pt 放大](qa/v12-wrap-zoom.png) | 編輯區與行號放大；工具列／選單不變；重新依寬度排版 |
| [英文](qa/v12-wrap-en.png)、[日文](qa/v12-wrap-ja.png) | 窄視窗介面及狀態文字；文件的中文原文不翻譯 |
| [註解](qa/v12-comments.png) | 選取兩行加上 Rust 註解、保留 Tab、外部文字及 BOM／CRLF；dirty 與選取狀態 |
| [排序與去重](qa/v12-lines.png) | 只處理選取範圍；apple、zebra、中文排序；保留範圍上下文字 |
| [放大與摺疊](qa/v12-fold-zoom.png) | 22 pt；gutter 1→7 保持來源行號，摺疊不產生假文字 |

四語系 catalog／placeholder 與新增命令文字檢查通過。原創圖示資源 [驗證](qa/v12-icon-resources.json) 通過：16、20、24、32、40、48、64、128、256 九種尺寸，ProductName InkPage，FileVersion／ProductVersion 0.12.0。

## 交付與界限

正式 EXE 8,120,320 bytes，舊 0.11 EXE 8,001,024 bytes，增加 119,296 bytes（約 1.49%）；只有檔案大小比較，沒有啟動或效能改善宣稱。可攜包使用白名單，包含 EXE、公開文件、原創素材、選定的本輪合成驗證資料及授權；排除 settings、session、lock、build cache 及使用者文件。套件 CRC、EXE bytes／SHA-256、解壓後執行與資料清單另存交付驗證 JSON。

原生實體鍵鼠／系統對話框／剪貼簿、完整 IME 及 Linux GUI 未執行；遠端 CI 未針對這個本機未提交 patch 執行。沒有任意 feature 組合、跨 target runtime、Miri 或全面安全／效能保證。顯示設定本次啟動共用、HTML／CSS 類註解逐行包裹、重排清除書籤等產品界限見 [DIFF-0.12](DIFF-0.12.md)。
