# Four Cycles, One Missing Piece

*An evidence-based read of L2 Cycles 1-4, produced by reading the four codebases rather than
by recollection. Resona, SiteAssure, Ziqpu and HouseCheck were each profiled independently
against the same questions — who is this built for, and what in the code proves it — then the
arc across them was traced.*

*Every claim below is grounded in a file, a line, or a measured count. Where the evidence did
not support a tidy story, it says so; the reversal in Cycle 2 is real and is not smoothed over.*

**Measured sizes, first-party code only:**

| Cycle | Project | Rust | Frontend |
|---|---|---:|---:|
| L2 C1 | Resona | 736 | 458 TS |
| L2 C2 | SiteAssure | 551 (+5,184 vendored) | 231 TS |
| L2 C3 | Ziqpu | 6,718 across 7 crates | Rust/WASM UI |
| L2 C4 | HouseCheck | 5,591 across 5 crates | 4,303 TS |

---

Across four projects and roughly 24,000 lines of first-party code, this developer never once built an authentication system, a user record, or any server-side state belonging to a specific person. Not in Resona, not in SiteAssure, not in Ziqpu, not in HouseCheck. Every profile's grep for `login|jwt|session|user_id|tenant` comes back empty of implementation hits. What changed across the four cycles is not *how many users* the code serves — it is *which non-author human she was designing against*, and that shifted axis by axis, with a real reversal in the middle.

## The trajectory, including the reversal

**Resona (C1) had the most ambitious multi-user intent and the least multi-user code.** A `Tier` enum, an `Entitlements` struct enforced at every command entry point, a shipped paywall modal with a 7-row feature matrix, a PRD specifying a genuinely multi-tenant Team plan at $25/user/mo with shared workspaces and admin controls, and success criteria stated as DAU, free→paid conversion, and NPS > 50. Underneath: `AppState { engine: Mutex<Option<Arc<WhisperEngine>>>, stream: ..., tier: ... }`, one process, one person; `start_dictation` explicitly tears down any prior session so concurrency is impossible by construction; `validate_license` is `key.starts_with("PRO-")`, marked DEMO ONLY; zero network calls anywhere.

**SiteAssure (C2) is the reversal.** Multi-user intent goes *down*, not up. "Single device, single user, offline" appears verbatim in the README, the kickoff brief, and the build plan; `schema.sql:11` comments the author column `-- author id (single user in v1)`; the kickoff reasons it out: "A conflict cannot even arise on one device." The Team tier, the conversion metrics, the paywall — all gone. But C2 introduces a genuinely new non-author human: the **verifier**. The SHA-256 hash chain (`entry_hash = sha256(prev_hash + payload_hash)`, append-only, re-walked by `verify()`) exists so an OSHA inspector or insurer can *disbelieve the author and check*, and DATA_POLICY.md designs a redacted export packet for exactly that reader. That is a real audience expansion along a different axis. It is also mostly unbuilt — 551 lines of first-party Rust, four of eight commands returning `Err("not implemented")`, all six React screens returning `null`.

**Ziqpu (C3) expands to installs and contributors, not users.** Twelve release tags, a 3-OS CI matrix, a CLA with a perpetual relicensing grant, DCO enforcement, CODEOWNERS, an MCP server so third-party hosts can drive the loop, and a `datasets/` scaffold with templates that two teammates actually used to land aviation and insurance sets. The item axis genuinely scales: Postgres 16 with a pooled 5 connections, 5,271 tickers, a 69,458-city gazetteer. The user axis does not: no auth, no rate limits, no pagination, one `ANTHROPIC_API_KEY` per process, hardcoded localhost, and a literal `demo_seeker()` with one birth moment in code. The tell is `synastry_readings`, keyed `PRIMARY KEY (user_chart_hash, choice_ticker)` — the only genuine multi-user data structure in three cycles — and **no code ever writes to it**, because the sidecar is architecturally read-only. It binds `0.0.0.0:8787` with nothing in front of it.

**HouseCheck (C4) is the first project whose request path was written against strangers.** The proof is specific and it is in the comments. The hand-rolled `RateLimiter` (10 req/60s, keyed off `Fly-Client-IP`) carries the reasoning: "/agent/chat is the first endpoint here that costs real money per request, so an unlimited public endpoint is a way for a stranger to run up the bill" — note *the first endpoint here*, she knows it is a first. `ConcurrencyLimitLayer::new(64)` answers a question that does not arise for one user, and the comment records that `tower_governor` was evaluated and rejected for a stated reason. `client_key` admits its own limits: "this is a spend guard, not an authentication boundary." The startup guard refuses to boot on an empty artifact "rather than serve a 404 for every address under a green health check" — an operator's worry about other people's requests. And correctness was finally evaluated at population scale: the Socrata paging bug was fixed and measured across all 250 buildings (mean score 69.5 → 63.0, 72 changed band), not spot-checked on one.

## What was inherited

Resona → SiteAssure is code inheritance done badly: `wisper-core` is vendored wholesale at 5,184 of 5,735 Rust lines (90%) carrying 53 of 53 tests, and the kickoff's own pruning list was ignored — `fetch/` with `download_url` and `check_for_update` is still `pub use`-exported inside a project whose first non-negotiable is zero egress. After that, code inheritance stops entirely. Ziqpu and HouseCheck both start from clean genesis commits. What carries forward instead is *practice and architecture*: SiteAssure's CI posture (clippy `-D warnings`, gitleaks, cargo-audit, phase gates) becomes Ziqpu's and HouseCheck's; Ziqpu's read-only-server-over-a-baked-artifact becomes HouseCheck's read-only SQLite in the Docker image; Ziqpu's measure/interpret firewall becomes HouseCheck's `scoring` crate with chrono deliberately absent from the workspace so no scoring path can read wall time; Ziqpu's `NOT_ADVICE` constant and buy/sell/hold refusal become HouseCheck's 9-domain law allowlist and injection defense; Ziqpu's NOTICE becomes HouseCheck's `/meta` provenance endpoint.

## The constant

In all four projects, the multi-user affordance is **modeled, annotated, and left inert** — and she is right about it every time. Resona's `licensing.rs` and ADR-005 both state that client-side gating is bypassable and that revenue needs an Ed25519-signed server entitlement, then assign it to P2. SiteAssure carves `status` and `role` into the schema labeled "v2 hook" and no code reads them. Ziqpu builds the `user_chart_hash` key and never writes the table. HouseCheck's `store.ts` header says "no accounts — design decision #3," and its single `Arc<Mutex<Connection>>` carries the tradeoff in a comment. Paired with a second constant — zero telemetry in Resona, "none" in SiteAssure's DATA_POLICY, zero in HouseCheck — this is the real finding: **the first non-author human she models is always an auditor, an attacker, or a cost risk; the customer only ever appears in a document.** The market research (Resona's Team tier, HouseCheck's 91,918-building ARR table) is consistently more populous than the code.

Said plainly: four cycles, four correct diagnoses of the same missing piece, zero implementations. The capstone reached *many concurrent anonymous clients*, which is a genuine step and a well-engineered one — but a read-only database baked into an image cannot hold a user, and localStorage is not an account. She got to no-user-at-scale, not to multi-user.