# Problem Definition Notes -> themed .docx
import io
import os
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from docbuild import build, md_to_html, split_front_matter  # noqa: E402

SRC = r"D:\L2 Cycle 4\Housecheck Antonin Idea\docs\classwork\problem-definition-notes.md"
OUT = r"D:\L2 Cycle 4\Housecheck Antonin Idea\docs\classwork"

md = io.open(SRC, encoding="utf-8").read()
body = split_front_matter(md)

cover = (
    '<p class="eyebrow">HOUSECHECK &middot; PROBLEM DEFINITION NOTES</p>'
    '<p class="cover-h">The Count Is Not the Argument</p>'
    '<p class="lede">Three concrete problems, the one worth committing to, and a problem '
    'statement specific enough to build from.</p>'
    '<p class="meta">Aisling &middot; Pursuit L2 Cycle 4 &middot; 9 August 2026</p>'
    '<blockquote><p><b>STANDARD USED THROUGHOUT</b></p>'
    '<p>Each problem separates what can be evidenced &mdash; a published dataset, a field the '
    'ingest does or does not fetch, a sourced figure &mdash; from what would require primary '
    'contact to assert. No time-cost figures appear anywhere in this document, because none '
    'have been measured, and inventing a plausible one to match the shape of the question is '
    'the exact failure this project exists to argue against.</p></blockquote>'
    '<hr>'
)

doc = build("The Count Is Not the Argument — HouseCheck problem definition notes",
            cover + md_to_html(body), "HouseCheck-Problem-Definition-Notes", OUT)
print("wrote %s (%.0f KB)" % (doc, os.path.getsize(doc) / 1024))
