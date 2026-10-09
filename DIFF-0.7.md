# InkPage 0.7

F36 工作階段還原完成，固定核心 28/36。資料處理先加入三個 stub 回歸，確認失敗後實作；71 tests、fmt、clippy -D warnings 通過（Windows Rust 1.99.0，-j1）。0.6 CI success：run 37899510752。

獨立快照 `InkPage.session`，版本 magic、FNV-1a 64-bit 意外損壞校验、嚴格長度/UTF-8/內容限制；不是加密或抗惡意篡改。上限 32 分頁／32 MiB，含 content/saved；保留 BOM、LF/CRLF/CR（含無换行內容）、游標與選取。背景載入及保存，快照與原始文件路徑衝突拒絕；還原只建記憶體文件，外部原檔不覆寫。所有快照原子保存先檢查前次 bytes；無效格式、外部改寫、唯讀或寫入失敗停止後續快照且明示。

預設執行檔旁可攜快照，`--session <path>` 可指定，`--no-session` 停用。第二程序因 `.lock` 停用快照；正常退出移除鎖。异常退出的鎖保守保留，確認程序全關閉後手動移除；不自動搶鎖。載入期間若已有使用者編輯，不自動覆蓋新內容，也停止覆寫舊快照，請重新啟動還原。正常退出「保留工作階段」等待最新快照成功；「放棄全部」以 saved 基準保存後退出。失敗不假稱已保存；若停用後強制放棄退出，先前快照仍保留。

證據：兩個獨立 Windows release 程序，先 session-save 再 session-restore；qa/v07-session-save.png 與 qa/v07-session-restore.png 已視覺檢查，兩個 dirty 分頁、中文🙂、UTF-8 BOM、CRLF、游標與選取還原。快照 258 bytes，正常退出鎖不存在。原始合成文件前後 SHA-256 相同：25718360E05D3C2D0963D1381E9DD4DAE5FCA789244EE4B9F861ADCC0CC96218。測試也改寫原檔後重新載入，證明外部原檔保持不變。

這是實際程序重啟／framebuffer 證據，seed 操作由自身 QA CLI 執行，沒有 OS 鍵鼠／對話框／剪貼簿驗證。現有七項保持原狀態。Linux GUI 未測；快照包含自己的未存內容，不打包或提交 QA cache。
