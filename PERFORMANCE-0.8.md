# InkPage 0.8 體積與啟動實測

同一台 Windows NT 10.0.22631，使用者標示 5900X，Rust 1.99.0 MSVC，release opt-level=s／thinLTO／codegen-units=1，cargo -j1。編譯結束後序列量測0.5五次、0.8五次；未清cache，非冷啟動，RDP／其他背景程序可能影響結果。原始 JSON：qa/performance-v08/baseline、current。scripts/measure.ps1 可重現。

| 項目 | InkPage 0.5 | InkPage 0.8 |
|---|---:|---:|
| EXE bytes | 7,469,568 | 7,594,496 |
| main 到第一張 framebuffer 中位 ms，n=5 | 368.6349 | 423.6045 |
| sampled startup PeakWorkingSet64 中位 bytes | 192,344,064 | 188,178,432 |

EXE 增加 124,928 bytes（1.67%）。此輪啟動多 54.9696 ms（14.91%），如實記錄；小樣本及順序/cache/背景環境不能建立統計因果，沒有速度提升宣稱。PeakWorkingSet不是穩態heap或編輯文件記憶體。

啟動probe為自己的真實 OpenGL framebuffer、OS CJK 字型、1350×900 pixels／1080×720 logical（125%），含readback/event回傳；probe停用工作階段，不量測大型快照還原。非原生按鍵反應或IME延遲。附帶headless只有16/128KiB grammar/LayoutJob/font/String插入小量基準，非大型benchmark；沒有對大型文件 throughput 或 Linux GUI 作承諾。

0.5 EXE SHA256：061A01402A109F8D6A1836FCF2D57E49D5007660A55803BCDA1759D525FF6812。
0.8 EXE SHA256：59AB06478800AED3E209A2B7768C6B9BA7E8FA71AA8E52470F1BD140726AA291。
