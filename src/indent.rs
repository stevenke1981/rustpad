use crate::core::Content;

fn offsets(content: &Content, range: (usize, usize)) -> Result<Vec<usize>, String> {
    let offsets: Vec<_> = content
        .text
        .char_indices()
        .map(|p| p.0)
        .chain(std::iter::once(content.text.len()))
        .collect();
    if range.0 > range.1 || range.1 >= offsets.len() {
        return Err("無效選取範圍".into());
    }
    Ok(offsets)
}
pub fn lines(
    content: &mut Content,
    range: (usize, usize),
    unit: &str,
    outdent: bool,
) -> Result<(usize, usize), String> {
    if unit.is_empty() || unit.len() > 16 || !unit.chars().all(|c| c == ' ' || c == '\t') {
        return Err("無效縮排單位".into());
    }
    let offsets = offsets(content, range)?;
    let start = content.text[..offsets[range.0]]
        .rfind('\n')
        .map_or(0, |p| p + 1);
    let end = if range.0 < range.1 && content.text[..offsets[range.1]].ends_with('\n') {
        offsets[range.1]
    } else {
        content.text[offsets[range.1]..]
            .find('\n')
            .map_or(content.text.len(), |p| offsets[range.1] + p + 1)
    };
    let body = &content.text[start..end];
    let mut replacement = String::new();
    for line in body.split_inclusive('\n') {
        if outdent {
            let remove = if line.starts_with('\t') {
                1
            } else {
                line.bytes()
                    .take(unit.len())
                    .take_while(|c| *c == b' ')
                    .count()
            };
            replacement.push_str(&line[remove..]);
        } else {
            replacement.push_str(unit);
            replacement.push_str(line);
        }
    }
    if body.is_empty() && !outdent {
        replacement.push_str(unit);
    }
    let begin = content.text[..start].chars().count();
    let selection = (begin, begin + replacement.chars().count());
    let mut candidate = content.clone();
    candidate.text.replace_range(start..end, &replacement);
    crate::core::encode(&candidate)?;
    *content = candidate;
    Ok(selection)
}
pub fn enter(content: &mut Content, range: (usize, usize)) -> Result<(usize, usize), String> {
    let offsets = offsets(content, range)?;
    let start = content.text[..offsets[range.0]]
        .rfind('\n')
        .map_or(0, |p| p + 1);
    let prefix: String = content.text[start..offsets[range.0]]
        .chars()
        .take_while(|c| *c == ' ' || *c == '\t')
        .collect();
    let replacement = format!("\n{prefix}");
    let cursor = range.0 + replacement.chars().count();
    let mut candidate = content.clone();
    candidate
        .text
        .replace_range(offsets[range.0]..offsets[range.1], &replacement);
    crate::core::encode(&candidate)?;
    *content = candidate;
    Ok((cursor, cursor))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn selected_lines_unicode_and_end_at_next_line_start() {
        let mut c = Content {
            text: "外\n中文🙂\n二\n尾".into(),
            bom: true,
            newline: crate::core::Newline::Crlf,
        };
        let before = c.clone();
        assert_eq!(lines(&mut c, (2, 8), "  ", false).unwrap(), (2, 12));
        assert_eq!(c.text, "外\n  中文🙂\n  二\n尾");
        lines(&mut c, (2, 12), "  ", true).unwrap();
        assert_eq!(c, before);
    }
    #[test]
    fn enter_replaces_selection_and_keeps_prefix_only() {
        let mut c = Content {
            text: "\t 中文🙂尾".into(),
            ..Default::default()
        };
        assert_eq!(enter(&mut c, (4, 5)).unwrap(), (7, 7));
        assert_eq!(c.text, "\t 中文\n\t 尾");
        let mut c = Content {
            text: "    尾".into(),
            ..Default::default()
        };
        enter(&mut c, (2, 2)).unwrap();
        assert_eq!(c.text, "  \n    尾");
    }
    #[test]
    fn rejected_range_and_growth_do_not_modify_content() {
        let mut c = Content {
            text: "x".repeat(16 * 1024),
            ..Default::default()
        };
        let before = c.clone();
        assert!(lines(&mut c, (0, 0), "  ", false).is_err());
        assert_eq!(c, before);
        assert!(enter(&mut c, (2, 1)).is_err());
        assert_eq!(c, before);
    }
}
