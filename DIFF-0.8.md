# InkPage 0.8

四個原本缺少的核心項目已分三個版本實作：0.6 括號跳轉／縮排，0.7 工作階段還原，0.8 摺疊。固定矩陣 29/36（80.6%），部分 6、未驗證 1、缺少 0；七項 OS 命令鏈未測仍算 0，沒有宣稱 36/36 或完整 Notepad++ 百分比。

Ctrl+Alt+F／編輯選單切換目前多行括號區塊，Ctrl+Alt+U 展開全部。保留 header／結束行，只隱藏內部完整行，gutter `>` 標記及真實文件行號。使用既有語法 grammar 的 string/comment scope 排除字串／註解；配對 () [] {}，選擇包含游標的最小區塊。純文字不推斷 string/comment；Python 縮排、HTML tag、Markdown heading 的專門摺疊未實作。

完整原文始終保留在 Content 與 LayoutJob.text；隱藏內部行以透明字型及零行高排版，沒有用假文字替換原文，不需將顯示字元反向猜回原文。選取／游標仍使用原始 Unicode scalar 座標，含選取方向。鍵盤導航、輸入、cut/paste、IME 事件、搜尋／跳行／書籤導航前展開；內容改動清除舊摺疊，避免落在錯行。摺疊不 dirty、不進 undo，不跨工作階段保存。不是大型文件的虛擬排版／性能最佳化承諾。

先加入 stub 的行高／座標測試確認失敗，才實作；測試抓到零字型尺寸限制，修正為正的極小尺寸、透明顏色及零行高。回歸包含：Unicode／EOF／巢狀／字串註解、原文與 BOM/CRLF bytes 保持不變、隱藏游標、切換展開、過期背景結果拒絕、widget copy 命令含完整隱藏原文、paste 前展開。Windows Rust 1.99.0 MSVC、-j1：75 tests、fmt、clippy -D warnings 與 release build 通過。

真實 Windows framebuffer：qa/v08-fold.png（暗）及 qa/v08-fold-expanded.png（亮），已檢視；摺疊行號 1→7，原文仍 149 字、UTF-8 BOM／CRLF、行1列11；展開顯示全部12行。操作來自自己的 CLI QA seed，沒有 OS 鍵鼠證據。v07-session-save / restore 是兩次獨立程序重新啟動，原檔雜湊不變；詳見 DIFF-0.7.md。

0.6 CI success run 37899510752；0.7 CI success run 37900483159。本版的最終 SHA／CI 終態由交付回報提供。Linux CI 僅 build/tests，沒有 Linux GUI 實測。剩餘原生驗收與最小解鎖需求見 QA-NATIVE.md。

未新增第三方依賴，Cargo.lock／授權清單保持既有鎖定版本。InkPage 名稱／圖示／原創 MIT 程式保留，RustPad 內部路徑及原 repo URL 保留；無公開新 repo、force push、tag 或 Release。
