# 墨頁 InkPage 0.5 性能與體積（2026-10-09）

同機 Windows NT 10.0.22631、Rust 1.99.0 MSVC、release opt-level=s / thin LTO / codegen-units=1，cargo -j1。使用者標示 5900X；CPU 型號未獨立驗證。編譯完成後依序量測原 0.4 與最終 0.5；桌面背景程式未停、cache 未清除，非冷啟動。內部 binary 仍 rustpad.exe；可攜包名稱 InkPage.exe 不改 bytes。

| 指標 | 原 0.4（6926642） | 最終 InkPage 0.5 | 差異 |
|---|---:|---:|---:|
| EXE bytes | 7,430,656 | 7,469,568 | +38,912（0.52%） |
| EXE 單檔 DEFLATE9 ZIP bytes | 3,652,807 | 3,669,636 | +0.46% |
| main→第一個 framebuffer 中位 ms（5 次） | 419.4204 | 410.0447 | -2.24% |
| 啟動 sampled PeakWorkingSet64 中位 bytes | 188,108,800 | 188,043,264 | 非穩態 heap |

約 9.4 ms 的差異屬小樣本/cache/排程敏感結果，不宣稱速度或記憶體提升。介面置中調整前曾有另一輪 5 次探針，方向相反；該草稿留在本機、不混入此最終比較。直接使用 unicode-segmentation 1.13.3 已存在的 locked 版本，沒有新增/升級其他套件。

| headless 5 次中位 ms | 0.4 | 0.5 |
|---|---:|---:|
| 16 KiB grammar + LayoutJob | 13.2495 | 13.1529 |
| 128 KiB grammar + LayoutJob | 105.0948 | 104.7396 |
| 128 KiB uncached font layout | 8.8142 | 8.3895 |
| 128 KiB cached font layout | 0.8377 | 0.8132 |
| 128 KiB String insert | 0.0081 | 0.0090 |

啟動是原生 OpenGL、OS CJK 字型、1350×900 pixels / 1080×720 logical（125%），包含 framebuffer readback/event 延遲；不是 photons 或真人按鍵延遲。每 10 ms 取 Process.PeakWorkingSet64，RDP 狀態未辨識。headless 排版用預設嵌入字型；String insert 排除 clone/history/gutter/redraw。單檔 ZIP 同名 InkPage.exe/DEFLATE9 比較，未包含公開說明／授權文件；完整可攜 ZIP 大小由交付檔記錄。

沒有書籤大量修改、字素分割 throughput、原生 IME／emoji shaping 或 Linux GUI benchmark，單元測試不是性能證據。書籤以文字快照/完整行 prefix-suffix 比較，無增量編輯性能承諾。原始最終数据：qa/performance-v05-final/baseline 與 current 的 JSON，scripts/measure.ps1 可重現同類探針。

0.4 EXE SHA256：8D425FD524372D8CFDB0D379AF83A7E6D6161F6A901E7792744479B5E55E53FE。
0.5 EXE SHA256：061A01402A109F8D6A1836FCF2D57E49D5007660A55803BCDA1759D525FF6812。
