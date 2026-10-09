use crate::syntax::Language;

pub fn matching(text: &str, language: Language, cursor: usize) -> Result<Option<usize>, String> {
    let chars: Vec<_> = text.char_indices().collect();
    if cursor > chars.len() {
        return Ok(None);
    }
    let target = if chars.get(cursor).is_some_and(|p| "()[]{}".contains(p.1)) {
        cursor
    } else if cursor > 0 && "()[]{}".contains(chars[cursor - 1].1) {
        cursor - 1
    } else {
        return Ok(None);
    };
    let mask = crate::syntax::Engine::new().code_mask(text, language)?;
    let mut stack = Vec::new();
    for (index, (byte, ch)) in chars.into_iter().enumerate() {
        if !mask[byte] {
            continue;
        }
        if "([{".contains(ch) {
            stack.push((index, ch));
        } else if ")]}".contains(ch)
            && let Some((open, opening)) = stack.pop()
        {
            if matches!((opening, ch), ('(', ')') | ('[', ']') | ('{', '}')) {
                if open == target {
                    return Ok(Some(index));
                }
                if index == target {
                    return Ok(Some(open));
                }
            } else {
                stack.clear();
            }
        }
    }
    Ok(None)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn ignores_comments_strings_and_uses_scalar_positions() {
        let text = "中{\n let s = r###\"}中文{\"###; /* } */\n [🙂] // }\n}";
        assert_eq!(
            matching(text, Language::Rust, 1).unwrap(),
            Some(text.chars().count() - 1)
        );
        assert_eq!(
            matching(text, Language::Rust, text.chars().count()).unwrap(),
            Some(1)
        );
        let comment = text.chars().position(|c| c == '}').unwrap();
        assert_eq!(matching(text, Language::Rust, comment).unwrap(), None);
    }
    #[test]
    fn python_multiline_strings_and_malformed_nesting() {
        let text = "中文(\n''' ) \n ( '''\n[1])";
        assert_eq!(
            matching(text, Language::Python, 2).unwrap(),
            Some(text.chars().count() - 1)
        );
        assert_eq!(matching("([)]", Language::Rust, 0).unwrap(), None);
        assert_eq!(matching("{}", Language::Rust, 99).unwrap(), None);
    }
}
