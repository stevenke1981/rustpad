# 0.2 實測：速度與體積

2026-10-09，Windows NT 10.0.22631 x86_64 MSVC，Rust 1.99.0；使用者指明 5900X，CIM 讀取遭拒，未獨立核實。全程 cargo -j1，沒有 UPX、panic=abort、移除 CJK 字體或關閉安全設定。不同編譯設定順序測量，不做大量 benchmark。

基線為 df67f9c1cc0945ef36976ad4ec1c361aaf759188。原始 exe/portable ZIP 保存在專案外 performance/baseline，未覆寫；啟動與排版時間使用該 commit 的另份 source 加入同一小型探針（有 40,960 bytes instrumentation overhead），不能宣稱原始 exe 本身已量測鍵鼠延遲。

## 體積與啟動

| 版本 | release 設定 | exe bytes | exe-only ZIP bytes | 首張 framebuffer 中位 ms | 取樣啟動峰值 MiB |
|---|---|---:|---:|---:|---:|
| 0.1 原始基線 | opt=3 / CGU=16 / thin LTO / strip | 5,444,608 | 2,430,599 | 380.45* | 177.39* |
| 0.2 速度預設 | opt=3 / CGU=16 / thin LTO / strip | 7,292,928 | 3,638,666 | 380.32 | 183.41 |
| **0.2 採用** | **opt=s / CGU=1 / thin LTO / strip** | **6,138,880** | **3,178,101** | **384.25** | **182.20** |

\*基線時間/記憶體使用 5,485,568-byte probe build；體積使用保留的原始 exe。ZIP 欄都是僅一份 exe、相同 DEFLATE level=9，用以隔離包裝/授權文件差異，**不是交付的 portable bundle 體積**。0.1 完整 portable ZIP 為 3,068,945 bytes；0.2 完整 bundle 大小於交付另列。

採用設定比同功能 0.2 預設 exe 小 **15.82%**，exe-only ZIP 小 **12.66%**。相對 0.1，完整語法功能增加後 exe 仍大 **12.75%**，不宣稱整體縮小。啟動差異約 4 ms，資料不足以宣稱改善。未做完全冷啟動或多機統計。

每版本在編譯完成後執行 5 次啟動、OS/檔案/字型快取未清空，從 `main()` 到第一個原生 OpenGL screenshot event 返回計時，包含 framebuffer 讀回與 event delivery，**不是螢幕呈現到光子的時間**。空白單文件、搜尋欄關閉，同一視窗與 Windows 系統字型；PNG 讀回可能影響耗時。原生截圖為 1350×900、邏輯視窗 1080×720，顯示縮放約 125%；遠端/RDP 狀態未可靠取得，不能外推一般本地桌面。記憶體是每 10 ms 讀 Process.PeakWorkingSet64 的啟動峰值，不是穩態編輯用量，也不是 egui 純 heap。

## 代表性內容

16 / 128 KiB Rust 測試內容包含中文、emoji、字串、數字、跨行註解；5 次修改前綴後取中位數，使用 headless egui 預設內建字型。這些是語法/排版 pipeline 微量測，**不是 native typing latency、CJK 系統字型性能或完整編輯事件**。String edit 計時只涵蓋 insert_str，不包含 clone、history、gutter、UI redraw。

| 版本 / bytes | 語法 + LayoutJob ms | 未快取 fonts.layout_job ms | 已快取 fonts.layout_job ms | String insert ms |
|---|---:|---:|---:|---:|
| 0.1 / 16,384 | 0.5952 | 1.5929 | 0.2262 | 0.0014 |
| 0.1 / 131,072 | 5.0260 | 11.3465 | 1.6855 | 0.0172 |
| 0.2 opt3 / 16,384 | 12.0631 | 1.1529 | 0.0831 | 0.0028 |
| 0.2 opt3 / 131,072 | 95.0476 | 8.3564 | 0.7534 | 0.0100 |
| 0.2 採用 / 16,384 | 13.1990 | 1.1441 | 0.0944 | 0.0012 |
| 0.2 採用 / 131,072 | 104.6942 | 8.6480 | 0.8133 | 0.0097 |

新增完整 grammar 明顯比簡單 keyword scanner 慢，兩者功能不同，不作等價 throughput 比較。採用小體積設定在 128 KiB 背景分析慢約 10.15%；取捨為較小 exe。背景 worker 取消過時任務、完成後重用快取，UI 不等語法分析完成，但仍會在 UI 執行字形排版/搜尋/編輯。>256 KiB 明示純文字；尚未承諾最壞狀況鍵鼠延遲。

空閒 frame 不再複製整份 Content；只在有 input events 時保留修改前快照。穩定畫面不重新解析 grammar。保留必要 default fonts / glow / x11 / wayland，原有 eframe default-features=false 已避免 wgpu；沒有移除 native dialog 或 clipboard。語法採 onig 以保留 PowerShell，fancy 版本語法集合缺 PowerShell，因此未為縮小體積犧牲指定功能。

可重現：

```powershell
cargo build --release --locked -j1
.\scripts\measure.ps1 -Exe .\target\release\rustpad.exe -Output .\target\measure
# exe --benchmark <JSON>：headless 微量測
# exe --startup-probe <JSON>：原生第一張 framebuffer；另產生同名 PNG
```

原始 JSON 與每次啟動 sample 保留 `qa/performance/`。探針 source `src/perf.rs`；本機 baseline archive/probe 與候選 exe 保留於專案外 `../performance`，不納入 Git 或 portable ZIP。採用 exe SHA256：`75BCC691B5ACEBE4A5F4EA3C0143F279FFCCE568ECD7506E8944CBE15D0FF82B`。
