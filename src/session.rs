use crate::core::Content;
use std::path::PathBuf;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Tab {
    pub path: Option<PathBuf>,
    pub content: Content,
    pub saved: Content,
    pub cursor: usize,
    pub selected: (usize, usize),
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Snapshot {
    pub tabs: Vec<Tab>,
    pub active: usize,
}
pub enum Event {
    Loaded(Result<Option<Snapshot>, String>),
    Saved(Snapshot, Result<(), String>),
}
pub struct Service {
    worker: Option<std::thread::JoinHandle<()>>,
    pub tx: std::sync::mpsc::Sender<Snapshot>,
    pub rx: std::sync::mpsc::Receiver<Event>,
    pub initial: Snapshot,
    pub persisted: Option<Snapshot>,
    pub loading: bool,
    pub busy: bool,
    pub blocked: bool,
    pub exit: bool,
    pub next: std::time::Instant,
}
struct Lock {
    path: PathBuf,
    _file: std::fs::File,
}
impl Drop for Lock {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.path);
    }
}
fn lock(path: &std::path::Path) -> Result<Lock, String> {
    let mut name = path.as_os_str().to_os_string();
    name.push(".lock");
    let path = PathBuf::from(name);
    let file = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&path)
        .map_err(|e| format!("無法鎖定工作階段（另一程序或上次異常結束的 .lock）：{e}"))?;
    Ok(Lock { path, _file: file })
}
impl Service {
    pub fn new(path: PathBuf, initial: Snapshot, ctx: eframe::egui::Context) -> Self {
        let (tx, requests) = std::sync::mpsc::channel::<Snapshot>();
        let (events, rx) = std::sync::mpsc::channel();
        let worker = std::thread::spawn(move || {
            let guard = match lock(&path) {
                Ok(guard) => guard,
                Err(e) => {
                    let _ = events.send(Event::Loaded(Err(e)));
                    ctx.request_repaint();
                    return;
                }
            };
            let mut bytes = match read(&path) {
                Ok(bytes) => bytes,
                Err(e) => {
                    let _ = events.send(Event::Loaded(Err(e)));
                    ctx.request_repaint();
                    return;
                }
            };
            let snapshot = bytes.as_deref().map(decode).transpose();
            let _ = events.send(Event::Loaded(snapshot));
            ctx.request_repaint();
            while let Ok(snapshot) = requests.recv() {
                let result = save(&path, &snapshot, bytes.as_deref()).map(|new| {
                    bytes = Some(new);
                });
                let failed = result.is_err();
                let _ = events.send(Event::Saved(snapshot, result));
                ctx.request_repaint();
                if failed {
                    break;
                }
            }
            drop(guard);
        });
        Self {
            worker: Some(worker),
            tx,
            rx,
            initial,
            persisted: None,
            loading: true,
            busy: false,
            blocked: false,
            exit: false,
            next: std::time::Instant::now(),
        }
    }
}
impl Drop for Service {
    fn drop(&mut self) {
        let (replacement, _) = std::sync::mpsc::channel();
        drop(std::mem::replace(&mut self.tx, replacement));
        if let Some(worker) = self.worker.take() {
            let _ = worker.join();
        }
    }
}
pub const MAX_SESSION: usize = 32 * 1024 * 1024;
const MAGIC: &[u8] = b"InkPage\0SESSION\x01";
fn hash(bytes: &[u8]) -> u64 {
    bytes.iter().fold(0xcbf29ce484222325, |h, b| {
        (h ^ u64::from(*b)).wrapping_mul(0x100000001b3)
    })
}
fn number(bytes: &mut Vec<u8>, value: usize) -> Result<(), String> {
    bytes.extend_from_slice(
        &u32::try_from(value)
            .map_err(|_| "工作階段數值超限")?
            .to_le_bytes(),
    );
    Ok(())
}
fn blob(bytes: &mut Vec<u8>, value: &[u8]) -> Result<(), String> {
    if bytes.len().saturating_add(value.len()).saturating_add(12) > MAX_SESSION {
        return Err("工作階段超過 32 MiB，前次快照保留".into());
    }
    number(bytes, value.len())?;
    bytes.extend_from_slice(value);
    Ok(())
}
fn content(bytes: &mut Vec<u8>, value: &Content) -> Result<(), String> {
    crate::core::encode(value)?;
    bytes.push(u8::from(value.bom));
    bytes.push(match value.newline {
        crate::core::Newline::Lf => 0,
        crate::core::Newline::Crlf => 1,
        crate::core::Newline::Cr => 2,
    });
    blob(bytes, value.text.as_bytes())
}
pub fn encode(snapshot: &Snapshot) -> Result<Vec<u8>, String> {
    if snapshot.tabs.is_empty()
        || snapshot.tabs.len() > 32
        || snapshot.active >= snapshot.tabs.len()
    {
        return Err("工作階段最多 32 分頁，前次快照保留".into());
    }
    let mut bytes = MAGIC.to_vec();
    number(&mut bytes, snapshot.tabs.len())?;
    number(&mut bytes, snapshot.active)?;
    for tab in &snapshot.tabs {
        let len = tab.content.text.chars().count();
        if tab.cursor > len || tab.selected.0 > tab.selected.1 || tab.selected.1 > len {
            return Err("工作階段游標無效".into());
        }
        bytes.push(u8::from(tab.path.is_some()));
        if let Some(path) = &tab.path {
            blob(
                &mut bytes,
                path.to_str()
                    .ok_or("工作階段不支援非 Unicode 路徑，前次快照保留")?
                    .as_bytes(),
            )?;
        }
        content(&mut bytes, &tab.content)?;
        content(&mut bytes, &tab.saved)?;
        number(&mut bytes, tab.cursor)?;
        number(&mut bytes, tab.selected.0)?;
        number(&mut bytes, tab.selected.1)?;
    }
    let checksum = hash(&bytes);
    bytes.extend_from_slice(&checksum.to_le_bytes());
    if bytes.len() > MAX_SESSION {
        return Err("工作階段超限".into());
    }
    Ok(bytes)
}
struct Reader<'a> {
    bytes: &'a [u8],
    offset: usize,
}
impl<'a> Reader<'a> {
    fn take(&mut self, len: usize) -> Result<&'a [u8], String> {
        let end = self.offset.checked_add(len).ok_or("快照長度錯誤")?;
        let value = self.bytes.get(self.offset..end).ok_or("工作階段截斷")?;
        self.offset = end;
        Ok(value)
    }
    fn byte(&mut self) -> Result<u8, String> {
        Ok(self.take(1)?[0])
    }
    fn number(&mut self) -> Result<usize, String> {
        Ok(u32::from_le_bytes(self.take(4)?.try_into().map_err(|_| "數值錯誤")?) as usize)
    }
    fn string(&mut self, limit: usize) -> Result<String, String> {
        let len = self.number()?;
        if len > limit {
            return Err("快照欄位超限".into());
        }
        String::from_utf8(self.take(len)?.to_vec()).map_err(|_| "快照不是 UTF-8".into())
    }
    fn content(&mut self) -> Result<Content, String> {
        let bom = match self.byte()? {
            0 => false,
            1 => true,
            _ => return Err("快照 BOM 錯誤".into()),
        };
        let newline = match self.byte()? {
            0 => crate::core::Newline::Lf,
            1 => crate::core::Newline::Crlf,
            2 => crate::core::Newline::Cr,
            _ => return Err("快照 EOL 錯誤".into()),
        };
        let content = Content {
            text: self.string(crate::core::MAX_BYTES)?,
            bom,
            newline,
        };
        crate::core::encode(&content)?;
        Ok(content)
    }
}
pub fn decode(bytes: &[u8]) -> Result<Snapshot, String> {
    if bytes.len() > MAX_SESSION || bytes.len() < MAGIC.len() + 16 || !bytes.starts_with(MAGIC) {
        return Err("工作階段格式／版本錯誤，保留原快照".into());
    }
    let end = bytes.len() - 8;
    if bytes[end..] != hash(&bytes[..end]).to_le_bytes() {
        return Err("工作階段完整性檢查失敗，保留原快照".into());
    }
    let mut reader = Reader {
        bytes: &bytes[..end],
        offset: MAGIC.len(),
    };
    let count = reader.number()?;
    let active = reader.number()?;
    if count == 0 || count > 32 || active >= count {
        return Err("快照分頁數無效".into());
    }
    let mut tabs = Vec::with_capacity(count);
    for _ in 0..count {
        let path = match reader.byte()? {
            0 => None,
            1 => Some(PathBuf::from(reader.string(32768)?)),
            _ => return Err("快照路徑錯誤".into()),
        };
        let content = reader.content()?;
        let saved = reader.content()?;
        let cursor = reader.number()?;
        let selected = (reader.number()?, reader.number()?);
        let len = content.text.chars().count();
        if cursor > len || selected.0 > selected.1 || selected.1 > len {
            return Err("快照游標錯誤".into());
        }
        tabs.push(Tab {
            path,
            content,
            saved,
            cursor,
            selected,
        });
    }
    if reader.offset != end {
        return Err("快照有額外資料".into());
    }
    Ok(Snapshot { tabs, active })
}
pub fn read(path: &std::path::Path) -> Result<Option<Vec<u8>>, String> {
    use std::io::Read;
    let meta = match std::fs::symlink_metadata(path) {
        Ok(meta) => meta,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(e) => return Err(e.to_string()),
    };
    if !meta.is_file() || meta.file_type().is_symlink() || meta.len() > MAX_SESSION as u64 {
        return Err("工作階段路徑必須為一般檔案且小於 32 MiB".into());
    }
    let mut bytes = Vec::new();
    std::fs::File::open(path)
        .map_err(|e| e.to_string())?
        .take((MAX_SESSION + 1) as u64)
        .read_to_end(&mut bytes)
        .map_err(|e| e.to_string())?;
    decode(&bytes)?;
    Ok(Some(bytes))
}
fn identity(path: &std::path::Path) -> Option<PathBuf> {
    path.canonicalize()
        .ok()
        .or_else(|| Some(path.parent()?.canonicalize().ok()?.join(path.file_name()?)))
}
pub fn save(
    path: &std::path::Path,
    snapshot: &Snapshot,
    expected: Option<&[u8]>,
) -> Result<Vec<u8>, String> {
    let bytes = encode(snapshot)?;
    if snapshot
        .tabs
        .iter()
        .filter_map(|t| t.path.as_deref())
        .any(|source| identity(source) == identity(path))
    {
        return Err("快照路徑不可與任何原始文件相同".into());
    }
    let current = read(path)?;
    if current.as_deref() != expected {
        return Err("工作階段已被外部修改，停止覆寫".into());
    }
    crate::core::atomic_bytes(path, &bytes)?;
    Ok(bytes)
}

#[cfg(test)]
mod tests {
    use super::*;
    fn fixture() -> Snapshot {
        Snapshot {
            tabs: vec![Tab {
                path: None,
                content: Content {
                    text: "中文🙂\n尾".into(),
                    bom: true,
                    newline: crate::core::Newline::Crlf,
                },
                saved: Content::default(),
                cursor: 3,
                selected: (1, 3),
            }],
            active: 0,
        }
    }
    #[test]
    fn restart_roundtrip_preserves_dirty_unicode_and_format() {
        let snapshot = fixture();
        let bytes = encode(&snapshot).unwrap();
        assert_eq!(decode(&bytes).unwrap(), snapshot);
        for end in 0..bytes.len() {
            assert!(decode(&bytes[..end]).is_err());
        }
        let mut bad = bytes.clone();
        bad.push(0);
        assert!(decode(&bad).is_err());
    }
    #[test]
    fn session_never_writes_original_and_rejects_conflicts_or_invalid_cache() {
        let dir = tempfile::tempdir().unwrap();
        let original = dir.path().join("original.txt");
        std::fs::write(&original, b"original").unwrap();
        let path = dir.path().join("InkPage.session");
        let mut snapshot = fixture();
        snapshot.tabs[0].path = Some(original.clone());
        let old = save(&path, &snapshot, None).unwrap();
        assert_eq!(std::fs::read(&original).unwrap(), b"original");
        assert_eq!(decode(&std::fs::read(&path).unwrap()).unwrap(), snapshot);
        std::fs::write(&path, b"external").unwrap();
        assert!(save(&path, &snapshot, Some(&old)).is_err());
        assert_eq!(std::fs::read(&path).unwrap(), b"external");
        assert!(save(&original, &snapshot, Some(b"original")).is_err());
        assert_eq!(std::fs::read(&original).unwrap(), b"original");
    }
    #[test]
    fn rejected_snapshot_and_atomic_failure_retain_previous_session() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("InkPage.session");
        let old = save(&path, &fixture(), None).unwrap();
        let mut bad = fixture();
        bad.tabs[0].content.text.push('\0');
        assert!(save(&path, &bad, Some(&old)).is_err());
        assert_eq!(std::fs::read(&path).unwrap(), old);
        assert!(save(&dir.path().join("missing").join("cache"), &fixture(), None).is_err());
        assert_eq!(std::fs::read(&path).unwrap(), old);
    }
}
