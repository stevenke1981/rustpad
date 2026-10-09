# RustPad（暫名）

原創 Rust / egui 緊湊桌面文字編輯器，致敬傳統 Notepad++ 工作流程。繁中選單、工具列、多分頁、行號、搜尋取代、亮暗主題與狀態列。原創程式採 MIT，無 Notepad++ 素材或 GPL 程式移植。

```powershell
cargo run -j1
cargo build --release -j1
cargo fmt --check
cargo clippy -j1 --all-targets -- -D warnings
cargo test -j1
```

Windows 需 Rust MSVC、MSVC build tools、Windows SDK。Linux 需 C compiler、pkg-config、OpenGL/X11/Wayland 開發套件及檔案對話框 portal。Ubuntu CI 配置見 `.github/workflows/ci.yml`；Linux 尚未本機實測。

繁中字型讀系統 Windows 微軟正黑體或 Linux Noto CJK／文泉驛。不包含或散布這些字型；Linux 建議安裝 `fonts-noto-cjk`。

快捷鍵：Ctrl+N 新建、Ctrl+O 開啟、Ctrl+S 儲存、Ctrl+Shift+S 另存、Ctrl+W 關頁、Ctrl+Z 復原、Ctrl+Y / Ctrl+Shift+Z 重做、Ctrl+F / H 搜尋取代、F3 下一個。支援 TextEdit 的選取、剪貼簿與 Tab 輸入。

首版只接受嚴格 UTF-8、UTF-8 BOM，保留 LF 或 CRLF，大小上限 2 MiB、20,000 行、單行 16 KiB。無效 UTF-8、NUL、混合換行、單獨 CR 會明示拒絕，不做 lossy 轉碼。BOM／換行可從「格式」切換。搜尋大小寫敏感、literal，不支援 regex。行列以 Unicode scalar 計數。

檔案寫入同目錄暫存後同步並替換，失敗不毀原檔。儲存前會檢查外部內容變更；衝突時拒絕覆寫，請另存新檔。未提供插件相容、矩形選取、多游標、大檔極致效能或完整 IME 保證。測試證據與限制見 `QA.md`，功能架構見 `SPEC.md`。
