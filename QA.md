# RustPad 0.2 驗證記錄

2026-10-09，本機 Windows NT 10.0.22631，x86_64 MSVC，Rust 1.99.0；5900X 為使用者提供的主機資訊，CIM 讀取遭拒，未獨立確認 CPU。cargo 全程 -j1。

## 自動測試

`cargo fmt --check`、`cargo clippy --locked -j1 --all-targets -- -D warnings`、`cargo test --locked -j1` 通過，27 passed / 0 failed。涵蓋：

- 嚴格 UTF-8、BOM、LF/CRLF/CR、無換行檔 byte-for-byte 往返，拒絕非法編碼、NUL、混合 EOL、超限。
- Unicode 搜尋/替換邊界、跨分頁禁止、選取與全部替換、單次 undo；未存关闭取消/捨棄；非同步儲存保留新修改 dirty。
- 注入暫存檔部分寫入失敗，舊檔 bytes 不變；非法/超限輸出不毀檔；外部變更比對。
- 各種語法存在且 sample 使用多色；跨行 Rust 註解、巢狀註解、Python multiline string；修改跨行關閉符後狀態改變；Unicode span byte boundaries、LayoutJob bytes 恆等；>256 KiB 純文字退回。
- 副檔名/shebang/手動選擇，切換語言、主題、Tab width、字型行高/DPI/空格尺寸使快取失效；另存新副檔名改變語言但不制造 dirty。
- Tab stops 往返、選取前綴欄位、CJK/emoji、組合字、Unicode 空白、尾端清理/最後換行、undo；Tab galley 字元索引與行高不變。
- 搜尋欄焦點下 Ctrl+Z 留給該欄位，編輯區歷史不誤消耗。

## 真實 Windows 繪製與限制

`qa/syntax-rust-light.png`、`qa/syntax-python-dark.png`、`qa/syntax-json-light.png` 由真正 Windows eframe/glow OpenGL framebuffer 擷取，等待背景著色完成；不是 HTML、設計稿或 headless mock。以真實像素檢查繁中與 emoji、三種語法配色、行號、工具列/搜尋/分頁/狀態列；不包含 OS 標題列。

```powershell
.\target\release\rustpad.exe --screenshot qa/syntax-rust-light.png --sample rust --whitespace
.\target\release\rustpad.exe --screenshot qa/syntax-python-dark.png --sample python --dark
.\target\release\rustpad.exe --screenshot qa/syntax-json-light.png --sample json
```

**NOT EXECUTED：** 真正 OS 鍵鼠互動、檔案對話框、剪貼簿、完整 IME、無障礙與 Linux 桌面 GUI。可用工具只有瀏覽器控制，原生輸入工具未提供；不能用單元測試或截圖宣稱這些通過。egui 合成鍵事件只證明 widget/資料流程，不是原生 IME。CJK fallback 字型可能不是嚴格等寬，Tab stops 邏輯顯示欄位與實際 pixel 仍有字型差異。

Windows/Linux CI 執行 fmt/clippy/test/build；CI 無 Linux GUI/IME 視覺實測。執行狀態與 URL 於交付時另列。Linux binary 尚未散布，Linux 專屬依賴授權待 packaging audit。

效能數據、可重現命令與取捨見 [PERFORMANCE.md](PERFORMANCE.md)。歷史 0.1 驗證见 [QA-0.1.md](QA-0.1.md)。
