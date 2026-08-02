# Chapter 10 — The Frontend That Assumes Nothing

> **The question this chapter answers:** How does the React layer handle a backend
> that may be absent, slow, or wrong — and what does its defensiveness hide?

---

## 1. Sixty percent of it is not the app

```
frontend/src                    10,380 lines
  app code                       4,297
  components/ui                  6,083   (53 files, shadcn scaffold)
```

I checked how many of those 53 scaffold components the application imports.

**Zero.**

Not "a few unused." None. Every page and component in this product is hand-written:
`ScoreRing`, `SectionCard`, `Sheet`, `SpectrumTrack`, `StatusPill`, `SubScoreTile`,
`CoverageMap`, `SourceLine`. The Radix-based kit — accordion, carousel, menubar,
command palette, calendar, chart — was scaffolded in and never touched.

Before calling that waste, I checked whether it ships. Grepping the production
bundle for every scaffold library:

```
radix: 0    embla: 0    cmdk: 0    recharts: 0    vaul: 0
react-day-picker: 0      sonner: 0
```

Vite tree-shook all of it. The 484 KB bundle contains none of it, so **the cost is
not bundle size** — it is 45 npm dependencies that exist only to satisfy imports in
files nothing imports. That is install time, lockfile churn, and audit surface: 45
packages whose CVEs a reviewer has to triage for a product that does not use them.

Same correction as Chapter 6, opposite direction. There the headline number was
inflated by tests that belonged in the file. Here it is inflated by scaffold that
does not belong at all, and the honest app is **4,297 lines**.

## 2. What it gets right

**The demo fallback is plumbed and actually rendered.** Every API call catches and
degrades:

```ts
} catch {
    return { data: mockSearch(query), source: "demo" };
}
```

The design note above it says the flag exists *"so the UI can label it honestly."*
Claims like that are usually where a codebase stops. This one follows through — the
flag is consumed in three places:

```
HealthCard.tsx:360     {source === "demo" && " · demo data"}
More.tsx:208           {source === "demo" && ( … )}
AgentSheet.tsx:266     source === "demo"
```

A user looking at fabricated demo buildings is told so. That is the difference
between a graceful degradation and a lie, and it is one `&&` away from being the
second one.

**The timeout reasoning is the best comment in the frontend.**

```ts
const LLM_TIMEOUT_MS = 70000;
```

> The backend allows the model 30s per attempt and retries once on a transient
> failure, so a worst-case round trip is roughly 60s. An 8s client abort would kill
> a slow but *successful* answer and silently swap in demo text — letting the
> client decide an outcome the server was still working on. Measured live: legal
> answers land in 12-27s, with an occasional retry pushing past 60s.

Derived from the server's actual budget, not guessed. Names the failure it prevents
— the client fabricating an answer the server was about to deliver. And it carries
observed latencies, so the next person can tell whether reality has moved. Two
different timeouts for two different classes of endpoint, each with a reason.

## 3. Provenance invented at the leaf

`HealthCard.tsx:15`:

```tsx
const DATA_MONTH = "Jul 2026";
```

Used five times — on the Census source line, the HPD source line, the DHCR line,
the DOB/MTA line, and in the footer that states the product's entire claim:

> Every number links to a NYC or Census source · Data from **Jul 2026**

That string is the only freshness claim the product makes, and it is a literal in a
`.tsx` file.

Chapter 7 established that `meta` holds exactly one row — `snapshot_year` — with no
ingest date. So the backend does not know when its data was gathered, cannot report
it, and the frontend fills the gap by asserting it. Re-run ingest tomorrow and this
string still says July. The number a tenant is asked to trust the freshness of is
maintained by hand, in a different language, in a different repository directory,
from the artifact it describes.

This is the same failure as Chapter 3's case-study description and Chapter 5's doc
comment: a claim about the system, written somewhere the system cannot check it. It
is the third instance, and by now the pattern is the finding.

## 4. The link that cannot contain the number

`HealthCard.tsx:463` gives the tract rent median a "check this" link:

```tsx
source={{ agency: "US Census B25064", date: DATA_MONTH,
          href: "https://data.census.gov/table/ACSDT1Y2023.B25064" }}
```

`ACSDT1Y2023` is the ACS **1-year** detailed table. The ingest queries
(`crates/ingest/src/sources.rs:204`):

```
https://api.census.gov/data/2023/acs/acs5?get=B25064_001E&for=tract:*&in=state:36&in=county:047
```

`acs5` — the **5-year** estimates.

These are different products with different values, and the mismatch is not
cosmetic: the Census Bureau does not publish 1-year estimates below a population
threshold of 65,000, and a census tract holds a few thousand people. **Tract-level
B25064 does not exist in the 1-year product at all.** A reader who follows the link
to verify the neighborhood median cannot find the number there, because it is not
in that table and could not be.

For a product whose thesis is that every figure links to a source you can check,
the check link pointing at the wrong dataset is the specific failure that matters
most. It is also a one-character-class fix: `ACSDT5Y2023`.

## 5. Absent rendered as clean

Now the part this chapter exists for.

`normalizeBuilding` coerces every field it cannot find:

```ts
open_violations: {
  a: v.a ?? v.class_a ?? v.A ?? 0,
  b: v.b ?? v.class_b ?? v.B ?? 0,
  c: v.c ?? v.class_c ?? v.C ?? 0,
```

Three fallback spellings then `0`. Defensive, reasonable, and it means **missing
data and zero violations are the same value** by the time anything renders.

And the render does not treat zero neutrally (`HealthCard.tsx:405`):

```tsx
{v.c > 0
  ? ` with ${v.c} hazardous violation${v.c > 1 ? "s" : ""} open`
  : " with a clean hazardous-violation record"}
```

Plus the summary line at `:77`: `"No hazardous violations"`.

A zero is not displayed as a zero. It is displayed as **an affirmative statement
that the building is clean.**

Now finish the chain this book has been building. 689 MYRTLE AVENUE:

| layer | what it holds or says |
|---|---|
| HPD | 7 open class-C, 5 open class-B |
| the artifact (Ch. 8) | 0, 0 — the `$limit` truncation dropped them |
| `grounding_block` (Ch. 9) | `Open HPD violations: 0 class-C (most serious)…` |
| the Health Card (here) | *"with a clean hazardous-violation record"* |
| the score | published **84, "strong"** · actual **39, "concern"** |

One unchecked integer in a query builder, four crates away, arrives at a
prospective tenant as an affirmative claim that a building with seven
immediately-hazardous violations has a clean record.

Every layer did its job. The ingest parsed correctly, the scoring computed
correctly, the agent refused to guess, and the frontend rendered exactly what it
was given. **Not one of the four layers has a mechanism for expressing that a
number might be missing rather than zero**, so the absence propagates upward and
gains confidence at every step.

The fix at this layer is small and worth stating precisely, because it is not "add
a null check." `open_violations` should be `number | null`, `null` should render as
*"no violation data for this building"*, and `0` should keep the clean-record
wording it has earned. That distinction costs a type change and one branch, and it
converts a false claim into an honest absence.

---

## The hardest question a reader can ask of this chapter

> *"Every problem you have listed here is one line. Scaffold nobody deleted, a
> hardcoded date, a wrong URL fragment, a `?? 0`. Is that a serious critique of a
> frontend, or nitpicking?"*

Three of them are nitpicking and one is not, and the distinction is the point.

The scaffold is cosmetic — it tree-shakes out, it costs npm audit noise and nothing
a user experiences. `DATA_MONTH` is a maintenance hazard that is currently
*accurate*; July is when ingest ran. `ACSDT1Y2023` is a broken verification link,
which matters more than it looks for this product specifically but breaks nothing
computational.

`?? 0` is different in kind. It is the last of four independent places where this
system converts "we do not have this" into "the value is zero," and it is the one
where the conversion becomes a sentence a person reads and acts on. A tenant does
not see the truncated query, the empty artifact row, or the grounding block. They
see *"a clean hazardous-violation record"* on a page that told them every number
links to a source.

So the honest answer is that the frontend's defects are individually small and one
of them is load-bearing for the worst outcome in the product. That is usually how
it goes: the layer closest to the user is where upstream uncertainty gets its final
coat of confidence, because rendering forces a decision — you cannot display a
maybe.

What to do, in order:

1. **`number | null` through the violation path**, with an explicit "no data" render.
   The only item here that changes what a user is told.
2. **Fix the Census link** to `ACSDT5Y2023`. One character class, and it repairs the
   claim the footer makes.
3. **Serve `DATA_MONTH` from the API.** It belongs in the `meta` rows Chapter 7 asked
   for, alongside the ingest date and coverage. Four chapters have now ended with
   "put it in `meta`."
4. **Delete `components/ui`.** 6,083 lines and 45 dependencies that nothing imports.
   Costs nothing to remove and makes the 4,297 real lines legible.

Only (1) is urgent. But (1) is urgent, and it is four lines.

---

*Next: **Chapter 11 — Accessibility as a Correctness Property.** What WCAG 2.2 AA
actually required of this build, which failures were measurable, and why the
contrast bugs were the easy half.*
