# Build prompts — HouseCheck Tenant Agent

**How to use this file.** Each section below is one work session. Copy the whole **Prompt** block into your AI assistant, along with the rules from `docs/agent/LLM-RULES.md` §2. Work through it, ship it, then come back for the next one.

Do them in order. Each one assumes the last one is merged and working.

**Before every session**, tell your assistant:

> Read `docs/agent/LLM-RULES.md` and follow it for this whole session. I am learning, not just shipping. Explain before you write, hand me the decisions, cite file:line.

**A note on why the prompts are written this way.** Each one starts by telling you what you're about to learn and why the step exists, then gives the assistant a narrow task with explicit boundaries. The boundaries matter as much as the task — "do not add tools yet" prevents your assistant from helpfully building three sessions of work at once and leaving you behind.

---

## Session 0 — Clear the ground

**What you'll learn:** how to read a git branch you didn't write, and why fixing honesty bugs comes before adding features.

**Why this exists:** Anthony has newer work on `origin/anthony-frontend` @ 9ee82e7 that rewrites 67 lines of the exact file you're about to change. Starting before that lands means doing the work twice. And four known bugs make the app say things that aren't true — those get worse, not better, once a real language model is generating the text.

> **Prompt**
>
> I'm working in the HouseCheck repo. Before I add an agent feature, I need to clear the ground.
>
> First, help me understand the situation. `origin/anthony-frontend` is at commit 9ee82e7 and is not merged into `main`. Show me what it changes in `frontend/src/components/AgentSheet.tsx` and explain, in plain language, whether merging it will conflict with adding a real network call to the `send()` function. Do not merge anything yet — explain first, then let me decide.
>
> Then, walk me through these four bugs one at a time. For each: show me the code, explain what goes wrong from the user's point of view, and let me write the fix myself. Do not fix them for me.
>
> 1. `frontend/src/components/AgentSheet.tsx:85` — `getSummary` returns a `source` field that says whether the data is live or demo, but this line throws it away. Result: when the backend is disabled, mock text renders under the confident label "Source: HPD · DHCR · Census B25064."
> 2. `frontend/src/lib/api.ts:108` sets `rent: raw.rent ?? null`, but the backend's `HealthCard` struct (`crates/model/src/lib.rs:107-113`) has no `rent` field at all. So `rent` is always null on live data, and the "Negotiate the rent?" chip always hits its fallback.
> 3. `frontend/src/lib/api.ts:31` aborts requests at 8 seconds, but the backend allows the language model 20 seconds (`crates/api/src/main.rs:530`). A slow success gets killed by the client.
> 4. `frontend/src/components/AgentSheet.tsx:233-242` — the attach button has no `onClick` and does nothing.
>
> For bug 1, before we touch anything, ask me what I think the fix should look like and why it matters for this product specifically.

**Done when:** Anthony's work is merged or explicitly rejected, all four bugs fixed, tests pass, one commit per bug.

---

## Session 1 — Make the model configurable

**What you'll learn:** environment variables, why secrets never live in code, and what "zero data retention" means.

**Why this exists:** the model name is currently a hardcoded Rust constant ending in `:free`. Free-tier OpenRouter logs your prompts. Our prompts contain a person's home address and what they pay in rent. Our own legal audit bars free tier from production. Because it's a compile-time constant, you cannot fix this with configuration — the code has to change first.

> **Prompt**
>
> In `crates/api/src/main.rs:424-427` there are two constants: `OPENROUTER_URL` and `SUMMARY_MODEL`. The model is hardcoded to a `:free` tier model, which logs prompts. We need it configurable.
>
> Before writing code, explain to me: (a) why a compile-time constant can't be changed by a deploy setting, (b) what an environment variable is and where it comes from in a Fly.io deployment, and (c) what "zero data retention" means and why it matters when the prompt contains someone's address.
>
> Then help me change it so the model comes from an `OPENROUTER_MODEL` environment variable with a sensible default. I want to write the fallback logic myself — explain the trade-off between failing loudly when it's unset versus falling back to a default, and let me choose.
>
> Also: `crates/api/src/main.rs:469` reads `OPENROUTER_API_KEY` from the environment on every single request. Explain why that's wasteful and show me where `AppState` is defined (`main.rs:36-45`) so I can move it there. Let me write that change.
>
> Do not add any new endpoint in this session.

**Done when:** `OPENROUTER_MODEL` works, the key is read once into `AppState`, existing tests still pass, `docs/API.md` env table updated.

---

## Session 2 — A conversational endpoint, no tools

**What you'll learn:** how a multi-turn conversation is actually represented, and why you must cap history.

**Why this exists:** `/summary` takes only `{bbl}` and returns one paragraph. There's no way to ask a follow-up. This session adds an endpoint that accepts a conversation. No tools yet — one new concept at a time.

> **Prompt**
>
> I'm adding `POST /agent/chat` to the Rust backend in `crates/api/src/main.rs`.
>
> Start by explaining how a chat conversation is represented in an LLM API call — what a "messages array" is, what the `role` field means, and why the whole history gets resent on every turn. Then explain what that implies for cost as a conversation grows.
>
> Read the existing `summary_handler` (`main.rs:446-576`) with me and point out exactly which parts I can reuse. I specifically want to reuse the grounding block at `main.rs:489-515` — explain what "grounding" means and why that block is the thing that keeps the model honest.
>
> The new endpoint takes `{ bbl, messages: [{role, content}] }` and returns `{ answer, citations }`.
>
> Three limits need to exist: `MAX_TOKENS`, `MAX_HISTORY_MESSAGES`, and a request timeout. Explain what each one protects against, then **let me pick the numbers and write those constants myself.**
>
> The system prompt is the most important part. The current one is a single sentence at `main.rs:420-422`. Explain what a system prompt does, then let me write the new one. It must cover: only use facts provided; never give legal advice; say "I don't have that" rather than guessing; never speculate about individuals. Review what I write and tell me what's missing and why — don't rewrite it for me.
>
> Do not add tool calling in this session.

**Done when:** the endpoint works end to end with a real key, refuses to exceed limits, and there's at least one test.

---

## Session 3 — Wire the frontend

**What you'll learn:** replacing a fake implementation with a real one, and keeping graceful degradation.

**Why this exists:** `AgentSheet.send()` is a `setTimeout` that returns canned text. Type "when was the boiler inspected?" and you get the "Explain this score" answer, because `CHIPS.includes(t)` is false and the code falls through to `CHIPS[0]`. This is the first slice a user can feel.

> **Prompt**
>
> Read `frontend/src/components/AgentSheet.tsx:121-140` with me and trace exactly what happens today when a user types a question that isn't one of the three canned chips. Walk the code path line by line so I can see why the answer comes back mismatched.
>
> Now help me replace it with a real call to `POST /agent/chat`. First add a `sendChat` function to `frontend/src/lib/api.ts` — study the existing `getSummary` (`api.ts:188-202`) and explain the pattern it follows, especially how it falls back to demo data and returns a `source` field.
>
> Important design decision, and I want to make it: `answerChip` (`AgentSheet.tsx:16-48`) is hand-written canned text, but it's genuinely grounded — every number comes from the real card. Should we delete it or keep it as the offline path when the API key isn't set? Give me the argument both ways, then let me decide and write it.
>
> The conversation history currently lives in `useState` at `AgentSheet.tsx:73` and is never sent anywhere. Explain what has to change for the backend to see it, and let me write that part.
>
> Also make sure the `source` field is actually used this time, so demo answers are labeled as demo. That was bug 1 in session 0 — explain how this session could reintroduce it if we're careless.

**Done when:** a free-text question reaches the backend and returns a grounded answer; with no API key the offline path works and is labeled demo.

---

## Session 4 — Tool calling, read-only tools

**What you'll learn:** the single most important concept in this build.

**Why this exists:** until now the model only sees a fixed block of facts. Tools let it *ask* for data it needs. We start with three read-only tools so a bug in the loop can't corrupt anything.

> **Prompt**
>
> This session is about tool calling. Before any code, explain it to me properly:
>
> - What actually gets sent to the model when you offer it tools (the JSON schema)
> - What comes back when the model wants to use one
> - Who executes the function — the model or my code — and why that distinction is the whole point
> - How the result gets back into the conversation
> - Why this loop can repeat, and what happens if it never stops
>
> Then ask me to explain it back to you before we write anything. Correct me where I'm wrong.
>
> Now help me implement the loop in `crates/api/src/main.rs` with three read-only tools: `get_building(bbl)` wrapping `card_for()` (`main.rs:145-181`), `get_open_violations(bbl)` wrapping the existing store function (`main.rs:154`), and `search_address(query)` wrapping the existing `/search` logic.
>
> All three already exist as working code. Explain what a "tool schema" has to describe and why the description text matters so much for whether the model picks the right tool.
>
> `MAX_TOOL_ITERATIONS` — explain what happens without it, then let me write the guard.
>
> Every fact in the final answer must be traceable to a tool result. Explain how citations should flow from the tool results into the `citations` array, and let me implement that.

**Done when:** asking "what violations does this building have?" triggers a real tool call and returns real rows with citations, and the iteration cap is tested.

---

## Session 5 — Comparison and priorities

**What you'll learn:** turning a prototype interaction into a production feature backed by real scoring.

**Why this exists:** the `jagger-agent` branch has a genuinely good idea — rank what matters to you, then compare buildings weighted by that ranking. The interaction design is the valuable part. The scoring behind it must come from `crates/scoring`, not from the MVP's own copy, which disagrees with the backend (a two-story walk-up scores 25 there and 75 in Rust).

> **Prompt**
>
> Read `mvp/src/components/CompareAgent.tsx:338-378` on branch `origin/jagger-agent` — the `RankPicker` component. Explain to me what makes this interaction good on mobile compared to drag-and-drop reordering.
>
> Then read `mvp/src/lib/compare.ts:157-174`, the weighted-average logic. Explain the `weight = n - index` formula in plain arithmetic with a worked example for 4 priorities.
>
> Now the important part. `mvp/src/lib/score.ts` computes scores that **disagree** with `crates/scoring/src/lib.rs` — different violation penalties, different access thresholds, no guard against the Census `-666666666` sentinel value. Explain to me why shipping two scoring engines is worse than shipping one with a bug.
>
> Help me port the ranking UI into the React frontend, keeping the interaction and discarding the scoring. Add a `rank_by_priorities` tool that calls the Rust scoring crate.
>
> Open question I want to decide with you: should ranking be a tool the model calls, or a plain UI flow that skips the model entirely? A form can't hallucinate and costs nothing. Give me both arguments before I choose.
>
> Also lift the hedged copy from `mvp/src/lib/compare.ts:200-226` — strings like "Likely rent-stabilized (best-available public signal — confirm unit)." Explain why that phrasing fits this product.

**Done when:** a user ranks priorities and gets a comparison scored by the Rust crate, matching the health card exactly.

---

## Session 6 — Legal help referrals

**What you'll learn:** where a software product must stop, and why verification is a task rather than an assumption.

**Why this exists:** someone asking about a lawyer is often in a real crisis. Open web search for attorneys surfaces lead-gen sites and scams that target exactly that desperation. A curated list of established nonprofits is reliable, free to the user, and keeps us on the right side of referral versus legal advice.

> **Prompt**
>
> I'm adding a `find_legal_help(borough, issue_type)` tool backed by a curated JSON file, not a web search.
>
> First explain to me the difference between giving a referral and giving legal advice, and why the second one is a liability for a software product. Give me concrete examples of a sentence that's fine and a sentence that isn't.
>
> Help me design the JSON shape. My starting fields: `name`, `url`, `phone`, `boroughs[]`, `issue_types[]`, `free`, `verified_on`. Ask me why `verified_on` might be the most important field in the record.
>
> The seed list is in `docs/agent/PRD-AGENT.md` §6.1. **Every URL and phone number must be checked by a human before this ships.** Help me build a checklist, but do not fill in the values — I'm verifying them myself, the same way we verified the statistics in our data-integrity ledger.
>
> Then help me write the system-prompt language that makes the agent refer rather than advise. Let me draft it; tell me what's missing.

**Done when:** every entry verified with a date, the tool returns correct results by borough and issue, and the agent refuses to give legal advice when tested.

---

## Session 7 — Web search, last and carefully

**What you'll learn:** prompt injection, and why this tool is deliberately built last.

**Why this exists:** everything until now used data we control. Web search introduces text written by strangers into the model's context — and some of that text is written specifically to hijack agents.

> **Prompt**
>
> Before any code, explain prompt injection to me with a concrete worked example: a web page containing hidden text that tries to override the agent's instructions, and what the model sees when that text lands in its context window next to my system prompt.
>
> Then explain honestly: is there a complete fix? If not, what do the partial mitigations actually buy us?
>
> Help me implement `web_search(query)` with these requirements, and explain what each one defends against:
> - All fetched content wrapped in explicit delimiters marking it untrusted
> - System prompt states that delimited content is data to summarize, never instructions to follow
> - No user-identifying information ever placed in a search URL
> - Results are summarized, never executed or acted upon
> - A tool result may not silently trigger another tool call
>
> Then help me write a test where the fetched content contains an injection attempt, and verify our agent ignores it. I want to see it fail first without the mitigations, then pass with them.

**Done when:** the injection test passes, and you can explain to a teammate why each mitigation exists.

---

## If you get stuck

Stuck for more than 30 minutes on the same thing means the step was too big. Say so — the fix is to split the session, not to have someone hand you the answer. `docs/agent/LLM-RULES.md` §4 lists the situations where you should ask a person rather than an AI.
