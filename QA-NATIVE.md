# 尚待原生驗收的七項

固定 36 項矩陣已實作 29 項的受限驗收；F01–05、F09、F10 有程式／資料回歸，但完整 OS 命令鏈仍未驗證，不能勾完成。此文件是待執行步驟，不是通過紀錄。

環境阻礙：computer-use skill 要求由 node_repl 載入 `@oai/sky`；本環境沒有 node_repl／sky 工具，現有 CUA 明示 Native computer APIs are disabled。未使用 SendKeys、UIAutomation 或自製輸入代理繞過限制。5900X 命令執行可用；曾短暫 exec transport disconnected，恢復後工作完整保留。Linux 沒有真實桌面 GUI 測試環境。

最小解鎖動作：在同一 5900X 提供支援的原生 computer-use 工具，並取得本程式的前景驗收時段；或由使用者依以下步驟操作並提供結果。只操作自己的 InkPage 与合成 fixture，不碰其他專案、私人文件、登入或瀏覽器。無需改動安全設定。

0.9 本輪使用者已授權 Computer Use 並確認 RDP 前景；再次呼叫官方 CUA getState，仍回傳 nativeApps=0、browserSurfaces=3、errors=[]。原生 API 明示 disabled，沒有 node_repl/sky。前景授權已足夠，阻礙是工具能力，七項仍未通過；未更改 RDP、登入或安全設定。

請先在自己的 QA 臨時資料夾建立 `original.txt`（UTF-8 BOM、CRLF，內容 `甲🙂乙`、下一行 `尾`），以及可寫的 `saved.txt`、唯讀的 `readonly.txt`；各原檔先記 SHA-256。執行 InkPage 的 `--session <QA資料夾>/native.session`，避免混用日常快照。開始前確認沒有自己的另一個 InkPage 使用同一快照。

| 項目 | 原生操作與應得結果 |
|---|---|
| F01 新建 | 用 Ctrl+N 及工具列各建立一次空白分頁；新分頁沒有 dirty，既有中文原文保留。 |
| F02 開啟 | Ctrl+O 開啟 OS 對話框，先取消再選 original.txt；取消無變化，成功 BOM／CRLF／中文🙂正確；嘗試非法 UTF-8 fixture，顯示錯誤且原分頁保留。 |
| F03 儲存 | 在 saved.txt 增加中文🙂後 Ctrl+S，磁碟 bytes／dirty 清除正確；在 readonly.txt 修改後儲存失敗，舊檔雜湊不變、dirty 保留。 |
| F04 另存 | Ctrl+Shift+S 先取消，原名稱／路徑不變；再另存新副檔名 .rs，名稱與語言改變、dirty 清除、原檔雜湊不變。 |
| F05 未存關閉 | 修改合成分頁，Ctrl+W 及分頁 X 各測；取消保留內容，再捨棄只關該分頁；另測儲存後關閉，對話框取消應仍開著。 |
| F09 剪貼簿 | 在合成文件選取 `甲🙂乙`，先原生 copy 自己的文字，再新分頁 paste；內容完全相同。測 cut、paste、undo／redo；顯示空白／EOL與摺疊時不把 gutter／CRLF標籤等顯示標記寫入文字。若 copy 未成功，不讀取或貼上既有私人剪貼簿。 |
| F10 分頁 | 原生新建、滑鼠切換兩個不同中文分頁，再關一個；另一個內容、dirty、格式與游標保留；搜尋命中後換分頁不能用舊命中取代。 |

每項保存操作過程／錯誤狀態的實際 screenshot、磁碟 bytes 或 SHA 與結果。IME 可另測中文組字／候選確認，但此表不以通過基本 copy/paste 推論完整 IME。Linux CI 的 build/test 也不代替 Linux GUI。
