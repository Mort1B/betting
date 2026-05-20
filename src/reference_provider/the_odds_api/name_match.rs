const IGNORED_SUFFIX_TOKENS: &[&str] = &[
    "fc", "cf", "sc", "ac", "rj", "sp", "mg", "rs", "pr", "ba", "go",
];

const COUNTRY_ALIASES: &[(&str, &str)] = &[
    ("algerie", "algeria"),
    ("aserbajdsjan", "azerbaijan"),
    ("belgia", "belgium"),
    ("bosnia hercegovina", "bosnia and herzegovina"),
    ("bosnia og hercegovina", "bosnia and herzegovina"),
    ("brasil", "brazil"),
    ("bulgaria", "bulgaria"),
    ("cote d ivoire", "ivory coast"),
    ("danmark", "denmark"),
    ("de forente arabiske emirater", "united arab emirates"),
    ("de forente stater", "united states"),
    ("den demokratiske republikken kongo", "dr congo"),
    ("dr kongo", "dr congo"),
    ("ecuador", "ecuador"),
    ("ekvatorial guinea", "equatorial guinea"),
    ("elfenbenskysten", "ivory coast"),
    ("england", "england"),
    ("estland", "estonia"),
    ("faeroyene", "faroe islands"),
    ("faroerne", "faroe islands"),
    ("frankrike", "france"),
    ("hellas", "greece"),
    ("hviterussland", "belarus"),
    ("irland", "ireland"),
    ("island", "iceland"),
    ("italia", "italy"),
    ("ivory coast", "ivory coast"),
    ("japan", "japan"),
    ("kamerun", "cameroon"),
    ("kapp verde", "cape verde"),
    ("kina", "china"),
    ("kongo dr", "dr congo"),
    ("korea dpr", "north korea"),
    ("korea republic", "south korea"),
    ("kroatia", "croatia"),
    ("kypros", "cyprus"),
    ("latvia", "latvia"),
    ("litauen", "lithuania"),
    ("marokko", "morocco"),
    ("nederland", "netherlands"),
    ("new zealand", "new zealand"),
    ("nord irland", "northern ireland"),
    ("nord korea", "north korea"),
    ("nord makedonia", "north macedonia"),
    ("norge", "norway"),
    ("ny zealand", "new zealand"),
    ("osterrike", "austria"),
    ("polen", "poland"),
    ("republikken irland", "ireland"),
    ("romania", "romania"),
    ("russland", "russia"),
    ("saudi arabia", "saudi arabia"),
    ("serbia", "serbia"),
    ("skottland", "scotland"),
    ("slovakia", "slovakia"),
    ("slovenia", "slovenia"),
    ("spania", "spain"),
    ("sverige", "sweden"),
    ("sveits", "switzerland"),
    ("sor afrika", "south africa"),
    ("sor korea", "south korea"),
    ("tsjekkia", "czechia"),
    ("tsjekkiske republikk", "czechia"),
    ("tyrkia", "turkey"),
    ("tyskland", "germany"),
    ("u s a", "united states"),
    ("ukraina", "ukraine"),
    ("ungarn", "hungary"),
    ("united states of america", "united states"),
    ("usa", "united states"),
    ("wales", "wales"),
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
    let name = tokens.join(" ");
    canonical_country_name(&name).unwrap_or(name)
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

fn canonical_country_name(name: &str) -> Option<String> {
    COUNTRY_ALIASES
        .iter()
        .find_map(|(alias, canonical)| (*alias == name).then(|| (*canonical).to_string()))
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
