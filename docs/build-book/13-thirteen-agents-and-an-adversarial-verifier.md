# Chapter 13 — Thirteen Agents and an Adversarial Verifier

> **The question this chapter answers:** How was this book researched, how often was
> the research wrong, and what does that say about the twelve chapters before it?

---

## 1. Starting with a correction to the chapter's own title

I announced this chapter as *"Thirteen Agents and an Adversarial Verifier,"* and
described the research elsewhere as producing **174 confirmed findings against 27
refuted**.

I cannot verify either number. The workflow journals are gone — the session
scratch directories survive, the run records do not. What survives is
`docs/build-book/OUTLINE.md`, 313 lines, committed. Its front matter states the
evidence position precisely:

> the domain/scoring chapters (2–7, 13) rest on an independently re-verified map —
> **29 of 34 checkable assertions confirmed** verbatim at cited lines, **five
> refuted** and carried below as corrections. The storage/ingest chapters (8–12)
> rest on a map that WAS independently verified — **24 assertions confirmed, 6
> refuted** — but the synthesis step that produced this outline received a
> truncated copy of that verdict, so chapters 8–12 should be re-read against the
> full storage verdict before drafting.

**Two maps. 64 checkable assertions. 53 confirmed, 11 refuted.** That is the number
this book's research actually rests on. "174 and 27" is a figure I carried forward
in a summary and repeated without an artifact behind it.

Beginning the methodology chapter by correcting the methodology chapter's own
headline number is not a rhetorical device. It is the third-most-embarrassing thing
in this chapter and it is exactly the failure mode Chapters 3, 5 and 10 documented
in the codebase: a claim, restated across contexts, drifting from its source
because nothing along the way could check it against the original.

The title stays. The number is 53 of 64.

## 2. What the maps got right

The maps were, where they quoted, precise. The outline cites
`crates/ingest/src/run.rs:295-297` for a claim about full-rebuild ingest. Reading
those exact lines today:

```rust
let _ = std::fs::remove_file(&cfg.out);
let conn = store::open_db(&cfg.out)?;
store::migrate(&conn)?;
```

Exact. The line numbers resolve, the code says what the map said it says, and the
observation that the delete's error is discarded is correct. The same held for
every citation I spot-checked while drafting: `model:71` for the stabilization doc
comment, `scoring:78-85` for the weights, `sources.rs:204` for the Census URL.

The verification pass also worked. Three corrections it produced are carried in the
outline's own text and all three are real:

- *"the earlier framing of 'two independent `Option`s give six representable
  combinations' is wrong… Six is only reachable by collapsing `Some(n>0)` and
  `Some(0)` — which is the exact distinction the invariant exists to protect. The
  point survives; the arithmetic did not."*
- *"that doc comment is at `model:71`, not `:72` — line 72 is the field itself."*
- *"'every scoring function ends in a defensive clamp' is false. Two of the six have
  no clamp and no cast at all."*

That last one became Chapter 2's Leak 3. An adversarial pass that overturns a
claim which had already been written into a plan is doing its job.

## 3. What they got wrong, and the pattern in it

Outline Chapter 8 is titled:

> **One Writer, One File, Read-Only Forever**

and its first beat reads *"a batch ETL binary writes the file once; **the API opens
it read-only**."*

`crates/store/src/lib.rs:15`:

```rust
let conn = Connection::open(path)?;
```

No flags. Read-write, create-if-missing. Chapter 7 demonstrated the consequence by
running it: point the binary at a path that does not exist and it creates a 36 KB
empty database, serves `/health` → `ok`, and 404s every building with no warning.

**A chapter title asserted a property the code does not have.**

Now the pattern, which is the useful part. Where did "read-only" come from? The
`Dockerfile`'s first line:

> The serving DB is a **read-only artifact** baked into the image

A comment. The map read a comment describing intent, and reported it as a property
of the code. Nothing in the output format distinguished that inference from the
verbatim transcription of `run.rs:295-297` two beats later — both arrived as
sentences in a bulleted list, at identical confidence.

So the research process failed in precisely the way this book says the codebase
fails. **Prose drifted from code, and the drift fooled the auditor rather than the
maintainer.** The thesis ate its own method. I would rather report that than
discover a reviewer had noticed it first.

Second error, smaller and checkable: outline Chapter 13 was titled *"What Thirteen
Tests Buy,"* scoped in its beats to *"13 in the scored core (1 in `model`, 12 in
`scoring`), plus 10 in `store`."* The scoped claim is nearly right — `model` 1,
`scoring` 12, `store` **11**, not 10. The title generalises 13 into a book chapter
about a workspace that has **111 tests**, of which 59 are in `api` and 28 in
`ingest` — neither mentioned.

## 4. The truncation, and the thing it actually cost

The outline flagged its own weakness in the front matter and repeated it inline
before Part III:

> Drafting note: chapters 8–12 rest on the storage/ingest map, which… arrived
> truncated. Re-verify every line citation in this part before drafting.

The warning was correct. What it got wrong was the *kind* of error to expect. It
predicted stale line citations. The actual cost was the subject matter.

| outline planned | what the chapter became |
|---|---|
| 8 — One Writer, One File, Read-Only Forever | 8 — The Ingest Nobody Runs Twice |
| 9 — Migrations Without a Version Number | 9 — Eight Tools and a System Prompt |
| 10 — The Join Key Is the Product | 10 — The Frontend That Assumes Nothing |
| 11 — Geometry Once, Never at Request Time | 11 — Accessibility as a Correctness Property |
| 12 — Failure Policy Is Editorial Policy | 12 — What the Tests Actually Test |

The plan for a product whose API crate is **60% LLM agent code by line** contained
**no agent chapter**. For a product with a 4,297-line React application and a
published WCAG 2.2 AA conformance claim, it contained **no frontend chapter and no
accessibility chapter**. It had five chapters on storage internals — join keys,
geometry caching, migration versioning — which are real topics and none of which
turned out to be where anything was wrong.

And the single largest finding in this book — that the shipped artifact holds
**50.4% of the violations HPD actually has**, biasing every score upward — appears
nowhere in the outline. Not as a chapter, not as a beat, not as a risk. The map
described the ingest pipeline's structure accurately and never asked what came back
from it.

## 5. My own error rate, drafting chapters 4 through 12

Stated plainly, because a chapter about research honesty that only audits the
agents would be doing the thing it criticises.

- **Chapter 6.** I filtered a grep with `^2[0-9][0-9][0-9]:` intending to exclude the
  test module at line 2083, and excluded everything from line 2000. That hid
  `/summary`'s rate-limit check at `:2011`, and I spent a step believing I had found
  an unguarded path to a paid model. I had not. The spend guard is symmetric and
  correct.
- **Chapter 8.** I computed band movement with four bands at 80/60/40 — thresholds I
  invented. The product has five, in `frontend/src/lib/score.ts:39-46`. Recomputed
  with the real ones before publishing. The headline held at 70 of 250, but I did
  not know that when I wrote the first version.
- **Chapter 4.** I predicted the neighborhood pillar was largely a population proxy
  — more units nearby, more complaints. The data refuted it: `r = -0.196` against
  `units_res`, and negative. I published the refutation because it was the honest
  result, and because a book that only reports confirmed hypotheses is not reporting.
- **Chapter 6 again.** A keyword classifier misfiled several tests between the REST
  and agent surfaces; corrected by hand.
- **Chapter 12.** A per-crate test count using `grep -c` piped into `awk -F:`
  returned zeros for every crate, because `grep -c` omits the filename prefix when
  given one file. Caught because "model: 0 tests" was obviously wrong.

Five errors across nine chapters. Four were caught by the result looking wrong; one
was caught by checking a threshold I had assumed. None were caught by a tool.

## 6. What actually produced the findings

Worth tabulating, because it is the chapter's conclusion:

| finding | how it was found |
|---|---|
| 311 truncation (Ch. 4) | `curl` count against NYC's API |
| HPD truncation, 50% loss (Ch. 8) | `curl` + matching 29,115 groups back to BBLs |
| scores overstated 6.5 pts (Ch. 8) | recomputing 250 buildings in Python |
| empty DB serves 200 OK (Ch. 7) | launching the release binary at a missing path |
| class I excluded, 187 open (Ch. 5) | `$group` query against HPD |
| composited contrast fails AA (Ch. 11) | compositing the gradient by hand |
| dead `!= "none"` branch (Ch. 9) | grep for a string literal |
| `?? 0` renders "clean record" (Ch. 10) | reading two files |

**Six of the eight required executing something.** Not one came from the maps. The
maps told me the ingest had five `$limit` sites; they did not, and structurally
could not, tell me that one of them was dropping half the data — because that
requires asking a server a question, and a code-reading agent reads code.

---

## The hardest question a reader can ask of this chapter

> *"You used AI agents to audit a codebase, the agents were wrong often enough to
> put a false claim in a chapter title, and you are the same kind of system. Why
> should anyone trust any of the twelve chapters before this one?"*

They should not trust them. They should check them, and the book was built so that
they can.

Every load-bearing claim in this book has a command behind it. The 219,199 matching
311 rows and the 134,837 matching HPD rows are `curl` invocations against a public
API that anyone can re-run. The 50.4% coverage figure is a `$group` query matched
against BBLs from the shipped database. The 6.5-point score bias is a
reimplementation of `condition_score` — which is possible only because Chapter 1's
architecture made it a pure function, and Chapter 2's made the year an argument
instead of a clock. The empty-database behaviour is three `curl`s against a binary
you can build. The contrast shortfall is `0.94 × 64 + 0.06 × 215 = 73` and a
luminance formula.

That is a different epistemic status from "an agent reported this," and the
distinction is the whole point of the chapter. The maps were **scaffolding** — they
were fast, they were mostly precise on quotation, and they told me which files
mattered. They were wrong exactly where they inferred intent from prose, which is
the failure mode this codebase has at every layer and which I should have expected
in the research and did not.

Three things I would do differently, and would tell anyone attempting this:

1. **Separate transcription from inference in the output format.** A map that
   printed quoted code differently from concluded properties would have shown
   "read-only forever" as an inference from a Dockerfile comment, and it would have
   been checked in seconds.
2. **Never let a synthesis step run on truncated evidence.** The outline knew it had
   been truncated and said so, which was good. It still produced a five-chapter plan
   for the wrong subjects, and that cost was invisible until the chapters were
   actually written.
3. **Budget for execution, not just reading.** The findings that changed what a
   tenant is told all came from running something. A research process weighted
   toward reading code will produce a book about code structure, and this codebase's
   structure is largely fine. Its problem was never in the code.

The last one is the finding under the finding. Twelve chapters of careful reading
found a stale doc comment, a dead branch, and some stringly-typed fields. One
`curl` found that half the data is missing.

---

*Next: **Chapter 14 — Honesty Is a Type-System Problem.** The whole argument, one
claim, and the ledger of everything this book says should change.*
