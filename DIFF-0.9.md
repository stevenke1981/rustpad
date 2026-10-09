# InkPage 0.9

本輪聚焦啟動成本與既有資料安全，不增加 IDE 功能。空白文件、純文字及超過著色上限的文件不再初始化 grammar；第一個有效程式碼著色請求才在既有背景執行緒載入一次，後續重用。首次著色需支付這筆延後成本，不宣稱免成本或穩定啟動加速。

修正啟動／截圖探針：非同步工作階段未就緒時，原本 target=1 會把 frame 重設成 11，從此錯過 target。現在到達目標後持續等就緒，僅送出一次 framebuffer 請求。此修正只影響明確指定的 QA CLI 探針；正常編輯不啟用擷取。

兩個回歸先 RED：空白服務立即載入 grammar、延遲還原後 target=1 不再重試。修正後 Windows 77 tests、fmt、clippy -D warnings 與 release build 通過。原有 Unicode、BOM/CRLF、dirty 多分頁還原、儲存失敗保留原檔、搜尋範圍與 undo 測試完整保留。沒有新增依賴或更改快照格式。背景還原的驗證、衝突、鎖定及原子儲存流程維持原有實作。

`--profile-services <json>` 是明確啟用的合成資料探針：8 分頁各 128 KiB，僅在記憶體生成文字，測 grammar 初始化、快照 clone/compare/decode，不讀取使用者文件。`scripts/compare-startup.ps1` 對兩個指定執行檔採 ABBA 交錯，每輪各 2 筆，30 秒逾時只終止自己啟動的探針；輸出資料夾必須不存在。

真實 Windows framebuffer 由程式自身 OpenGL renderer 產生，CLI 合成內容包含程式碼著色與 dirty 工作階段恢復；不等於原生鍵鼠、對話框、剪貼簿或 IME 驗收。使用者提供 RDP 前景時段後官方原生工具清單仍為 0，詳見 QA-NATIVE.md。固定矩陣維持 29/36，六項部分完成、一項未驗證，沒有增加完成數。

本輪 GitHub 唯讀權限檢查：目前登入 urtiger101-tw，stevenke1981/rustpad 回傳 push=false。本機提交與可攜包可交付；遵照指示未切換帳號、改憑證、建立公開 repo、push、tag 或 Release。本版沒有新遠端 CI 結果；既有 Windows/Ubuntu workflow 保留，等待有寫入權限的帳號推送。本輪 Linux 未執行，不能沿用 0.8 CI 當作 0.9 證据。
