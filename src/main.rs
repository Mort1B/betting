mod agents;
mod ai;
mod domain;
mod input;
mod norsk_tipping;
mod notify;
mod reference;
mod report;
mod research;

use std::env;
use std::process;

use agents::DailyBetOrchestrator;
use ai::{AiOptions, run_ai_workflow};
use domain::BettingRules;
use input::load_candidates_from_csv;
use norsk_tipping::LiveOddsOptions;
use notify::{DeliveryOptions, deliver_report};
use reference::{ReferenceOddsOptions, apply_reference_odds};
use report::render_recommendation;
use research::{MarketResearchClient, ResearchOptions, load_sources};

#[derive(Debug)]
struct CliOptions {
    source: CandidateSource,
    rules: BettingRules,
    research: ResearchOptions,
    reference_odds: ReferenceOddsOptions,
    delivery: DeliveryOptions,
    ai: AiOptions,
}

#[derive(Debug)]
enum CandidateSource {
    Csv(String),
    NorskTippingLive(LiveOddsOptions),
}

fn main() {
    let args = env::args().skip(1).collect::<Vec<_>>();
    if args
        .first()
        .is_some_and(|arg| arg == "--help" || arg == "-h")
    {
        println!("{}", CliOptions::usage());
        return;
    }

    let options = match CliOptions::parse(args.into_iter()) {
        Ok(options) => options,
        Err(message) => {
            eprintln!("{message}");
            eprintln!();
            eprintln!("{}", CliOptions::usage());
            process::exit(2);
        }
    };

    let mut candidates = match load_candidates(&options.source, &options.rules) {
        Ok(candidates) => candidates,
        Err(error) => {
            eprintln!("failed to load candidates: {error}");
            process::exit(1);
        }
    };
    match apply_reference_odds(candidates, &options.reference_odds) {
        Ok(enriched) => candidates = enriched,
        Err(error) => {
            eprintln!("failed to apply reference odds: {error}");
            process::exit(1);
        }
    }

    let research_digest = match load_research(&options.research) {
        Ok(digest) => digest,
        Err(error) => {
            eprintln!("failed to run market research: {error}");
            process::exit(1);
        }
    };

    let orchestrator = DailyBetOrchestrator::new(options.rules.clone());
    let recommendation = orchestrator.recommend(candidates, research_digest.as_ref());
    let deterministic_report = render_recommendation(&options.rules, &recommendation);
    let report = match run_ai_workflow(&deterministic_report, &options.ai) {
        Ok(Some(ai_report)) => ai_report.final_output,
        Ok(None) => deterministic_report,
        Err(error) => {
            eprintln!("failed to run OpenAI agent workflow: {error}");
            process::exit(1);
        }
    };
    println!("{report}");

    match deliver_report(&report, &options.delivery) {
        Ok(delivered) if !delivered.is_empty() => {
            eprintln!("delivered report via {}", delivered.join(", "));
        }
        Ok(_) => {}
        Err(error) => {
            eprintln!("failed to deliver report: {error}");
            process::exit(1);
        }
    }
}

impl CliOptions {
    fn parse<I>(mut args: I) -> Result<Self, String>
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
                "--research" => {
                    research.source_path = Some(next_value(&mut args, "--research")?);
                }
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

    fn usage() -> &'static str {
        "Usage:\n\
           betting <norsk-tipping-candidates.csv> [options]\n\
           betting --norsk-tipping-live --date YYYY-MM-DD [options]\n\
         \n\
         Options:\n\
           --norsk-tipping-live       load live candidates from Norsk Tipping Oddsen\n\
           --nt-events-per-sport N    live source event page size, default 35\n\
           --nt-earliest-start TEXT   live source cutoff, e.g. 2026-05-16T16:00\n\
           --date YYYY-MM-DD          only consider events on this date\n\
           --min-odds N               default 1.15\n\
           --max-odds N               default 1.30\n\
           --min-probability N        default 0.79\n\
           --min-confidence N         default 0.65\n\
           --min-edge N               default 0.015\n\
           --min-ev N                 default 0.000\n\
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

fn load_candidates(
    source: &CandidateSource,
    rules: &BettingRules,
) -> Result<Vec<domain::BetCandidate>, String> {
    match source {
        CandidateSource::Csv(path) => load_candidates_from_csv(path),
        CandidateSource::NorskTippingLive(options) => {
            norsk_tipping::load_candidates_from_live_odds(rules, options)
        }
    }
}

fn load_research(options: &ResearchOptions) -> Result<Option<research::ResearchDigest>, String> {
    if options.source_path.is_none() {
        return Ok(None);
    }

    let sources = load_sources(options)?;
    if sources.is_empty() {
        return Ok(Some(research::ResearchDigest::empty()));
    }

    let client = MarketResearchClient::new()?;
    Ok(Some(client.fetch(&sources, options)))
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
