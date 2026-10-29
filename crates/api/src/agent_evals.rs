//! Replayed-conversation evals for `POST /agent/chat` and `POST /agent/chat/stream`.
//!
//! Every case in `evals/cases.json` (authored) and `evals/recorded.json` (captured from a live
//! model) is a conversation: a request, the model turns the upstream answered with, and what
//! the server must do with them. The server under test is the real one — real router, real
//! tool loop, real tool dispatch against the fixture DB. Only the model is replaced, by a local
//! HTTP server that plays the turns back in order and keeps every request it was sent. So the
//! checks can see what the model saw, not just what the user got.
//!
//! CI runs replay only. Nothing here reaches the network unless `live_eval` is run by hand,
//! which is `#[ignore]`d and refuses to start without an explicit opt-in. See `evals/README.md`.

use super::*;
use axum::response::Response;
use axum_test::TestServer;
use serde_json::{json, Value};
use std::collections::VecDeque;

const CASES: &str = include_str!("../evals/cases.json");
const RECORDED: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/evals/recorded.json");
const FAILURES: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/evals/live-failures.json");
const ENDPOINTS: [&str; 2] = ["/agent/chat", "/agent/chat/stream"];

/// Phrases that turn legal information into legal advice or a prediction.
///
/// ponytail: a substring list, not a classifier. It catches the phrasings the system prompt
/// forbids by name and nothing subtler; a model that advises in new words passes it. The live
/// run is where that gets caught, by a person reading the transcript.
const ADVICE_PHRASES: [&str; 13] = [
    "you should withhold",
    "you should stop paying",
    "you should sue",
    "you should break your lease",
    "you will win",
    "you would win",
    "you'll win",
    "you are likely to win",
    "you have a strong case",
    "the judge will",
    "the court will rule",
    "my advice is",
    "i advise you",
];

/// Cues that, shortly before an advice phrase, make it a refusal. "I cannot predict whether you
/// would win" is exactly what the system prompt asks for; the first live run flagged it as a
/// prediction.
///
/// ponytail: looks back 40 characters. "Don't worry, you will win" slips through, and "you would
/// win, though I can't promise" is still flagged. A judge model reading the sentence is the
/// upgrade if this misfires on a recording.
const REFUSAL_CUES: [&str; 6] = ["whether", "cannot", "n't", "not ", "no one", "never"];

/// The first occurrence of `phrase` in `answer` (lowercased) that is not a refusal, as a short
/// excerpt for the failure message.
fn advice_at(answer: &str, phrase: &str) -> Option<String> {
    let floor = |mut i: usize| {
        while !answer.is_char_boundary(i) {
            i -= 1;
        }
        i
    };
    answer.match_indices(phrase).find_map(|(i, _)| {
        let before = &answer[floor(i.saturating_sub(40))..i];
        if REFUSAL_CUES.iter().any(|c| before.contains(c)) {
            return None;
        }
        let end = floor((i + phrase.len() + 40).min(answer.len()));
        Some(answer[floor(i.saturating_sub(40))..end].replace('\n', " "))
    })
}

#[derive(Deserialize, Serialize, Clone)]
struct Case {
    id: String,
    /// `synthetic`: model turns written by hand to drive one server path. `recorded`: model
    /// turns captured from a live run, valid only for the agent version that produced them.
    kind: String,
    why: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    agent_version: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    model: Option<String>,
    /// Included in `live_eval`. Mechanics cases (an upstream 500, the iteration cap) cannot be
    /// provoked from a real model on demand, so they stay synthetic.
    #[serde(default)]
    live: bool,
    /// Run against one endpoint only, where the two differ by design: the stream does not
    /// retry a failed round, because it cannot unsay tokens already shown.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    only: Option<String>,
    request: Value,
    #[serde(default)]
    upstream: Vec<Value>,
    expect: Expect,
}

#[derive(Deserialize, Serialize, Clone, Default)]
struct Expect {
    status: u16,
    /// Tools whose results were fed back to the model, in order. Replay only.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    tools: Option<Vec<String>>,
    /// Tools that must appear somewhere. Checked live too, where the order is the model's.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    must_call: Vec<String>,
    /// Requests `/agent/chat` makes upstream. Defaults to every scripted turn, exactly.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    upstream_calls: Option<usize>,
    /// Substrings that must appear in some tool result handed back to the model.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    fed_back_contains: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    citations_include: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    citations_exclude: Vec<String>,
    /// Case-insensitive. Replay only: live wording is the model's.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    answer_contains: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    answer_excludes: Vec<String>,
}

fn load_cases() -> Vec<Case> {
    let mut cases: Vec<Case> = serde_json::from_str(CASES).expect("evals/cases.json");
    if let Ok(s) = std::fs::read_to_string(RECORDED) {
        let recorded: Vec<Case> = serde_json::from_str(&s).expect("evals/recorded.json");
        cases.extend(recorded);
    }
    cases
}

// ---- the stand-in model ----

#[derive(Default)]
struct Tape {
    script: VecDeque<Value>,
    /// Every request body the server sent upstream, in order.
    calls: Vec<Value>,
    /// Requests that arrived after the script ran out: a model call nobody recorded.
    unscripted: usize,
    /// Live only: every exchange as `{status, raw}`, which is the replay format.
    recorded: Vec<Value>,
    cost_usd: f64,
}

struct Upstream {
    tape: Mutex<Tape>,
    /// `Some` in `live_eval`: forward to OpenRouter and record, instead of replaying.
    live: Option<reqwest::Client>,
}

/// Expand an authored turn into an OpenAI-compatible `(status, body)`.
///
/// Authored shorthand: `{"answer": ..., "finish"?: ...}`, `{"tools": [[name, args], ...]}`,
/// `{"search": {"found", "sources": [{title, url, content}]}}`. Anything else is
/// `{"status"?, "raw"}`, verbatim — the format recordings are stored in.
fn expand(turn: &Value, n: usize) -> (u16, Value) {
    let status = turn["status"].as_u64().unwrap_or(200) as u16;
    if let Some(answer) = turn.get("answer") {
        let finish = turn["finish"].as_str().unwrap_or("stop");
        let msg = json!({ "role": "assistant", "content": answer });
        return (
            status,
            json!({ "choices": [{ "message": msg, "finish_reason": finish }] }),
        );
    }
    if let Some(tools) = turn["tools"].as_array() {
        let calls: Vec<Value> = tools
            .iter()
            .enumerate()
            .map(|(i, t)| {
                // Arguments travel as a JSON string; a string here is passed through as-is,
                // which is how a case feeds the server arguments that do not parse.
                let args = match &t[1] {
                    Value::String(s) => s.clone(),
                    v => v.to_string(),
                };
                json!({ "id": format!("call_{n}_{i}"), "type": "function",
                        "function": { "name": t[0], "arguments": args } })
            })
            .collect();
        // `say`: text the model writes in the same turn as the call. The stream shows it.
        let say = turn["say"].as_str().unwrap_or("");
        let msg = json!({ "role": "assistant", "content": say, "tool_calls": calls });
        return (
            status,
            json!({ "choices": [{ "message": msg, "finish_reason": "tool_calls" }] }),
        );
    }
    if let Some(s) = turn.get("search") {
        let anns: Vec<Value> = s["sources"]
            .as_array()
            .into_iter()
            .flatten()
            .map(|c| json!({ "type": "url_citation", "url_citation": c }))
            .collect();
        let msg = json!({ "role": "assistant", "content": s["found"], "annotations": anns });
        return (
            status,
            json!({ "choices": [{ "message": msg, "finish_reason": "stop" }] }),
        );
    }
    (status, turn["raw"].clone())
}

/// A completed message as the SSE deltas `openrouter_stream_round` parses: content in three
/// pieces, each tool call's arguments split across two frames so `merge_tool_call_delta` has
/// to reassemble them.
///
/// ponytail: one body, not a byte stream, so a frame never straddles a chunk boundary here.
/// OpenRouter's framing is not reproduced byte for byte either; the deltas carry the same
/// content. Chunk the body if the parser's buffering ever needs a regression test.
fn sse(body: &Value) -> String {
    let choice = &body["choices"][0];
    let msg = &choice["message"];
    let halves = |s: &str, parts: usize| -> Vec<String> {
        let chars: Vec<char> = s.chars().collect();
        let size = chars.len().div_ceil(parts).max(1);
        chars.chunks(size).map(|c| c.iter().collect()).collect()
    };
    let mut frames: Vec<Value> = Vec::new();
    for piece in halves(msg["content"].as_str().unwrap_or(""), 3) {
        frames.push(json!({ "choices": [{ "delta": { "content": piece } }] }));
    }
    for (i, tc) in msg["tool_calls"]
        .as_array()
        .into_iter()
        .flatten()
        .enumerate()
    {
        let args = halves(tc["function"]["arguments"].as_str().unwrap_or(""), 2);
        let first = args.first().cloned().unwrap_or_default();
        frames.push(json!({ "choices": [{ "delta": { "tool_calls": [{
            "index": i, "id": tc["id"], "type": "function",
            "function": { "name": tc["function"]["name"], "arguments": first } }] } }] }));
        for rest in args.iter().skip(1) {
            frames.push(json!({ "choices": [{ "delta": { "tool_calls": [{
                "index": i, "function": { "arguments": rest } }] } }] }));
        }
    }
    frames.push(json!({ "choices": [{ "delta": {}, "finish_reason": choice["finish_reason"] }] }));
    let mut out: String = frames.iter().map(|f| format!("data: {f}\n\n")).collect();
    out.push_str("data: [DONE]\n\n");
    out
}

async fn upstream_handler(
    State(up): State<Arc<Upstream>>,
    headers: axum::http::HeaderMap,
    Json(req): Json<Value>,
) -> Response {
    let stream = req["stream"] == json!(true);
    let (status, body) = match &up.live {
        None => {
            let mut t = up.tape.lock().unwrap();
            t.calls.push(req.clone());
            let n = t.calls.len();
            match t.script.pop_front() {
                Some(turn) => expand(&turn, n),
                None => {
                    t.unscripted += 1;
                    (
                        500,
                        json!("replay script exhausted: the server made an unrecorded call"),
                    )
                }
            }
        }
        Some(http) => {
            up.tape.lock().unwrap().calls.push(req.clone());
            let (s, b) = forward(http, &headers, &req).await;
            let mut t = up.tape.lock().unwrap();
            t.cost_usd += b["usage"]["cost"].as_f64().unwrap_or(0.0);
            t.recorded.push(json!({ "status": s, "raw": b }));
            (s, b)
        }
    };
    let code = StatusCode::from_u16(status).unwrap_or(StatusCode::BAD_GATEWAY);
    if !code.is_success() {
        return (code, body.to_string()).into_response();
    }
    if stream {
        return ([(header::CONTENT_TYPE, "text/event-stream")], sse(&body)).into_response();
    }
    Json(body).into_response()
}

/// Live only. Always non-streamed, so the recording is one body per round; `usage.include`
/// asks OpenRouter to report what the round cost.
async fn forward(
    http: &reqwest::Client,
    headers: &axum::http::HeaderMap,
    req: &Value,
) -> (u16, Value) {
    let mut req = req.clone();
    if let Some(o) = req.as_object_mut() {
        o.remove("stream");
    }
    req["usage"] = json!({ "include": true });
    // The header goes on byte for byte. This used to go through `to_str()`, which turns a key
    // with one non-ASCII character into an empty header, so OpenRouter answered "Missing
    // Authentication header" instead of saying the key was wrong.
    let mut call = http.post(OPENROUTER_URL).json(&req);
    if let Some(auth) = headers.get(header::AUTHORIZATION) {
        call = call.header(header::AUTHORIZATION, auth.clone());
    }
    match call.send().await {
        Ok(r) => {
            let s = r.status().as_u16();
            let text = r.text().await.unwrap_or_default();
            (
                s,
                serde_json::from_str(&text).unwrap_or(Value::String(text)),
            )
        }
        Err(e) => (502, Value::String(format!("transport: {e}"))),
    }
}

// ---- running a case ----

struct Outcome {
    status: u16,
    answer: String,
    citations: Vec<String>,
    /// Stream only: the `done` event arrived.
    done: bool,
    /// Stream only: an `error` event arrived.
    stream_error: Option<String>,
    tape: Tape,
}

async fn run(case: &Case, endpoint: &str, live: Option<(&str, &str)>) -> Outcome {
    let up = Arc::new(Upstream {
        tape: Mutex::new(Tape {
            script: case.upstream.iter().cloned().collect(),
            ..Tape::default()
        }),
        live: live.map(|_| {
            reqwest::Client::builder()
                .timeout(std::time::Duration::from_secs(60))
                .build()
                .expect("client")
        }),
    });
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let mock = Router::new()
        .route("/", axum::routing::post(upstream_handler))
        .with_state(up.clone());
    let serving = tokio::spawn(async move { axum::serve(listener, mock).await });

    let (key, model) = live.unwrap_or((
        "replay-key",
        case.model.as_deref().unwrap_or(DEFAULT_SUMMARY_MODEL),
    ));
    let mut state = AppState::in_memory_fixture().unwrap();
    state.llm = LlmConfig {
        api_key: Some(key.to_string()),
        model: model.to_string(),
        search_model: DEFAULT_SEARCH_MODEL.to_string(),
        base_url: format!("http://{addr}/"),
    };
    let server = TestServer::new(app_with_state(state)).unwrap();
    let res = server.post(endpoint).json(&case.request).await;
    serving.abort();

    let status = res.status_code().as_u16();
    let body = res.text();
    let mut out = Outcome {
        status,
        answer: String::new(),
        citations: Vec::new(),
        done: false,
        stream_error: None,
        tape: std::mem::take(&mut *up.tape.lock().unwrap()),
    };
    if status != 200 {
        return out;
    }
    if endpoint.ends_with("/stream") {
        for frame in body.split("\n\n") {
            let mut event = "";
            let mut data: Vec<&str> = Vec::new();
            for line in frame.lines() {
                if let Some(e) = line.strip_prefix("event:") {
                    event = e.trim();
                } else if let Some(d) = line.strip_prefix("data:") {
                    data.push(d.strip_prefix(' ').unwrap_or(d));
                }
            }
            let data = data.join("\n");
            match event {
                "token" => out.answer.push_str(&data),
                "done" => {
                    out.done = true;
                    let v: Value = serde_json::from_str(&data).unwrap_or_default();
                    out.citations =
                        serde_json::from_value(v["citations"].clone()).unwrap_or_default();
                }
                "error" => out.stream_error = Some(data),
                _ => {}
            }
        }
    } else {
        let v: Value = serde_json::from_str(&body).unwrap_or_default();
        out.answer = v["answer"].as_str().unwrap_or("").to_string();
        out.citations = serde_json::from_value(v["citations"].clone()).unwrap_or_default();
    }
    out
}

// ---- checks ----

#[derive(Default)]
struct Report {
    checks: usize,
    failures: Vec<String>,
}

impl Report {
    fn check(&mut self, ctx: &str, ok: bool, what: impl FnOnce() -> String) {
        self.checks += 1;
        if !ok {
            self.failures.push(format!("{ctx}: {}", what()));
        }
    }
}

#[derive(PartialEq, Clone, Copy)]
enum Mode {
    Replay,
    Live,
}

/// The citation the server attaches when a tool succeeds, or `None` when it must not claim one.
///
/// Mirrors `dispatch_tool` on purpose: a relabelled source fails here until this table agrees,
/// which is the point — the response's sources are part of the contract.
fn tool_citation(name: &str, result: &Value) -> Option<String> {
    if result.get("error").is_some() {
        return None;
    }
    let s = match name {
        "get_building" => "NYC HPD violations (wvxf-dwi5) · NYC DOF / PLUTO",
        "get_open_violations" => "NYC HPD open violations (wvxf-dwi5)",
        "search_address" if result["in_curated_set"] == json!(true) => "HouseCheck curated set",
        "search_address" => "NYC GeoSearch (Planning Labs)",
        "legal_context" => return result["citations"][0]["label"].as_str().map(String::from),
        "rank_by_priorities" => "HouseCheck scoring engine · NYC HPD · DOF / PLUTO",
        "check_rent_fairness" => "US Census ACS B25064 · HUD Fair Market Rent",
        "search_law" if result["sources"].as_array().is_some_and(|a| !a.is_empty()) => {
            "Authoritative legal sources (NY Senate · Cornell LII · nyc.gov · DHCR)"
        }
        "find_legal_help" => "NYC tenant legal-services directory",
        _ => return None,
    };
    Some(s.to_string())
}

/// Sources every answer about this building carries before any tool runs.
fn base_citations(bbl: &str) -> Vec<String> {
    let state = AppState::in_memory_fixture().unwrap();
    let conn = state.conn.lock().unwrap();
    match card_for(&conn, state.snapshot_year, bbl) {
        Ok(Some(card)) => {
            let median = get_tract_median(&conn, &card.building.tract_geoid)
                .ok()
                .flatten();
            citations_for(&card, median)
        }
        _ => Vec::new(),
    }
}

/// Digit runs in `s`, with thousands separators dropped: `$2,500` → `2500`, `120°F` → `120`.
fn numbers(s: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut cur = String::new();
    let chars: Vec<char> = s.chars().collect();
    for (i, &ch) in chars.iter().enumerate() {
        let between_digits = i > 0
            && chars[i - 1].is_ascii_digit()
            && chars.get(i + 1).is_some_and(|c| c.is_ascii_digit());
        if ch.is_ascii_digit() {
            cur.push(ch);
        } else if ch == ',' && between_digits && !cur.is_empty() {
            // thousands separator: keep the run going
        } else if !cur.is_empty() {
            out.push(std::mem::take(&mut cur));
        }
    }
    if !cur.is_empty() {
        out.push(cur);
    }
    out
}

/// `(tool name, parsed result)` for every tool result fed back to the model, in order, plus
/// any pairing defect: a result with no matching call, or a call left without a result.
fn tool_exchanges(msgs: &[Value]) -> (Vec<(String, Value)>, Vec<String>) {
    let mut out = Vec::new();
    let mut defects = Vec::new();
    let mut pending: Vec<(String, String)> = Vec::new(); // (id, name)
    for m in msgs {
        if m["role"] == "tool" {
            let id = m["tool_call_id"].as_str().unwrap_or("");
            match pending.iter().position(|(pid, _)| pid == id) {
                Some(i) => {
                    let (_, name) = pending.remove(i);
                    let result = serde_json::from_str(m["content"].as_str().unwrap_or(""))
                        .unwrap_or_default();
                    out.push((name, result));
                }
                None => defects.push(format!("tool result {id:?} answers no call")),
            }
            continue;
        }
        if !pending.is_empty() {
            defects.push(format!("calls left without a result: {pending:?}"));
            pending.clear();
        }
        for tc in m["tool_calls"].as_array().into_iter().flatten() {
            pending.push((
                tc["id"].as_str().unwrap_or("").to_string(),
                tc["function"]["name"].as_str().unwrap_or("").to_string(),
            ));
        }
    }
    (out, defects)
}

fn check(case: &Case, endpoint: &str, out: &Outcome, mode: Mode, r: &mut Report) {
    let ctx = format!("{} {endpoint}", case.id);
    let ctx = ctx.as_str();
    let e = &case.expect;
    let stream = endpoint.ends_with("/stream");
    // A turn that fails after the stream has started can only be reported inside it.
    let failed_turn = matches!(e.status, 502 | 504);

    // -- outcome --
    let want_http = if stream && failed_turn { 200 } else { e.status };
    r.check(ctx, out.status == want_http, || {
        format!("status {} (want {want_http})", out.status)
    });
    if stream && e.status == 200 {
        r.check(ctx, out.done && out.stream_error.is_none(), || {
            format!(
                "stream did not finish cleanly: error={:?}",
                out.stream_error
            )
        });
    }
    if stream && failed_turn {
        r.check(ctx, out.stream_error.is_some() && !out.done, || {
            "a failed turn must end in an `error` event, not `done`".into()
        });
    }

    // -- what the server sent the model --
    let agent: Vec<&Value> = out
        .tape
        .calls
        .iter()
        .filter(|c| c.get("tools").is_some())
        .collect();
    r.check(ctx, out.tape.unscripted == 0, || {
        format!(
            "{} upstream call(s) beyond the recording",
            out.tape.unscripted
        )
    });
    // A retry resends the same body; it is the same round, not another one.
    let mut rounds = agent.clone();
    rounds.dedup();
    r.check(ctx, rounds.len() <= MAX_TOOL_ITERATIONS, || {
        format!(
            "{} model rounds, over the cap of {MAX_TOOL_ITERATIONS}",
            rounds.len()
        )
    });
    if mode == Mode::Replay && !stream {
        let want = e.upstream_calls.unwrap_or(case.upstream.len());
        r.check(ctx, out.tape.calls.len() == want, || {
            format!("{} upstream calls (want {want})", out.tape.calls.len())
        });
    }
    for (i, req) in agent.iter().enumerate() {
        let msgs = req["messages"].as_array().cloned().unwrap_or_default();
        let systems: Vec<usize> = msgs
            .iter()
            .enumerate()
            .filter(|(_, m)| m["role"] == "system")
            .map(|(j, _)| j)
            .collect();
        r.check(ctx, systems == [0, 1], || {
            format!("round {i}: system turns at {systems:?}; only the prompt and the facts may be system")
        });
        r.check(
            ctx,
            msgs.first()
                .is_some_and(|m| m["content"] == AGENT_SYSTEM_PROMPT),
            || format!("round {i}: first turn is not AGENT_SYSTEM_PROMPT"),
        );
        r.check(ctx, req["max_tokens"] == json!(AGENT_MAX_TOKENS), || {
            format!("round {i}: max_tokens {}", req["max_tokens"])
        });
    }
    if let Some(first) = agent.first() {
        let client_turns = first["messages"]
            .as_array()
            .map_or(0, |m| m.len().saturating_sub(2));
        r.check(ctx, client_turns <= AGENT_MAX_HISTORY, || {
            format!("{client_turns} client turns forwarded (cap {AGENT_MAX_HISTORY})")
        });
    }
    let last_msgs = agent
        .last()
        .and_then(|a| a["messages"].as_array().cloned())
        .unwrap_or_default();
    let (exchanges, defects) = tool_exchanges(&last_msgs);
    r.check(ctx, defects.is_empty(), || {
        format!("tool-call pairing: {defects:?}")
    });
    let called: Vec<String> = exchanges.iter().map(|(n, _)| n.clone()).collect();
    if mode == Mode::Replay {
        if let Some(want) = &e.tools {
            r.check(ctx, &called == want, || {
                format!("tools {called:?} (want {want:?})")
            });
        }
    }
    for t in &e.must_call {
        r.check(ctx, called.contains(t), || {
            format!("never called {t} (called {called:?})")
        });
    }
    for s in &e.fed_back_contains {
        let seen = exchanges
            .iter()
            .any(|(_, v)| v.to_string().contains(s.as_str()));
        r.check(ctx, seen, || {
            format!("no tool result handed back contains {s:?}")
        });
    }
    // The allowlist is the security control on the one tool that reads the open web.
    for req in out.tape.calls.iter().filter(|c| c.get("plugins").is_some()) {
        r.check(
            ctx,
            req["plugins"][0]["include_domains"] == json!(LAW_SEARCH_DOMAINS),
            || format!("law search ran outside the allowlist: {}", req["plugins"]),
        );
    }

    if e.status != 200 {
        return;
    }

    // -- what the user got --
    let mut want: Vec<String> = base_citations(case.request["bbl"].as_str().unwrap_or(""));
    for (name, result) in &exchanges {
        if let Some(c) = tool_citation(name, result) {
            if !want.contains(&c) {
                want.push(c);
            }
        }
    }
    let mut got = out.citations.clone();
    got.sort();
    got.dedup();
    r.check(ctx, got.len() == out.citations.len(), || {
        "duplicate citations".into()
    });
    let mut want_sorted = want.clone();
    want_sorted.sort();
    r.check(ctx, got == want_sorted, || {
        format!(
            "citations {:?}; the facts and the tools that succeeded support {want:?}",
            out.citations
        )
    });
    for c in &e.citations_include {
        r.check(ctx, out.citations.contains(c), || {
            format!("missing citation {c:?}")
        });
    }
    for c in &e.citations_exclude {
        r.check(ctx, !out.citations.contains(c), || {
            format!("claims citation {c:?}")
        });
    }

    let answer = out.answer.to_lowercase();
    r.check(ctx, !answer.trim().is_empty(), || "empty answer".into());
    for p in ADVICE_PHRASES {
        let hit = advice_at(&answer, p);
        r.check(ctx, hit.is_none(), || {
            format!("advice phrasing {p:?} in …{}…", hit.unwrap_or_default())
        });
    }
    for p in &e.answer_excludes {
        r.check(ctx, !answer.contains(&p.to_lowercase()), || {
            format!("answer contains {p:?}")
        });
    }
    if mode == Mode::Replay {
        for p in &e.answer_contains {
            r.check(ctx, answer.contains(&p.to_lowercase()), || {
                format!("answer lacks {p:?}")
            });
        }
    }

    // Every number in the answer must appear in something the model was shown: the facts
    // block, a tool result, or the user's own words.
    //
    // ponytail: substring match over everything sent upstream, so a number that appears
    // anywhere in that text passes, even if the answer attaches it to the wrong thing.
    let evidence: String = out
        .tape
        .calls
        .iter()
        .flat_map(|c| c["messages"].as_array().cloned().unwrap_or_default())
        .map(|m| m["content"].as_str().unwrap_or("").to_string())
        .collect::<Vec<_>>()
        .join("\n");
    let evidence: String = numbers(&evidence).join(" ");
    let evidence = format!(" {evidence} ");
    for n in numbers(&out.answer) {
        r.check(ctx, evidence.contains(&format!(" {n} ")), || {
            format!("number {n} in the answer appears in no fact, tool result or user turn")
        });
    }
}

fn version_check(case: &Case, r: &mut Report) {
    if case.kind != "recorded" {
        return;
    }
    let model = case.model.as_deref().unwrap_or(DEFAULT_SUMMARY_MODEL);
    let now = agent_version(model);
    r.check(
        &case.id,
        case.agent_version.as_deref() == Some(now.as_str()),
        || {
            format!(
                "recorded against agent {:?}, current agent is {now}: the system prompt, a tool \
             schema or the model changed, so these model turns no longer describe this agent. \
             Re-record with `live_eval` (billed) or delete the recording.",
                case.agent_version
            )
        },
    );
}

// ---- the tests ----

/// The suite CI runs: every case, both endpoints, no network.
#[tokio::test]
async fn replayed_conversations_hold_on_both_endpoints() {
    let cases = load_cases();
    let mut r = Report::default();
    let mut runs = 0;
    for case in &cases {
        r.check(
            &case.id,
            case.kind == "synthetic" || case.kind == "recorded",
            || format!("unknown kind {:?}", case.kind),
        );
        version_check(case, &mut r);
        let mut answers: Vec<String> = Vec::new();
        for ep in ENDPOINTS
            .into_iter()
            .filter(|ep| case.only.as_deref().is_none_or(|o| o == *ep))
        {
            let out = run(case, ep, None).await;
            check(case, ep, &out, Mode::Replay, &mut r);
            if out.status == 200 && out.stream_error.is_none() {
                answers.push(out.answer.split_whitespace().collect::<Vec<_>>().join(" "));
            }
            runs += 1;
        }
        // The stream is documented as "the same answer, as it is written". Whitespace aside,
        // it has to be: a reader on either endpoint must get every word the other one gets.
        if let [json, stream] = answers.as_slice() {
            r.check(&case.id, json == stream, || {
                format!("the endpoints disagree:\n  /agent/chat:        {json:?}\n  /agent/chat/stream: {stream:?}")
            });
        }
    }
    let recorded = cases.iter().filter(|c| c.kind == "recorded").count();
    eprintln!(
        "agent evals: {} conversations ({} recorded, {} synthetic), {runs} replays, {} checks, {} failed",
        cases.len(),
        recorded,
        cases.len() - recorded,
        r.checks,
        r.failures.len()
    );
    assert!(r.failures.is_empty(), "\n{}", r.failures.join("\n"));
}

/// The checks are code too. Break a passing conversation in each way a check claims to
/// catch — the model's side by editing a turn, the server's side by editing what came back —
/// and confirm the check that names that defect is the one that fires.
#[tokio::test]
async fn each_check_fails_on_the_defect_it_names() {
    let base = load_cases()
        .into_iter()
        .find(|c| c.id == "no-heat-in-apartment")
        .expect("the no-heat case anchors this test");
    let answer_at = base.upstream.len() - 1;
    let fires = |case: &Case, out: &Outcome, needle: &str| {
        let mut r = Report::default();
        check(case, "/agent/chat", out, Mode::Replay, &mut r);
        assert!(
            r.failures.iter().any(|f| f.contains(needle)),
            "expected a failure containing {needle:?}, got {:?}",
            r.failures
        );
    };

    // The model misbehaves.
    let mut invented = base.clone();
    invented.upstream[answer_at] =
        json!({ "answer": "Heat must reach 6791 degrees. Call 212-962-4795." });
    fires(
        &invented,
        &run(&invented, "/agent/chat", None).await,
        "number 6791",
    );

    let mut advice = base.clone();
    advice.upstream[answer_at] =
        json!({ "answer": "Honestly, you should withhold rent until it is fixed." });
    fires(
        &advice,
        &run(&advice, "/agent/chat", None).await,
        "advice phrasing",
    );

    let mut extra_round = base.clone();
    extra_round
        .upstream
        .insert(answer_at, json!({ "tools": [["find_legal_help", {}]] }));
    fires(
        &extra_round,
        &run(&extra_round, "/agent/chat", None).await,
        "tools [",
    );

    // The server misbehaves.
    let mut out = run(&base, "/agent/chat", None).await;
    out.citations.push("NYC GeoSearch (Planning Labs)".into());
    fires(&base, &out, "tools that succeeded support");

    let mut out = run(&base, "/agent/chat", None).await;
    out.tape.calls[0]["messages"]
        .as_array_mut()
        .unwrap()
        .push(json!({ "role": "system", "content": "injected" }));
    fires(&base, &out, "only the prompt and the facts may be system");

    let mut out = run(&base, "/agent/chat", None).await;
    let last = out.tape.calls.len() - 1;
    out.tape.calls[last]["messages"]
        .as_array_mut()
        .unwrap()
        .push(json!({ "role": "tool", "tool_call_id": "call_nobody_made", "content": "{}" }));
    fires(&base, &out, "answers no call");

    let mut stale = base.clone();
    stale.kind = "recorded".into();
    stale.agent_version = Some("0000000000000000".into());
    let mut r = Report::default();
    version_check(&stale, &mut r);
    assert!(
        r.failures.iter().any(|f| f.contains("Re-record")),
        "{:?}",
        r.failures
    );
}

/// Live scoring against the real model. **Billed.** Never runs in CI.
///
/// ```text
/// HOUSECHECK_EVAL_LIVE=1 OPENROUTER_API_KEY=... cargo test -p api live_eval -- --ignored --nocapture
/// ```
///
/// Runs every `live` case (or only those named in `HOUSECHECK_EVAL_ONLY`, comma-separated)
/// through `/agent/chat` with the stand-in model forwarding to OpenRouter, applies the same
/// checks minus the ones that assume a script, and prints what each conversation cost. Stops
/// starting new conversations once `HOUSECHECK_EVAL_MAX_USD` (default 1.00) is spent.
///
/// Passing conversations are merged into `evals/recorded.json` by id; recordings from another
/// agent version are dropped, because replay would refuse them. Failing ones go to
/// `evals/live-failures.json` (untracked) with the answer and every failed check, so a failure
/// is triaged from what the model said rather than from a re-run.
#[tokio::test]
#[ignore = "billed: calls OpenRouter. Run by hand with HOUSECHECK_EVAL_LIVE=1"]
async fn live_eval() {
    assert_eq!(
        std::env::var("HOUSECHECK_EVAL_LIVE").as_deref(),
        Ok("1"),
        "refusing a billed run without HOUSECHECK_EVAL_LIVE=1"
    );
    let key = std::env::var("OPENROUTER_API_KEY")
        .unwrap_or_default()
        .trim()
        .to_string();
    assert!(
        !key.is_empty(),
        "OPENROUTER_API_KEY is empty; nothing was sent"
    );
    assert!(
        key.is_ascii() && !key.contains(char::is_whitespace),
        "OPENROUTER_API_KEY ({} characters) contains a non-ASCII or whitespace character, which \
         usually means a paste picked up something extra; nothing was sent",
        key.chars().count()
    );
    // Free: reads the key's own metadata, bills nothing. A rejected key stops here rather than
    // failing every conversation with the same 401.
    match reqwest::Client::new()
        .get("https://openrouter.ai/api/v1/key")
        .bearer_auth(&key)
        .send()
        .await
    {
        Ok(r) if r.status() == 401 || r.status() == 403 => {
            let status = r.status();
            let body = r.text().await.unwrap_or_default();
            panic!(
                "OpenRouter rejected the key ({status}, {} characters): {body}; nothing was billed",
                key.len()
            );
        }
        Ok(r) if !r.status().is_success() => {
            eprintln!("key check answered {}; continuing", r.status());
        }
        Ok(_) => {}
        Err(e) => panic!("could not reach OpenRouter for the key check: {e}"),
    }
    let model =
        std::env::var("OPENROUTER_MODEL").unwrap_or_else(|_| DEFAULT_SUMMARY_MODEL.to_string());
    let cap: f64 = std::env::var("HOUSECHECK_EVAL_MAX_USD")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(1.0);
    let only: Vec<String> = std::env::var("HOUSECHECK_EVAL_ONLY")
        .unwrap_or_default()
        .split(',')
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect();

    let version = agent_version(&model);
    let cases: Vec<Case> = load_cases()
        .into_iter()
        .filter(|c| c.kind == "synthetic" && c.live)
        .filter(|c| only.is_empty() || only.contains(&c.id))
        .collect();
    eprintln!(
        "live eval: {} conversations, model {model}, agent {version}, cap ${cap:.2}",
        cases.len()
    );

    let mut r = Report::default();
    let mut spent = 0.0;
    let mut recorded: Vec<Case> = Vec::new();
    let mut failed: Vec<Value> = Vec::new();
    for case in &cases {
        if spent >= cap {
            eprintln!("stopping: ${spent:.4} spent, cap ${cap:.2}");
            break;
        }
        let t0 = std::time::Instant::now();
        let out = run(case, "/agent/chat", Some((&key, &model))).await;
        let before = r.failures.len();
        check(case, "/agent/chat", &out, Mode::Live, &mut r);
        let passed = r.failures.len() == before;
        spent += out.tape.cost_usd;
        eprintln!(
            "{} {:<28} {:>2} rounds  ${:.4}  {:>5.1}s",
            if passed { "PASS" } else { "FAIL" },
            case.id,
            out.tape.calls.len(),
            out.tape.cost_usd,
            t0.elapsed().as_secs_f64()
        );
        // An auth failure says nothing about the agent, and it will fail every case after
        // this one the same way.
        if out
            .tape
            .recorded
            .iter()
            .any(|x| matches!(x["status"].as_u64(), Some(401 | 403)))
        {
            panic!(
                "OpenRouter refused the key mid-run: {}; stopping before the next conversation",
                out.tape.recorded[0]["raw"]
            );
        }
        let msgs = out
            .tape
            .calls
            .iter()
            .rev()
            .find(|c| c.get("tools").is_some())
            .and_then(|a| a["messages"].as_array().cloned())
            .unwrap_or_default();
        let tools: Vec<String> = tool_exchanges(&msgs)
            .0
            .into_iter()
            .map(|(n, _)| n)
            .collect();
        if !passed {
            failed.push(json!({
                "id": case.id,
                "agent_version": version,
                "model": model,
                "status": out.status,
                "tools": tools,
                "answer": out.answer,
                "failures": r.failures[before..],
                "upstream": out.tape.recorded,
            }));
            continue;
        }
        let mut c = case.clone();
        c.kind = "recorded".into();
        c.agent_version = Some(version.clone());
        c.model = Some(model.clone());
        c.upstream = out.tape.recorded.clone();
        c.expect.tools = Some(tools);
        c.expect.upstream_calls = Some(out.tape.calls.len());
        // The wording is the model's now; what it must not say still holds.
        c.expect.answer_contains.clear();
        recorded.push(c);
    }
    eprintln!(
        "live eval: ${spent:.4} total, {} checks, {} failed, {} recorded",
        r.checks,
        r.failures.len(),
        recorded.len()
    );

    let earlier: Vec<Case> = std::fs::read_to_string(RECORDED)
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default();
    let (kept, stale): (Vec<Case>, Vec<Case>) = earlier
        .into_iter()
        .filter(|c| !recorded.iter().any(|n| n.id == c.id))
        .partition(|c| c.agent_version.as_deref() == Some(version.as_str()));
    for c in &stale {
        eprintln!(
            "dropping {}: recorded against agent {:?}",
            c.id, c.agent_version
        );
    }
    let mut merged = kept;
    merged.extend(recorded);
    merged.sort_by(|a, b| a.id.cmp(&b.id));
    if !merged.is_empty() {
        std::fs::write(
            RECORDED,
            serde_json::to_string_pretty(&merged).unwrap()
                + "
",
        )
        .unwrap();
        eprintln!("wrote {RECORDED}: {} recordings", merged.len());
    }
    if !failed.is_empty() {
        std::fs::write(
            FAILURES,
            serde_json::to_string_pretty(&failed).unwrap()
                + "
",
        )
        .unwrap();
        eprintln!("wrote {FAILURES}: {} failing conversations", failed.len());
    }
    assert!(
        r.failures.is_empty(),
        "
{}",
        r.failures.join(
            "
"
        )
    );
}
