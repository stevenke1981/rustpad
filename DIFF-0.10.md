# InkPage 0.10

本版加入檔案拖入、分頁拖曳排序、四種介面語系與原創應用程式圖示。沒有增加執行期依賴；Cargo crate／來源編譯產物仍為 rustpad，可攜版為 InkPage.exe。

檔案拖入接收 egui 原生 dropped_files，在背景 canonicalize、確認一般檔案並沿用 core::read_file 的嚴格 UTF-8、BOM、LF／CRLF／CR、2 MiB、20,000 行、單行 16 KiB 驗證。逐檔處理，既有文件與未存內容保留；已開啟路徑只切換分頁。資料夾、沒有本機路徑、上限與解碼錯誤列在結果視窗。佇列與拖入可開啟分頁各最多 32，包含既存分頁。檔案工作、工作階段載入及關閉確認期間延後處理。

分頁名稱以 click_and_drag 提供穩定 document ID payload，目標左右半邊決定放置於前／後，顯示插入位置；可拖至列尾，兩側自動水平捲動。只在釋放時移動完整 Document，Esc 由 egui 取消。保留 active ID、history、cursor／selection、書籤、語法快取與內容；新順序沿用 session 快照保存。修正關閉 active 前方分頁時，active 跟隨原文件 ID。未加入釘選或選取文字拖曳。

介面提供 zh-TW、zh-CN、en、ja，集中於 locales/catalog.tsv，涵蓋選單、工具列、未命名標籤、自訂對話框、搜尋結果條件、狀態及應用錯誤。文件、路徑與搜尋字串不翻譯。狀態於顯示時翻譯，切換更新既有訊息；較長搜尋列可換列。「語法語言」與「介面語言」分開。原生對話框的 OS 按鈕及系統診斷仍依系統語言。

獨立有版本 InkPage.settings 最多 512 bytes，只存語系；同目錄原子寫入，格式錯誤／偵測外部改寫保留原檔。寫入失敗仍允許本次切換。--settings、--no-settings、--lang 提供位置、停用與本次覆蓋；截圖探針預設不讀使用者設定。不改舊 session 格式。外部比對至原子替換間仍有競態窗口，不宣稱跨程序鎖定。

新圖示由內建 image_gen 生成原始透明 PNG，另匯出 16／20／24／32／40／48／64／128／256 PNG 與九尺寸 ICO。viewport 內嵌 PNG，Windows MSVC build.rs 呼叫 SDK rc.exe 嵌入 ICO 與版本資源；Linux 跳過 Windows 資源編譯。提示詞與素材說明見 assets/ASSETS.md；程式執行不依賴外置圖示檔。

fmt、clippy -D warnings、87 項測試與 Windows release build 通過。新增事件層測試包含 raw file drop、egui pointer press／move／release 排序、Esc 取消，以及語系選單點選／設定重讀／視窗標題。這些屬框架事件驗證，並非檔案總管真實 OLE 拖曳。實際畫面與原生查驗見 QA-0.10.md。固定 36 項歷史矩陣不另計新增功能。
