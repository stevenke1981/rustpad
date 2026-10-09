# InkPage 0.9 啟動調查

Windows NT 10.0.22631、使用者標示 5900X、Rust 1.99.0 MSVC；release opt-level=s / thin LTO / codegen-units=1，cargo -j1。真實 OpenGL framebuffer 1350×900 pixels / 1080×720 logical（125%），含系統 CJK 字型、readback 與 screenshot event 回傳。無清 cache、非冷開機；RDP、背景程序與 OS cache 可能影響結果。探針使用空白文件、不載入工作階段，並非輸入或 IME 延遲。

先以原始 0.5 / 0.8 已交付 EXE 做 ABBA 交錯 3 輪，各 n=6：0.5 中位數 429.5493 ms（404.7967–516.5736），0.8 413.4635 ms（397.1070–484.1165）。方向與上一輪順序測量的 +14.91% 相反，範圍重疊；無法把先前差異歸因於新增功能。原始樣本見 qa/performance-v09/recheck-v05-v08/samples.json。

最終 0.8 / 0.9 再以同一輪 ABBA 交錯 6 輪，各 n=12；兩個 EXE 已完成編譯後才測量，不並行跑其他 InkPage 探針。原始樣本與雜湊見 qa/performance-v09/compare-v08-v09/{samples,summary}.json。

| 項目 | 0.8 | 0.9 |
|---|---:|---:|
| EXE bytes | 7,594,496 | 7,602,176 |
| main 到第一個 framebuffer 中位數 ms | 370.3549 | 378.2008 |
| 最小／最大 ms | 357.6607 / 390.8159 | 347.9619 / 400.0070 |
| sampled startup PeakWorkingSet64 中位數 bytes | 188,192,768 | 185,929,728 |

本輪 0.9 中位數多 7.8460 ms（2.12%），EXE 多 7,680 bytes（0.10%）；沒有穩定加速或顯著回歸的結論。這不是統計顯著性檢驗。峰值是每 10 ms 抽樣的 process PeakWorkingSet64，不是穩態 heap，也可能漏過程序退出前峰值；不將本轮少 2,263,040 bytes（1.20%）宣稱為可靠記憶體改善。

服務成本探針使用 8 分頁，各 131,072 bytes（含 Unicode），content/saved 快照編碼後 2,097,384 bytes。0.8 原行為加診斷 CLI 的 release binary，5 次中位數：grammar 初始化 2.4341 ms、snapshot clone 0.6853 ms、compare 0.0628 ms、decode 6.1672 ms。這是同程序合成資料成本、非冷啟動、非 OS 輸入延遲，也不是完整工作階段恢復時間；見 services-baseline.json。0.9 同探針見 services-current.json。語法資料在空白／纯文字／超過著色上限時完全不載入，由注入 loader 的回歸測試確認；第一個有效著色請求才在背景初始化一次。快照解碼已有背景工作者，本輪不為小樣本改寫安全流程。

另以既有合成 dirty session 的副本執行獨立程序恢復：qa/v09-session-restore.png 可見兩個 dirty 分頁、中文🙂、選取範圍及 UTF-8 BOM / CRLF。`--startup-probe ... --session ...` 成功退出且產生 framebuffer，單筆 374.5466 ms，只證明探針可與非同步恢復共同完成，不作速度比較。原文件與 session bytes SHA 均維持相同，退出後沒有留下 lock；見 restore-safety.json。qa/v09-rust.png 可見延遲載入後實際語法顏色、繁中字型與多分頁；兩圖已檢視。

0.8 EXE SHA256：59AB06478800AED3E209A2B7768C6B9BA7E8FA71AA8E52470F1BD140726AA291。
0.9 EXE SHA256：BA19C9A49D31215736C5602822A89D9C1947C4048ABDA5B4B7D95D237FF3AD52。

Windows fmt、clippy -D warnings、77 tests、release build 通過。GitHub 目前帳號僅 pull 權限，沒有推送本輪，所以 0.9 Linux CI 未執行；Linux 桌面與七項 OS 命令鏈也未驗證。詳見 DIFF-0.9.md / QA-NATIVE.md。

重現（新的輸出資料夾，避免覆蓋證据）：

```powershell
./scripts/compare-startup.ps1 -Baseline ../performance/v08-final/rustpad.exe -Current target/release/rustpad.exe -Output qa/performance-v09/new-comparison -Rounds 6
target/release/rustpad.exe --profile-services qa/performance-v09/new-services.json
```
