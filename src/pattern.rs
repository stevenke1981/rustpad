use crate::{actions::SearchOptions, core::Content};
pub const MAX_MATCHES: usize = 20_000;
pub fn compile(query: &str, options: SearchOptions) -> Result<regex::Regex, String> {
    if query.is_empty() || query.len() > 16_384 {
        return Err("正規表示式須為 1 至 16 KiB".into());
    }
    regex::RegexBuilder::new(query)
        .case_insensitive(!options.match_case)
        .size_limit(1024 * 1024)
        .dfa_size_limit(1024 * 1024)
        .build()
        .map_err(|e| format!("正規表示式錯誤：{e}"))
}
#[cfg(test)]
pub fn validate(query: &str, options: SearchOptions) -> Result<(), String> {
    compile(query, options).map(|_| ())
}
fn boundaries(text: &str) -> Vec<usize> {
    text.char_indices()
        .map(|p| p.0)
        .chain(std::iter::once(text.len()))
        .collect()
}
pub fn accepted(text: &str, start: usize, end: usize, options: SearchOptions) -> bool {
    !options.whole_word
        || (!text[..start]
            .chars()
            .next_back()
            .is_some_and(crate::actions::word)
            && !text[end..].chars().next().is_some_and(crate::actions::word))
}
pub fn ranges(
    text: &str,
    query: &str,
    options: SearchOptions,
) -> Result<Vec<(usize, usize)>, String> {
    let re = compile(query, options)?;
    let offsets = boundaries(text);
    let mut out = Vec::new();
    for (index, m) in re.find_iter(text).enumerate() {
        if index >= MAX_MATCHES {
            return Err("匹配超過 20,000 處上限；未執行操作".into());
        }
        if accepted(text, m.start(), m.end(), options) {
            out.push((
                offsets.binary_search(&m.start()).unwrap(),
                offsets.binary_search(&m.end()).unwrap(),
            ));
        }
    }
    Ok(out)
}
fn append(out: &mut String, value: &str) -> Result<(), String> {
    if out.len().saturating_add(value.len()) > crate::core::MAX_BYTES {
        return Err("取代結果超過 2 MiB 上限".into());
    }
    out.push_str(value);
    Ok(())
}
// Bounded equivalent of regex capture expansion: $name, ${name}, $$.
fn expand(out: &mut String, replacement: &str, caps: &regex::Captures<'_>) -> Result<(), String> {
    let mut rest = replacement;
    while let Some(dollar) = rest.find('$') {
        append(out, &rest[..dollar])?;
        rest = &rest[dollar + 1..];
        if let Some(tail) = rest.strip_prefix('$') {
            append(out, "$")?;
            rest = tail;
            continue;
        }
        let (name, tail) = if let Some(braced) = rest.strip_prefix('{') {
            if let Some(end) = braced.find('}') {
                (&braced[..end], &braced[end + 1..])
            } else {
                append(out, "$")?;
                continue;
            }
        } else {
            let end = rest
                .find(|ch: char| !ch.is_ascii_alphanumeric() && ch != '_')
                .unwrap_or(rest.len());
            if end == 0 {
                append(out, "$")?;
                continue;
            }
            (&rest[..end], &rest[end..])
        };
        if let Some(m) = name
            .parse::<usize>()
            .ok()
            .and_then(|n| caps.get(n))
            .or_else(|| caps.name(name))
        {
            append(out, m.as_str())?;
        }
        rest = tail;
    }
    append(out, rest)
}
pub fn replace(
    content: &mut Content,
    query: &str,
    replacement: &str,
    options: SearchOptions,
    scope: Option<(usize, usize)>,
    one: bool,
) -> Result<usize, String> {
    if replacement.len() > 16_384 {
        return Err("取代模板超過 16 KiB 上限".into());
    }
    let re = compile(query, options)?;
    let offsets = boundaries(&content.text);
    let scope = scope.unwrap_or((0, offsets.len() - 1));
    if scope.0 > scope.1 || scope.1 >= offsets.len() {
        return Err("無效選取範圍".into());
    }
    let mut out = String::new();
    let mut end = 0;
    let mut count = 0;
    for (index, caps) in re.captures_iter(&content.text).enumerate() {
        if index >= MAX_MATCHES {
            return Err("匹配超過 20,000 處上限；未取代".into());
        }
        let m = caps.get(0).unwrap();
        if m.start() < offsets[scope.0]
            || m.end() > offsets[scope.1]
            || (one && (m.start() != offsets[scope.0] || m.end() != offsets[scope.1]))
            || !accepted(&content.text, m.start(), m.end(), options)
        {
            continue;
        }
        append(&mut out, &content.text[end..m.start()])?;
        expand(&mut out, replacement, &caps)?;
        end = m.end();
        count += 1;
        if one {
            break;
        }
    }
    append(&mut out, &content.text[end..])?;
    let mut candidate = content.clone();
    candidate.text = out;
    crate::core::encode(&candidate)?;
    *content = candidate;
    Ok(count)
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn replacement_template_matches_rust_regex_and_is_bounded() {
        for template in [
            "${name}-$2-$$",
            "$missing/$99",
            "$1a/${1}a",
            "$",
            "${missing}",
            "${",
            "${name}",
        ] {
            let query = r"(?P<name>中)(🙂)?";
            let mut c = Content {
                text: "中🙂 中".into(),
                ..Default::default()
            };
            let expected = regex::Regex::new(query)
                .unwrap()
                .replace_all(&c.text, template)
                .into_owned();
            replace(&mut c, query, template, options(), None, false).unwrap();
            assert_eq!(c.text, expected, "{template}");
        }
        let mut c = Content {
            text: "中".repeat(3000),
            ..Default::default()
        };
        let old = c.clone();
        assert!(replace(&mut c, ".+", &"$0".repeat(8192), options(), None, false).is_err());
        assert_eq!(c, old);
    }
    #[test]
    fn large_regex_limits_do_not_partially_replace_and_line_modes_are_explicit() {
        let mut c = Content {
            text: "中文 12\n".repeat(18_000),
            bom: true,
            newline: crate::core::Newline::Lf,
        };
        assert_eq!(
            ranges(&c.text, r"(?m)^中文 (\d+)$", options())
                .unwrap()
                .len(),
            18_000
        );
        assert!(ranges(&c.text, r"^中文", options()).unwrap().len() == 1);
        let old = c.clone();
        assert!(replace(&mut c, r"(?:)", "x", options(), None, false).is_err());
        assert_eq!(c, old);
        assert!(ranges(&c.text, r"(?:)", options()).is_err());
        assert!(validate(r"\w{1000}", options()).is_err());
    }
    fn options() -> SearchOptions {
        SearchOptions {
            regex: true,
            ..Default::default()
        }
    }
    #[test]
    fn regex_unicode_zero_length_and_capture_replacement() {
        assert_eq!(
            ranges("中🙂", r"^|$", options()).unwrap(),
            vec![(0, 0), (2, 2)]
        );
        let mut c = Content {
            text: "中文 12\n中文 34\n".into(),
            bom: true,
            newline: crate::core::Newline::Crlf,
        };
        assert_eq!(
            replace(
                &mut c,
                r"(?m)^(中文) (\d+)$",
                "${2}:$1 $$",
                options(),
                Some((0, 6)),
                false
            )
            .unwrap(),
            1
        );
        assert_eq!(c.text, "12:中文 $\n中文 34\n");
        assert!(
            crate::core::encode(&c)
                .unwrap()
                .starts_with(&[239, 187, 191])
        );
        assert!(
            crate::core::encode(&c)
                .unwrap()
                .windows(2)
                .any(|p| p == b"\r\n")
        );
    }
    #[test]
    fn invalid_regex_and_failed_expansion_preserve_content() {
        assert!(validate("(", options()).is_err());
        assert!(validate(r"(?=x)", options()).is_err());
        let mut c = Content {
            text: "中".into(),
            ..Default::default()
        };
        let old = c.clone();
        assert!(replace(&mut c, ".", &"x".repeat(16_385), options(), None, false).is_err());
        assert_eq!(c, old);
        assert!(replace(&mut c, "(", "x", options(), None, false).is_err());
        assert_eq!(c, old);
    }
    #[test]
    fn empty_pattern_is_explicit_and_zero_length_batch_terminates() {
        assert!(validate("", options()).is_err());
        let mut c = Content {
            text: "中🙂".into(),
            ..Default::default()
        };
        assert_eq!(
            replace(&mut c, r"(?:)", "|", options(), None, false).unwrap(),
            3
        );
        assert_eq!(c.text, "|中|🙂|");
    }
}
