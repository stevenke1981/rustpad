# InkPage 0.11

本輪把三個日常入口改得更接近傳統桌面編輯器：緊湊圖示工具列、分頁導航／安全關閉、分段狀態列。既有 0.10.1 四語系、原創應用圖示、拖入與分頁排序原先未提交，已先備份並單獨保存為提交 4ba1e69，再繼續本輪，沒有覆蓋或退回 0.9。

工具列使用十個原創線條圖示：新建、開啟、儲存、另存、關閉，復原／重做，尋找／取代／跳行。24 logical 按鈕、16 logical 圖示，直接由 Rust painter 繪製，不依賴圖示字型或第三方素材。各按鈕有四語系 tooltip、快捷鍵與 accessible label；檔案忙碌時停用相關操作，復原／重做按各分頁歷史顯示可用狀態。選單內的復原／重做移到編輯選單上方。

分頁支援 Ctrl+Tab／Ctrl+Shift+Tab 按目前分頁順序循環，切换後恢復編輯區焦點；沒有 MRU popup。長名稱收斂到最多 180 logical，完整路徑與 dirty 說明放 hover；active 加底線，未存仍顯示 ●，不只靠顏色。中鍵關閉與右鍵關閉都走既有未存確認，取消保留原文；右鍵可切換到該分頁。新增「視窗」選單列出所有分頁及上下切換入口，鍵盤切到列外分頁會捲動至可見位置。原有拖曳排序、Esc 取消與歷史保留。

狀態列分段顯示語言、實際輸出 UTF-8 bytes／總行數、行列、選取字元／行數、EOL、BOM 與縮排模式。大小包含 BOM 與 CRLF 的額外 CR bytes；字元與行列沿用 Unicode scalar，不把 emoji byte 長度當字元數，也不宣稱字素或視覺 Tab 欄位。選取行數只計實際選中文字所在的行，尾端 newline 不額外算下一空白行。空文件為一行、位置 1:1，超範圍游標與選取安全截斷。位置格雙擊開啟既有跳行視窗，預填目前行。一般就緒訊息收進狀態資訊，重要操作／錯誤仍顯示；窄視窗允許換列，不裁掉編碼與位置。gutter 提高亮暗主題對比。

保留存檔、undo、regex、多檔搜尋、session 與格式管線，不修改 session 格式。三個回歸先 RED：Ctrl+Tab 未切換、中鍵未觸發 dirty 確認、缺少 Unicode/BOM/EOL 統計。新測試另覆蓋視窗選單切換與狀態雙擊跳行。GUI 測試工具改依實際第一行 glyph bounds 定位，正確處理 horizontal-wrapped label 的縮排。完整測試結果及畫面見 QA-0.11.md。

官方參考為 [Notepad++ UI 手冊](https://github.com/notepad-plus-plus/npp-usermanual/blob/master/content/docs/user-interface.md) 與其 sb-full.png／tabNavNextPrev.gif：參考資訊結構、圖示分組、路徑 hover 與切換快捷鍵。官方圖片只存研究資料夾，未進入 app、素材或 ZIP；未複製 Notepad++ 標誌、程式碼或圖示。設計規則見 DESIGN-0.11.md。

CLI `--narrow` 啟動 760×420 logical 視窗；既有 `--screenshot ... --qa-flow chrome` 使用自己的八個合成分頁，示範 dirty／長檔名／BOM／CRLF／選取。`--qa-tooltip` 只在截圖模式向自己的 raw input 注入 hover 並縮短 tooltip 等候，沒有發送 OS 鍵鼠，也不操作其他視窗。這些畫面是實際 app framebuffer，不是 mock；CLI seed 不能代替桌面驗收。

固定核心矩陣維持 29/36；未宣稱 Notepad++ 全功能、插件相容、完整 IME 或原生鍵鼠通過。原生 Computer Use 本輪仍 nativeApps=0，七項既有驗收待辦保留。沒有新增套件版本；Cargo.lock、授權清單與原創品牌保留。既有 repo/main 非強制推送已獲授權，使用 process-level 官方 gh helper，不改持久憑證／global 設定；無 tag、Release 或 merge。
