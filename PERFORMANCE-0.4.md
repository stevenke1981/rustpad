# RustPad 0.4 性能與體積（2026-10-09）

同機 Windows NT 10.0.22631、Rust 1.99.0 MSVC、release opt-level=s / thin LTO / codegen-units=1，cargo -j1。使用者標示 5900X，CIM 不允許讀 CPU，未獨立驗證型號。編譯完成後依序測量 0.3 與 0.4，背景桌面程式未停，檔案 cache 未清除；非冷啟動、非跨機比較。

| 指標 | 0.3 ef88f98 | 0.4 | 差異 |
|---|---:|---:|---:|
| EXE bytes | 6,196,736 | 7,430,656 | +1,233,920（19.91%） |
| EXE 單檔 DEFLATE9 ZIP bytes | 3,204,716 | 3,652,807 | +13.98% |
| main→第一個 framebuffer 中位 ms（5 次） | 384.8894 | 407.4335 | +5.86% |
| 啟動 sampled PeakWorkingSet64 中位 bytes | 191,094,784 | 188,125,184 | 不代表穩態 heap |

EXE 增量包含完整 Unicode regex engine 與功能程式，不是單独 regex 相依的精確歸因。這輪未測得速度提升，不以小樣本宣稱顯著退化或改善；約 22.5 ms 的啟動差異可能受 cache/排程影響。單檔壓縮比較不包含說明與授權文件，完整可攜 ZIP 另由交付檔實際大小記錄。

| headless 5 次中位 ms | 0.3 | 0.4 |
|---|---:|---:|
| 16 KiB grammar + LayoutJob | 12.9925 | 13.2128 |
| 128 KiB grammar + LayoutJob | 103.5970 | 105.3039 |
| 128 KiB uncached font layout | 8.6453 | 9.0530 |
| 128 KiB cached font layout | 0.8374 | 0.8320 |
| 128 KiB String insert | 0.0085 | 0.0062 |

啟動使用原生 OpenGL、OS CJK 字型、1350×900 pixels / 1080×720 logical（125%），包含 framebuffer readback/event 延遲；不是畫面 photons／真人按鍵延遲。每 10 ms 取 Process.PeakWorkingSet64；RDP 狀態未辨識。headless fixture 使用預設嵌入字型，String insert 不包含 clone/history/gutter/redraw。沒有 regex throughput／原生 IME／Linux GUI 性能量測；18,000 行 regex 測試是功能回歸，不能當性能 benchmark。

原始數據：qa/performance-v04/baseline 與 current 的 summary.json、startup-samples.json、startup-0..4.json、benchmark.json。scripts/measure.ps1 可重現同樣的 opt-in 探針；避免把目前結果與上輪不同 cache/背景環境的數值直接混比。

0.3 保存的 EXE SHA256：EA7A0C1AF02F65A564485A401FD0E649BFC69A9375DB229C5556C8BF2AB2D839。
0.4 EXE SHA256：8D425FD524372D8CFDB0D379AF83A7E6D6161F6A901E7792744479B5E55E53FE。
