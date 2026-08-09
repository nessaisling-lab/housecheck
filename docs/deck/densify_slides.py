# Rebuild the seven sparsest slides with real detail, citations and outbound links.
#
# Method: define NEW components and re-point the registry at them. The originals stay in
# the bundle untouched as dead code. Rewriting a minified function in place means finding
# its exact extent and getting it byte-perfect; re-pointing a registry entry cannot corrupt
# a neighbour, and reverting is a one-line change.
#
# Two hazards this file guards against, both previously shipped broken:
#   1. Precompiled Tailwind. A class absent from the bundle fails SILENTLY. Every class is
#      extracted and checked against the compiled CSS before a byte is written.
#   2. `IS`, the hardcoded slide count that keyboard/wheel nav clamps on. Not touched here
#      (slide count is unchanged) but asserted anyway, because it cost a broken deck once.
#
# Links use INLINE styles, not classes: the bundle contains no <a> elements at all, so no
# anchor styling is compiled and any link class would be a silent no-op.
import io
import re
import sys

P = r"D:\L2 Cycle 4\Housecheck Antonin Idea\docs\deck\HouseCheck-Presentation.html"

# Only URLs verified in docs/classwork/industry-research-notes.md, or checked live this
# session. data.cityofnewyork.us/d/<id> is Socrata's stable short form for a dataset.
U = {
    "hpd_viol":  "https://data.cityofnewyork.us/d/wvxf-dwi5",
    "311":       "https://data.cityofnewyork.us/d/erm2-nwe9",
    "pluto":     "https://data.cityofnewyork.us/d/64uk-42ks",
    "dob":       "https://data.cityofnewyork.us/d/e5aq-a4j2",
    "dohmh":     "https://data.cityofnewyork.us/d/43nn-pn8j",
    "vacancy":   "https://www.nyc.gov/site/hpd/news/007-24/new-york-city-s-vacancy-rate-reaches-historic-low-1-4-percent-demanding-urgent-action-new",
    "nycvs":     "https://rentguidelinesboard.cityofnewyork.us/research/nyc-housing-vacancy-survey/",
    "app":       "https://housecheck-wine.vercel.app",
    "meta":      "https://housecheck-nessa.fly.dev/meta",
    "openigloo": "https://therealdeal.com/new-york/2025/08/28/openigloo-won-over-tenants-can-it-do-the-same-with-landlords/",
}


def js(s):
    return '"%s"' % s.replace("\\", "\\\\").replace('"', '\\"')


def link(label, url):
    return ('K.jsx("a",{href:%s,target:"_blank",rel:"noreferrer",'
            'style:{color:cD,textDecoration:"underline",textUnderlineOffset:3},children:%s})'
            % (js(url), js(label)))


def sources(items):
    """A citation strip. Real links, so a reader can check the claim rather than trust it."""
    kids = ",".join(
        'K.jsxs("span",{style:{color:WB},children:[%s," ",%s]})' % (js(lbl + " ·"), link(txt, url))
        for lbl, txt, url in items
    )
    return ('K.jsx("div",{className:"flex items-center gap-6 mt-5 text-sm",children:[%s]})' % kids)


H2 = ('K.jsxs("h2",{className:"font-semibold leading-tight tracking-[-0.035em] mt-5 mb-6",'
      'style:{fontSize:"clamp(36px, 4.4vw, 54px)",color:aA},children:[%s," ",'
      'K.jsx("span",{style:{color:cD},children:%s})]})')

LEDE = ('K.jsx("p",{className:"text-lg leading-relaxed mb-6",style:{color:WA,maxWidth:1120},children:%s})')

GRID4 = ('K.jsx("div",{className:"grow grid grid-cols-4 gap-4",children:R.map(E=>'
         'K.jsxs("div",{className:"rounded-2xl p-5 flex flex-col",'
         'style:{backgroundColor:"rgba(255,255,255,0.42)",border:"1px solid rgba(28,28,30,0.08)"},'
         'children:[K.jsx("p",{className:"font-semibold text-base mb-1",style:{color:aA},children:E.k}),'
         'K.jsx("p",{className:"text-lg font-semibold",style:{color:cD},children:E.when}),'
         'K.jsx("p",{className:"text-base leading-relaxed mt-3",style:{color:WA},children:E.body}),'
         'E.note?K.jsx("p",{className:"text-sm leading-relaxed mt-3",style:{color:WB},children:E.note}):null]},E.k))})')

FLEX = ('K.jsx("div",{className:"grow flex items-stretch gap-6",children:R.map(E=>'
        'K.jsxs("div",{className:"flex-1 rounded-2xl p-5 flex flex-col",'
        'style:{backgroundColor:"rgba(255,255,255,0.42)",border:"1px solid rgba(28,28,30,0.08)"},'
        'children:[K.jsx("p",{className:"font-semibold text-xl mb-1",style:{color:aA},children:E.k}),'
        'K.jsx("p",{className:"text-lg font-semibold",style:{color:cD},children:E.when}),'
        'K.jsx("p",{className:"text-base leading-relaxed mt-3",style:{color:WA},children:E.body}),'
        'E.note?K.jsx("p",{className:"text-sm leading-relaxed mt-3",style:{color:WB},children:E.note}):null]},E.k))})')

SHELL = ('function %s(){const R=[%s];return K.jsxs("div",{className:"relative w-full h-full overflow-hidden",'
         'style:{backgroundColor:"transparent"},children:[K.jsx(Xq,{}),'
         'K.jsxs("div",{className:"relative z-10 w-full h-full flex flex-col",'
         'style:{padding:"48px 64px 40px"},children:[K.jsx(yl,{children:%s}),%s,%s,%s,%s]})]})}')


def cards(items):
    return ",".join(
        "{k:%s,when:%s,body:%s,note:%s}" % (js(k), js(w), js(b), js(n) if n else "null")
        for k, w, b, n in items
    )


def slide(fn, eyebrow, h2a, h2b, lede, items, layout, srcs):
    return SHELL % (fn, cards(items), js(eyebrow), H2 % (js(h2a), js(h2b)),
                    LEDE % js(lede), layout, sources(srcs))


SLIDES = {}

# ---- 4 · The Change --------------------------------------------------------------
SLIDES["hcD4"] = slide(
    "hcD4", "The Change", "Before you sign.", "Not after.",
    "The record is already public. What is missing is a way to read it in the fifteen minutes "
    "you actually get, standing in the apartment, with other applicants in the room.",
    [
        ("Before · three portals",  "Nothing talks to anything",
         "HPD Online holds violations by building. DHCR holds stabilization status by written "
         "request, not lookup. Census holds what rent is normal, in a data table.",
         "Three agencies, three vocabularies, none cross-referenced."),
        ("Before · codes, not conditions", "Class C means nothing to a renter",
         "A violation reads as a class letter and a code. Nothing tells you that Class C means "
         "immediately hazardous — no heat, no hot water, exposed wiring.",
         "11.1M violation records are published and effectively unreadable."),
        ("After · one address in",  "A 0–100 card in seconds",
         "Four pillars scored separately, so you can see which one drags the number down "
         "instead of arguing with a single verdict.",
         "Measured at 2.2 ms per card on a $2/month virtual machine."),
        ("After · every number opens", "Traceable to the row behind it",
         "Each pillar names the dataset it came from. Where the record cannot support a claim, "
         "it says unverified rather than guessing.",
         "A wrong yes on a legal-rights tool is worse than an honest blank."),
    ],
    GRID4,
    [("Violations", "HPD wvxf-dwi5", U["hpd_viol"]),
     ("Vacancy 1.41%", "NYC HPD", U["vacancy"]),
     ("Try it", "housecheck-wine.vercel.app", U["app"])],
)

# ---- 5 · HouseCheck, the product -------------------------------------------------
SLIDES["hcD5"] = slide(
    "hcD5", "HouseCheck", "One address in.", "One honest card out.",
    "Four pillars, weighted equally, each scored from a named public dataset. A pillar with no "
    "supporting record is marked unverified — never scored as a zero, because a missing record "
    "is not the same as a clean one.",
    [
        ("Condition", "HPD violations · 311",
         "Open violations by class, weighted by severity. The score starts at 100 and subtracts, "
         "so it counts every class.",
         "A building can show no hazardous violations and still score low on volume alone."),
        ("Legal", "DHCR · Good Cause",
         "Rent-stabilization and Good Cause coverage. Roughly 1M stabilized units house about "
         "2.5M tenants who hold rights only if they know the status.",
         "Where the record cannot support a claim, it reads unverified."),
        ("Neighborhood", "NYC 311 · erm2-nwe9",
         "Complaint density near the address on a logarithmic curve, so genuinely dense blocks "
         "still separate from each other rather than all pinning at the floor.",
         "Not rent. Rent is a separate check against the Census tract median."),
        ("Accessibility", "DOB · MTA ADA",
         "Elevator on record, and metres to a step-free subway entrance — the difference between "
         "a walk-up being inconvenient and being impossible.",
         "897 m to an ADA station reads differently at 30 than at 70."),
    ],
    GRID4,
    [("Violations", "wvxf-dwi5", U["hpd_viol"]),
     ("Complaints", "erm2-nwe9", U["311"]),
     ("Lot data", "PLUTO 64uk-42ks", U["pluto"]),
     ("Elevators", "DOB e5aq-a4j2", U["dob"])],
)

# ---- 6 · The Evidence ------------------------------------------------------------
SLIDES["hcD6"] = slide(
    "hcD6", "The Evidence", "Live demo.", "Run it yourself.",
    "Everything below is running right now against 250 real Bed-Stuy buildings. Nothing is "
    "mocked, and the API will answer you directly if you would rather read JSON than a slide.",
    [
        ("1 · Search", "Type any pilot address",
         "Covered buildings surface as you type. Anything outside the pilot is labelled as "
         "outside it, never quietly guessed at.",
         None),
        ("2 · Score", "A 0–100 card in seconds",
         "Four pillars, each with the reason underneath it, so the number is an argument rather "
         "than a verdict.",
         None),
        ("3 · Open it up", "Every number opens",
         "Rent against the tract median, condition by violation class, legal status, "
         "accessibility — each traced back to the dataset it came from.",
         None),
        ("4 · Ask", "The assistant, with citations",
         "It answers from that building's own record. It cites a statute for every legal claim, "
         "refuses to predict outcomes, and ends with a named free hotline.",
         "Search is restricted to nine government and academic domains."),
    ],
    GRID4,
    [("The app", "housecheck-wine.vercel.app", U["app"]),
     ("The raw provenance", "housecheck-nessa.fly.dev/meta", U["meta"])],
)

# ---- 10 · Under the Hood ---------------------------------------------------------
SLIDES["hcD10"] = slide(
    "hcD10", "Under the Hood", "A Rust API and a", "read-only database.",
    "Everything expensive happens once, at ingest, on a laptop. The request path does a handful "
    "of integer operations against a database small enough to sit in page cache — which is why "
    "the capacity numbers are absurd and the coverage ceiling is the real constraint.",
    [
        ("Rust + Axum", "Five crates",
         "model, scoring, store, ingest, api. Scoring is a pure function of the record, so it is "
         "testable without a server and a database.",
         "106 tests. fmt and clippy clean."),
        ("SQLite, baked in", "Opened read-only",
         "Shipped inside the container image and opened with SQLITE_OPEN_READ_ONLY. There is no "
         "database server to breach and no write path to abuse.",
         "Boot refuses to start on an empty database rather than serve 404s under a green health check."),
        ("Measured, not estimated", "2.2 ms per card",
         "21 ms to score all 250. On a 256 MB shared-CPU machine that is roughly 400,000 to "
         "1,300,000 daily users before capacity binds.",
         "Rate limited to 10 requests per 60s per client — a spend guard, not an auth boundary."),
        ("The honest ceiling", "~40,000 buildings",
         "A database baked into an image is free to serve and impossible to grow. At about "
         "40,000 buildings the artifact exceeds the VM and the design has to change.",
         "250 today at 1.3 MB. All 180,000 HPD multifamily would need ~914 MB."),
    ],
    GRID4,
    [("Live provenance endpoint", "/meta", U["meta"]),
     ("Source datasets", "NYC Open Data", U["hpd_viol"])],
)

# ---- 12 · Guardrails -------------------------------------------------------------
SLIDES["hcD12"] = slide(
    "hcD12", "Guardrails", "It explains the law.", "It does not practice it.",
    "This is a legal-rights tool used by people under time pressure, so the failure mode that "
    "matters is not a crash. It is a confident, wrong answer that someone acts on.",
    [
        ("No legal advice", "NY Judiciary Law §§ 478, 484",
         "Practising law without a licence is a crime in New York. Every legal answer carries a "
         "disclaimer and a citation to the statute it rests on.",
         "It explains what a rule says. It never tells you what to do about your case."),
        ("No promised outcomes", "FTC v. DoNotPay · $193,000",
         "That settlement was over unevidenced capability claims. The assistant refuses to "
         "predict how a case will go, because it has no data that could support the prediction.",
         "Refusing to predict reads like caution. It is really just refusing to invent."),
        ("Grounded by construction", "Nine allowed domains",
         "The model never queries the database directly and cannot browse freely. Legal search is "
         "restricted to government and academic sources.",
         "It answers from the building's own record or it says it does not know."),
        ("Real humans, verified", "Three errors found",
         "Every tenant hotline in the referral directory was checked against that organisation's "
         "own page. Three were wrong — bad hours, a dead domain. Fixed.",
         "Every legal answer ends with a named free hotline."),
    ],
    GRID4,
    [("Housing rights information", "NYC Rent Guidelines Board", U["nycvs"]),
     ("Try the assistant", "housecheck-wine.vercel.app", U["app"])],
)

# ---- 14 · Live Now ---------------------------------------------------------------
SLIDES["hcD14"] = slide(
    "hcD14", "Live Now", "Bed-Stuy today.", "Five boroughs next.",
    "The pilot is one community district — CD 303 — and every figure on this slide comes out of "
    "the shipped artifact, which stamps its own provenance. You can read it yourself at the "
    "/meta endpoint rather than taking the number from a slide.",
    [
        ("250 buildings", "Every one a full record",
         "Not a sample of a larger set. Every building we hold a complete record for in the pilot "
         "district, scored on all four pillars.",
         "1.3 MB artifact, small enough to sit entirely in page cache."),
        ("26,306 violations", "Class A, B and C",
         "Pulled complete rather than capped. An earlier ingest silently truncated at a $limit "
         "and lost roughly half the rows.",
         "That bug could only ever flatter a landlord — the score subtracts from 100."),
        ("219,977 complaints", "311, near every address",
         "Used for neighbourhood density on a log curve, so a genuinely busy block still separates "
         "from a very busy one.",
         "Snapshot year 2026, stamped in the database itself."),
        ("Eight sources", "All public, all named",
         "HPD, 311, PLUTO, DOB, DOHMH, Census ACS5 B25064, DHCR and JustFix. No proprietary feed, "
         "no scraped data, nothing behind a login.",
         "The same data exists city-wide. The pilot is scoped, not limited."),
    ],
    GRID4,
    [("Provenance, live", "/meta", U["meta"]),
     ("Vacancy 1.41%, 33,210 units", "NYC HPD", U["vacancy"]),
     ("Housing survey", "NYCHVS", U["nycvs"])],
)

# ---- 15 · What It Makes Possible -------------------------------------------------
SLIDES["hcD15"] = slide(
    "hcD15", "What It Makes Possible", "Give renters the record", "before the signature.",
    "Ranked by what is committed rather than what sounds best. The first item is the one the "
    "problem statement rests on, and it is a correctness fix as much as a feature.",
    [
        ("Violation meaning", "Committed · next",
         "HPD publishes the text of every notice. We report counts. Nobody turns seven open Class "
         "C into no heat, twice, unresolved since March.",
         "A count without its meaning can be read backwards — as this deck's own card shows."),
        ("Coverage past one district", "Blocking, not polish",
         "The ingest pipeline is already city-wide; the pilot is scoped to Bed-Stuy, not limited "
         "to it. A tool covering 0.1% of a caseload does not get adopted.",
         "Requires a storage rethink past ~40,000 buildings."),
        ("Live data, not a snapshot", "Scheduled refresh",
         "Today the artifact is a point-in-time bundle and nothing re-ingests on a schedule. The "
         "card states the date it was built.",
         "Stated on the card rather than hidden, but still a limitation."),
        ("Read it any way you need", "Shipped",
         "Screen-reader announcements and an in-app text-size control, so a violation history is "
         "readable on a phone, in a hallway, by someone who is not twenty-five.",
         "Accessibility is a pillar in the score and a property of the tool."),
    ],
    GRID4,
    [("Why the moment is small", "The Real Deal on Openigloo", U["openigloo"]),
     ("Try it", "housecheck-wine.vercel.app", U["app"])],
)

# --- validate -----------------------------------------------------------------------
s = io.open(P, encoding="utf-8", errors="replace").read()
body = "".join(SLIDES.values())

classes = set()
for blob in re.findall(r'className:"([^"]+)"', body):
    classes.update(blob.split())
missing = [c for c in sorted(classes)
           if not re.search(r'\.%s(?![\w-])' % re.sub(r'([\[\]\.\(\)])', r'\\\\?\1', c), s)]
print("  classes used: %d" % len(classes))
if missing:
    print("  MISSING FROM COMPILED CSS -> would fail silently:")
    for m in missing:
        print("    -", m)
    sys.exit(1)
print("  all classes present in the compiled CSS")

for fn in SLIDES:
    assert ("function %s(" % fn) not in s, "%s already defined" % fn

OLD = "[AL,VL,nB1,nB2,qL,nB3,lL,pL,nA1,nA2,nA3,nA4,nA5,uL,nB4,hcU1,hcU2,hcU3,nB5,rL]"
NEW = "[AL,VL,nB1,hcD4,hcD5,hcD6,lL,pL,nA1,hcD10,nA3,hcD12,nA5,hcD14,hcD15,hcU1,hcU2,hcU3,nB5,rL]"
assert s.count(OLD) == 1, "registry not unique"
assert len(OLD.strip("[]").split(",")) == len(NEW.strip("[]").split(",")), "slide count changed"

anchor = "function pL(){"
assert s.count(anchor) == 1
s = s.replace(anchor, body + anchor, 1)
s = s.replace(OLD, NEW, 1)

n = len(re.search(r"const OS=\[([^\]]+)\]", s).group(1).split(","))
assert int(re.search(r"\bIS=(\d+),", s).group(1)) == n, "IS disagrees with OS -- nav would clamp"

io.open(P, "w", encoding="utf-8", newline="").write(s)
print("  rebuilt 7 slides; %d total, IS=%d, nav bound intact" % (n, n))
