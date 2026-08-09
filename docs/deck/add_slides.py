# Add three audience slides to the compiled deck bundle.
#
# The bundle ships a fixed, precompiled Tailwind set. A class that is not in that set
# fails SILENTLY -- no error, no style. So every class used below is extracted and
# checked against the compiled CSS before a single byte is written.
import io
import re
import sys

P = r"D:\L2 Cycle 4\Housecheck Antonin Idea\docs\deck\HouseCheck-Presentation.html"

# --- the three slides -------------------------------------------------------------
# Layout note: only grid-cols-4 exists in this bundle. grid-cols-2 and grid-cols-3 are
# absent, so anything that is not four-across uses flex-1, the way the Integrity slide does.

H2 = ('K.jsxs("h2",{className:"font-semibold leading-tight tracking-[-0.035em] mt-5 mb-10",'
      'style:{fontSize:"clamp(40px, 5vw, 60px)",color:aA},children:[%s," ",'
      'K.jsx("span",{style:{color:cD},children:%s})]})')

SHELL = ('function %s(){%sreturn K.jsxs("div",{className:"relative w-full h-full overflow-hidden",'
         'style:{backgroundColor:"transparent"},children:[K.jsx(Xq,{}),'
         'K.jsxs("div",{className:"relative z-10 w-full h-full flex flex-col",'
         'style:{padding:"52px 64px 44px"},children:[K.jsx(yl,{children:%s}),%s,%s,%s]})]})}')

QUOTE = ('K.jsx("div",{className:"rounded-2xl px-7 py-6 mt-6",'
         'style:{backgroundColor:aA,borderLeft:"5px solid "+cA},'
         'children:K.jsx("p",{className:"font-semibold leading-tight tracking-[-0.02em]",'
         'style:{fontSize:%d,color:cA},children:%s})})')

# Panels laid out with flex-1 (2 or 3 across).
FLEX = ('K.jsx("div",{className:"grow flex items-stretch gap-6",children:R.map(E=>'
        'K.jsxs("div",{className:"flex-1 rounded-2xl p-5 flex flex-col justify-center",'
        'style:{backgroundColor:"rgba(255,255,255,0.42)",border:"1px solid rgba(28,28,30,0.08)"},'
        'children:[K.jsx("p",{className:"font-semibold text-2xl",style:{color:aA},children:E.k}),'
        'K.jsx("p",{className:"text-lg font-semibold mt-1",style:{color:cD},children:E.when}),'
        'K.jsx("p",{className:"text-lg leading-relaxed mt-3",style:{color:WA},children:E.body})]},E.k))})')

# Four across, reusing the exact card shape from the "We Show Our Work" slide.
GRID4 = ('K.jsx("div",{className:"grow grid grid-cols-4 gap-4",children:R.map(E=>'
         'K.jsxs("div",{className:"rounded-2xl p-5 flex flex-col justify-center",'
         'style:{backgroundColor:"rgba(255,255,255,0.42)",border:"1px solid rgba(28,28,30,0.08)"},'
         'children:[K.jsx("p",{className:"font-semibold text-base mb-1",style:{color:aA},children:E.k}),'
         'K.jsx("p",{className:"text-lg font-semibold",style:{color:cD},children:E.when}),'
         'K.jsx("p",{className:"text-lg leading-relaxed mt-3",style:{color:WA},children:E.body})]},E.k))})')

LEDE = ('K.jsx("p",{className:"text-lg leading-relaxed mb-8",style:{color:WA,maxWidth:1040},children:%s})')

def js(s):
    """A JS double-quoted string literal."""
    return '"%s"' % s.replace("\\", "\\\\").replace('"', '\\"')


def arr(items):
    return "const R=[%s];" % ",".join(
        "{k:%s,when:%s,body:%s}" % (js(k), js(w), js(b)) for k, w, b in items
    )


# ---- Slide A: both audiences, side by side --------------------------------------
A = SHELL % (
    "hcU1",
    arr([
        ("The renter", "Once every few years",
         "About 33,210 units are on the market at any moment, so roughly one renter in seventy-five "
         "can act on this at all. They get fifteen minutes in a hallway and commit around $40,000 "
         "over the next year."),
        ("The professional", "Twice a day",
         "Tenant lawyers, Legal Aid staff, organisers and housing court staff. Hundreds citywide at a "
         "countable number of named organisations, opening the same records on every case."),
    ]),
    js("Who It's For"),
    H2 % (js("One record."), js("Two very different clocks.")),
    LEDE % js("The renter is the mission and the volume. The professional is the one who finds every "
              "gap in the product, because they are the only one who opens it daily."),
    FLEX + "," + QUOTE % (31, js("The people who need it most often are not the people there are most of.")),
)

# ---- Slide B: the daily user, in detail ------------------------------------------
B = SHELL % (
    "hcU2",
    arr([
        ("Who", "Housing attorney",
         "A Legal Aid lawyer or paralegal preparing an HP action to force a landlord to make repairs."),
        ("When", "Intake, then drafting",
         "Twice on the same matter, and several times a day across a caseload."),
        ("What breaks", "The count is the answer",
         "Every tool reports seven open Class C. A petition cannot plead a total; it has to name "
         "conditions, dates and units."),
        ("What they do now", "Copy it out by hand",
         "Open HPD Online, key in the BBL, page through the notices, transcribe the descriptions. "
         "Once per client, and it never compounds."),
    ]),
    js("The Daily User"),
    H2 % (js("A count cannot"), js("go in a filing.")),
    LEDE % js("HPD already publishes the text of every notice of violation. No product in this "
              "landscape surfaces it, ours included, so the meaning is fetched by hand."),
    GRID4 + "," + QUOTE % (28, js("Seven open hazardous violations, but not what they are.")),
)

# ---- Slide C: the bet, including the part that does not work yet -----------------
C = SHELL % (
    "hcU3",
    arr([
        ("Frequency finds bugs", "Design for them",
         "Someone who opens this twice a day finds every gap in a month. A renter cannot ask for "
         "violation descriptions, because they do not know the field exists."),
        ("A superset, not a fork", "Build the harder one",
         "Descriptions, coverage, history and export cover everything the renter card needs. The "
         "renter version is a subtraction from it rather than a rewrite."),
        ("And they cannot pay", "Named honestly",
         "They are grant-funded and sit at the top of the alignment gradient: closest to the tenant, "
         "least able to buy. We are charging neither group yet, on purpose."),
    ]),
    js("The Bet"),
    H2 % (js("Design for the daily user."), js("Reach the renter.")),
    LEDE % js("Two comparables spent twelve years discovering that willingness to pay runs opposite "
              "to alignment. Choosing this user is a product decision, not a revenue one."),
    FLEX + "," + QUOTE % (31, js("Reach was never the constraint. Timing was.")),
)

# --- validate every class against the compiled CSS --------------------------------
s = io.open(P, encoding="utf-8", errors="replace").read()

classes = set()
for blob in re.findall(r'className:"([^"]+)"', A + B + C):
    classes.update(blob.split())

missing = []
for c in sorted(classes):
    # Tailwind escapes special chars in the emitted selector: .tracking-\[-0\.035em\]
    esc = re.sub(r'([\[\]\.\(\)])', r'\\\\?\1', c)
    if not re.search(r'\.%s(?![\w-])' % esc, s):
        missing.append(c)

print("  classes used: %d" % len(classes))
if missing:
    print("  MISSING FROM COMPILED CSS -> would fail silently:")
    for m in missing:
        print("    -", m)
    sys.exit(1)
print("  all classes present in the compiled CSS")

# --- insert -----------------------------------------------------------------------
OLD_REG = "[AL,VL,nB1,nB2,qL,nB3,lL,pL,nA1,nA2,nA3,nA4,nA5,uL,nB4,nB5,rL]"
NEW_REG = "[AL,VL,nB1,nB2,qL,nB3,lL,pL,nA1,nA2,nA3,nA4,nA5,uL,nB4,hcU1,hcU2,hcU3,nB5,rL]"
assert s.count(OLD_REG) == 1, "registry not unique: %d" % s.count(OLD_REG)
for name in ("hcU1", "hcU2", "hcU3"):
    assert ("function %s(" % name) not in s, "%s already defined" % name

anchor = "function pL(){"
assert s.count(anchor) == 1
s = s.replace(anchor, A + B + C + anchor, 1)
s = s.replace(OLD_REG, NEW_REG, 1)

# `IS` is a HARDCODED slide count, sitting in a const chain declared *before* OS, so it
# cannot be written as OS.length. Keyboard and wheel navigation clamp on it:
#     Math.min(IS-1, x+o)
# Miss it and the deck silently refuses to advance past the old final slide, while the
# dots keep working because they set the index directly -- so a dot-based check passes
# and a presenter walking the deck with arrow keys hits a wall. That is exactly how it
# shipped the first time. Assert agreement rather than trusting it.
m = re.search(r"IS=(\d+),", s)
assert m, "could not find the IS slide-count constant"
n_slides = len(re.search(r"const OS=\[([^\]]+)\]", s).group(1).split(","))
s = s[:m.start(1)] + str(n_slides) + s[m.end(1):]
assert int(re.search(r"IS=(\d+),", s).group(1)) == n_slides, "IS still disagrees with OS"
print("  IS synced to OS.length = %d (keyboard/scroll nav bound)" % n_slides)

io.open(P, "w", encoding="utf-8", newline="").write(s)
print("  inserted 3 slides -> 20 total")
