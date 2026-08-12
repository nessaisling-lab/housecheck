# Owner linkage — how Who Owns What does it, and what we would do

`docs/BACKLOG.md` defers owner/portfolio linkage as *"blocked on an HPD registration dataset
that has never been ingested."* Two findings change that:

1. **It is not blocked.** Both datasets are public on Socrata, on the same path the ingest
   already uses, and carry every column needed.
2. **It is largely solved.** JustFix's [`who-owns-what`][wow] has been doing this in
   production for years. Read below before writing any of it.

[wow]: https://github.com/JustFixNYC/who-owns-what

---

## 1. What it actually does

Given a BBL, it returns the other buildings **probably controlled by the same people**, plus
a graph showing *why* it thinks so. It is not a public "who owns this" registry — no such
registry exists, which is the entire problem. It is an inference over who registered the
building with HPD.

That distinction matters for us. A tenant asking *"who is my landlord really"* is asking a
question the City does not directly answer. HPD requires a registration naming responsible
parties; the LLC on the deed is often a shell. So the honest claim is never *"this is the
owner"* — it is *"these buildings share a registered contact or a business address."*

---

## 2. Where the data comes from

Two HPD datasets, joined on `registrationid`:

| Dataset | Socrata id | Rows (checked 2026-08-12) | What it gives |
|---|---|---|---|
| Multiple Dwelling Registrations | `tesw-yqqr` | **203,236** | `registrationid`, `boroid`/`block`/`lot` (→ BBL), `registrationenddate`, `lastregistrationdate` |
| Registration Contacts | `feu5-w2e2` | **782,024** | `registrationid`, `type`, `firstname`/`lastname`, `corporationname`, `businesshousenumber`/`streetname`/`apartment`/`city`/`state`/`zip` |

Every column their query needs is present in the Socrata copy. They read it from NYCDB; we
would pull it the same way we already pull HPD violations.

**Which contacts count.** Only four roles are trusted:

```
HeadOfficer, IndividualOwner, CorporateOwner, JointOwner
```

Managing agents and site managers are excluded — they work for whoever is paying, so two
buildings sharing a managing agent are not related in any way a tenant cares about. Rows
with no name, or with fewer than three characters of business address, are dropped.

**One contact per building.** For each BBL: the most recent registration by
`registrationenddate`, tie-broken by `lastregistrationdate`, then the contact type ranked
`IndividualOwner > HeadOfficer > JointOwner > CorporateOwner`, then name for determinism.
Deliberate: an individual is more identifying than a corporate shell.

---

## 3. How the linkage works

**Step 1 — standardise the business address.** Addresses are geocoded through NYC DCP's
Geosupport to normalise house number and street name. Apartment/suite is *not* covered by
the geocoder, so it gets a hand-written regex ladder collapsing `SUITE`→`STE`,
`FLOOR|FLR|FL`→`FL`, `BASEMENT|BSMNT`→`BSMT`, and so on. Reading that ladder tells you what
the data really looks like: `12 FL`, `FL 12`, `GRNDFL`, `GDFL` all mean the same floor.

**Step 2 — build a graph.** Each distinct `(name, business address)` pair is a node carrying
the BBLs registered to it. Edges connect nodes by two rules with different confidence:

| Edge | Rule | Weight |
|---|---|---|
| **Business address** | exact house-number + street + zip, and apartment numerals equal *or either side blank* | `2 + name_similarity` (2–3) |
| **Name** | exact name match **and** same zip **and** trigram similarity of house+street > 0.9, or > 0.8 with matching apartment numerals | `1 + street_similarity + 0.5×apt_match` (1–2.3) |

The asymmetry is the interesting judgment. A shared business address is treated as stronger
evidence than a shared name, because "JOHN SMITH" recurs across unrelated people while
`123 Main St, Ste 400` does not. A name match alone is never enough — it must be corroborated
by a near-matching address.

**Step 3 — cut portfolios down to size.** Connected components become portfolios, but a
single bad edge (a registration agent's address, a P.O. box) can fuse hundreds of unrelated
buildings. So any component over **300 BBLs** is recursively split with the **Louvain
community-detection** algorithm at resolution 0.1, weighted by those confidence scores. If a
split does not actually reduce the size, it stops rather than looping.

That 300-BBL cap is a false-positive guard, and it is also an admission: above that size the
method stops being trustworthy, so they cut rather than assert.

---

## 4. What this means for HouseCheck

**We should not port their code.** `who-owns-what` is **GPL-3.0**. Copying its SQL or Python
into this tree would put HouseCheck under GPL-3.0. Reading it to understand the problem is
fine and is what this document is. Credit is owed either way.

**We can use the same public inputs.** `tesw-yqqr` and `feu5-w2e2` are City open data under
NYC's own terms, not JustFix's. Building our own linkage from those datasets is clean.

**Scope makes this much easier than it is for them.** They cover the whole city and therefore
need Louvain splitting, a 300-BBL cap, and Geosupport. We have **250 buildings in one
community district**. At that scale:

- Fetch only registrations whose BBL is in our set — hundreds of rows, not 203,236
- Exact-match on standardised business address gets most of the value; the trigram fuzzy
  name path is the expensive part and the least confident one
- No Louvain needed. A portfolio that exceeds a sane size within 250 buildings is a bug
  worth showing, not a component to split
- Geosupport is a heavy native dependency. For 250 buildings a much smaller normaliser plus
  their apartment regex ladder (reimplemented, not copied) is proportionate

**What we would show, and how to say it honestly.** This fits the three-state pattern the
rest of the card already uses:

| State | When | Card says |
|---|---|---|
| `Linked` | shares a registered contact **and** business address with other pilot buildings | "Registered to the same contact as N other buildings here" |
| `RegisteredAlone` | has a valid registration, no matches in our set | "No other building in this district shares its registration" |
| absent | no current registration on file | say exactly that — a missing registration is itself a finding |

The middle state is the one that would be dropped by accident, and dropping it would make a
landlord with one building look identical to one whose registration has lapsed.

**Wording discipline.** Never "owner". The claim the data supports is *registered contact*.
`RegisteredAlone` must not be read as "small landlord" — it may mean the portfolio is held
behind different LLCs with different business addresses, which is exactly what someone
hiding a portfolio would arrange. Say what was matched, and link to the registration.

---

## 5. Why this is worth doing even though they built it

Their tool answers *"who else does this landlord own?"* for tenants and organisers. Ours
would answer a narrower question inside a document that a stranger can verify: **the export
already carries a signed hash chain, and a registration-linkage row inside that signed region
is a citable fact rather than a screenshot of someone else's website.**

That is the actual gap. Who Owns What is a live web tool; you cannot attach it to a filing.
We can put the same inference, with its evidence and its retrieval timestamp, inside a record
that survives leaving us.

---

## 6. Before building: talk to them

`hello@justfix.org`. They have already tested empirically whether landlord-level data helps
advocates, and they will know whether the provenance problem is real. See
`docs/legal-aid-call.md` — they are the first contact listed there, and this gives the
conversation a concrete subject rather than a general "would you use this".

Worth asking: whether they would want the export format themselves, and whether GPL is a
hard requirement on their side or an artefact of the project's history.

---

*Reviewed 2026-08-12 against `who-owns-what` at `main` (211 stars, last push 2026-08-11).
Files read: `portfoliograph/graph.py`, `portfoliograph/standardize.py`,
`portfoliograph/sql/landlords_to_standardize.sql`,
`portfoliograph/sql/landlords_with_connections.sql`. Dataset row counts queried live from
Socrata the same day.*
