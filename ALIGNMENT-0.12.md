# InkPage 0.12 與 Notepad++ 對照

2026-10-10。依 Notepad++ 官方手冊的 [Views 原始文件](https://raw.githubusercontent.com/notepad-plus-plus/npp-usermanual/master/content/docs/views.md) 與 [Editing 原始文件](https://raw.githubusercontent.com/notepad-plus-plus/npp-usermanual/master/content/docs/editing.md) 核對工作流程；自行實作，未取用 Notepad++ 程式碼或素材。

| 本輪項目 | 官方手冊對照 | InkPage 的結果／範圍 | 本機證據 |
|---|---|---|---|
| 畫面自動換行 | Views / Word wrap：顯示折行，不修改文字 | 檢視開關；所有分頁；依寬度折行、長單字可分列；實際行號／EOL 保留 | GUI wrap、Unicode 複製與貼上／復原測試；亮暗及窄視窗 framebuffer |
| 字體縮放 | Views / Zoom：Ctrl+鍵或滾輪調整編輯文字 | Ctrl++／=、Ctrl+-、Ctrl+0、編輯區 Ctrl+滾輪；9–32 pt；本次啟動所有分頁共用 | GUI 快捷鍵、上下限、焦點及組字保護；UI scale 保持；放大畫面 |
| 語言註解 | Editing / Commenting：依語言選註解界符 | Ctrl+Q／選單切換、加入、移除；14 種非 Plain／JSON 語言；區塊界符逐行包裹 | UTF-8、縮排、空白行、外部選取、BOM/EOL、失敗不修改及一次復原測試 |
| 字元排序／反轉 | Editing / Line Operations：字典字元排序、反轉選取行或全文 | 升冪、降冪、反轉；區分大小寫，保留最後換行；未實作自然數字／欄位排序 | 選取與全文測試；選單操作及復原；排序畫面 |
| 去重 | Editing / Line Operations：保留首筆或處理連續重複行 | 全部重複／連續重複兩種；選取外文字不動 | 中文、重複非相鄰行、空白行、末尾換行測試 |
| 四語系與既有功能 | 延續 InkPage 0.11 的檢視及分頁工作流程 | 新增操作全部四語系；既有原生拖入、格式及歷史保留 | catalog／placeholder 測試、完整測試、原生 WM_DROPFILES 探針 |

此表記錄新增範圍，不改寫 [固定 36 項歷史矩陣](ALIGNMENT.md)。該矩陣仍是 29/36（80.6% 的固定核心集合），不代表整個 Notepad++ 的產品完成率。新增功能以本機自動測試及真實程式畫面驗證；沒有把 framebuffer 截圖當作完整原生鍵鼠或 IME 驗收。

後續主要差距仍包含矩形選取、多游標、更多編碼、自然數字／欄位排序、插件及跨平台原生操作。InkPage 的字體共用設定、區塊註解逐行方式與書籤清除規則已在 [DIFF-0.12](DIFF-0.12.md) 明列。
