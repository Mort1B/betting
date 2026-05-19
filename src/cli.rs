use crate::ai::AiOptions;
use crate::domain::{BettingRules, SportScope};
use crate::norsk_tipping::LiveOddsOptions;
use crate::notify::DeliveryOptions;
use crate::reference::ReferenceOddsOptions;
use crate::research::ResearchOptions;

#[derive(Debug)]
pub(crate) struct CliOptions {
    pub(crate) source: CandidateSource,
    pub(crate) rules: BettingRules,
    pub(crate) research: ResearchOptions,
    pub(crate) reference_odds: ReferenceOddsOptions,
    pub(crate) delivery: DeliveryOptions,
    pub(crate) ai: AiOptions,
}

#[derive(Debug)]
pub(crate) enum CandidateSource {
    Csv(String),
    NorskTippingLive(LiveOddsOptions),
}

impl CliOptions {
    pub(crate) fn parse<I>(mut args: I) -> Result<Self, String>
    where
        I: Iterator<Item = String>,
    {
        let mut rules = BettingRules::default();
        let mut research = ResearchOptions::default();
        let mut reference_odds = ReferenceOddsOptions::default();
        let mut delivery = DeliveryOptions::default();
        let mut ai = AiOptions::default();
        let mut input_path = None;
        let mut use_norsk_tipping_live = false;
        let mut live_options = LiveOddsOptions::default();
        while let Some(flag) = args.next() {
            match flag.as_str() {
                "--norsk-tipping-live" => use_norsk_tipping_live = true,
                "--nt-events-per-sport" => {
                    live_options.events_per_sport = parse_usize(&mut args, "--nt-events-per-sport")?
                }
                "--nt-earliest-start" => {
                    live_options.earliest_start =
                        Some(next_value(&mut args, "--nt-earliest-start")?)
                }
                "--date" => rules.date = Some(next_value(&mut args, "--date")?),
                "--sport-scope" => {
                    rules.sport_scope = SportScope::parse(&next_value(&mut args, "--sport-scope")?)?
                }
                "--min-odds" => rules.min_odds = parse_f64(&mut args, "--min-odds")?,
                "--max-odds" => rules.max_odds = parse_f64(&mut args, "--max-odds")?,
                "--min-probability" => {
                    rules.min_estimated_probability = parse_f64(&mut args, "--min-probability")?
                }
                "--min-confidence" => {
                    rules.min_confidence = parse_f64(&mut args, "--min-confidence")?
                }
                "--min-edge" => rules.min_edge = parse_f64(&mut args, "--min-edge")?,
                "--min-ev" => rules.min_expected_value = parse_f64(&mut args, "--min-ev")?,
                "--pick-count" => rules.pick_count = parse_usize(&mut args, "--pick-count")?,
                "--research" => research.source_path = Some(next_value(&mut args, "--research")?),
                "--reference-odds" => {
                    reference_odds.source_path = Some(next_value(&mut args, "--reference-odds")?);
                }
                "--max-research-pages" => {
                    research.max_pages = parse_usize(&mut args, "--max-research-pages")?;
                }
                "--max-research-items" => {
                    research.max_items_per_source = parse_usize(&mut args, "--max-research-items")?;
                }
                "--send-email" => delivery.send_email = true,
                "--send-pushover" => delivery.send_pushover = true,
                "--subject" => delivery.subject = next_value(&mut args, "--subject")?,
                "--ai" => ai.enabled = true,
                "--openai-model" => ai.model = next_value(&mut args, "--openai-model")?,
                "--ai-max-output-tokens" => {
                    ai.max_output_tokens = parse_u32(&mut args, "--ai-max-output-tokens")?;
                }
                value if value.starts_with("--") => return Err(format!("unknown option: {value}")),
                value => {
                    if input_path.replace(value.to_string()).is_some() {
                        return Err(format!("unexpected positional argument: {value}"));
                    }
                }
            }
        }

        rules.validate()?;
        if live_options.events_per_sport == 0 {
            return Err("--nt-events-per-sport must be greater than 0".to_string());
        }
        let source = if use_norsk_tipping_live {
            if rules.date.is_none() {
                return Err("--norsk-tipping-live requires --date YYYY-MM-DD".to_string());
            }
            CandidateSource::NorskTippingLive(live_options)
        } else {
            CandidateSource::Csv(input_path.ok_or_else(|| "missing input CSV path".to_string())?)
        };

        Ok(Self {
            source,
            rules,
            research,
            reference_odds,
            delivery,
            ai,
        })
    }

    pub(crate) fn usage() -> &'static str {
        "Usage:\n\
           betting <norsk-tipping-candidates.csv> [options]\n\
           betting --norsk-tipping-live --date YYYY-MM-DD [options]\n\
         \n\
         Options:\n\
           --norsk-tipping-live       load live candidates from Norsk Tipping Oddsen\n\
           --nt-events-per-sport N    live source event page size, default 35\n\
           --nt-earliest-start TEXT   live source cutoff, e.g. 2026-05-16T16:00\n\
           --date YYYY-MM-DD          only consider events on this date\n\
           --sport-scope TEXT         football or all, default football\n\
           --min-odds N               preferred floor, default 1.10\n\
           --max-odds N               preferred ceiling, default 1.30; 1.35 hard research ceiling\n\
           --min-probability N        default 0.79\n\
           --min-confidence N         default 0.65\n\
           --min-edge N               default 0.015\n\
           --min-ev N                 default 0.000\n\
           --pick-count N             default 5\n\
           --research PATH            research source file, name|kind|url\n\
           --reference-odds PATH      optional external reference odds CSV\n\
           --max-research-pages N     default 10\n\
           --max-research-items N     default 10 for listing sources\n\
           --send-email               send report through SMTP env vars\n\
           --send-pushover            send report to iPhone through Pushover\n\
           --subject TEXT             notification subject\n\
           --ai                       run the 4-agent OpenAI workflow\n\
           --openai-model MODEL       default gpt-5.5\n\
           --ai-max-output-tokens N   default 900 per agent"
    }
}

fn next_value<I>(args: &mut I, flag: &str) -> Result<String, String>
where
    I: Iterator<Item = String>,
{
    args.next()
        .ok_or_else(|| format!("{flag} requires a value"))
}

fn parse_f64<I>(args: &mut I, flag: &str) -> Result<f64, String>
where
    I: Iterator<Item = String>,
{
    let raw = next_value(args, flag)?;
    raw.parse::<f64>()
        .map_err(|_| format!("{flag} requires a number, got {raw}"))
}

fn parse_usize<I>(args: &mut I, flag: &str) -> Result<usize, String>
where
    I: Iterator<Item = String>,
{
    let raw = next_value(args, flag)?;
    raw.parse::<usize>()
        .map_err(|_| format!("{flag} requires a positive integer, got {raw}"))
}

fn parse_u32<I>(args: &mut I, flag: &str) -> Result<u32, String>
where
    I: Iterator<Item = String>,
{
    let raw = next_value(args, flag)?;
    raw.parse::<u32>()
        .map_err(|_| format!("{flag} requires a positive integer, got {raw}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_csv_source_by_default() {
        let options = CliOptions::parse(
            [
                "examples/norsk_tipping_candidates.csv",
                "--date",
                "2026-05-16",
            ]
            .into_iter()
            .map(str::to_string),
        )
        .expect("valid options");

        match options.source {
            CandidateSource::Csv(path) => assert_eq!(path, "examples/norsk_tipping_candidates.csv"),
            CandidateSource::NorskTippingLive(_) => panic!("expected CSV source"),
        }
        assert_eq!(options.rules.date.as_deref(), Some("2026-05-16"));
    }

    #[test]
    fn parses_norsk_tipping_live_source() {
        let options = CliOptions::parse(
            [
                "--norsk-tipping-live",
                "--date",
                "2026-05-16",
                "--nt-events-per-sport",
                "50",
                "--pick-count",
                "7",
            ]
            .into_iter()
            .map(str::to_string),
        )
        .expect("valid options");

        match options.source {
            CandidateSource::NorskTippingLive(live) => {
                assert_eq!(live.events_per_sport, 50);
                assert!(live.earliest_start.is_none());
            }
            CandidateSource::Csv(_) => panic!("expected live source"),
        }
        assert_eq!(options.rules.pick_count, 7);
    }

    #[test]
    fn parses_norsk_tipping_live_start_cutoff() {
        let options = CliOptions::parse(
            [
                "--norsk-tipping-live",
                "--date",
                "2026-05-16",
                "--nt-earliest-start",
                "2026-05-16T16:00",
            ]
            .into_iter()
            .map(str::to_string),
        )
        .expect("valid options");

        match options.source {
            CandidateSource::NorskTippingLive(live) => {
                assert_eq!(live.earliest_start.as_deref(), Some("2026-05-16T16:00"));
            }
            CandidateSource::Csv(_) => panic!("expected live source"),
        }
    }

    #[test]
    fn parses_reference_odds_path() {
        let options = CliOptions::parse(
            [
                "--norsk-tipping-live",
                "--date",
                "2026-05-16",
                "--reference-odds",
                "reference_odds.csv",
            ]
            .into_iter()
            .map(str::to_string),
        )
        .expect("valid options");

        assert_eq!(
            options.reference_odds.source_path.as_deref(),
            Some("reference_odds.csv")
        );
    }
}
