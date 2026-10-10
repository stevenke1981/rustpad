# InkPage 0.13：選取與行編輯

2026-10-10，在 5900X 原專案接續。發現既有未提交 0.12 工作，先保存完整 source.zip／tracked.patch，重新確認 101 passed、1 ignored，再以 0cca92f 獨立保存；沒有覆蓋既有檔案、交付包或使用者視窗。

| 編輯 → 行操作 | 行為 |
|---|---|
| 複製選取文字／目前行 · Ctrl+D | 有選取時在選取後插入原文副本並選取副本；沒有選取時沿用目前行複製 |
| 上移／下移選取行／目前行 · Ctrl+Shift+↑／↓ | 移動觸及的整行；末端恰在下一行開頭時不包含下一行；游標／選取跟隨 |
| 選取目前行 · Ctrl+Alt+L | 選取游標所在的實際來源行，含已有的換行；不修改內容、dirty 或 undo 歷史 |

原有「編輯 → 複製目前行」保留，這個明確的選單命令仍忽略選取範圍；Ctrl+D 現在改走選取感知入口。Ctrl+Alt+L 是 InkPage 的選取快捷鍵，不宣稱與 Notepad++ 的 Ctrl+L 相同（官方為剪下目前行）。對照官方 [Editing 手冊](https://github.com/notepad-plus-plus/npp-usermanual/blob/master/content/docs/editing.md) 的行移動與 SCI_SELECTIONDUPLICATE 工作流程，使用原創 Rust 實作，未複製其程式或素材。

字元索引沿用 Unicode scalar，複製保留原有 combining mark／emoji 及所有 UTF-8 bytes，未轉換字素或編碼。行移動保持文件的最後換行有無；文件上下邊界及末端 newline 的虛擬空白列不移動。一般內部空白行可移動。移動到沒有終止換行的末行時，選取尾端會截至實際 EOF。

`src/selectionops.rs` 負責驗證、候選與 App 提交；重用 0.12 的 Unicode／整行邊界 helpers。有效修改只記一筆 undo；無變更不記錄。保留 BOM、LF／CRLF／CR，候選先通過既有 2 MiB、行數與單行上限。錯誤保留原文、游標、選取與歷史。移動造成行重排時清除目前分頁書籤；複製沿用既有書籤同步，成功的導航／編輯展開摺疊；不修改其他分頁。

快捷鍵只在編輯區有焦點、沒有忙碌／關閉／結束／組字時處理。方向鍵命令在自身 raw-input hook 中先處理，避免 egui 同時移動介面焦點；選單也走同一資料命令。新字串有繁中、簡中、英文與日文。

QA 的 `--qa-flow line-move`／`selection-duplicate` 只建立自己的合成文件。截圖 viewport 使用 `.with_active(false)`，並以 hidden 啟動，不啟用使用者視窗；沒有 OS 鍵鼠、剪貼簿或私人文件存取。正式互動啟動仍維持原行為。

沒有新增依賴、公開資料格式、unsafe 或背景執行服務。核心矩陣維持 29/36，七項原生鍵鼠、完整 IME 與 Linux GUI 未驗證；本輪可用工具沒有受支援的原生電腦控制入口。不是 Notepad++ 全功能相容。驗證見 [QA-0.13](QA-0.13.md)。
