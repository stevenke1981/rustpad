# InkPage 0.13 驗證

5900X／Windows x64 MSVC、Rust 1.99.0，2026-10-10，Cargo -j1。繼承 0.12，沒有另建專案。完整 source／patch 備份在專案外的 backups/InkPage-v12-before-v13；0.12 保存提交 0cca92f。

本輪三個 GUI 回歸先 RED：選取感知 Ctrl+D、選取行移動、目前行選取。另抓到 egui 方向鍵導致焦點轉移並修正輸入入口。最終完整測試 111 passed、0 failed、1 ignored（Windows Shell drop 探針本輪未執行）。新增十個測試覆蓋 Unicode／組合字／emoji、選取與下一行邊界、末行有無 newline、BOM 與三種 EOL、無效索引／超限不修改、分頁隔離、阻擋狀態、一次 undo／redo、選單與焦點、IME event 不執行命令；選取目前行不 dirty。既有存檔失敗保留原檔、外部衝突、搜尋替換、session、0.12 換行縮放及註解排序回歸全部通過。

```text
cargo fmt --check
cargo clippy --locked --offline -j1 --all-targets -- -D warnings
cargo test --locked --offline -j1
cargo build --release --locked --offline -j1
python scripts/check_alignment.py
git diff --check
```

全部成功。最初資料測試在沙箱返回 os error 5（暫存目錄及 canonicalize／遍歷限制）；未修改產品程式或全域設定，使用同程序 TEMP／TMP 指向 target/qa-temp-v13，依批准執行同專案純資料測試後通過。這不是原生電腦控制或其替代。fmt／clippy 也使用同一批准環境。

正式 release 經自己的 OpenGL framebuffer 截圖並逐張檢視：

| 畫面 | 結果 |
|---|---|
| [行移動亮色](qa/v13-move-light.png) | 1350×900 physical；兩行已移至上方、选取跟隨、外部文字保留、94 bytes／5 行、BOM／CRLF |
| [行移動窄暗色](qa/v13-move-dark.png) | 950×525 physical；行號、選取、dirty 與分段狀態可讀 |
| [選取複製](qa/v13-duplicate.png)、[窄暗色](qa/v13-duplicate-dark.png) | 只複製甲🙂及組合字，選取副本，第二行不變；4 scalar 選取、BOM／CRLF |
| [英文](qa/v13-move-en.png)、[日文](qa/v13-move-ja.png) | 窄視窗的翻譯、提示及狀態布局 |

QA 使用 --no-session --no-settings，非啟用 viewport 與 hidden 啟動，未搶佔焦點、未關閉使用者視窗。沒有原生鍵鼠或 IME 實體輸入，不能用 framebuffer 或 egui event 測試宣稱這些通過。原生 Shell drop 的 0.12 歷史證據保留，不算本輪執行。

release EXE 8,131,072 bytes；既有 0.12 為 8,120,320 bytes，增加 10,752 bytes（約 0.13%）。沒有啟動速度／效能改善宣稱。嵌入圖示與版本資源 0.13.0 驗證成功。可攜包只含 EXE、公開文件、原創素材、合成 QA 與授權，無 settings／session、私人文件或 build cache。解壓執行、CRC 與 EXE 精確 bytes／SHA 另存交付驗證。

本輪最終提交的 Windows／Ubuntu CI 結果會另存交付紀錄，必須與新 SHA 相同，不沿用舊 CI。固定矩陣仍 29/36；原生 GUI 七項、完整 IME、Linux GUI 未測。操作界限見 DIFF-0.13.md。
