# RustPad（暫名）

原創 Rust / egui 桌面文字編輯器，致敬 Notepad++ 的緊湊傳統桌面工作流程。選單、工具列、多分頁、行號、搜尋取代、亮暗主題、繁中介面；無 WebView。原創程式為 MIT，未複製 Notepad++ GPL 程式、標誌或素材。

```powershell
cargo run --locked -j1
cargo build --release --locked -j1
cargo fmt --check
cargo clippy --locked -j1 --all-targets -- -D warnings
cargo test --locked -j1
```

Windows 需 Rust MSVC、MSVC build tools、Windows SDK。Linux 需 C compiler、pkg-config、OpenGL/X11/Wayland 開發套件及原生檔案對話框 portal。Ubuntu CI 套件見 `.github/workflows/ci.yml`。繁中字型讀取 Windows 微軟正黑體或 Linux Noto CJK；字型不隨程式散布，Linux 建議安裝 `fonts-noto-cjk`。

快捷鍵：Ctrl+N 新建、Ctrl+O 開啟、Ctrl+S 儲存、Ctrl+Shift+S 另存、Ctrl+W 關閉、Ctrl+Z 復原、Ctrl+Y / Ctrl+Shift+Z 重做、Ctrl+F / H 搜尋取代、F3 下一個。搜尋欄擁有自己的 Ctrl+Z，不會誤動編輯區歷史。

0.2 支援副檔名/shebang 與手動語言選擇，16 種模式含 Rust、Python、JS/TS、HTML/CSS、JSON/YAML/TOML、Markdown、C/C++、Shell/PowerShell。各語言規則含跨行状態，背景著色、快取排版；超過 256 KiB 明示純文字。詳見 [0.2 功能與差距](V02.md)、[規格與架構](SPEC.md)、[驗證記錄](QA.md)、[效能實測](PERFORMANCE.md)。

嚴格 UTF-8 / UTF-8 BOM，保留 LF、CRLF、舊式 CR；非 UTF-8、NUL、混合 EOL 明示拒絕，不做 lossy 解碼。檔案上限 2 MiB、20,000 行、單行 16 KiB。搜尋大小寫敏感、literal、Unicode 安全。行列數按 Unicode scalar 計數。

「格式」可改 BOM/EOL；「檢視」顯示空格、Tab、CRLF/LF/CR/EOF，標記不進入複製/儲存內容。「空白」提供 1–8 欄 Tab stops、Tab 插入空格、選取或全文轉換、清理 ASCII 尾端空白、加入/移除一個最後換行；保留其他 Unicode 空白、BOM、EOL，內容修改可 undo，顯示設定不改 dirty。

同目錄暫存寫完同步後原子替換，失敗保留旧檔；外部變更衝突拒絕覆寫，請另存。尚未提供插件相容、regex、多游標、矩形選取、Big5/UTF-16 或巨型檔案效能保證。Windows 真實 framebuffer 已檢視；原生鍵鼠/對話框/剪貼簿/完整 IME 與 Linux 桌面 GUI 尚未實测，詳見 QA。

鎖定依賴與授權見 Cargo.lock、[THIRD_PARTY_LICENSES.md](THIRD_PARTY_LICENSES.md)、[語法資料授權](licenses/SYNTAXES.md)。
