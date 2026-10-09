use crate::actions::SearchOptions;
use std::{
    path::{Path, PathBuf},
    sync::atomic::{AtomicBool, Ordering},
};
#[derive(Clone, Debug)]
pub struct Hit {
    pub path: PathBuf,
    pub line: usize,
    pub preview: String,
    pub range: (usize, usize),
    pub fingerprint: u64,
}
#[derive(Default)]
pub struct Report {
    pub hits: Vec<Hit>,
    pub files: usize,
    pub skipped: usize,
    pub cancelled: bool,
    pub limited: bool,
}
pub const MAX_FILES: usize = 1_000;
pub const MAX_HITS: usize = 2_000;
pub fn fingerprint(text: &str) -> u64 {
    use std::hash::{Hash, Hasher};
    let mut hash = std::collections::hash_map::DefaultHasher::new();
    text.hash(&mut hash);
    hash.finish()
}
fn linked(meta: &std::fs::Metadata) -> bool {
    if meta.file_type().is_symlink() {
        return true;
    }
    #[cfg(windows)]
    {
        use std::os::windows::fs::MetadataExt;
        meta.file_attributes() & 0x400 != 0
    }
    #[cfg(not(windows))]
    {
        false
    }
}
pub fn scan(
    root: &Path,
    query: &str,
    options: SearchOptions,
    cancel: &AtomicBool,
) -> Result<Report, String> {
    if query.is_empty() || query.len() > 16_384 {
        return Err("搜尋字串須為 1 至 16 KiB".into());
    }
    let regex = if options.regex {
        Some(crate::pattern::compile(query, options)?)
    } else {
        None
    };
    let meta = std::fs::symlink_metadata(root).map_err(|e| e.to_string())?;
    if !meta.is_dir() || linked(&meta) {
        return Err("掃描根必須是實體資料夾；不接受 symlink/junction".into());
    }
    let root = root.canonicalize().map_err(|e| e.to_string())?;
    let mut report = Report::default();
    let mut pending = vec![(root.clone(), 0)];
    let mut entries = 0;
    let mut bytes = 0;
    'scan: while let Some((dir, depth)) = pending.pop() {
        if cancel.load(Ordering::Relaxed) {
            report.cancelled = true;
            break;
        }
        let directory = match std::fs::read_dir(&dir) {
            Ok(d) => d,
            Err(_) => {
                report.skipped += 1;
                continue;
            }
        };
        for item in directory {
            if cancel.load(Ordering::Relaxed) {
                report.cancelled = true;
                break 'scan;
            }
            entries += 1;
            if entries > 10_000 {
                report.limited = true;
                break 'scan;
            }
            let path = match item {
                Ok(e) => e.path(),
                Err(_) => {
                    report.skipped += 1;
                    continue;
                }
            };
            let meta = match std::fs::symlink_metadata(&path) {
                Ok(m) => m,
                Err(_) => {
                    report.skipped += 1;
                    continue;
                }
            };
            if linked(&meta) {
                report.skipped += 1;
                continue;
            }
            let canonical = match path.canonicalize() {
                Ok(p) if p.starts_with(&root) => p,
                _ => {
                    report.skipped += 1;
                    continue;
                }
            };
            if meta.is_dir() {
                if depth >= 32 {
                    report.limited = true;
                    report.skipped += 1;
                } else if !path
                    .file_name()
                    .is_some_and(|n| n == ".git" || n == "target")
                {
                    pending.push((canonical, depth + 1));
                } else {
                    report.skipped += 1;
                }
                continue;
            }
            if !meta.is_file() {
                report.skipped += 1;
                continue;
            }
            if report.files >= MAX_FILES {
                report.limited = true;
                break 'scan;
            }
            report.files += 1;
            if meta.len() > crate::core::MAX_BYTES as u64 {
                report.skipped += 1;
                continue;
            }
            if bytes + meta.len() > 32 * 1024 * 1024 {
                report.limited = true;
                break 'scan;
            }
            bytes += meta.len();
            let content = match crate::core::read_file(&canonical) {
                Ok(c) => c,
                Err(_) => {
                    report.skipped += 1;
                    continue;
                }
            };
            let offsets: Vec<_> = content
                .text
                .char_indices()
                .map(|p| p.0)
                .chain(std::iter::once(content.text.len()))
                .collect();
            let literal;
            let ranges: Box<dyn Iterator<Item = (usize, usize)> + '_> = if let Some(re) = &regex {
                Box::new(
                    re.find_iter(&content.text)
                        .filter(|m| {
                            crate::pattern::accepted(&content.text, m.start(), m.end(), options)
                        })
                        .map(|m| {
                            (
                                offsets.binary_search(&m.start()).unwrap(),
                                offsets.binary_search(&m.end()).unwrap(),
                            )
                        }),
                )
            } else {
                literal = crate::actions::matches(&content.text, query, options);
                Box::new(literal.into_iter())
            };
            let hash = fingerprint(&content.text);
            for range in ranges {
                if cancel.load(Ordering::Relaxed) {
                    report.cancelled = true;
                    break 'scan;
                }
                if report.hits.len() >= MAX_HITS {
                    report.limited = true;
                    break 'scan;
                }
                let start = offsets[range.0];
                let line = content.text[..start]
                    .bytes()
                    .filter(|b| *b == b'\n')
                    .count()
                    + 1;
                let line_start = content.text[..start].rfind('\n').map_or(0, |p| p + 1);
                let preview = content.text[line_start..]
                    .split('\n')
                    .next()
                    .unwrap_or("")
                    .chars()
                    .take(160)
                    .collect();
                report.hits.push(Hit {
                    path: canonical.clone(),
                    line,
                    preview,
                    range,
                    fingerprint: hash,
                });
            }
        }
    }
    Ok(report)
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn result_cap_and_large_files_are_explicit() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("large.txt"),
            vec![b'a'; crate::core::MAX_BYTES + 1],
        )
        .unwrap();
        std::fs::write(dir.path().join("hits.txt"), "中文\n".repeat(MAX_HITS + 10)).unwrap();
        let report = scan(
            dir.path(),
            "中文",
            Default::default(),
            &AtomicBool::new(false),
        )
        .unwrap();
        assert!(report.limited);
        assert_eq!(report.hits.len(), MAX_HITS);
        let large = dir.path().join("large.txt");
        std::fs::remove_file(dir.path().join("hits.txt")).unwrap();
        let report = scan(dir.path(), "a", Default::default(), &AtomicBool::new(false)).unwrap();
        assert_eq!(report.skipped, 1);
        assert!(report.hits.is_empty());
        assert!(large.exists());
    }
    #[cfg(unix)]
    #[test]
    fn symlink_loop_and_external_root_are_skipped() {
        let dir = tempfile::tempdir().unwrap();
        let external = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("safe.txt"), "Rust").unwrap();
        std::fs::write(external.path().join("secret.txt"), "Rust").unwrap();
        std::os::unix::fs::symlink(dir.path(), dir.path().join("loop")).unwrap();
        std::os::unix::fs::symlink(external.path(), dir.path().join("outside")).unwrap();
        let report = scan(
            dir.path(),
            "Rust",
            Default::default(),
            &AtomicBool::new(false),
        )
        .unwrap();
        assert_eq!(report.hits.len(), 1);
        assert_eq!(report.skipped, 2);
        assert!(
            scan(
                &dir.path().join("loop"),
                "Rust",
                Default::default(),
                &AtomicBool::new(false)
            )
            .is_err()
        );
    }
    #[test]
    fn scan_unicode_bom_crlf_binary_and_scope() {
        let dir = tempfile::tempdir().unwrap();
        let outside = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("a.txt"),
            "\u{feff}中文🙂\r\n第二行 Rust\r\n",
        )
        .unwrap();
        std::fs::write(dir.path().join("binary.bin"), b"Rust\0").unwrap();
        std::fs::write(outside.path().join("outside.txt"), "Rust").unwrap();
        let report = scan(
            dir.path(),
            "Rust",
            Default::default(),
            &AtomicBool::new(false),
        )
        .unwrap();
        assert_eq!(report.hits.len(), 1);
        assert_eq!(report.hits[0].line, 2);
        assert_eq!(report.hits[0].range, (8, 12));
        assert!(report.hits[0].preview.contains("第二行"));
        assert_eq!(report.skipped, 1);
    }
    #[test]
    fn cancelled_scan_does_not_read_and_invalid_pattern_errors() {
        let dir = tempfile::tempdir().unwrap();
        assert!(
            scan(dir.path(), "a", Default::default(), &AtomicBool::new(true))
                .unwrap()
                .cancelled
        );
        assert!(
            scan(
                dir.path(),
                "(",
                SearchOptions {
                    regex: true,
                    ..Default::default()
                },
                &AtomicBool::new(false)
            )
            .is_err()
        );
    }
}
