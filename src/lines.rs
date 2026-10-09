use crate::core::Content;
pub fn transform(
    content: &mut Content,
    selection: (usize, usize),
    width: Option<usize>,
) -> Result<(usize, usize), String> {
    use unicode_segmentation::UnicodeSegmentation;
    if width.is_some_and(|w| !(1..=1000).contains(&w)) {
        return Err("分割寬度須為 1–1000 個字素".into());
    }
    let offsets: Vec<_> = content
        .text
        .char_indices()
        .map(|p| p.0)
        .chain(std::iter::once(content.text.len()))
        .collect();
    if selection.0 > selection.1 || selection.1 >= offsets.len() {
        return Err("無效選取範圍".into());
    }
    if width.is_none() && selection.0 == selection.1 {
        return Err("請先選取跨行文字".into());
    }
    let start = content.text[..offsets[selection.0]]
        .rfind('\n')
        .map_or(0, |p| p + 1);
    let end = if selection.1 > selection.0 && content.text[..offsets[selection.1]].ends_with('\n') {
        offsets[selection.1]
    } else {
        content.text[offsets[selection.1]..]
            .find('\n')
            .map_or(content.text.len(), |p| offsets[selection.1] + p + 1)
    };
    let selected = &content.text[start..end];
    let trailing = selected.ends_with('\n');
    let body = selected.strip_suffix('\n').unwrap_or(selected);
    let mut replacement = String::new();
    if let Some(width) = width {
        for (line_index, line) in body.split('\n').enumerate() {
            if line_index > 0 {
                replacement.push('\n');
            }
            for (index, grapheme) in line.graphemes(true).enumerate() {
                if index > 0 && index % width == 0 {
                    replacement.push('\n');
                }
                if replacement.len().saturating_add(grapheme.len()) > crate::core::MAX_BYTES {
                    return Err("分割結果超過 2 MiB 上限".into());
                }
                replacement.push_str(grapheme);
            }
        }
    } else {
        if !body.contains('\n') {
            return Err("請先選取至少兩行".into());
        }
        replacement = body.replace('\n', " ");
    }
    if trailing {
        replacement.push('\n');
    }
    let index = content.text[..start].chars().count();
    let range = (index, index + replacement.chars().count());
    let mut candidate = content.clone();
    candidate.text.replace_range(start..end, &replacement);
    crate::core::encode(&candidate)?;
    *content = candidate;
    Ok(range)
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn join_preserves_outside_bom_crlf_and_excludes_end_line_start() {
        let mut c = Content {
            text: "外\n中文🙂\n二\n尾".into(),
            bom: true,
            newline: crate::core::Newline::Crlf,
        };
        let range = transform(&mut c, (2, 8), None).unwrap();
        assert_eq!(c.text, "外\n中文🙂 二\n尾");
        assert_eq!(range, (2, 8));
        assert!(c.bom);
        assert_eq!(c.newline, crate::core::Newline::Crlf);
        assert!(
            crate::core::encode(&c)
                .unwrap()
                .windows(2)
                .any(|p| p == b"\r\n")
        );
    }
    #[test]
    fn split_respects_extended_graphemes_empty_lines_and_eof() {
        let mut c = Content {
            text: "外\ne\u{301}🇹🇼👩‍👩‍👧‍👦中🙂\n\n尾".into(),
            ..Default::default()
        };
        let end = c.text.chars().count() - 1;
        transform(&mut c, (2, end), Some(2)).unwrap();
        assert_eq!(c.text, "外\ne\u{301}🇹🇼\n👩‍👩‍👧‍👦中\n🙂\n\n尾");
        let mut c = Content {
            text: "中文🙂尾".into(),
            ..Default::default()
        };
        transform(&mut c, (0, 0), Some(2)).unwrap();
        assert_eq!(c.text, "中文\n🙂尾");
    }
    #[test]
    fn rejected_limits_and_empty_join_preserve_original() {
        let mut c = Content {
            text: "中文".into(),
            ..Default::default()
        };
        let old = c.clone();
        assert!(transform(&mut c, (0, 0), None).is_err());
        assert_eq!(c, old);
        assert!(transform(&mut c, (0, 2), Some(0)).is_err());
        assert_eq!(c, old);
        c.text = "中\n".repeat(19_999) + "中中";
        let old = c.clone();
        let end = c.text.chars().count();
        assert!(transform(&mut c, (0, end), Some(1)).is_err());
        assert_eq!(c, old);
        c.text = ("x".repeat(9_000) + "\n").repeat(2);
        let old = c.clone();
        assert!(transform(&mut c, (0, 18_002), None).is_err());
        assert_eq!(c, old);
    }
}
