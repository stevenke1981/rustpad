# 墨頁 InkPage 0.5 規格與架構

目前功能矩陣：[ALIGNMENT.md](ALIGNMENT.md)；本輪操作與差異：[DIFF-0.5.md](DIFF-0.5.md)。0.2 歷史規格：[V02.md](V02.md)。

選用 eframe/egui 0.33 + glow 原生 OpenGL：同碼 Windows/Linux，MIT/Apache-2.0，快速實現傳統桌面工具列與 TextEdit。iced 的 text_editor 也可行，但首版重用現有 egui 檔案/歷史流程；Slint 額外的編輯元件與授權選項增加實作成本。egui TextEdit 並非 Scintilla，IME、無障礙與大型檔案需獨立實測。

`core.rs`：內部 LF 正規化的 Content（BOM/EOL 分開保存）、嚴格解碼、同目錄原子儲存、Unicode 搜尋與替換、Tab stops 轉換、history（最多 64 筆/16 MiB）。不依赖 UI。

`main.rs`：Document 與穩定分頁 ID、UI、檔案對話框/I/O channel workers、dirty 關閉/退出確認。讀寫期間 UI 可繪製，儲存回覆按文件 ID 與已存 snapshot 比對，避免覆蓋新修改的 dirty。

`syntax.rs`：syntect/two-face 語法、原創亮暗色盤、單背景 worker、較新 ticket 淘汰舊任務、各分頁著色结果、LayoutJob cache。每次內容改變重新解析整份 <=256 KiB 文件，跨行 state 不重設；不是增量 lexer。語法 byte ranges 與 TextEdit char cursors 分開；LayoutJob.text 原始 bytes 不變。Tab glyph 按下一個邏輯停止點調整，維持固定行高。

`perf.rs`：明確啟用的啟動/排版探針；`scripts/measure.ps1` 五次取中位數；不作原生鍵鼠或 IME 驗證。

輸入與輸出 2 MiB / 20,000 行 / 單行 16 KiB，開檔最多讀上限+1 byte。NUL、非法 UTF-8、混合 EOL 拒絕，錯誤不改原檔。原子儲存 write_all/sync_all/persist，失敗不先刪舊檔；外部變更比對仍存在檢查至替換間的競態。搜尋/排版/編輯仍需 UI 執行，未承諾巨型檔案流暢。

官方參考：https://github.com/notepad-plus-plus/notepad-plus-plus （GPLv3）；InkPage 原創程式 MIT。語法資料由鎖定官方 crates 提供，授權另列，不使用 Notepad++ 程式碼/圖示/品牌。

## 0.3 搜尋/編輯架構

`actions.rs` 提供 Unicode 原文/小寫搜尋映射、方向導航、全字條件、選取取代、跳行、行操作、大小寫轉換。命中以 scalar 範圍返回，批次取代建立候選 Content、encode/上限驗證後提交；原文 BOM/EOL 不變。

App 中 >256 KiB 文件或 >128-byte query 的搜尋/count/replace 放背景 thread，ResultEvent 帶 document ID、Content snapshot、query、replacement、條件及 task。UI 回覆核對當前分頁/內容/條件，過期不提交。短操作仍 UI 執行；不宣稱鍵鼠延遲保證。每個批次命令只 record 一筆 undo，導覽不 dirty；程序捲動立即顯示目標。

固定母項/驗收/狀態見 ALIGNMENT.md，使用方式見 DIFF-0.3.md；不足不以縮分母達標。

## 0.4 背景搜尋

`pattern.rs` 使用 pinned regex 1.13.1，編譯 NFA/DFA 各 1 MiB、query/template 各 16 KiB、單文件最多 20,000 匹配；有界 capture 展開先驗證候選再提交。regex 操作一律背景執行，沿用 snapshot guard 與單次 undo。literal 行為不變。

`filesearch.rs` 以獨立 worker/channel 與 AtomicBool 執行唯讀 root scan。跳過 symlink/reparse points，canonical containment 檢查，既有安全解碼；讀取總預算 32 MiB、1000 檔、10000 目錄項目、深度 32、2000 命中。結果保存路徑、行號、scalar 範圍、160 scalar 預覽與內容 fingerprint；點擊以 I/O worker 重讀，比對磁碟與已開啟分頁後選取。取消不是中斷 OS syscall 或單次 regex engine 呼叫；不宣稱對惡意並行更換 symlink 的無競態 sandbox。

## 0.5 書籤與行處理

`bookmarks.rs` 以每分頁 BTreeSet 儲存 0-based 行標記；帶前次 text snapshot，按共同完整行 prefix/suffix 對應未改行，單行對單行修改保留，其他不可靠改寫區段清除。標記不進 Content/dirty/history，沒有持久設定或 session restore；內容 undo 后可重新對應尚存標記，不復活已清除標記。

`lines.rs` 整行範圍候選轉換，選取末端為下一行起點時排除該行；join 僅把區段內 LF 轉 ASCII space，split 每行按 unicode-segmentation 1.13.3 extended grapheme（1–1000）插入 LF。encode/所有原有上限通過才提交，一筆 undo；不實作詞語折行或像素寬度換行。這個 crate 已在 locked 依賴圖內，本輪僅增加直接使用，沒有升級其版本。

UI/title/about 與可攜包改用 InkPage，原 repo/crate/cache key 繼續相容；不更名 GitHub repo、不新增 persistent 設定。code folding 暫不做：現有 TextEdit 直接使用完整 Content，安全摺疊需要獨立顯示字串與原文 cursor/range/gutter 映射。

## 0.6–0.8 核心補齊

`indent.rs` 提供候選內容驗證後的行縮排／Enter，單次 history 交易。`structure.rs` 以 syntect ParseState／ScopeStack 取得 code mask，括號配對忽略 string/comment；括號／摺疊分析在 worker，套用前核對文件 snapshot。`fold.rs` 保留完整 LayoutJob.text 與 scalar 座標，只將完整内部行排版高度設為零；編輯前展開、內容改動清除，gutter 使用真實來源行號。

`session.rs` 用有版本、長度上限及校验的獨立 binary snapshot，最多 32 分頁／32 MiB；content 與 saved 分開保留，游標嚴格驗證。worker 背景載入／原子保存、預期舊 bytes 核對及獨占 .lock；normal exit 最新 snapshot 成功後才結束。只還原記憶體，不寫原文件；損壞／衝突停用並保留原快照。只讀目錄明示停用，可 --session 指定或 --no-session 停用。前0.5沒有持久快照；不搬移舊路徑或改RustPad內部專案/repo名稱。
