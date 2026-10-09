# 墨頁 InkPage

0.8 已補括號跳轉、縮排／自動縮排、工作階段還原與區塊摺疊。固定核心覆蓋率 29/36（80.6%），不是完整 Notepad++ 產品百分比；[功能矩陣](ALIGNMENT.md)、[操作與限制](DIFF-0.8.md)、[本輪實測](PERFORMANCE-0.8.md)。

以 Rust / eframe-egui 編寫的原創桌面文字編輯器，致敬 Notepad++ 的緊湊傳統桌面工作流程。選單、工具列、多分頁、行號、搜尋取代、亮暗主題、繁中介面；無 WebView。InkPage 原創程式為 MIT，未複製 Notepad++ GPL 程式、標誌或素材。

```powershell
cargo run --locked -j1
cargo build --release --locked -j1
cargo fmt --check
cargo clippy --locked -j1 --all-targets -- -D warnings
cargo test --locked -j1
```

Windows 需 Rust MSVC、MSVC build tools、Windows SDK。Linux 需 C compiler、pkg-config、OpenGL/X11/Wayland 開發套件及原生檔案對話框 portal。Ubuntu CI 套件見 `.github/workflows/ci.yml`。繁中字型讀取 Windows 微軟正黑體或 Linux Noto CJK；字型不隨程式散布，Linux 建議安裝 `fonts-noto-cjk`。

新增：Ctrl+G 跳行、Shift+F3 上一個、Ctrl+D 複製行、Ctrl+Shift+K 刪除行、Ctrl+Shift+U/ Ctrl+U 選取轉大/小寫。

快捷鍵：Ctrl+N 新建、Ctrl+O 開啟、Ctrl+S 儲存、Ctrl+Shift+S 另存、Ctrl+W 關閉、Ctrl+Z 復原、Ctrl+Y / Ctrl+Shift+Z 重做、Ctrl+F / H 搜尋取代、F3 下一個。搜尋欄擁有自己的 Ctrl+Z，不會誤動編輯區歷史。

0.2 支援副檔名/shebang 與手動語言選擇，16 種模式含 Rust、Python、JS/TS、HTML/CSS、JSON/YAML/TOML、Markdown、C/C++、Shell/PowerShell。各語言規則含跨行状態，背景著色、快取排版；超過 256 KiB 明示純文字。詳見 [0.2 功能與差距](V02.md)、[規格與架構](SPEC.md)、[驗證記錄](QA.md)、[效能實測](PERFORMANCE.md)。

嚴格 UTF-8 / UTF-8 BOM，保留 LF、CRLF、舊式 CR；非 UTF-8、NUL、混合 EOL 明示拒絕，不做 lossy 解碼。檔案上限 2 MiB、20,000 行、單行 16 KiB。搜尋可選 literal 或 Rust regex，支援大小寫條件、全字、循環、前後導航、計數及選取範圍取代；Unicode 原文位置安全。行列數按 Unicode scalar 計數。

「格式」可改 BOM/EOL；「檢視」顯示空格、Tab、CRLF/LF/CR/EOF，標記不進入複製/儲存內容。「空白」提供 1–8 欄 Tab stops、Tab 插入空格、選取或全文轉換、清理 ASCII 尾端空白、加入/移除一個最後換行；保留其他 Unicode 空白、BOM、EOL，內容修改可 undo，顯示設定不改 dirty。

同目錄暫存寫完同步後原子替換，失敗保留旧檔；外部變更衝突拒絕覆寫，請另存。尚未提供插件相容、多檔寫入取代、多游標、矩形選取、Big5/UTF-16 或巨型檔案效能保證。Windows 真實 framebuffer 已檢視；原生鍵鼠/對話框/剪貼簿/完整 IME 與 Linux 桌面 GUI 尚未實测，詳見 QA。

鎖定依賴與授權見 Cargo.lock、[THIRD_PARTY_LICENSES.md](THIRD_PARTY_LICENSES.md)、[語法資料授權](licenses/SYNTAXES.md)。

正規表示式：搜尋列勾選「正規表示式」；`(?m)` 多行錨點、`(?s)` 讓點號包含換行。取代使用 `$1`、`${name}`、`$$`；不存在的群組展開為空字串。Rust regex 不支援 look-around 或 pattern backreference，錯誤明示；不是 PCRE 全相容。內部 LF 搜尋，不以 `\r\n` 匹配 CRLF 原始 bytes；儲存保留 BOM/EOL。

多檔搜尋：「搜尋 → 多檔搜尋」，選擇實體根資料夾，再開始／取消。只搜尋磁碟，未存分頁不包含；結果點擊定位前重新核對內容，不覆寫 dirty 分頁。詳見 DIFF-0.4.md 的數量、大小、連結及取消限制。

0.5 快捷鍵（編輯区焦點）：Ctrl+F2 切換目前行書籤，F2 / Shift+F2 下一／上一書籤，Ctrl+J 合併選取行，Ctrl+Shift+J 按設定字素寬度分割選取／目前行；選單也可操作。書籤不改 dirty，僅保存在目前分頁生命週期；無法可靠對應的多行改寫清除該區書籤，undo 不復活已清除標記。

「墨頁」取自書寫與頁面；名稱從開發工作名 RustPad 改為「墨頁 InkPage」，產品名與可攜檔案使用 InkPage。為保留連結／工具相容，GitHub URL、checkout 資料夾、Cargo crate 與 source build binary 仍是 rustpad；Windows 可攜包為 `InkPage/InkPage.exe`。未移動使用者檔案或設定；沒有需迁移的新增持久設定。名稱不主張商標獨占，本專案未做完整商標查核。

0.6：Ctrl+B 跳至游標處／前一字元的配對括號；語法模式忽略 string/comment scope。編輯選單增加／減少縮排；跨行選取 Tab 增加、Shift+Tab 減少；Enter 延续目前行 ASCII 空格／Tab 前綴。IME 組字事件交由既有 widget，未宣稱實測。

工作階段預設保存在執行檔旁 `InkPage.session`，每秒背景保存一次變動快照，含分頁、未存內容、saved 基準、游標與選取。結束時可選「保留工作階段並結束」，等待成功再結束；「放棄全部」保存 saved 基準。`--session <path>` 指定獨立快照、`--no-session` 停用。最多 32 分頁／32 MiB（含原文及 saved 副本）；唯讀目錄、損壞快照、外部改寫或其他程序鎖定會明示停用且保留舊快照。還原不寫原文件；原檔外部改寫仍由既有儲存衝突保護。異常退出可能留下 `.lock`，確認所有 InkPage 已關閉後自行移除該鎖；不自動刪除。快照含純文字未存內容，請保存在自己的私有目錄，不隨 ZIP 散布。手動語言、書籤、undo 歷史及檢視設定不在此版快照範圍。

0.8：Ctrl+Alt+F 或編輯選單切換目前多行括號區塊摺疊；Ctrl+Alt+U 展開全部。隱藏内部完整行，保留開始／結束行，gutter `>` 標記；原文與 Unicode 座標不替換為假文字。編輯、键盤導航、搜尋／跳行／書籤導航先展開；任何內容改動清除舊摺疊，避免錯位。語法字串／註解排除沿用 grammar；純文字不推斷字串。僅括號 () [] {} 定義區塊，未提供 Python 縮排、HTML tag 或 Markdown heading 摺疊。摺疊狀態不跨工作階段保存。
