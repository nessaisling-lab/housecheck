# Eval card

**What this is:** a map of verification that already exists, written down so it is legible
to someone skimming the repo. It is documentation, not a new test format. Every test named
here runs today under `cargo test --workspace` — **162 tests, clippy clean, 8 crates.**

**What this is not:** a synthetic prompt suite. The strongest evidence in this project was
not produced by an eval harness, and saying otherwise would be the kind of claim the
product itself refuses to make.

---

## Golden example — 603 Putnam Avenue

The building the honest-design decisions were made for. An 1899 walk-up with a long
violation record and effectively no repair history.

| | Expected | Where it is checked |
|---|---|---|
| Score | **27 / 100** | `card::tests::a_covered_bbl_produces_a_scored_card` (shape), pillar arithmetic in `scoring::tests::*` |
| Pillars | condition **0**, legal **60**, neighborhood **60**, accessibility **30** | `scoring::tests::total_is_weighted_sum_rounded`, `accessibility_walkup_lowrise_is_higher` |
| Open violations | **11 A · 22 B · 0 C** (33 total) | `model::tests::counts_only_open_violations_by_class` |
| Repair speed | **"nothing closed since 2023"** — not a zero, not blank | `model::tests::a_building_that_closes_nothing_says_so_rather_than_going_blank` |
| Coverage limit stated | every response | `mcp` provenance line, read from the artifact's `meta` |

**Why this building.** Under a two-state repair-speed metric it rendered *blank*, because
it has 33 open violations and exactly one closure in its entire record — dated 2017-10-18.
Blank made the landlord who fixes nothing look **emptier** than one who fixes things
slowly. The third state exists because of this building. **26 of 250** pilot buildings sit
in the same shape.

**The state machine, tested explicitly:**

- `a_building_that_closes_nothing_says_so_rather_than_going_blank` — `NothingClosed`
- `too_few_closures_produce_no_median` — absent rather than a median from a sample of two
- `a_closure_dated_before_its_issue_is_discarded_not_counted` — bad data drops out
- `one_ancient_closure_does_not_move_the_median` — median, not mean, on purpose
- `each_state_is_distinguishable_on_the_wire` — the three states cannot collapse in JSON

**Reproduce it live, two ways, one card:**

```bash
curl -s https://housecheck-nessa.fly.dev/building/3016440063        # HTTP
HOUSECHECK_DB=data/housecheck.db cargo run -p mcp                    # MCP, over stdio
```

Both read `crates/card`. That crate exists so an agent and the website cannot report
different scores for the same building — which would be the quiet failure, since each
answer would be internally consistent.

---

## Adversarial case — the re-signed forgery

**The document that passed.** A forger rewrites a violation row to read *"NO VIOLATIONS OF
ANY KIND AT THIS ADDRESS"*, recomputes the entire hash chain over the altered row, and
signs the new chain head with **their own keypair**. Every check inside the document
passes, because the forger computed every one of them. It verified as `SIGNED AND INTACT`.

**This was found in production, not in a test.** An independent verifier written in Python
— different language, different crypto library, the document as its only input — was built
to confirm the export worked, and instead found that signing alone establishes nothing
about origin. The fix was publishing the public key at `/meta`, so a reader has something
to compare against that did not travel with the document.

| Attack | Expected | Test |
|---|---|---|
| One character edited | tampered, row located | `export::tests::one_edited_character_is_detected` |
| Row's own hash recomputed | still tampered | `recomputing_a_rows_own_hash_does_not_rescue_it` |
| Two rows swapped | chain link breaks | `reordering_rows_breaks_the_chain` |
| A row deleted | chain link breaks | `deleting_a_row_breaks_the_chain` |
| Signed with the wrong key | signature fails | `signing_round_trips_and_a_wrong_key_fails` |
| No key configured | **no signature**, not a fake one | `a_missing_key_produces_no_signature_rather_than_a_fake_one` |
| **Whole chain rebuilt and re-signed** | **rejected — unknown key** | `mcp::tests::a_re_signed_forgery_is_rejected_even_though_it_is_internally_consistent` |
| Signed, but no published key to compare | **inconclusive**, never "verified" | `without_a_published_key_the_answer_is_inconclusive_rather_than_verified` |

**The test design point.** The forgery test first asserts the forged document **passes
`verify()`** — `SignedAndIntact` — before asserting the tool rejects it. Without that
assertion the rejection could come from a broken chain and the test would prove nothing
about the key comparison it exists to cover. An earlier attempt did exactly that: mutating
an already-built document only yields `Tampered`, so it never exercised the interesting
path at all. A real forger rebuilds the chain, and the test now does.

**Four outcomes plus a refusal**, in `crates/mcp`: `TAMPERED`, `INTACT BUT UNSIGNED`,
`VERIFIED`, `REJECTED — SIGNED BY AN UNKNOWN KEY`, and `INCONCLUSIVE` when no published key
is configured. That last one is not a verdict, and it matters: claiming verification
against a key that arrived inside the document is precisely the hole this whole section
describes, reintroduced somewhere new.

---

## Agent behaviour

The `/summary` assistant is where a wrong answer would be most persuasive, so what it is
*forbidden* to do is tested rather than prompted.

- `system_prompt_forbids_advice_and_outcome_prediction` — it never predicts case outcomes
- `citations_only_claim_sources_that_were_actually_used` — citations list what fed the
  answer, not a hardcoded line
- `grounding_block_states_when_there_is_no_median_instead_of_omitting_it` — a missing
  figure is stated, not silently dropped
- `law_search_domains_are_authoritative_only` — legal lookups cannot wander off .gov/.org
- `legal_help_directory_entries_are_actionable_and_free` — no lead-generation sites in
  front of someone in a housing crisis
- `tool_schemas_declare_every_tool_with_a_usable_description`

Bounded, too: `a_retried_round_can_never_outlive_the_budget_that_scheduled_it`,
`the_server_gives_up_before_the_client_does`, and a rate limiter with three tests.

---

## What this does not cover

- **No LLM output evals.** There is no scored suite over generated prose. The agent is
  constrained by architecture and by the tests above; the *quality* of its wording is
  unmeasured.
- **The Python verifier is not in CI.** It is a one-off script, and it remains the
  strongest evidence here precisely because it was written independently. The Rust test
  reproduces its finding in-process; it does not replace the outside check.
- **Coverage is 250 buildings.** Every figure above is true of one Brooklyn community
  district, roughly 0.1% of the city.
- **Nobody has dialled the legal-help numbers.** They are verified against each
  organisation's own published page and re-checked 2026-08-12; that establishes the pages,
  not that a human answers.

---

## Running it

```bash
cargo test --workspace                              # 162 tests
cargo clippy --workspace --all-targets -- -D warnings
```

*Written 2026-08-12. Figures re-checked against production the same day.*
