mod agents;
mod ai;
mod cli;
mod domain;
mod football_context;
mod history;
mod history_pipeline;
mod input;
mod norsk_tipping;
mod notify;
mod reference;
mod report;
mod research;
mod settlement;
mod timing;

use std::env;
use std::process;

use agents::DailyBetOrchestrator;
use ai::run_ai_workflow;
use cli::{CandidateSource, CliOptions};
use domain::BettingRules;
use input::load_candidates_from_csv;
use notify::deliver_report;
use reference::apply_reference_odds;
use report::render_recommendation;
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

    match apply_reference_odds(candidates, &options.reference_odds) {
        Ok(enriched) => candidates = enriched,
        Err(error) => {
            eprintln!("failed to apply reference odds: {error}");
            process::exit(1);
        }
    }
    timings.mark("reference_enrichment");

    let research_digest = match load_research(&options.research) {
        Ok(digest) => digest,
        Err(error) => {
            eprintln!("failed to run market research: {error}");
            process::exit(1);
        }
    };
    timings.mark("research_fetch");

    let orchestrator = match DailyBetOrchestrator::from_env(options.rules.clone()) {
        Ok(orchestrator) => orchestrator,
        Err(error) => {
            eprintln!("failed to load learning history: {error}");
            process::exit(1);
        }
    };
    timings.mark("learning_history_load");

    let recommendation = orchestrator.recommend(candidates, research_digest.as_ref());
    timings.mark("deterministic_scoring");

    if let Err(error) = history_pipeline::write_history_from_env(&recommendation, &options.rules) {
        eprintln!("failed to write pick history: {error}");
        process::exit(1);
    }
    timings.mark("history_write");

    let deterministic_report = render_recommendation(&options.rules, &recommendation);
    timings.mark("report_render");

    let report = match run_ai_workflow(&deterministic_report, &options.ai) {
        Ok(Some(ai_report)) => ai_report.final_output,
        Ok(None) => deterministic_report,
        Err(error) => {
            eprintln!("failed to run OpenAI agent workflow: {error}");
            process::exit(1);
        }
    };
    timings.mark("ai_review");

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
