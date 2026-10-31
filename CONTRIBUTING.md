# Contributing to HouseCheck

Thanks for helping. HouseCheck turns NYC public records into a building health card a renter
can check before signing. Everything here leans on one rule: **every claim the product makes
has a source, and where the data can't support a claim, the product says so.** Contributions
are held to the same rule.

## Claiming an issue

1. Pick an open issue. Issues labelled `good first issue` are scoped to be done without
   knowing the whole codebase.
2. Comment on it to say you'd like to take it. Wait for a maintainer to assign it to you
   before starting, so two people don't build the same thing.
3. One issue at a time. If you go quiet for two weeks, the issue may be reassigned.
4. Something not covered by an issue? Open one first and describe the change. A short
   discussion is cheaper than a rewritten PR.

## Build and run

You need a stable Rust toolchain and Node 20 (frontend only).

```bash
# Build a small database from the built-in fixture: no keys, no network
cargo run -p ingest -- --fixture --out data/housecheck.db

# Run the API on it
HOUSECHECK_DB=data/housecheck.db cargo run -p api
curl http://127.0.0.1:8787/building/3000010001
```

Real data needs a free [Census key](https://api.census.gov/data/key_signup.html) and, optionally,
an NYC Open Data app token, both read from the environment. See the README's "Configuration and
secrets" table.

Frontend:

```bash
cd frontend
npm ci
npm run dev
```

The frontend talks to the live API by default. To use your local API, set
`VITE_API_URL=http://127.0.0.1:8787` (see `frontend/.env.example`).

## Checks your PR must pass

CI runs these on Linux, macOS and Windows. Run them locally first:

```bash
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace --locked
```

```bash
cd frontend
npm run lint
npm run build
```

`cargo test` never calls a paid API. The live agent evals are `#[ignore]`d and refuse to run
without `HOUSECHECK_EVAL_LIVE=1`; you don't need to run them, and a maintainer re-records them
when an agent change needs it.

## What a good PR looks like

- **Small and about one thing.** Open it against `main` and link the issue (`Closes #12`).
- **Tests for logic.** A branch, a parser, a score: leave one test that would fail if it broke.
- **Measurements over adjectives.** If a PR says something is faster or smaller, the
  description gives the number and how it was measured.
- **Honest scope.** If the change covers only part of the city or part of a dataset, say which
  part, in the code comment or doc it touches.
- **Deliberate shortcuts are marked.** A simplification with a known limit gets a
  `ponytail:` comment naming the limit and when to lift it.

## Data and law

- **Building data** comes from the public datasets listed in the README. Never hand-edit a
  number in the database; change the ingest that produces it.
- **Law text** (`data/law.json`) is copied from the official source and each row records its
  source URL, retrieval date and who verified it. Changes to law rows need the official
  source linked in the PR, and a maintainer verifies them before merge. HouseCheck states
  information, not legal advice; wording that tells a renter what they should do legally will
  not be merged.
- **No secrets in the repo.** API keys stay in your environment. CI scans every commit with
  gitleaks.

## License

HouseCheck is licensed under the [Apache License 2.0](LICENSE). By submitting a contribution,
you agree it is licensed under the same terms (section 5 of the license).

## Conduct

Everyone taking part follows the [code of conduct](CODE_OF_CONDUCT.md).
