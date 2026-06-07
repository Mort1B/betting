mod agents;
mod ai;
mod cli;
mod domain;
mod football_context;
mod football_data_provider;
mod history;
mod history_pipeline;
mod input;
mod norsk_tipping;
mod notify;
mod reference;
mod reference_provider;
mod report;
mod research;
mod settlement;
mod team_name;
mod timing;

use std::env;
use std::process;

use agents::DailyBetOrchestrator;
use ai::run_ai_workflow;
use cli::{CandidateSource, CliOptions};
use domain::BettingRules;
use football_data_provider::apply_football_data;
use input::load_candidates_from_csv;
use notify::deliver_report;
use reference::apply_reference_odds;
use report::{JsonReportMeta, render_recommendation, write_json_report};
use research::{MarketResearchClient, ResearchOptions, load_sources};
use timing::StageTimings;

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

    let mut timings = StageTimings::from_env();
    let mut candidates = match load_candidates(&options.source, &options.rules) {
        Ok(candidates) => candidates,
        Err(error) => {
            eprintln!("failed to load candidates: {error}");
            process::exit(1);
        }
    };
    timings.mark("candidate_loading");

    let reference_provider_notes = match apply_reference_odds(candidates, &options.reference_odds) {
        Ok(enriched) => {
            candidates = enriched.candidates;
            enriched.provider_report_notes
        }
        Err(error) => {
            eprintln!("failed to apply reference odds: {error}");
            process::exit(1);
        }
    };
    timings.mark("reference_enrichment");

    let football_data_result =
        apply_football_data(candidates, &options.rules, &options.football_data);
    candidates = football_data_result.candidates;
    let football_data_provider_notes = football_data_result.provider_report_notes;
    timings.mark("football_data_enrichment");

    let research_digest = match load_research(&options.research) {
        Ok(digest) => digest,
        Err(error) => {
            eprintln!("failed to run market research: {error}");
            process::exit(1);
        }
    };
    timings.mark("research_fetch");

    let mut history_state = match history_pipeline::HistoryState::from_env() {
        Ok(history_state) => history_state,
        Err(error) => {
            eprintln!("failed to load learning history: {error}");
            process::exit(1);
        }
    };
    let orchestrator = DailyBetOrchestrator::with_learning_agent(
        options.rules.clone(),
        history_state.learning_agent(),
    );
    timings.mark("learning_history_load");

    let recommendation = orchestrator.recommend(candidates, research_digest.as_ref());
    timings.mark("deterministic_scoring");

    if let Err(error) = history_state.write_recommendation(&recommendation, &options.rules) {
        eprintln!("failed to write pick history: {error}");
        process::exit(1);
    }
    timings.mark("history_write");

    let deterministic_report = render_recommendation(
        &options.rules,
        &recommendation,
        &reference_provider_notes,
        &football_data_provider_notes,
    );
    timings.mark("report_render");

    let mut ai_used = false;
    let mut ai_fallback_reason = None;
    let report = match run_ai_workflow(&deterministic_report, &options.ai) {
        Ok(Some(ai_report)) => {
            ai_used = true;
            ai_report.final_output
        }
        Ok(None) => deterministic_report.clone(),
        Err(error) => {
            let reason = format!(
                "OpenAI workflow unavailable; deterministic report published: {}",
                public_error(&error)
            );
            eprintln!("{reason}");
            ai_fallback_reason = Some(reason.clone());
            format!("{deterministic_report}\nAI review note: {reason}\n")
        }
    };
    timings.mark("ai_review");

    if let Ok(path) = env::var("BETTING_JSON_OUTPUT") {
        let meta = JsonReportMeta {
            final_text_report: report.clone(),
            deterministic_text_report: deterministic_report,
            ai_enabled: options.ai.enabled,
            ai_used,
            ai_fallback_reason,
            reference_provider_notes,
            football_data_provider_notes,
        };
        if let Err(error) = write_json_report(&path, &options.rules, &recommendation, &meta) {
            eprintln!("{error}");
            process::exit(1);
        }
    }
    timings.mark("json_report_write");

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
    timings.mark("delivery");
    timings.finish();
}

fn public_error(error: &str) -> String {
    error
        .replace("OPENAI_API_KEY", "OpenAI API key")
        .replace("BETTING_ODDS_API_KEY", "Odds API key")
        .replace("BETTING_FOOTBALL_DATA_API_KEY", "Football data API key")
}

fn load_candidates(
    source: &CandidateSource,
    rules: &BettingRules,
) -> Result<Vec<domain::BetCandidate>, String> {
    match source {
        CandidateSource::Csv(path) => rules.filter_by_sport_scope(load_candidates_from_csv(path)?),
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
