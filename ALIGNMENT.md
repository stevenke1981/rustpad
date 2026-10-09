# 固定核心功能矩陣（36 項，2026-10-09 台灣）

這是 **已定義核心範圍覆蓋率**，不等於 Notepad++ 全產品功能百分比。分母本輪開工固定為 36，之後不因達標而移除缺項；語言數量不分拆計分。來源為官方手冊 commit `41d96945ec8d11d4100356293f4f30e60c1418c0` 的 [Editing](https://github.com/notepad-plus-plus/npp-usermanual/blob/41d96945ec8d11d4100356293f4f30e60c1418c0/content/docs/editing.md)、[Searching](https://github.com/notepad-plus-plus/npp-usermanual/blob/41d96945ec8d11d4100356293f4f30e60c1418c0/content/docs/searching.md)、[Preferences](https://github.com/notepad-plus-plus/npp-usermanual/blob/41d96945ec8d11d4100356293f4f30e60c1418c0/content/docs/preferences.md)、[Views](https://github.com/notepad-plus-plus/npp-usermanual/blob/41d96945ec8d11d4100356293f4f30e60c1418c0/content/docs/views.md)。只摘要能力，不複製產品程式碼或介面資產。

完成：列出的 RustPad 受限驗收全部有自動資料/egui widget 證據，UI 類另有真正 Windows renderer 輸出查驗。部分：有程式但驗收不全。未驗證：所需工具/平台未實測。缺少：沒有實作。**部分與未驗證皆算 0**。這個驗收等級不包含真人 OS 鍵鼠/完整 IME，也不宣稱與 Scintilla 每種邊界完全等價；檔案大小/編碼及字詞定義限制另列。

v0.2 `4fb9b16` 基準完成 **11/36 = 30.6%**；所有對話框端到端未驗項目保守列部分。v0.3 完成 **21/36 = 58.3%**；部分 6、未驗證 1、缺少 8。v0.4 完成 **23/36 = 63.9%**；部分 6、未驗證 1、缺少 6。新增完成 F16 regex 與 F21 唯讀多檔搜尋，其限制見 DIFF-0.4.md。新建/未存關閉/基本分頁雖補資料流程測試及 renderer，但完整命令鏈仍保守列部分；不回寫基準。

| ID | 母項／獨立能力 | 可操作驗收（固定） | v0.2 | v0.3 | v0.4 |
|---|---|---|---|---|---|
| F01 | 文件／新建文件 | 新建空白非 dirty 文件，原分頁保留 | 部分 | 部分 | 部分 |
| F02 | 文件／開啟本機文件 | 對話框取消不變、成功內容正確、失敗保留原文件 | 部分 | 部分 | 部分 |
| F03 | 文件／儲存 | 原子寫入、失敗舊檔不變，UI dirty 正確 | 部分 | 部分 | 部分 |
| F04 | 文件／另存 | 選新路徑、取消不變、成功名稱/語言變更 | 部分 | 部分 | 部分 |
| F05 | 文件／未存關閉保護 | 有確認畫面，取消不丟資料，捨棄只移除目標分頁 | 部分 | 部分 | 部分 |
| F06 | 文件／UTF-8/BOM 往返 | 中文/emoji 與 BOM bytes 保留，非法編碼明示拒絕 | 完成 | 完成 | 完成 |
| F07 | 文件／EOL 保留與轉換 | LF/CRLF/CR 往返與 undo，混合 EOL 拒絕 | 完成 | 完成 | 完成 |
| F08 | 編輯／復原重做 | 文件隔離、取代一次 undo、取消 redo 分支 | 完成 | 完成 | 完成 |
| F09 | 編輯／剪貼簿 | OS copy/cut/paste Unicode 且不複製顯示標記 | 未驗證 | 未驗證 | 未驗證 |
| F10 | 文件／基本分頁切換 | 新建/切換/關閉保持其他分頁內容與搜尋隔離 | 部分 | 部分 | 部分 |
| F11 | 導航／行號與行列狀態 | Unicode scalar 行列與真實文件行號對應 | 完成 | 完成 | 完成 |
| F12 | 搜尋／下一個 literal | Unicode 位置正確，空 query 無命中，跨分頁不取代 | 完成 | 完成 | 完成 |
| F13 | 搜尋／上一個 literal | Shift+F3/按鈕，向前/文件開頭循環選項正確 | 缺少 | 完成 | 完成 |
| F14 | 搜尋／大小寫條件 | 忽略大小寫保留原始 Unicode 範圍，可切回敏感 | 缺少 | 完成 | 完成 |
| F15 | 搜尋／全字條件 | 不命中 word 內部，中文/組合字邊界依明示定義 | 缺少 | 完成 | 完成 |
| F16 | 搜尋／正規表示式 | regex 錯誤回報、群組取代及邊界測試 | 缺少 | 缺少 | 完成 |
| F17 | 搜尋／取代目前命中 | 改 query/文件後不誤替換，Unicode 安全、可 undo | 部分 | 完成 | 完成 |
| F18 | 搜尋／全部取代 | 同一文件計數、一次 undo，超限不部分修改 | 部分 | 完成 | 完成 |
| F19 | 搜尋／選取範圍取代 | 選取外不變，空選取拒絕，BOM/EOL 不變 | 缺少 | 完成 | 完成 |
| F20 | 搜尋／命中計數 | 與取代條件一致，全文或選取範圍不改內容 | 缺少 | 完成 | 完成 |
| F21 | 搜尋／跨檔案搜尋 | 多文件結果與路徑導航 | 缺少 | 缺少 | 完成 |
| F22 | 導航／跳至指定行 | Ctrl+G/對話框，Unicode/空白/EOF 行，非法輸入不移動 | 缺少 | 完成 | 完成 |
| F23 | 導航／書籤 | 切換標記，上一/下一書籤，修改後位置處理 | 缺少 | 缺少 | 缺少 |
| F24 | 導航／括號配對跳轉 | 跳往配對，忽略字串/註解內括號 | 缺少 | 缺少 | 缺少 |
| F25 | 程式／語法色 | 各類 token、跨行狀態、Unicode bytes 不改，亮暗 renderer | 完成 | 完成 | 完成 |
| F26 | 程式／語言選擇 | 副檔名/shebang/手動選擇及另存失效，未知純文字 | 完成 | 完成 | 完成 |
| F27 | 程式／摺疊區塊 | 收合/展開且不改內容或游標映射 | 缺少 | 缺少 | 缺少 |
| F28 | 空白／可見空白/EOL | 真實 renderer 標記，原文 bytes/字元映射不變 | 完成 | 完成 | 完成 |
| F29 | 空白／Tab stops 與轉換 | 選取/全文對齊停止點，Unicode/undo/BOM/EOL 正確 | 完成 | 完成 | 完成 |
| F30 | 空白／尾端與末換行清理 | 只清 ASCII 尾空白，保留 Unicode 空白與空白行，可 undo | 完成 | 完成 | 完成 |
| F31 | 編輯／行複製與刪除 | 目前行、末行/EOF/空文件，單次 undo、超限原文不變 | 缺少 | 完成 | 完成 |
| F32 | 編輯／行合併與分割 | 選取跨行合併、指定寬度分割不損 Unicode | 缺少 | 缺少 | 缺少 |
| F33 | 編輯／選取大小寫轉換 | Unicode 上/下大小寫含展開，選取外/BOM/EOL 保留、undo | 缺少 | 完成 | 完成 |
| F34 | 編輯／縮排與自動縮排 | 選取行增減縮排、Enter 持續前行縮排 | 缺少 | 缺少 | 缺少 |
| F35 | 檢視／亮暗主題 | 控制項/文字/語法色可讀，切換不 dirty | 完成 | 完成 | 完成 |
| F36 | 文件／工作階段還原 | 重啟還原分頁/未存內容，原檔不被覆寫 | 缺少 | 缺少 | 缺少 |

基本分頁能力不含排序/釘選；語法色不含 LSP；全字不是語言學斷詞。非 UTF-8、混合 EOL、>2 MiB、>20,000 行、單行>16 KiB 仍拒絕。>256 KiB 語法色明示純文字。插件、宏、列選、多游標、完整多編碼、無障礙、列印、大型檔案等完整產品能力不在此 36 項分母，因此不得把此數字稱為全產品百分比。

## 證據索引

- F06–08、F28–30：core Unicode/BOM/EOL/失敗寫入/history/whitespace 測試、既有 app whitespace 回歸、0.2 語法/空白 renderer，見 QA-0.2.md。
- F11–15、F17–20：actions `unicode_search_options_preserve_original_ranges`、`previous_next_and_no_wrap_boundaries`、`cursor_inside_a_match_preserves_next_previous_boundaries`、`selection_replace_is_atomic_unicode_and_one_undo`；app count/replace/undo、changed-query 安全測試；v03-search renderer。
- F22：actions line navigation、app invalid input/cursor/dirty 回歸；v03-goto / v03-navigation renderer 實際顯示第 350 行。
- F25–26、F35：syntax 跨行/grammar/spans/cache 測試與既有亮暗語法 renderer。
- F31、F33：actions EOF/上限/Unicode expansion 測試、app edit/dirty/history/cursor 回歸；v03-edits renderer。
- 大檔背景工作：18,000 行 thread/channel 回歸，過期內容/query/不同分頁不能修改；不另計分。

原生鍵鼠、剪貼簿、IME、Linux GUI 未驗，完成只代表上面明示的資料/widget/renderer 等級。不得用 63.9% 宣稱完整 Notepad++ 的 63.9%，或把未測項目藏進完成欄。

- F16：pattern Unicode/零長度/capture/非法語法/輸出與匹配上限；app regex worker 錯誤與游標，v04-regex / v04-regex-error renderer。
- F21：filesearch 根範圍/BOM/CRLF/binary/取消/結果上限；app background/stale disk/dirty tab/定位，v04-files / v04-file-navigation renderer；Linux symlink loop 測試由 Ubuntu CI 執行。
