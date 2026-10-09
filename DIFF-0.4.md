# RustPad 0.4 操作與差距

| 功能 | 操作 | 保證與限制 |
|---|---|---|
| Regex 搜尋／取代 | 搜尋列勾選正規表示式；原有前後、計數、目前／全部／選取取代按鈕 | Rust regex 1.13.1 Unicode 語法，非 PCRE；不支援 look-around／pattern backreference |
| Capture 取代 | `$1`、`${name}`、`$$` | 最長名稱規則：`$1a` 找名為 1a 的群組；`${1}a` 才是第 1 組加 a；未知／未參與群組為空 |
| 多檔搜尋 | 搜尋選單 → 選根 → 開始；底部結果點擊定位 | 唯讀磁碟，保留既有分頁；只接受實體資料夾，略過連結與 .git/target |
| 取消 | 底部取消工作 | 合作式取消，已找到結果保留並標示部分；不能立即中斷讀檔／一次 regex 引擎呼叫 |

搜尋文字是已解碼、內部 LF 的完整文件；`^`/`$` 預設文件邊界，`(?m)` 行邊界，`.` 預設不匹配 LF，`(?s)` 包含 LF。CRLF/BOM 僅在讀存轉換，不以原始 `\r\n` 搜尋。regex 忽略大小寫採 regex Unicode simple case folding，與 literal 的逐 scalar lowercase 規則可能不同；均不做正規化或中文斷詞。全字沿用相鄰 Unicode alphanumeric/underscore/零寬組合字規則。

空 regex 明示拒絕；`(?:)` 可指定零長度匹配。連續下一個會前進一個 scalar，在 EOF 依循環選項處理；上一個排除目前零長度位置。批次使用 engine 的非重疊匹配序列，零長度與非空匹配的相鄰抑制遵循 regex find_iter/captures_iter。選取範圍包含完整落在其中的匹配，包含位於邊界的零長度匹配；空選取仍拒絕範圍取代。語法錯誤與限制錯誤不改內容，一個命令一筆 undo。

單文件 query/template 各 16 KiB，regex 編譯與 DFA cache 各 1 MiB，最多 20,000 engine 匹配（含被全字／scope 排除的匹配）；超限不部分修改。取代輸出仍須符合 2 MiB、20,000 行、單行 16 KiB、禁止 CR/NUL。所有 regex 操作背景執行；不保證任意複雜 regex 的即時完成。

多檔限制：1000 個檔案候選、32 MiB metadata 宣告讀取總預算、2000 命中、10000 目錄項目、深度 32。單檔 >2 MiB、binary NUL、非法 UTF-8、混合換行或超出既有編輯上限略過並計數；錯誤、達限、取消皆明示。目錄遍歷順序由 OS 決定，結果非全量時明示不完整。每次 read 最多 2 MiB+1 byte；檔案並行增長可能使實讀量略大於 metadata 預算。預覽最多 160 scalar，完整路徑可懸停查看。

多檔搜尋不包含 dirty 分頁，不提供跨檔寫入取代或檔名 glob 過濾。點擊結果時重新讀取並比對 normalized text fingerprint；磁碟變更或已開分頁內容不同就拒絕定位，保留內容。fingerprint 非安全雜湊，不是對惡意碰撞的安全保證。symlink/junction 以 metadata 與 canonical 根範圍跳過，並行替換路徑仍有 TOCTOU；不宣稱安全沙箱。

原生 OS 鍵鼠、系統對話框、剪貼簿、完整 IME 與 Linux GUI 未實測。Linux CI 的 build/tests 不等於桌面 GUI 驗證。固定 36 項母項只新增 F16、F21 完成，不增加分母；其餘差距見 ALIGNMENT.md。

官方 regex 語法與 replacement 規則：[regex 1.13.1](https://docs.rs/regex/1.13.1/regex/struct.Regex.html)。
