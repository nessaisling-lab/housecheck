# The 14-chapter audit -> one themed .docx book.
import io
import os
import re
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import docbuild  # noqa: E402
from docbuild import build, inline, md_to_html  # noqa: E402

SRC = r"D:\L2 Cycle 4\Housecheck Antonin Idea\docs\build-book"
OUT = r"D:\L2 Cycle 4\Housecheck Antonin Idea\docs\build-book"

# 8pt clears every one of the 445 code lines; the widest is 97 chars and 8.5pt fits ~95.
docbuild.CSS = docbuild.CSS.replace("font-size: 8.5pt;", "font-size: 8pt;")

PARTS = {
    1: ("PART ONE", "The Constraint",
        "Why every load-bearing decision in this codebase was forced by a trust constraint "
        "rather than a performance one \u2014 and what that bought."),
    4: ("PART TWO", "What Drifted",
        "The places a constraint was written in prose, a comment, or a String instead of in "
        "a type, and drifted from the code it described."),
    8: ("PART THREE", "What the Data Said",
        "Where reading the code stopped being enough, and the findings only a query against "
        "the source could produce."),
    13: ("PART FOUR", "Method",
        "How this book was researched, how often the research was wrong, and the one claim "
        "the whole thing rests on."),
}

chapters = []
for fn in sorted(os.listdir(SRC)):
    m = re.match(r"^(\d{2})-(.+)\.md$", fn)
    if not m:
        continue
    n = int(m.group(1))
    md = io.open(os.path.join(SRC, fn), encoding="utf-8").read()
    title = re.sub(r"^Chapter\s+\d+\s+[\u2014-]\s*", "",
                   re.search(r"^#\s+(.*)$", md, re.M).group(1)).strip()
    q = re.search(r">\s*\*\*The question this chapter answers:\*\*\s*(.*?)(?=\n\s*\n|\n---)",
                  md, re.S)
    question = re.sub(r"\s+", " ", re.sub(r"\s*>\s*", " ", q.group(1))).strip() if q else ""
    body = re.sub(r"^#\s+.*?\n+>\s*\*\*The question.*?\n---\n", "", md, flags=re.S)
    if body == md:
        body = re.sub(r"^#\s+.*$", "", md, count=1, flags=re.M)
    # The running "Next: Chapter N" footers are a web affordance; a bound book has a contents page.
    body = re.sub(r"\n---\n+\*Next: \*\*.*$", "", body, flags=re.S)
    body = re.sub(r"\n---\n+\*End of book\..*$", "", body, flags=re.S)
    chapters.append({"n": n, "title": title, "q": question,
                     "html": md_to_html(body), "words": len(re.findall(r"\w+", md))})

esc = lambda s: s.replace("&", "&amp;").replace("<", "&lt;").replace(">", "&gt;")
total = sum(c["words"] for c in chapters)

# ---- front matter -------------------------------------------------------------------
fm = [
    '<p class="eyebrow">HOUSECHECK &middot; A BUILD AUDIT</p>',
    '<p class="cover-h">Confident,<br>Fabricated Numbers</p>',
    '<p class="lede">Fourteen chapters on building a tenant-facing score in Rust, and on '
    'auditing it hard enough to find out it was wrong.</p>',
    '<p class="meta">Aisling &middot; Pursuit L2 Cycle 4 &middot; August 2026</p>',
    '<p class="meta">%s chapters &middot; %s words</p>' % (len(chapters), "{:,}".format(total)),
    '<div class="callout"><p class="k">THE ARGUMENT, ONCE</p><p>Every load-bearing decision '
    'in this codebase was forced by a trust constraint, not a performance one &mdash; and the '
    'places where the code is weakest are precisely the places where a trust constraint was '
    'expressed in prose, a comment, or a <code>String</code> instead of in a type.</p></div>',
    '<div class="pagebreak"></div><h2>How to read this</h2>',
    '<p>Every chapter opens with the question it answers and closes by stating the strongest '
    'objection to itself and answering it. Those closing sections are the fastest way through '
    'the book if you are looking for the argument rather than the evidence.</p>',
    '<p>Every load-bearing number has a command behind it. The row counts come from '
    '<code>curl</code> against NYC Open Data, the score changes from recomputing 250 buildings, '
    'the contrast ratios from compositing a gradient by hand. Where a claim could not be '
    'verified, the chapter says so.</p>',
    '<p>Chapter 13 audits this book&rsquo;s own research, including five errors it made and a '
    'false claim it put in a chapter title.</p>',
    '<h2>Contents</h2>',
]
rows = []
for c in chapters:
    if c["n"] in PARTS:
        _, pt, _ = PARTS[c["n"]]
        rows.append('<tr><td colspan="2"><b>%s</b></td></tr>' % esc(pt))
    rows.append('<tr><td>%02d</td><td>%s</td></tr>' % (c["n"], inline(c["title"])))
fm.append('<table><tr><th>#</th><th>Chapter</th></tr>%s</table>' % "".join(rows))

# ---- body ---------------------------------------------------------------------------
body_parts = ["".join(fm)]
for c in chapters:
    if c["n"] in PARTS:
        eyebrow, pt, blurb = PARTS[c["n"]]
        body_parts.append(
            '<h1>%s</h1><p class="eyebrow">%s</p><p class="lede">%s</p>'
            % (esc(pt), esc(eyebrow), esc(blurb)))
    body_parts.append('<h1>%d. %s</h1>' % (c["n"], inline(c["title"])))
    if c["q"]:
        body_parts.append('<blockquote><p><b>The question this chapter answers:</b> %s</p>'
                          '</blockquote>' % esc(c["q"]))
    body_parts.append(c["html"])

doc = build("Confident, Fabricated Numbers — a HouseCheck build audit",
            "".join(body_parts), "HouseCheck-Build-Audit", OUT)
print("wrote %s (%.0f KB)" % (doc, os.path.getsize(doc) / 1024))
print("  %d chapters, %s words, %d part dividers" % (len(chapters), "{:,}".format(total), len(PARTS)))
