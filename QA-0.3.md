# RustPad 0.3 驗證記錄

2026-10-09（台灣），Windows NT 10.0.22631 / x86_64 MSVC / Rust 1.99.0。主機 5900X 為使用者提供，CIM 未獨立核實。編譯 -j1；不安裝新依賴，不改其他專案或重啟客戶端。

## 自動驗收與 RED

fmt、clippy `--locked -j1 --all-targets -- -D warnings` 與 test 通過：**42 tests / 0 failed**。沿用 0.2 的 Unicode/BOM/EOL、原子失敗保存舊檔、dirty 取消/關閉、undo、語法跨行與 byte ranges 等 27 項，加 15 項回歸。

RED 記錄：

1. 搜尋條件/前後導航/選取取代/跳行四項先在未實作 stub 上失敗，最小實作後轉綠。
2. 行複製/刪除與 Unicode 大小寫兩項先失敗，再實作；含 EOF 空行、ß→SS、BOM/CRLF、無選取、20k 行上限不部分修改。
3. 改 query 或搜尋條件後沿用舊命中會把「Rust」改成錯誤 replacement：先重現失敗，再加入搜尋 signature 保護。
4. 游標位於重疊命中內，向前/向後錯過最近匹配：先重現失敗，再使用游標範圍的正向 find / 反向 rfind；計數/批次取代仍維持不重疊。
5. 希臘字母 sigma 的字串/單字元小寫規則不同，來源與 query 原本不對稱，連相同 `ΟΣ` 都找不到：先重現，再統一兩側的 per-scalar lowercase 映射；不承諾 full case folding。
6. 重疊範圍已找到但單項取代沿用全文件不重疊集合，導致無法取代：先重現，再讓單項取代驗證當前游標命中，批次計數/取代仍不重疊。

另外驗證分頁內容隔離、新建非 dirty、選取範圍 count/replace 共用條件、一次 undo/redo、非法跳行不 dirty。背景工作測試使用 18,000 行（>256 KiB）實際 thread/channel：count、批次取代、undo，以及工作期間修改內容、改 query、切分頁時結果不得修改新狀態。這兩項背景回歸在實作後加入，未冒稱先行 RED。

## 真實 renderer

最後 release 以原生 Windows eframe/glow screenshot event 輸出，逐張查看：

- `qa/v03-search.png`：暗色搜尋，忽略大小寫/全字/循環條件、上一個選取、文件 count=4。
- `qa/v03-navigation.png`：500 行文件跳至第 350 行，目標在視窗內，gutter 與內容行號一致，內容不 dirty。
- `qa/v03-goto.png`：繁中行號對話框與範圍提示。
- `qa/v03-close.png`：dirty 分頁與儲存/捨棄/取消確認。
- `qa/v03-edits.png`：Unicode 大寫展開及複製後 STRASSE，dirty、選取、CRLF/EOF 覆蓋標記。

```powershell
.\target\release\rustpad.exe --screenshot qa/v03-search.png --qa-flow search --dark
.\target\release\rustpad.exe --screenshot qa/v03-navigation.png --qa-flow navigation
.\target\release\rustpad.exe --screenshot qa/v03-goto.png --qa-flow goto
.\target\release\rustpad.exe --screenshot qa/v03-close.png --qa-flow close
.\target\release\rustpad.exe --screenshot qa/v03-edits.png --qa-flow edits
```

這些是 app 自己的 QA fixture 經真正 renderer 輸出的 pixels，不是 mock；**沒有使用 OS 原生鍵鼠操作**。矩陣完成等級明確限定資料/egui widget 與 renderer 驗收，不能推出真人端到端通過。

**NOT EXECUTED：** 原生鍵鼠、檔案對話框/剪貼簿、完整 IME、Linux 桌面 GUI、無障礙。對話框/剪貼簿仍在矩陣列部分或未驗證。Windows/Linux CI 的 fmt/clippy/test/build 終態另附交付 URL，不等同 Linux GUI。Linux binary 未散布，平台專屬授權 packaging audit 待做。

測量見 [PERFORMANCE-0.3.md](PERFORMANCE-0.3.md)，用例/差異見 [DIFF-0.3.md](DIFF-0.3.md)。歷史驗收：[0.2](QA-0.2.md)、[0.1](QA-0.1.md)。
