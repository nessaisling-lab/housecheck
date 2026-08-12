# The Legal Aid call — prep

Top item in `docs/BACKLOG.md`. It is the cheapest possible way to find out the export is
aimed at nothing, and it is a phone call rather than a build, which is why it keeps not
happening.

Everything below is ready to use. Contact details re-verified **2026-08-12**.

---

## 1. What is actually under test

Two claims, and **either can fail on its own**:

1. Producing a *trustworthy* record of what HPD says is an expensive part of a tenant
   lawyer's job.
2. A portable, independently checkable file is a form they can actually use.

The manual pass may not cost enough time to matter. The provenance of an HPD printout may
never be challenged in the first place. Those are different failures with different
responses, so the call has to separate them.

---

## 2. Who to call — and who not to

**Do not cold-call The Legal Aid Society's intake line (212-577-3300) for this.**

That number, and Met Council's hotline, exist for tenants in crisis. A person with no heat
in February is competing for that capacity. Spending a slot on product research is a real
cost to a real person, and it is not made acceptable by the research being well-intentioned.
The item is named "the Legal Aid call" but what it needs is *a housing practitioner's
workflow*, and there are several routes to that which cost nobody their place in a queue.

Ranked by "will teach the most per unit of harm":

| # | Who | Route | Why them |
|---|---|---|---|
| 1 | **JustFix** | `hello@justfix.org` | A 501(c)3 that builds tenant tech in NYC — a peer conversation, not a crisis line. They already run the landlord-research tool described in §6, and your ingest already cites their `nyc-doffer`. Warmest possible opening, and email respects their time. |
| 2 | **Housing Court Answers** | 212-962-4795, Mon–Fri 9–5 | Their stated work includes running trainings on Housing Court procedure for community groups and unions. Explaining process *is* the job, so a process question is not an imposition. Best phone call. |
| 3 | **Law school housing clinics** | New York Law School Housing Rights Clinic; CUNY Housing Justice Practicum / CED Clinic | Clinics exist to teach. A supervising attorney has both the workflow knowledge and a reason to spend twenty minutes on a student researcher. |
| 4 | **Legal Aid, non-intake** | communications / pro bono / volunteer coordinator | If it must be Legal Aid, reach them through a channel that is not the client queue. |

Open with what you are not asking for: *"I'm not a tenant and this isn't an intake call —
I'm a student who built a tool and I'm trying to find out whether it's aimed at a real
problem. Twenty minutes, and I'm happy to send questions in writing instead."*

---

## 3. The script

**Ask about the workflow, never about the product.** "Would you use this" gets a polite yes
and teaches nothing.

1. Walk me through the last time you needed a building's violation history. What did you
   actually do?
2. How long did that take, start to finish?
3. What did you do with the output — did it go into a filing, a letter, a negotiation, or
   just your own notes?
4. Has anyone ever questioned where those records came from? What happened?
5. Is there a version of this that already works fine for you?
6. If a colleague handed you a violation history for a building, what would make you
   distrust it?

**The one artifact question, asked last:** would you rather have a PDF, or a document you
can edit into a filing?

Ask 6 before mentioning hash chains. If they volunteer provenance unprompted, that is the
strongest evidence available; if they only agree it matters after you raise it, that is
politeness, not a finding. Write down which happened.

---

## 4. Kill conditions — pre-registered so they cannot be argued away afterwards

Copied from the backlog deliberately unchanged. Decide the response *before* hearing the
answer.

| If you hear | Then |
|---|---|
| *"Nobody has ever challenged where an HPD printout came from."* | The hash chain solves a problem that does not exist. It **stays** — it is built and costs nothing to keep — but it stops being the headline. Value becomes speed and legibility, and `docs/design/pdf-export.md` becomes **more** important, not less. |
| *"We'd need a certified copy from HPD or a sworn declaration."* | Mechanism right, packaging wrong. Add a declaration page and find out the accepted authentication route. **Do not defend the chain.** |
| *"The lookup takes two minutes and we already have a way."* | The MVP is aimed at nothing. Change the *user*, not the feature. The renter-at-the-moment-of-decision is the other candidate and is its own open question. That pivot is not free. |

**What a bad call does not invalidate:** ingest, scoring, the card, the agent, address
resolution and the provenance stamp are all user-agnostic. The export is one route plus one
module. That is the entire reason this call is cheap.

---

## 5. Capture sheet

Fill in during the call, not after.

```
Date / who / role / org:
Route used (and did it consume crisis capacity? y/n):

Q1 last time they pulled violation history — what they did:
Q2 how long it took:
Q3 what the output was used for:
Q4 has provenance ever been challenged:      volunteered / only after prompting / never
Q5 existing tool that already works:
Q6 what would make them distrust a record:

Artifact preference:                          PDF / editable / neither / other

Kill condition triggered?                     none / 1 / 2 / 3
Direct quote worth keeping:

What I expected to hear and did not:
```

That last line is the one that matters. A call that only confirms what you already believed
usually means the questions were leading.

---

## 6. Prior art found while preparing this — read before the call

**JustFix runs [Who Owns What](https://github.com/JustFixNYC/who-owns-what)** (211 stars,
actively developed — last push 2026-08-11). It links NYC buildings to a common
landlord/owner portfolio, which is precisely the owner-linkage feature this backlog defers
as *"blocked on HPD registration data that has never been ingested."*

Two consequences:

- **It is not necessarily a competitor.** They target tenants and organisers; the export
  and the court-usable record are not their problem. But building owner-linkage without
  having looked at theirs first would be doing solved work badly.
- **License caution — `who-owns-what` is GPL-3.0.** Copyleft. Reading it for approach is
  fine; taking code from it would pull HouseCheck under GPL-3.0, which conflicts with the
  standing requirement that dependencies be commercially viable. Their *data* pipelines and
  the public datasets underneath are a different question and are worth asking about.

This also makes JustFix the most productive first contact: they have already answered
"is landlord-level data useful to advocates" empirically, and they will know whether the
provenance problem is real.

---

## 7. Directory re-verification, 2026-08-12

`legal_help_directory()` in `crates/api/src/main.rs` carries the note *"re-verify
periodically — a stale hotline number for a person with no heat is a real harm, not a
broken link."* Last verified 2026-07-26. Re-checked today, all five entries:

| Entry | URL | Phone on page |
|---|---|---|
| Housing Court Answers | 200 | ✅ 212-962-4795 |
| Met Council hotline | 403 to scripts; confirmed live by fetch | ✅ 212-979-0611, Mon/Wed 1:30–8pm, Fri 1:30–5pm |
| The Legal Aid Society — Housing | 200 | ✅ 212-577-3300 |
| LawHelpNY | 200 | n/a |
| NYC 311 | 200 | n/a |

**No drift in 17 days.** Met Council's 403 is bot-blocking, not an outage — the number and
the hours still match what the code serves, verbatim.

Still true, and still the honest caveat in that function: **nobody has dialled these.**
Reaching a human on one of them would close that gap as a side effect of making the call
above.
