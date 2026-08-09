# Cycle 4 — Working Agreement

Capstone: **HouseCheck**. Pair: Aisling (backend, data, docs) and Anthony/Antonin Lesov
(frontend, design). Third group member: Jagger.

Written after the fact, during backlog catch-up. Everything below describes a pairing that
already ran its full course, so the answers are what actually happened rather than what we
planned to happen.

---

## Question 1 — What surprised you about how you two actually worked?

We didn't get along.

Anthony and I got along. The person who was also part of our group, Jagger, was a
life-learning lesson — it was very difficult dealing with them. A lot of things didn't play
out right. We had to redo this build two other times. It was a very dysfunctional capstone.

I'm very proud of what we made. We made something pretty incredible. But we did not get
along. If anything it was a lot of tension, and there was a lack of collaboration. It's
amazing we had the presentation and the work we were able to present.

**The surprise, stated plainly:** that the tension and the quality moved independently of
each other. I expected a dysfunctional team to produce a dysfunctional product. It didn't.
We shipped a working Rust backend, a live React client, an agent that refuses to fabricate,
and a real data pipeline over eight municipal sources — while barely collaborating.

I don't think that means the dysfunction was free. It means the cost landed somewhere other
than quality: it landed in rework.

---

## What the repository says about it

The git history is unusually blunt about where the rework came from.

**Two frontends were started on the same day.** On 25 July 2026:

| author | commit |
|---|---|
| Anthony | `feat(frontend): React+Vite+Tailwind app wired to live HouseCheck API` |
| Jagger | `Add Next.js MVP with compare agent under mvp/.` |

Two applications, two frameworks, one repository. `mvp/` shipped its own `package.json`,
`next.config.ts`, eslint config, `README.md`, and its own `CLAUDE.md` — it was not a
contribution to the shared app, it was a second app.

`mvp/` is no longer in the working tree. That work was deleted.

**Four person-branches, all stale.** `aisling-backend`, `anthony-frontend`, `db-analyst`,
`jagger-agent` — last touched 22–23 July, none of them the branch the product actually
shipped from.

**Final contribution shape:**

```
Aisling    110 commits   crates/ingest · crates/api · docs · build-book
Anthony      9 commits   frontend/src/components · pages · wireframes
Jagger       1 commit    mvp/  (deleted)
```

Anthony's nine commits all land inside a four-day window, 25–28 July, in large batched
passes. Mine are spread across the whole project in small increments. Neither is wrong;
they fail differently. And twelve files carry both our names, including seven of the eight
components the app actually imports — so where we did collaborate, we genuinely co-owned
the code rather than dividing it.

---

## Question 2 — The working agreement

One commitment, specific enough to fail:

> **Every branch merges into `main` within 48 hours, or we delete it and say out loud why.**

### Why this one

It is aimed at the failure that actually cost us the capstone, not at a failure we imagined.

Our problem was never that we didn't talk. It was that work sat in isolation until it was too
expensive to reconcile — a whole Next.js application, discovered incompatible after it
existed, and thrown away. Three rebuilds is what "we'll integrate later" costs.

A 48-hour rule makes divergence visible while it is still cheap. Two days of incompatible
work is a conversation. Two weeks of it is a deletion.

It also has the property a working agreement needs and "communicate better" does not:
**you can check it.** `git branch --format='%(committerdate:relative)'` answers it in one
command. Nobody has to interpret whether we honoured it.

### What it commits each of us to

- If it isn't mergeable in 48 hours, it was scoped too big. Cut it down and merge the part
  that works.
- If it can't merge because it conflicts with someone's direction, that is the signal to
  stop and settle the direction — that day, not at integration time.
- Deleting a branch is a normal outcome, not a punishment. Saying *why* out loud is the
  part that stops the same branch being re-created next week.

### The corollary that would have saved the build

> **No second implementation of something that already exists on `main`.** You replace it in
> place, or you don't start it.

If either of these had been in force on 25 July 2026, `mvp/` would have been a conversation
instead of a deletion.
