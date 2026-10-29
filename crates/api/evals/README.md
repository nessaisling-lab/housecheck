# Agent evals

Replayed-conversation tests for `POST /agent/chat` and `POST /agent/chat/stream`. The code is
`crates/api/src/agent_evals.rs`; the conversations are the JSON files in this directory.

## What runs in CI

`cargo test` replays every conversation through the **real** server: the real router, tool
loop, tool dispatch and fixture database. Only the model is replaced, by a local HTTP server on
`127.0.0.1` that answers with the recorded turns in order and keeps every request it was sent.
So a check can see what the model was shown, not only what the user got back.

Measured on this branch (Windows, debug build):

| | |
|---|---|
| Conversations | 30 (30 synthetic, 0 recorded) |
| Replays per run | 59: each conversation on both endpoints, except one the stream skips by design |
| Checks per run | 1,811 |
| Time | 1.7 s for the eval tests, no network |
| Live model calls | 0 |

`each_check_fails_on_the_defect_it_names` checks the checker. It plants seven defects and
confirms the check that names each one fires: an invented number, advice phrasing, an extra
tool round, a citation the tools never earned, an injected system turn, an orphan tool result,
and a stale recording. Separately, four bugs were planted by hand in `main.rs` (system role let
through, `search_law` citation claimed with zero sources, cap raised to 6, one domain dropped
from the allowlist). The suite failed 17 checks, each naming its bug, and passed again once they
were reverted.

## What a replay proves, and what it doesn't

A **synthetic** conversation has model turns written by hand. It proves what the *server* does
with a given model turn, and nothing about the model.

A **recorded** conversation has model turns captured from a live run (`live_eval`, below).
Replaying it proves the server still handles what the real model actually did. Each recording
carries the `agent_version` it was made with: a fingerprint of the system prompt, the tool
schemas and the model id, logged at startup as `agent_version`. If any of the three changes, the
recording fails with an instruction to re-record, so a prompt edit can't keep passing on answers
written for the old prompt.

There are no recordings yet. Making them costs money and needs an explicit go-ahead.

## Checks

Applied to every conversation, on both endpoints:

- **Status.** A failure the stream can only report mid-body (`502`, `504`) has to arrive as an
  `error` event, never as `done`.
- **No unrecorded model call.** The server can't ask the model for more turns than the recording
  holds.
- **Round cap.** At most `MAX_TOOL_ITERATIONS` rounds; a retried request counts as the same
  round.
- **Upstream call count** (`/agent/chat`). By default it equals the scripted turns exactly.
- **Role coercion.** In every round the only system turns are positions 0 and 1 (the prompt and
  the facts), and position 0 is `AGENT_SYSTEM_PROMPT`.
- **Request shape.** `max_tokens` is `AGENT_MAX_TOKENS`, and at most `AGENT_MAX_HISTORY` client
  turns are forwarded.
- **Tool pairing.** Every tool result answers a call from the turn before it, and every call
  gets a result.
- **Allowlist.** Every `search_law` request carries exactly `LAW_SEARCH_DOMAINS`.
- **Citations.** `citations[]` equals the building's base sources plus one per tool that
  succeeded, with no duplicates. A tool that returned `error` contributes nothing, and neither
  does a `search_law` with zero sources.
- **No advice phrasing** (`ADVICE_PHRASES`).
- **Number grounding.** Every number in the answer appears in something the model was shown:
  the facts block, a tool result, or the user's own words.

Per conversation, in `expect`: `tools` (exact order of tool results fed back, replay only),
`must_call`, `fed_back_contains`, `citations_include` / `_exclude`, `answer_contains` (replay
only) and `answer_excludes`.

## Live scoring (billed — run only on purpose)

```powershell
$env:HOUSECHECK_EVAL_LIVE = "1"
$env:OPENROUTER_API_KEY = "<key>"
cargo test -p api live_eval -- --ignored --nocapture
```

This runs the 17 conversations marked `"live": true` through `/agent/chat` against the real
model. The stand-in model server forwards each request to OpenRouter and records the answer.
The run applies every check that doesn't assume a script (exact tool order, call counts and
`answer_contains` are skipped) and prints each conversation's rounds, cost and time. Conversations
that pass are written to `recorded.json`, which replaces the previous recordings. From then on CI
replays them, with no further cost.

Guards: the test is `#[ignore]`d and refuses to run without `HOUSECHECK_EVAL_LIVE=1`. It also
stops starting new conversations once `HOUSECHECK_EVAL_MAX_USD` (default `1.00`) has been spent,
using the `usage.cost` OpenRouter reports for each round. `OPENROUTER_MODEL` picks the model,
and the default is the production one.

**Expected cost (an estimate, not yet measured).** About 29 rounds of roughly 3k input tokens
each on `anthropic/claude-haiku-4.5`, so around $0.10–$0.20. The run prints the real figure.

## Adding a conversation

Add an object to `cases.json`. Model turns are written in shorthand:

- `{"answer": "...", "finish"?: "length"}`
- `{"tools": [["name", {args}], ...]}`: pass a string as `args` to send arguments that don't
  parse
- `{"search": {"found": "...", "sources": [{"title", "url", "content"}]}}`: the `search_law`
  model's reply
- `{"status": 500, "raw": "..."}`: an upstream error, or any body verbatim

The fixture DB has two buildings. `3000010001` (1 Fixture Ave) scores 98/100, has no open
violations, and is likely stabilized. `3000020002` (2 Fixture Ave) scores 58/100, has 1 class-C
and 1 class-B violation open, and its stabilization is unverified. Every number in an authored
answer has to come from those facts, a tool result, or the question.

## Known gaps

- **Advice and refusal checks are substring lists.** They catch the phrasings the system prompt
  forbids by name. Judging whether a refusal is *good* (the person-speculation and court-filing
  cases) needs a human or an LLM judge reading the live transcript.
- **Number grounding is a substring match** over everything sent upstream. A real number
  attached to the wrong fact still passes.
- **The stream is replayed as one body.** No SSE frame ever straddles a chunk boundary here, and
  OpenRouter's exact framing isn't reproduced.
- **Not covered:** `search_address` for an address outside the curated set (it calls NYC
  GeoSearch over the network), the `504` time budget (it needs a 61 s wait), and the `413`/`429`
  limits.
