# InkPage 0.10 驗證（2026-10-09，台灣）

Windows x64 MSVC：cargo fmt --check、cargo clippy --locked -j1 --all-targets -- -D warnings、cargo test --locked -j1、cargo build --release --locked -j1 通過；87 項測試全部通過。無新增執行期依賴。git diff --check 與既有固定矩陣檢查通過。

新增測試覆蓋多檔拖入、dirty 文件保留、重複路徑、UTF-8 BOM／CRLF／中文 emoji、NUL 與資料夾拒絕、busy／關閉確認延後、32 項佇列限制；分頁重排後 active ID、cursor／selection、history、關閉目標及 session 順序；設定重啟、外部衝突、損壞原檔保留；四語系完整列、全部 placeholder 一致、UI key 覆蓋、既有訊息即時翻譯與 opaque 值保留；PNG RGBA／透明像素及九尺寸 ICO。

完整 App::update 的框架事件測試：raw dropped_files 經背景讀取開啟文件且原始 bytes／未存分頁保持；pointer press／move／release 在真實分頁 widget 完成排序；Esc 取消後順序保持；點擊介面語言選單選日本語，驗證控制項、viewport title 與設定檔重讀。框架事件不是 OS 檔案總管的 OLE 拖曳。

最終 release 的實際 OpenGL renderer 圖片：qa/v10-zh-TW.png、qa/v10-zh-CN.png、qa/v10-en.png、qa/v10-ja.png，四次程序 exit=0。人工檢視四語系選單／搜尋列／分頁／狀態列；內容中的繁中、emoji 與 query 保留。qa/v10-icon-sizes.png 檢視 16–256 圖示；16 px 仍能辨識紙頁與筆尖。

scripts/verify_windows_icon.py 以資源讀取模式檢查最終 EXE，未啟動程式：RT_GROUP_ICON 含 16／20／24／32／40／48／64／128／256，各 RT_ICON 長度一致；ProductName=InkPage、FileDescription=InkPage Text Editor、FileVersion／ProductVersion=0.10.0。結果 qa/v10-icon-resources.json。

原生 Windows 工具初始 session_status ready=true，測試程式視窗成功列出英文標題 InkPage — Text editor。準備擷取時远端桌面停止渲染，capture 與後續關閉工具回覆 GetCursorPos permission denied；session_status ready=false、blocker=remote-not-rendering。依 autogui-control 技能停止原生操作，未改 RDP／鎖定／權限設定。只清理本輪以独立設定與 --no-session 啟動的空白 QA 程序。尚未完成真實檔案總管拖曳、原生滑鼠分頁拖曳或檔案總管圖示快取顯示驗收；還原 RDP 視窗可再測。Linux GUI、本輪 Linux build 與完整 IME 未執行。

可攜包使用明示白名單打包執行檔、公開文件、授權及原創圖示，排除 InkPage.settings／InkPage.session／鎖檔與使用者內容。ZIP 與解壓執行檔的 SHA-256 與來源 release 核對；包內程式另以獨立截圖流程啟動驗證。
