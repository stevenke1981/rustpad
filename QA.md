# 0.1 驗證記錄

2026-10-09；本機 Windows NT 10.0.22631（Windows 11），x86_64 MSVC，Rust 1.99.0。5900X 為委派環境提供的型號；CIM 讀取被拒，實際可讀 processor identifier 為 AMD64 Family 25 Model 33 Stepping 2。cargo 全程 -j1，未做大型 benchmark。

## 自動驗證

12 個單元／應用狀態測試：UTF-8／BOM／LF／CRLF 空檔與 Unicode byte-for-byte 往返；非法編碼、NUL、混合換行與大小限制；Unicode 搜尋與替換邊界、空 query、循環搜尋；取消 dirty 分頁關閉、放棄後建立新分頁；獨立 undo/redo、全部取代單次復原；搜尋結果不得跨分頁替換；儲存途中修改保持 dirty；失敗／取消儲存保持 dirty；注入暫存部分寫入失敗後原檔仍為原 bytes；不合法輸出拒絕且不毀檔；無換行空檔／單行檔以 byte 比對避免 CRLF 格式造成外部變更誤判。

`cargo test --locked -j1`：12 passed，0 failed。`cargo clippy --locked -j1 --all-targets -- -D warnings`：通過。`cargo fmt --check`：通過。Windows debug build 已成功。

Windows release build 通過，首次建置 2 分 53 秒（依賴已有下載、release 編譯快取為空、-j1、thin LTO；包含編譯，非啟動時間）。release exe 約 5.06 MiB（5,304,832 bytes；修訂後以實際檔案為準）。未做啟動時間／峰值記憶體／大型檔案效能 benchmark。

## 真實 GUI

`qa/light.png`、`qa/dark.png` 為本機 Windows 原生 eframe/glow OpenGL framebuffer 截圖，非設計稿、HTML 或 headless mock。用內建 `--screenshot` 測試模式載入自有 sample，再透過 egui ViewportCommand::Screenshot 取得真正繪製的像素。人工視覺檢查：緊湊選單／工具列／多分頁／搜尋列，繁中字型、基礎高亮、行號與狀態列。截圖不包含 OS 標題列。

```powershell
.\target\release\rustpad.exe --screenshot qa/light.png
.\target\release\rustpad.exe --screenshot qa/dark.png --dark
```

目前無可呼叫的 Windows 原生輸入控制工具：computer-use skill 要求 node_repl + @oai/sky，但此環境未提供 node_repl；cua 僅允許瀏覽器。未注入 OS 鍵盤／滑鼠，未宣稱完整 GUI 輸入、剪貼簿、檔案對話框或注音／拼音 IME 已通過。繁中渲染與 Unicode 資料測試不等於 IME 測試。

## 未驗證與限制

Linux 未實機 build/run；已有 Windows/Linux CI 配置但尚未推送或執行。Linux 特定第三方授權仍需在正式封裝時補稽核。Windows native dialog、選取後中文組字／取消、跨分頁組字、IME undo、滑鼠與剪貼簿是後續人工 QA 清單。

字型從系統載入；Emoji 以系統字型 fallback 嘗試顯示，未保證跨平台所有 glyph／彩色 emoji。行列以 Unicode scalar 計算。語法高亮只辨認部分關鍵字，256 KiB 以上自動純文字。檔案最多 2 MiB／20,000 行／單行 16 KiB；排版、搜尋、編輯仍在 UI 執行，未宣稱極致大檔效能。儲存會比對磁碟內容但「檢查與替換之間」仍有外部寫入 race；不提供跨程序檔案鎖。原子替換不保留全部 ACL／硬連結關係，也未保證電源故障下目錄持久性。

視覺 QA 曾找到 Windows 系統主題覆寫初始深色設定的問題；修正為 update 時套用使用者主題。Emoji 符號缺字問題透過系統 Segoe UI Emoji fallback 修正，本機呈現為單色。兩項均以重新編譯後真實截圖複查。
