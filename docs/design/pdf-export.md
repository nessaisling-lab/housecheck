# Design Session — Server-Side PDF Export

**The question:** the record already exports as verifiable JSON and as a plain-text transcript.
Printing hands off to the browser's print dialog. A lawyer asked for something they can file.
What does a generated PDF have to be, and what would it cost to build?

**Answer: build it, as a *rendering* of the signed record and never as a second source of truth.**
One dependency (`printpdf`, MIT), one new crate, and three hard gates before any of it ships.

**Dated:** 2026-08-11. **Status:** scope and recommendation, nothing built.

---

## 1. Why a PDF at all, when text already exists

Three reasons, in order of how much they matter.

1. **Legibility for a legal reader.** `?format=text` produces a correct transcript that looks like
   a log file. A filing exhibit is read by a judge, a clerk, and opposing counsel, and a document
   that looks like terminal output invites the question "what is this?" before the question "is it
   true?". Formatting is not decoration here; it is what stops the document being dismissed.
2. **Live links.** A citation the reader cannot follow is a claim they have to take on trust. A
   PDF can carry a clickable link back to the exact NYC Open Data dataset behind every figure. The
   text format cannot, the browser print dialog will not.
3. **Something the agent can hand over.** Backlog item "let the agent hand over documents, not just
   prose" needs an artifact to hand over. This is that artifact. The two items are one piece of
   work and should be built in that order.

What the browser print path gets wrong today: the output is under the reader's control (their
margins, their headers, their "print backgrounds" setting), so two people printing the same record
get two different documents and neither is reproducible.

---

## 2. The honesty problem, which is the whole design

**A PDF looks more authoritative than the JSON while carrying less proof.** That asymmetry is the
central risk, and it is the same failure this export was built to avoid.

The JSON is the verifiable artifact: a hash chain anyone can recompute offline, plus an optional
Ed25519 signature over the chain head. A PDF has no such property. Re-serialising the rows into a
page changes the bytes, so the chain does not survive the rendering. Anyone with a PDF editor can
change a number.

If we ship a nicely-typeset document with a seal-shaped hash on it and say nothing further, we have
built exactly the thing the `IntactUnsigned` case exists to prevent — an unverified document that
reads as an authenticated one.

**So the rules for the PDF are:**

- Every page footer carries the **record hash** (`chain_head`) and the page number, so a page
  separated from its document still identifies itself.
- The verification block states, in the document's own words, that **this PDF is a rendering** and
  that the checkable document is the JSON export of the same record — with the URL to fetch it and
  the URL that checks it.
- If the record is **unsigned**, the PDF says so in the same place it would have named the signer.
  Not a footnote. The word "unsigned" appears at the same size as everything else in that block.
- The PDF never uses the word "verified" about itself.

This is the same three-state discipline already in `VerifyOutcome`: signed-and-intact,
intact-but-unsigned, tampered. Collapsing the middle state is the mistake, on paper as in code.

---

## 3. The dependency decision

**Recommendation: `printpdf` 0.12.5.**

| | licence | what it gives | what it costs |
|---|---|---|---|
| **`printpdf` 0.12.5** | **MIT** | `BuiltinFont` (no font file to ship), `LinkAnnotation` + `Actions` for clickable URLs, page/text ops, `Mm`/`Pt` units | large optional feature surface (HTML, SVG, seven image formats) that must be switched off |
| `pdf-writer` 0.15 | MIT OR Apache-2.0 | lowest-level, very clean, used by Typst | no fonts, no layout, no text measurement — every glyph width and line break is ours to write |
| `genpdf` | Apache-2.0 | high-level layout | wraps a years-old `printpdf`; not maintained |

Both candidate licences are commercially safe (MIT, and MIT/Apache-2.0), which was the stated
constraint. `printpdf` is actively released — 0.12.5 shipped 29 July 2026 — with ~995k recent
downloads, so it is not a one-author risk.

`pdf-writer` is the better-engineered crate and the wrong choice here: it would mean hand-writing
Helvetica width tables to wrap a 120-character violation description. That is a week of work to
save a dependency we are allowed to have.

**Add it as:**

```toml
printpdf = { version = "0.12", default-features = false }
```

**Gate 1 — the feature surface must actually be reducible.** Verify `default-features = false`
builds, and measure `cargo tree -p render` and the release binary size before and after. The Fly
image is a baked artifact on a small machine; a PDF writer that drags in seven image decoders is
not worth a prettier document. **Reject the dependency if the stripped build is not clean.**

**Gate 2 — it must stay pure Rust.** `model` deliberately has no C toolchain in its dependency
graph. If `printpdf` minus default features still needs one, the Docker build changes and the
decision goes back on the table.

---

## 4. Where the code lives

**A new crate: `crates/render`.** Depends on `model`; `api` depends on it.

Not in `model` — `model` is the crate that can be constructed in four lines in a test and never
fails to build, and that property is worth more than the convenience. Not in `api` — a renderer
buried in a 2,000-line request handler cannot be tested without standing up a server.

```
model (ExportDocument)  ->  render (ExportDocument -> Vec<u8>)  ->  api (HTTP)
```

The signature is the point of the design:

```rust
pub fn to_pdf(doc: &ExportDocument, links: &LinkSet) -> Result<Vec<u8>, RenderError>;
```

`ExportDocument` in, bytes out, no database, no clock, no network. That makes the renderer a pure
function of a document that tests already build, and it means a PDF can never contain a fact the
signed document does not.

---

## 5. Document design

One page size (US Letter), one font family (Helvetica, built in — no font file to license or ship),
three type sizes.

```
+--------------------------------------------------------------+
|  HOUSECHECK RECORD                                            |
|  603 PUTNAM AVENUE                    Brooklyn, BBL 3016440063|
|  Exported 2026-08-11                                          |
+--------------------------------------------------------------+
|  33 open HPD violations on record. 0 immediately hazardous     |
|  (Class C).                                                    |
+--------------------------------------------------------------+
|  #  CLASS  ISSUED       OPEN      VIOLATION                    |
|  1  C      2026-03-14   148 days  § 27-2033 ADM CODE PROVIDE  |
|                                   ADEQUATE HEAT ...            |
|  2  B      (not recorded)  age unknown   ...                   |
+--------------------------------------------------------------+
|  SOURCES                                                       |
|  wvxf-dwi5    26,343 rows   retrieved 2026-08-08   [open ->]   |
+--------------------------------------------------------------+
|  VERIFICATION                                                  |
|  Record hash  a3f1...  |  Unsigned (hash-chained, not          |
|  attributed to an issuer).                                     |
|  This PDF is a rendering. The checkable document is the JSON    |
|  export of the same record: [link]  Check it: [link]           |
+--------------------------------------------------------------+
  footer, every page:  record a3f1...  ·  page 2 of 4
```

Reuses the ordering and the wording already proved in `to_plain_text()` — including
`age unknown (no issue date on record)`, which exists so the document never prints a confident
zero for an age it does not know.

**Text wrapping is the one non-obvious piece of work.** Descriptions average 120 characters and
83% are distinct, so the row height varies and pagination has to be computed rather than assumed.
`printpdf` exposes font metrics; whether it exposes them for *built-in* fonts without a parsed TTF
is unverified. **If it does not, the fallback is to embed a metrics table for Helvetica** (the
base-14 widths are published and unencumbered) rather than to ship a font file.

---

## 6. Links — and the rule for which ones ship

**A link that has not been fetched does not ship.** The deck already carries this debt: NY Judiciary
Law §§ 478/484 and *FTC v. DoNotPay* are cited by name with no link because the URLs were never
verified. Repeating that in a court exhibit is worse.

| link | status | note |
|---|---|---|
| NYC Open Data dataset page per `SourceStamp` | **build it** | `https://data.cityofnewyork.us/d/{dataset}` — one URL pattern, covers every source, and the dataset id is already inside the signed region |
| HouseCheck JSON export of this record | **build it** | our own route, known-good |
| HouseCheck verifier | **build it** | our own route, known-good |
| NYC Admin Code section per violation (`§ 27-2033`) | **do not ship yet** | requires parsing a section number out of HPD free text *and* a stable public URL for it. Two unverified assumptions. Separate item. |

**Gate 3 — fetch every URL pattern and confirm a 200 before it is written into a document.** Two
dataset ids have already turned out not to resolve during this project; that is the measured base
rate, and it is not low.

---

## 7. Budgets and failure behaviour

- **Time:** target under 200 ms for a 33-row record on the Fly machine. Measure; do not assume.
  If it exceeds that, the endpoint gets its own timeout rather than sharing the card budget.
- **Size:** target under 200 KB. Helvetica is not embedded, so the floor is the text itself.
- **Row cap:** `OPEN_DETAIL_CAP` (50) already bounds the rows, and the document already prints
  `showing N of M`. The PDF inherits both. It must not silently print 50 of 300.
- **Fail closed.** If the record cannot be built, there is no PDF — the same rule the export
  already follows on demo data. A person who cannot get their record is far better served than a
  person handed a plausible one.

---

## 8. Sequence

Each step is separately shippable, and the gates come first because they can kill the approach.

1. **Gate 1 + 2** — add `printpdf` with `default-features = false` in a scratch branch; measure
   dependency tree, binary size, and whether a C toolchain appears. *This decides the crate.*
2. **`crates/render`** with `to_pdf`, one page, header + verification block only. Golden test:
   render the existing `doc()` test fixture, assert the bytes parse as a PDF and contain the
   record hash.
3. **Text wrapping and pagination**, driven by the 33-row `603 PUTNAM AVENUE` record — the one that
   already exposed the "no hazardous violations" reading problem, so it is the honest test case.
4. **Gate 3** — verify the dataset URL pattern; then link annotations.
5. **`?format=pdf`** on `GET /building/{bbl}/export`, `Content-Type: application/pdf`,
   `Content-Disposition: attachment`. Unknown formats still fall through to JSON.
6. **Frontend:** a fourth destination on the card next to copy / download / print. The print button
   stays — it is still the right answer for someone who wants paper right now.
7. **The agent hands it over** — when the agent cites a building's record, it offers the PDF.

Steps 1–2 are the only ones that can fail in an interesting way. If step 1 fails, the fallback is
`pdf-writer` plus a Helvetica width table, which is roughly three times the work and still MIT.

---

## 9. Open questions

- **Does a lawyer want a PDF, or a Word document they can edit into a filing?** Unknown, and it
  changes the artifact. Goes on the Legal Aid call — it costs one sentence to ask.
- **Does the exhibit need a declaration page** (who produced it, under what process) to be
  admissible, or is the hash chain the wrong shape of proof entirely? Also for the call.
- **Should the PDF carry the JSON inside it as a PDF file attachment**, making one file both
  readable and checkable? That is the elegant answer, and `printpdf` 0.12.5 does not appear to
  expose embedded-file streams. Revisit only if step 1 pushes us to `pdf-writer`, which can write
  the object directly.
