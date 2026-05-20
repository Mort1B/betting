const IGNORED_SUFFIX_TOKENS: &[&str] = &[
    "fc", "cf", "sc", "ac", "rj", "sp", "mg", "rs", "pr", "ba", "go",
];

pub(super) fn names_match(left: &str, right: &str) -> bool {
    let left = comparable_name(left);
    let right = comparable_name(right);
    !left.is_empty() && !right.is_empty() && left == right
}

pub(super) fn comparable_name(value: &str) -> String {
    let mut tokens = normalize_tokens(value);
    while tokens
        .last()
        .is_some_and(|token| IGNORED_SUFFIX_TOKENS.contains(&token.as_str()))
    {
        tokens.pop();
    }
    tokens.join(" ")
}

pub(super) fn normalize_tokens(value: &str) -> Vec<String> {
    fold_to_ascii(value)
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() {
                ch.to_ascii_lowercase()
            } else {
                ' '
            }
        })
        .collect::<String>()
        .split_whitespace()
        .map(str::to_string)
        .collect()
}

fn fold_to_ascii(value: &str) -> String {
    let mut folded = String::new();
    for ch in value.chars() {
        match ch {
            'À' | 'Á' | 'Â' | 'Ã' | 'Ä' | 'Å' | 'à' | 'á' | 'â' | 'ã' | 'ä' | 'å' => {
                folded.push('a');
            }
            'Ç' | 'ç' => folded.push('c'),
            'È' | 'É' | 'Ê' | 'Ë' | 'è' | 'é' | 'ê' | 'ë' => folded.push('e'),
            'Ì' | 'Í' | 'Î' | 'Ï' | 'ì' | 'í' | 'î' | 'ï' => folded.push('i'),
            'Ñ' | 'ñ' => folded.push('n'),
            'Ò' | 'Ó' | 'Ô' | 'Õ' | 'Ö' | 'Ø' | 'ò' | 'ó' | 'ô' | 'õ' | 'ö' | 'ø' => {
                folded.push('o');
            }
            'Ù' | 'Ú' | 'Û' | 'Ü' | 'ù' | 'ú' | 'û' | 'ü' => folded.push('u'),
            'Ý' | 'Ÿ' | 'ý' | 'ÿ' => folded.push('y'),
            'Æ' | 'æ' => folded.push_str("ae"),
            'Œ' | 'œ' => folded.push_str("oe"),
            _ => folded.push(ch),
        }
    }
    folded
}
