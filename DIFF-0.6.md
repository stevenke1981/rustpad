# InkPage 0.6

新增 F24 / F34，固定矩陣 27/36；原生七項仍保守維持原狀態。

Ctrl+B 背景解析括號，使用原有 syntect grammar 的 string/comment scope；不以著色顏色猜字串。Unicode scalar 座標，配對 () [] {}，不匹配破損交叉巢狀。純文字模式不推斷字串／註解；非程式語言 grammar 能辨識的 scope 才排除。分析回來核對分頁、內容、語言與游標，過期結果不套用。

編輯選單提供選取行增減縮排。Tab 跨行選取增加縮排；Shift+Tab 減少一個 Tab 或最多設定寬度的 ASCII 空格。選取終點在下一行開頭不處理該行。Enter 繼承目前行游標前的 ASCII 空格／Tab 前綴，替換選取；不插入配對括號或改 Unicode 空白。所有修改先驗證輸出上限，一次 undo，保留 BOM/EOL。組字事件不攔截 Enter；原生 IME 尚未驗證。

驗證：先加入 stub，縮排兩項與括號兩項回歸確實失敗，再實作。Windows Rust 1.99.0，cargo -j1：67 tests 通過、clippy -D warnings 通過。真實 Windows framebuffer：qa/v06-bracket.png、qa/v06-indent.png（CLI 種子操作，不是 OS 鍵鼠）。原生工具缺 node_repl / sky，現有 CUA 原生控制停用；因此不把現有 F01–05、F09、F10 算完成。Linux 僅 CI build/test。
