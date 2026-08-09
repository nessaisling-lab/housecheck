# Make slides 8 and 20 fit a 1280x720 projector.
#
# These two clipped before any of this session's work -- they are original slides, not
# rebuilt ones. The brief is "fill the deck out more", so the fix reduces SPACING and
# decorative scale, never content. No sentence is removed from either slide.
#
# Edits are applied inside a single function body and then swapped in as one unique
# substring, so an anchor like "gap-10" cannot match a neighbouring slide by accident.
import io
import re
import sys

P = r"D:\L2 Cycle 4\Housecheck Antonin Idea\docs\deck\HouseCheck-Presentation.html"


def fn_body(s, name):
    i = s.find("function %s(){" % name)
    assert i >= 0, "%s not found" % name
    j = s.find("function ", i + 10)
    return s[i:j if j > 0 else len(s)]


def patch_in(s, name, edits):
    body = fn_body(s, name)
    new = body
    for old, rep, why in edits:
        n = new.count(old)
        assert n == 1, "%s: anchor %r occurs %d times in this function" % (name, old, n)
        new = new.replace(old, rep, 1)
        print("    %-34s %s" % (old[:34], why))
    assert s.count(body) == 1, "%s body is not unique in the file" % name
    return s.replace(body, new, 1)


s = io.open(P, encoding="utf-8", errors="replace").read()

# --- slide 8, "We Show Our Work" -- clipped by 71px at 720 -------------------------
# The screenshot carries width:100% and NO height cap, so its intrinsic aspect ratio sets
# the height of the bottom row and nothing bounds it. Capping the height with a contain fit
# is the fix; the rest is margin.
print("  slide 8 (pL):")
s = patch_in(s, "pL", [
    ('display:"block",width:"100%"',
     'display:"block",width:"100%",maxHeight:250,objectFit:"contain"',
     "screenshot: cap height at 250px, contain fit"),
    ('tracking-[-0.035em] mt-5 mb-10', 'tracking-[-0.035em] mt-4 mb-5',
     "heading margins 5/10 -> 4/5"),
    ('grid grid-cols-4 gap-4 mb-8', 'grid grid-cols-4 gap-4 mb-4',
     "source-card row mb-8 -> mb-4"),
    ('flex items-center gap-8 mt-6', 'flex items-center gap-8 mt-4',
     "bottom row mt-6 -> mt-4"),
    ('padding:"52px 64px 44px"', 'padding:"44px 64px 30px"',
     "slide padding 52/44 -> 44/30"),
])

# --- slide 20, the close -- clipped by 89px at 720 ---------------------------------
# A centred stack: heading, team avatars, wordmark, footer. The logo is decorative and
# is the cheapest 80px in the deck.
print("  slide 20 (rL):")
s = patch_in(s, "rL", [
    ('items-center justify-center gap-10', 'items-center justify-center gap-6',
     "stack gap-10 -> gap-6"),
    ("svg\",{width:320", "svg\",{width:180",
     "wordmark 320 -> 180px"),
    ('fontSize:"clamp(52px, 6.5vw, 82px)"', 'fontSize:"clamp(46px, 5.6vw, 68px)"',
     "closing headline max 82 -> 68px"),
    # NOTE: this stack is justify-center, so trimming padding does nothing once the
    # content is taller than the viewport -- it simply overflows both ends equally.
    # Only reducing actual content height moves it, which is why the wordmark shrinks.
    ('padding:"52px 64px"', 'padding:"36px 64px"',
     "slide padding 52 -> 36"),
])

# The classes introduced must exist in the precompiled CSS, or they fail silently.
for c in ["mt-4", "mb-5", "mb-4", "gap-6"]:
    if not re.search(r"\.%s(?![\w-])" % c, s):
        print("  MISSING CLASS:", c)
        sys.exit(1)
print("  introduced classes all present in compiled CSS")

n = len(re.search(r"const OS=\[([^\]]+)\]", s).group(1).split(","))
assert int(re.search(r"\bIS=(\d+),", s).group(1)) == n, "IS/OS drift"

io.open(P, "w", encoding="utf-8", newline="").write(s)
print("  written; %d slides, IS=%d" % (n, n))
