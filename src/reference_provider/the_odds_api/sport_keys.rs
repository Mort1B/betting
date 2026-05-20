use crate::domain::BetCandidate;

const AUTO_SPORT_KEY: &str = "auto";
const FALLBACK_SPORT_KEYS: &[&str] = &["soccer_norway_eliteserien"];

struct SportKeyMapping {
    patterns: &'static [&'static str],
    sport_key: &'static str,
}

const SPORT_KEY_MAPPINGS: &[SportKeyMapping] = &[
    SportKeyMapping {
        patterns: &["copa libertadores", "libertadores"],
        sport_key: "soccer_conmebol_copa_libertadores",
    },
    SportKeyMapping {
        patterns: &["copa sudamericana", "sudamericana"],
        sport_key: "soccer_conmebol_copa_sudamericana",
    },
    SportKeyMapping {
        patterns: &["eliteserien"],
        sport_key: "soccer_norway_eliteserien",
    },
    SportKeyMapping {
        patterns: &["allsvenskan"],
        sport_key: "soccer_sweden_allsvenskan",
    },
    SportKeyMapping {
        patterns: &["superettan"],
        sport_key: "soccer_sweden_superettan",
    },
    SportKeyMapping {
        patterns: &["denmark superliga", "danish superliga", "superligaen"],
        sport_key: "soccer_denmark_superliga",
    },
    SportKeyMapping {
        patterns: &["veikkausliiga"],
        sport_key: "soccer_finland_veikkausliiga",
    },
    SportKeyMapping {
        patterns: &["major league soccer", "mls"],
        sport_key: "soccer_usa_mls",
    },
    SportKeyMapping {
        patterns: &["copa america"],
        sport_key: "soccer_conmebol_copa_america",
    },
    SportKeyMapping {
        patterns: &[
            "fifa world cup qualifiers europe",
            "world cup qualifiers europe",
            "european world cup qualifiers",
            "vm kvalifisering europa",
        ],
        sport_key: "soccer_fifa_world_cup_qualifiers_europe",
    },
    SportKeyMapping {
        patterns: &[
            "fifa world cup qualifiers south america",
            "world cup qualifiers south america",
            "south american world cup qualifiers",
            "vm kvalifisering sor amerika",
        ],
        sport_key: "soccer_fifa_world_cup_qualifiers_south_america",
    },
    SportKeyMapping {
        patterns: &["fifa club world cup", "club world cup"],
        sport_key: "soccer_fifa_club_world_cup",
    },
    SportKeyMapping {
        patterns: &[
            "fifa world cup",
            "world cup",
            "fotball vm",
            "verdensmesterskapet",
            "vm",
        ],
        sport_key: "soccer_fifa_world_cup",
    },
    SportKeyMapping {
        patterns: &["brazil serie a", "brasileirao", "campeonato brasileiro"],
        sport_key: "soccer_brazil_campeonato",
    },
    SportKeyMapping {
        patterns: &["brazil serie b"],
        sport_key: "soccer_brazil_serie_b",
    },
    SportKeyMapping {
        patterns: &["argentina primera", "primera division argentina"],
        sport_key: "soccer_argentina_primera_division",
    },
    SportKeyMapping {
        patterns: &["uefa champions league", "champions league"],
        sport_key: "soccer_uefa_champs_league",
    },
    SportKeyMapping {
        patterns: &["europa conference league", "conference league"],
        sport_key: "soccer_uefa_europa_conference_league",
    },
    SportKeyMapping {
        patterns: &["europa league"],
        sport_key: "soccer_uefa_europa_league",
    },
    SportKeyMapping {
        patterns: &["england premier league", "english premier league", "epl"],
        sport_key: "soccer_epl",
    },
    SportKeyMapping {
        patterns: &["la liga 2", "segunda division"],
        sport_key: "soccer_spain_segunda_division",
    },
    SportKeyMapping {
        patterns: &["la liga"],
        sport_key: "soccer_spain_la_liga",
    },
    SportKeyMapping {
        patterns: &["italy serie a", "serie a italy"],
        sport_key: "soccer_italy_serie_a",
    },
    SportKeyMapping {
        patterns: &["bundesliga 2", "2. bundesliga"],
        sport_key: "soccer_germany_bundesliga2",
    },
    SportKeyMapping {
        patterns: &["german bundesliga", "bundesliga germany"],
        sport_key: "soccer_germany_bundesliga",
    },
    SportKeyMapping {
        patterns: &["ligue 1"],
        sport_key: "soccer_france_ligue_one",
    },
    SportKeyMapping {
        patterns: &["eredivisie"],
        sport_key: "soccer_netherlands_eredivisie",
    },
    SportKeyMapping {
        patterns: &["primeira liga"],
        sport_key: "soccer_portugal_primeira_liga",
    },
    SportKeyMapping {
        patterns: &["liga mx"],
        sport_key: "soccer_mexico_ligamx",
    },
    SportKeyMapping {
        patterns: &["j league"],
        sport_key: "soccer_japan_j_league",
    },
    SportKeyMapping {
        patterns: &["k league"],
        sport_key: "soccer_korea_kleague1",
    },
];

pub(super) fn resolve_sport_keys(
    configured_sport_keys: &[String],
    candidates: &[BetCandidate],
) -> Vec<String> {
    let mut resolved = Vec::new();
    let mut use_auto = false;

    for sport_key in configured_sport_keys
        .iter()
        .map(|sport_key| sport_key.trim())
        .filter(|sport_key| !sport_key.is_empty())
    {
        if sport_key.eq_ignore_ascii_case(AUTO_SPORT_KEY) {
            use_auto = true;
        } else {
            push_unique_string(&mut resolved, sport_key);
        }
    }

    if use_auto {
        for candidate in candidates {
            for sport_key in inferred_sport_keys(candidate) {
                push_unique_string(&mut resolved, sport_key);
            }
        }
    }

    if resolved.is_empty() {
        for sport_key in FALLBACK_SPORT_KEYS {
            push_unique_string(&mut resolved, sport_key);
        }
    }

    resolved
}

fn inferred_sport_keys(candidate: &BetCandidate) -> Vec<&'static str> {
    let competition = normalized_text(&candidate.competition);
    let mut sport_keys = Vec::new();

    for mapping in SPORT_KEY_MAPPINGS {
        if mapping
            .patterns
            .iter()
            .any(|pattern| competition_matches_pattern(&competition, pattern))
        {
            push_unique_str(&mut sport_keys, mapping.sport_key);
        }
    }

    sport_keys
}

fn normalized_text(value: &str) -> String {
    value
        .chars()
        .map(|character| {
            if character.is_ascii_alphanumeric() {
                character.to_ascii_lowercase()
            } else {
                ' '
            }
        })
        .collect::<String>()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

fn competition_matches_pattern(value: &str, pattern: &str) -> bool {
    if matches!(pattern, "fifa world cup" | "world cup")
        && (value.contains("qualifier") || value.contains("club") || value.contains("women"))
    {
        return false;
    }
    if pattern == "vm" && (value.contains("kvalifisering") || value.contains("kvinner")) {
        return false;
    }
    if pattern
        .chars()
        .all(|character| character.is_ascii_alphanumeric())
    {
        return value.split_whitespace().any(|word| word == pattern);
    }
    value.contains(pattern)
}

fn push_unique_string(values: &mut Vec<String>, value: &str) {
    if !values.iter().any(|existing| existing == value) {
        values.push(value.to_string());
    }
}

fn push_unique_str(values: &mut Vec<&'static str>, value: &'static str) {
    if !values.contains(&value) {
        values.push(value);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn candidate(competition: &str) -> BetCandidate {
        BetCandidate {
            id: "c1".to_string(),
            sport: "Football".to_string(),
            competition: competition.to_string(),
            event: "Home - Away".to_string(),
            market: "Main market".to_string(),
            selection: "Home".to_string(),
            norsk_tipping_odds: 1.22,
            model_probability: None,
            reference_odds: None,
            confidence: Some(0.80),
            starts_at: "2026-05-21T02:00:00+02:00".to_string(),
            notes: String::new(),
        }
    }

    #[test]
    fn auto_infers_conmebol_sport_keys_from_candidates() {
        let sport_keys = resolve_sport_keys(
            &["auto".to_string()],
            &[
                candidate("Copa Sudamericana, South America"),
                candidate("Copa Libertadores, Internasjonal"),
            ],
        );

        assert_eq!(
            sport_keys,
            vec![
                "soccer_conmebol_copa_sudamericana".to_string(),
                "soccer_conmebol_copa_libertadores".to_string()
            ]
        );
    }

    #[test]
    fn auto_keeps_explicit_keys_and_adds_inferred_keys() {
        let sport_keys = resolve_sport_keys(
            &["soccer_norway_eliteserien".to_string(), "auto".to_string()],
            &[candidate("Copa Libertadores")],
        );

        assert_eq!(
            sport_keys,
            vec![
                "soccer_norway_eliteserien".to_string(),
                "soccer_conmebol_copa_libertadores".to_string()
            ]
        );
    }

    #[test]
    fn explicit_sport_keys_do_not_infer_candidate_sports() {
        let sport_keys = resolve_sport_keys(
            &["soccer_sweden_allsvenskan".to_string()],
            &[candidate("Copa Libertadores")],
        );

        assert_eq!(sport_keys, vec!["soccer_sweden_allsvenskan".to_string()]);
    }

    #[test]
    fn auto_infers_world_cup_without_collapsing_qualifiers() {
        let sport_keys = resolve_sport_keys(
            &["auto".to_string()],
            &[
                candidate("FIFA World Cup"),
                candidate("FIFA World Cup Qualifiers - Europe"),
                candidate("FIFA World Cup Qualifiers - South America"),
            ],
        );

        assert_eq!(
            sport_keys,
            vec![
                "soccer_fifa_world_cup".to_string(),
                "soccer_fifa_world_cup_qualifiers_europe".to_string(),
                "soccer_fifa_world_cup_qualifiers_south_america".to_string()
            ]
        );
    }
}
