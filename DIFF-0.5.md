# 墨頁 InkPage 0.5 操作與差距

| 功能 | 操作 | 語意 |
|---|---|---|
| 行書籤 | Ctrl+F2／搜尋選單 | gutter `*`；不 dirty，不寫入檔案 |
| 書籤導航 | F2／Shift+F2 | 每分頁隔離，前後循環，跳到標記行首 |
| 清除書籤 | 搜尋選單 | 只清目前分頁 |
| 合併行 | 選取跨行後 Ctrl+J | 擴到完整行，把中間 LF 改為單個 ASCII space，保留其他空白與末換行 |
| 分割行 | Ctrl+Shift+J／編輯選單 | 選取完整行；空選取則目前行；寬度在編輯選單設定，預設 80 個字素 |
| 關於 | 點選選單列「墨頁 InkPage」 | 名稱與版本、技術/授權文件提示 |

編輯區焦點才消耗快捷鍵；選單可在其他焦點操作。分割寬度是 1–1000 個 extended grapheme clusters，非 byte/scalar、像素、Tab stop 或語言學詞語。組合音標、旗幟與 ZWJ family 不從中切開；不改寫其他字元或正規化。依據 [unicode-segmentation 1.13.3 官方說明](https://docs.rs/unicode-segmentation/1.13.3/unicode_segmentation/trait.UnicodeSegmentation.html)。選取末端恰為下一行起點時排除該行；處理範圍外不變。合併只接受至少兩個選取行；不移除原本行首/行尾空白，可能超過單行上限而明示拒絕。

行操作保留 BOM/CRLF/LF/CR 與空白行／EOF 格式，先建立候選並 encode 驗證，任何大小／行數／單行限制失敗都不部分修改或移動選取，一筆 undo／redo。沿用原有 2 MiB、20,000 行、單行 16 KiB 限制；不能藉分割開啟原本拒絕的大檔。

書籤在完整未改 prefix/suffix 行之間映射，單行對單行內容改寫維持該行。多行插入/刪除移動未改區段標記；被刪或無法可靠映射的改寫區段標記清除。重複相同行文字的映射按 prefix/suffix 決定，不宣稱語意追蹤。書籤不是內容 undo 歷史的一部分：尚存標記可在 undo/redo 時跟隨未改行，已清除標記不會復活；關閉分頁／程式結束不保存。大量書籤仍使用完整文字快照與行比較，沒有增量編輯器效能保證。

本輪完成固定母項 F23 與 F32，分母仍 36，不把改名或快捷鍵另算分。F24 括號語法導航、F27 摺疊、F34 自動縮排、F36 工作階段還原仍缺少。現有 TextEdit 摺疊會牽涉顯示文字與真實游標映射，本轮保留原文編輯架構。

產品名「墨頁 InkPage」不含語言名；Rust/egui 技術在 README 說明。原 GitHub URL / rustpad crate / source binary 保留，可攜 Windows 包改為 InkPage.exe。沒有設定迁移或檔案搬移。历史 0.1–0.4 文件與著作權中的 RustPad 名稱保留以便追溯；沒有使用其他產品圖示/品牌素材。名稱非商標獨占宣稱，簡短網路查找不是完整名稱／商標清查。

原生鍵鼠、系統對話框／剪貼簿、完整 IME、Windows junction fixture 與 Linux 桌面 GUI 尚未實測；本輪 renderer 與 model tests 不冒充這些端到端結果。

已觀察到 egui／OS font 的 ZWJ family 顯示為多個 glyph；分割不打斷資料字素，但未完成 emoji shaping 驗證。
