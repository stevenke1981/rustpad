# 0.11 編輯器方向

給熟悉 Notepad++ 的 Windows／Linux 使用者，讓文件操作、分頁切換與目前選取狀態可直接辨識。沿用既有四語系、原創品牌、字型、編輯器與資料管線。

## 可驗收規則

1. 保留選單 → 工具列 → 分頁 → 文件的順序；不加入 dashboard 或側邊卡片。
2. 文件是主視覺；常用工具使用 24×24 logical 原創線條圖示，不以長文字塞滿工具列。
3. 工具列按文件、歷史、搜尋分組；禁止加入尚未實作的按鈕。
4. 每個圖示都有可讀 tooltip、快捷鍵與 accessible label；禁止依靠字型特殊符號猜動作。
5. 亮／暗主題沿用中性底色，強調色只標示 active 與選取；禁止品牌漸層背景。
6. dirty 仍有文字符號，加細線區分 active；不只靠色彩傳达未儲存。
7. 分頁保留水平捲動與原有拖曳排序；切換、右鍵與中鍵操作不可改文件內容。
8. 狀態列分段提供語言、文件大小、行列、選取、BOM/EOL；禁止將不足寬度的資訊裁掉。
9. 窄視窗允許狀態格與選單換列；文件區仍保有主要高度，不使用巨大留白。
10. gutter 與一般文字使用可讀對比；不把編譯或事件測試當成视觉驗收。

## Tokens 與範圍

UI 字體保留目前系統繁中 fallback；程式碼 font/line-height 保持不動。工具列 24 logical、圖示 16 logical、線寬 1.5、組內間距 3、組間分隔 8；狀態字體 12、行高至少 20。gutter 亮色 #5E6673／暗色 #ADB6C2；狀態文字使用 theme 正常 text color，次要文字只放 hover。不要改寫 editor 的座標、history、regex、存檔或 session 格式。

參考官方 [UI 手冊](https://github.com/notepad-plus-plus/npp-usermanual/blob/master/content/docs/user-interface.md) 的圖示工具列、Ctrl+Tab／Ctrl+Shift+Tab、完整路徑 hover、分段狀態列。官方 sb-full.png 與 tabNavNextPrev.gif 只存研究目錄，沒有進入程式素材或散布包。Notepad++ 的 bytes／字元資訊啟發資訊結構；InkPage 自訂選取行數以實際選中文字所在的行計算，尾端 newline 不多算下一個空白行。

baseline 為既有 0.10.1，qa/v11-before.png 是同機真實 framebuffer。0.10.1 原有未提交工作已備份並單獨提交，避免新設計覆蓋。兩個技能附屬 reference 文件提供者無法讀取，依已讀取的 design-with-intent 主 SKILL 流程鎖定以上規則。
