use std::env;
use std::time::{Duration, Instant};

#[derive(Debug)]
pub(crate) struct StageTimings {
    enabled: bool,
    last_mark: Instant,
    stages: Vec<(&'static str, Duration)>,
}

impl StageTimings {
    pub(crate) fn from_env() -> Self {
        let enabled = env::var("BETTING_TIMINGS")
            .ok()
            .is_some_and(|value| matches!(value.as_str(), "1" | "true" | "yes" | "on"));
        Self {
            enabled,
            last_mark: Instant::now(),
            stages: Vec::new(),
        }
    }

    pub(crate) fn mark(&mut self, stage: &'static str) {
        if !self.enabled {
            return;
        }

        let now = Instant::now();
        self.stages
            .push((stage, now.duration_since(self.last_mark)));
        self.last_mark = now;
    }

    pub(crate) fn finish(self) {
        if !self.enabled {
            return;
        }

        let total = self
            .stages
            .iter()
            .fold(Duration::ZERO, |total, (_, duration)| total + *duration);
        for (stage, duration) in self.stages {
            eprintln!("timing {stage}: {:.3}s", duration.as_secs_f64());
        }
        eprintln!("timing total: {:.3}s", total.as_secs_f64());
    }
}
