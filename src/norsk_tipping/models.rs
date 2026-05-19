use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Serialize)]
pub(crate) struct ContentRequest<'a> {
    #[serde(rename = "contentId")]
    pub(crate) content_id: ContentId<'a>,
    #[serde(rename = "clientContext")]
    pub(crate) client_context: ClientContext<'a>,
}

#[derive(Debug, Serialize)]
pub(crate) struct ContentId<'a> {
    #[serde(rename = "type")]
    pub(crate) kind: &'a str,
    pub(crate) id: &'a str,
}

#[derive(Debug, Serialize)]
pub(crate) struct ClientContext<'a> {
    pub(crate) language: &'a str,
    #[serde(rename = "ipAddress")]
    pub(crate) ip_address: &'a str,
}

#[derive(Debug, Deserialize)]
#[serde(bound(deserialize = "T: Deserialize<'de>"))]
pub(crate) struct ContentResponse<T> {
    #[serde(default)]
    pub(crate) data: Vec<T>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct SportType {
    idfosporttype: Option<String>,
    sporttypename: Option<String>,
    name: Option<String>,
}

impl SportType {
    pub(crate) fn id(&self) -> Option<String> {
        self.idfosporttype
            .as_deref()
            .filter(|id| !id.trim().is_empty())
            .map(str::to_string)
    }

    pub(crate) fn display_name(&self) -> String {
        self.sporttypename
            .as_deref()
            .or(self.name.as_deref())
            .or(self.idfosporttype.as_deref())
            .unwrap_or("Sport")
            .to_string()
    }
}

#[derive(Debug, Deserialize)]
pub(crate) struct Event {
    pub(crate) idfoevent: Option<String>,
    pub(crate) name: Option<String>,
    pub(crate) participantname_home: Option<String>,
    pub(crate) participantname_away: Option<String>,
    pub(crate) sporttypename: Option<String>,
    pub(crate) tournamentname: Option<String>,
    pub(crate) tsstart: Option<String>,
    #[serde(default)]
    pub(crate) markets: Vec<Market>,
}

impl Event {
    pub(crate) fn event_name(&self) -> String {
        self.name
            .as_deref()
            .filter(|name| !name.trim().is_empty())
            .map(str::to_string)
            .or_else(|| {
                Some(format!(
                    "{} - {}",
                    self.participantname_home.as_deref()?,
                    self.participantname_away.as_deref()?
                ))
            })
            .unwrap_or_else(|| "Unknown event".to_string())
    }
}

#[derive(Debug, Deserialize)]
pub(crate) struct Market {
    idfomarket: Option<String>,
    name: Option<String>,
    isheadmarket: Option<bool>,
    ismainline: Option<bool>,
    istradable: Option<bool>,
    #[serde(default)]
    selections: Vec<Selection>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum CandidateMarketKind {
    Main,
    Expanded(&'static str),
}

impl CandidateMarketKind {
    pub(crate) fn note(&self) -> &'static str {
        match self {
            Self::Main => "main football market",
            Self::Expanded(label) => label,
        }
    }
}

impl Market {
    pub(crate) fn candidate_market_kind(&self) -> Option<CandidateMarketKind> {
        if self.istradable == Some(false) || self.selections.is_empty() {
            return None;
        }
        if self.isheadmarket == Some(true)
            || self.ismainline == Some(true)
            || self.name.as_deref() == Some("HUB")
        {
            return Some(CandidateMarketKind::Main);
        }
        expanded_market_kind(self.name.as_deref().unwrap_or(""))
    }

    pub(crate) fn display_name(&self) -> String {
        match self.name.as_deref() {
            Some("HUB") => "Main market".to_string(),
            Some(name) if !name.trim().is_empty() => name.to_string(),
            _ => "Main market".to_string(),
        }
    }

    pub(crate) fn identifier(&self) -> String {
        self.idfomarket
            .as_deref()
            .or(self.name.as_deref())
            .unwrap_or("market")
            .to_string()
    }

    pub(crate) fn selections(&self) -> impl Iterator<Item = &Selection> {
        self.selections.iter()
    }
}

fn expanded_market_kind(name: &str) -> Option<CandidateMarketKind> {
    let normalized = name.to_ascii_lowercase();
    let contains_any = |terms: &[&str]| terms.iter().any(|term| normalized.contains(term));
    if contains_any(&["begge lag scorer", "both teams to score", "btts"]) {
        Some(CandidateMarketKind::Expanded(
            "expanded both-teams-score market",
        ))
    } else if contains_any(&["goal scorer", "goalscorer", "målscorer", "anytime scorer"]) {
        Some(CandidateMarketKind::Expanded(
            "expanded player scorer market",
        ))
    } else if contains_any(&["corner", "corners", "hjørne", "hjørnespark"]) {
        Some(CandidateMarketKind::Expanded("expanded corners market"))
    } else if contains_any(&["card", "cards", "kort"]) {
        Some(CandidateMarketKind::Expanded("expanded cards market"))
    } else if contains_any(&["over/under", "over ", "under ", "totalt", "total", "mål"]) {
        Some(CandidateMarketKind::Expanded("expanded goals market"))
    } else {
        None
    }
}

#[derive(Debug, Deserialize)]
pub(crate) struct Selection {
    idfoselection: Option<String>,
    name: Option<String>,
    currentpriceup: Option<Value>,
    currentpricedown: Option<Value>,
    idfobolifestate: Option<String>,
}

impl Selection {
    pub(crate) fn name(&self) -> Option<&str> {
        self.name.as_deref()
    }

    pub(crate) fn is_live(&self) -> bool {
        self.idfobolifestate
            .as_deref()
            .is_none_or(|state| state == "N")
    }

    pub(crate) fn decimal_odds(&self) -> Option<f64> {
        let price_up = value_as_f64(self.currentpriceup.as_ref()?)?;
        let price_down = value_as_f64(self.currentpricedown.as_ref()?)?;
        if price_down <= 0.0 {
            return None;
        }
        Some(round_to_two_decimals(1.0 + (price_up / price_down)))
    }

    pub(crate) fn identifier(&self, fallback: &str) -> String {
        self.idfoselection
            .as_deref()
            .filter(|id| !id.trim().is_empty())
            .unwrap_or(fallback)
            .to_string()
    }
}

fn value_as_f64(value: &Value) -> Option<f64> {
    match value {
        Value::Number(number) => number.as_f64(),
        Value::String(text) => text.trim().parse::<f64>().ok(),
        _ => None,
    }
}

fn round_to_two_decimals(value: f64) -> f64 {
    (value * 100.0).round() / 100.0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn converts_fractional_price_fields_to_decimal_odds() {
        let selection = Selection {
            idfoselection: Some("s1".to_string()),
            name: Some("Manchester City WFC".to_string()),
            currentpriceup: Some(Value::String("3".to_string())),
            currentpricedown: Some(Value::String("20".to_string())),
            idfobolifestate: Some("N".to_string()),
        };

        assert_eq!(selection.decimal_odds(), Some(1.15));
    }

    #[test]
    fn skips_suspended_selections() {
        let selection = Selection {
            idfoselection: Some("s1".to_string()),
            name: Some("Suspended".to_string()),
            currentpriceup: Some(Value::String("3".to_string())),
            currentpricedown: Some(Value::String("20".to_string())),
            idfobolifestate: Some("S".to_string()),
        };

        assert!(!selection.is_live());
    }

    #[test]
    fn accepts_supported_expanded_football_markets() {
        let market = Market {
            idfomarket: Some("m1".to_string()),
            name: Some("Over/under 2.5 mål".to_string()),
            isheadmarket: Some(false),
            ismainline: Some(false),
            istradable: Some(true),
            selections: vec![Selection {
                idfoselection: Some("s1".to_string()),
                name: Some("Under 2.5".to_string()),
                currentpriceup: Some(Value::String("1".to_string())),
                currentpricedown: Some(Value::String("5".to_string())),
                idfobolifestate: Some("N".to_string()),
            }],
        };

        assert!(market.candidate_market_kind().is_some());
        assert_eq!(
            market.candidate_market_kind().map(|kind| kind.note()),
            Some("expanded goals market")
        );
    }
}
