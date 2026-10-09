# RustPad 0.4 驗證

Windows 11 / Rust 1.99.0 MSVC，cargo -j1。fmt、clippy --all-targets -D warnings、locked test、release build；Windows 54 tests，Ubuntu CI 另有 Unix symlink loop test（55 tests）。CI 終態以本次提交的 GitHub Actions 為準。

RED → GREEN：先放 regex stub，Unicode/capture 與零長度批次 2 tests 失敗；實作後通過。再放 filesearch stub，BOM/CRLF/binary/root 與取消 2 tests 失敗；實作後通過。新增 app 零長度連續下一個 regression，實際停留 (0,0) 而預期 (1,1) 失敗；修正前進一個 scalar，通過，capture 展開後游標按實際長度定位。另有範圍計算 fixture 原預期 (9,13)，手算修正 normalized scalar (8,12)；不是產品 bug。

其餘補充 regression 在功能實作後加入：replacement template 與官方 regex replace_all 對照、18,000 行搜尋、編譯／匹配／輸出上限原文不變、非法語法 UI 可見、background 多檔結果與磁碟變更拒絕定位、dirty tab 不覆寫、結果上限／大檔略過。既有 42 tests 保留。Linux symlink 循環／外根連結在 Ubuntu CI 執行；Windows junction 實體 fixture 未執行，只有 Windows reparse skip 程式分支。

## 真正 Windows framebuffer

四張 PNG 是 release executable 透過 eframe/glow Screenshot 事件讀回的原生 renderer pixels，明確 opt-in QA fixtures 後自動退出：

- qa/v04-regex.png：群組替換成 `12:中文 $`、`34:中文 $`，dirty、UTF-8 BOM/CRLF 保留。
- qa/v04-regex-error.png：非法 `(` 明示 parse error，內容未變。
- qa/v04-files.png：3 檔候選、1 binary 略過、4 處磁碟命中，底部根/條件/路徑/行號/預覽。
- qa/v04-file-navigation.png：背景重讀後開啟 example.rs、選取第 2 行 Rust，保留既有分頁。

重現：`rustpad.exe --screenshot <output.png> --qa-flow regex|regex-error|files|file-navigation --qa-root <qa/v04-fixtures absolute path> --dark`。後兩者才需要 root；QA 根必須是自有 fixtures。畫面已逐張檢視，檔案面板使用相對路徑避免把本機完整路徑放進公開截圖。截圖不代表真人輸入或系統事件已驗證。

**NOT EXECUTED**：原生键鼠/剪貼簿/系統選檔與資料夾對話框、完整繁中 IME、Linux desktop GUI、Windows junction fixture、並行 symlink 攻擊與硬即時取消。單元測試/egui model 與 renderer 不冒充這些端到端驗證。

歷史紀錄：QA-0.1.md、QA-0.2.md、QA-0.3.md。效能與體積見 PERFORMANCE-0.4.md；此處不宣稱任意 regex／大型檔案的流暢度。
