# LangGraph Evaluation

LangGraph is not necessary for the current project.

The current workflow is a small fixed pipeline:

1. Load Norsk Tipping candidates from CSV.
2. Fetch research sources.
3. Filter candidates by date and odds band.
4. Score probability, value, risk, and research signals.
5. Publish a static top-3 report.
6. Optionally ask ChatGPT/Codex to review the report manually.

Rust handles that pipeline clearly with simple modules and tests. Adding
LangGraph now would introduce a second runtime, most likely Python or
TypeScript, without removing the need for good input data, reference odds, or
human judgment.

## When LangGraph Would Help

LangGraph becomes useful if the project grows into stateful, branching
automation:

- persistent state across days,
- human approval checkpoints before publishing,
- retries and resumable execution,
- separate tool nodes for odds collection, web research, model scoring, review,
  and publishing,
- conditional routing, such as `NO BET`, `needs more research`, or `publish`,
- multi-agent traces you want to inspect after each run.

In that version, the Rust binary should remain the deterministic scoring tool.
LangGraph would orchestrate around it:

```text
collect_candidates
  -> fetch_research
  -> run_rust_scoring_tool
  -> explorer_review
  -> reviewer_check
  -> risk_manager_gate
  -> human_approval_optional
  -> output_writer
  -> publish_report
```

## Recommended Decision

Do not add LangGraph now.

The current GitHub Actions plus Rust pipeline is cheaper, simpler, and easier to
debug. Reconsider LangGraph when the system needs durable state, branching,
manual approval checkpoints, or multiple external tools that must be resumed
after failures.

## References

- LangGraph workflows and agents:
  https://docs.langchain.com/oss/python/langgraph/workflows-agents
- LangGraph durable execution:
  https://langchain-5e9cc07a.mintlify.app/oss/python/langgraph/durable-execution
- LangGraph human-in-the-loop:
  https://docs.langchain.com/langgraph-platform/add-human-in-the-loop
