# 0.3 與 0.2 同環境實測

2026-10-09（台灣），Windows NT 10.0.22631 / x86_64 MSVC / Rust 1.99.0，使用者指明 5900X（CIM 未獨立核實）。同機順序量測、編譯已結束，其他桌面程式未停止；OS/檔案/字型快取未清空。cargo -j1，release opt=s / codegen-units=1 / thin LTO / strip 都保持不變，沒有新增依賴、UPX、panic=abort 或移除字型/平台支援。

| 指標 | 0.2 / 4fb9b16 | 0.3 |
|---|---:|---:|
| exe bytes | 6,138,880 | 6,196,736 |
| 僅 exe ZIP bytes（DEFLATE 9） | 3,178,101 | 3,204,716 |
| 首張 framebuffer 中位 ms（n=5） | 422.1249 | 421.9483 |
| 取樣啟動峰值 MiB | 179.34 | 179.36 |
| 16 KiB 語法 + LayoutJob 中位 ms | 12.7835 | 13.2730 |
| 128 KiB 語法 + LayoutJob 中位 ms | 102.8008 | 104.3055 |
| 128 KiB 未快取 fonts.layout_job 中位 ms | 8.6804 | 8.5815 |
| 128 KiB 已快取 fonts.layout_job 中位 ms | 0.8109 | 0.8167 |
| 128 KiB String insert 中位 ms | 0.0091 | 0.0080 |

新能力增加 exe **57,856 bytes（0.94%）**，僅 exe ZIP 增加 **26,615 bytes（0.84%）**。啟動差異不足 1 ms，不主張速度改善；其他小差異也可能來自量測/排程波動。此輪優先修復安全與導航功能，不承諾速度和體積同時變好。完整 portable ZIP 含文件與授權，其大小於交付另列，不能用僅 exe ZIP 欄代替。

啟動從 main 到第一張真正 OpenGL screenshot event 返回，包含 GPU framebuffer 讀回與 event delivery，不是螢幕光子延遲。空白單文件、搜尋欄關閉、相同視窗/Windows CJK 字型；1350×900 pixels 對應 1080×720 邏輯尺寸，約125%縮放。遠端/RDP 狀態未可靠取得。每10ms取 Process.PeakWorkingSet64，是啟動峰值、不是穩態 heap。

headless 語法/排版測量為同一 src/perf.rs 的中文/emoji/跨行註解 Rust 16/128 KiB fixture，每組5次修改後中位數，使用 egui 內建預設字型；不是 native typing、CJK 系統字型或新搜尋 UI 延遲。String insert 不含 clone/history/gutter/redraw。新搜尋的>256 KiB 或>128-byte query 放背景，正確性以18,000行 thread/channel 測試驗證，未把它當 native latency benchmark。

原始 JSON：`qa/performance-v03/{baseline,current}-{summary,benchmark,startup-samples}.json`。0.2 原始 exe 保存在 `../performance/v02-final/rustpad.exe`，SHA256仍為 `75BCC691B5ACEBE4A5F4EA3C0143F279FFCCE568ECD7506E8944CBE15D0FF82B`，沒有覆寫。0.1 原始檔也保留。

0.3 exe SHA256：`EA7A0C1AF02F65A564485A401FD0E649BFC69A9375DB229C5556C8BF2AB2D839`。

```powershell
cargo build --release --locked -j1
.\scripts\measure.ps1 -Exe .\target\release\rustpad.exe -Output .\target\measure
```

完整條件與 0.1/0.2 編譯設定對照保留 [PERFORMANCE.md](PERFORMANCE.md)。
