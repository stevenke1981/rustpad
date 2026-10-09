use std::collections::BTreeSet;
#[derive(Default)]
pub struct Bookmarks {
    pub lines: BTreeSet<usize>,
    snapshot: Option<String>,
}
impl Bookmarks {
    pub fn toggle(&mut self, text: &str, line: usize) {
        self.sync(text);
        if line > text.bytes().filter(|b| *b == b'\n').count() {
            return;
        }
        if !self.lines.remove(&line) {
            self.lines.insert(line);
        }
        self.snapshot = (!self.lines.is_empty()).then(|| text.to_owned());
    }
    pub fn sync(&mut self, text: &str) {
        if self.lines.is_empty() {
            self.snapshot = None;
            return;
        }
        let Some(old) = &self.snapshot else {
            self.snapshot = Some(text.to_owned());
            return;
        };
        if old == text {
            return;
        }
        let old_lines: Vec<_> = old.split('\n').collect();
        let new_lines: Vec<_> = text.split('\n').collect();
        let prefix = old_lines
            .iter()
            .zip(&new_lines)
            .take_while(|(a, b)| a == b)
            .count();
        let suffix = old_lines[prefix..]
            .iter()
            .rev()
            .zip(new_lines[prefix..].iter().rev())
            .take_while(|(a, b)| a == b)
            .count();
        let old_middle = old_lines.len() - prefix - suffix;
        let new_middle = new_lines.len() - prefix - suffix;
        self.lines = self
            .lines
            .iter()
            .filter_map(|&line| {
                if line < prefix {
                    Some(line)
                } else if line >= old_lines.len() - suffix {
                    Some(new_lines.len() - suffix + (line - (old_lines.len() - suffix)))
                } else if old_middle == 1 && new_middle == 1 {
                    Some(prefix)
                } else {
                    None
                }
            })
            .collect();
        self.snapshot = (!self.lines.is_empty()).then(|| text.to_owned());
    }
    pub fn next(&self, line: usize, backward: bool) -> Option<usize> {
        if backward {
            self.lines
                .range(..line)
                .next_back()
                .copied()
                .or_else(|| self.lines.last().copied())
        } else {
            self.lines
                .range(line.saturating_add(1)..)
                .next()
                .copied()
                .or_else(|| self.lines.first().copied())
        }
    }
    pub fn clear(&mut self) {
        self.lines.clear();
        self.snapshot = None;
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn navigation_wrap_and_unicode_edit_positions() {
        let mut b = Bookmarks::default();
        b.toggle("中文🙂\n二\n三", 0);
        b.toggle("中文🙂\n二\n三", 2);
        assert_eq!(b.next(0, false), Some(2));
        assert_eq!(b.next(0, true), Some(2));
        assert_eq!(b.next(2, false), Some(0));
        b.sync("新增\n中文🙂\n二\n三");
        assert_eq!(b.lines, BTreeSet::from([1, 3]));
        b.sync("新增\n中文改🙂\n二\n三");
        assert_eq!(b.lines, BTreeSet::from([1, 3]));
        b.sync("新增\n中文改🙂\n二\n");
        assert_eq!(b.lines, BTreeSet::from([1, 3]));
        b.sync("新增\n中文改🙂\n二");
        assert_eq!(b.lines, BTreeSet::from([1]));
        b.clear();
        assert_eq!(b.next(0, false), None);
    }
    #[test]
    fn changed_multi_line_blocks_clear_ambiguous_markers_and_eof_is_valid() {
        let mut b = Bookmarks::default();
        b.toggle("一\n二\n三\n", 1);
        b.toggle("一\n二\n三\n", 3);
        b.sync("一\n合併二三\n");
        assert_eq!(b.lines, BTreeSet::from([2]));
        b.toggle("一\n合併二三\n", 99);
        assert_eq!(b.lines, BTreeSet::from([2]));
        b.toggle("一\n合併二三\n", 2);
        assert!(b.lines.is_empty());
    }
}
