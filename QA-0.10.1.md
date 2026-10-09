# InkPage 0.10.1 驗證（2026-10-09，台灣）

Windows x64 MSVC，Rust 1.99.0；未修改系統環境。修正前保留 0.10 原始檔與未提交差異，舊版下載包仍保留。

故障基準：完整 eframe 原生視窗，使用 App::new 與 PostMessageW 投遞包含真實 HGLOBAL / DROPFILES / UTF-16 路徑的 WM_DROPFILES。0.10 未接收該訊息，四秒後測試失敗：`native drop timed out: 1 tabs, status 就緒 · UTF-8 編輯器 · 2 MiB 上限`。此測試没有注入 egui dropped_files，也沒有使用滑鼠或檔案總管；測到原生 Windows 訊息入口，不代表已重播使用者的那一次桌面拖曳。

修正後同一入口通過，原生探針另外加入第二批重複拖入。使用真實 eframe/winit 視窗與正式 App 初始化，核對 WS_EX_ACCEPTFILES，再經 Windows 訊息佇列 → 原生回呼 → raw_input_hook → 背景讀檔 → 文件分頁。測試覆蓋中文／emoji 路徑、UTF-16 長度超過 260 的路徑、多檔開啟、UTF-8 BOM／CRLF 保留、原有 dirty 文件保留；NUL 檔案、不存在檔案與資料夾列出三項錯誤。第二批重複拖入不新增分頁，即使磁碟改成無效內容也保留已開啟文件的未存修改；同批重複略過一項。核對測試檔案 bytes，開啟流程不寫原始文件。

命令：`cargo test --locked -j1 windows_shell_drop -- --ignored --nocapture`。原生測試為顯式執行項目，避免一般無桌面 CI 環境自動開啟視窗。`INKPAGE_NATIVE_DROP_SCREENSHOT` 可指定原生探針成功後的程式 renderer 截圖；本次 qa/v101-native-drop.png 已檢視，三個分頁、dirty 指示與「已開啟 2、略過 1」結果正常。

一般測試 87 項通過，原生測試另 1 項通過，共 88 個測試案例；cargo fmt --check、cargo clippy --locked -j1 --all-targets -- -D warnings 通過。打包前檢查 release build、嵌入圖示與 0.10.1 版本資源；包內不含個人設定或工作階段。套件檢查結果記錄於 qa/v101-package-verification.json。

遠端桌面 session_status 回覆 ready=false、blocker=remote-not-rendering、cursor_access=false。依 [autogui-control](C:/Users/steven/.codex/skills/autogui-control/SKILL.md) 的「If ready is false, report its blocker codes and advice to the user instead of retrying; never try to unlock, reconnect or change session settings yourself」停止桌面輸入；沒有改 RDP 或嘗試解鎖。此版已驗證原生訊息與實際 renderer，檔案總管實體滑鼠拖入仍待 RDP 客戶端恢復顯示後人工確認；Linux GUI／Linux build／完整 IME 未執行。
