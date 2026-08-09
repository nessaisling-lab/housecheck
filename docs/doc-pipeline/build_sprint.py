# Solution Design Sprint -> themed .docx
import io
import os
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from docbuild import build, md_to_html, split_front_matter  # noqa: E402

SRC = r"D:\L2 Cycle 4\Housecheck Antonin Idea\docs\classwork\solution-design-sprint.md"
OUT = r"D:\L2 Cycle 4\Housecheck Antonin Idea\docs\classwork"

md = io.open(SRC, encoding="utf-8").read()
body = split_front_matter(md)

cover = (
    '<p class="eyebrow">HOUSECHECK &middot; SOLUTION DESIGN SPRINT</p>'
    '<p class="cover-h">Three Sketches, One Commitment</p>'
    '<p class="lede">The simplest solution, the full-featured one, and the one that inverts who '
    'pays &mdash; then the decision, and an MVP scoped to a single core feature.</p>'
    '<p class="meta">Aisling &middot; Pursuit L2 Cycle 4 &middot; 9 August 2026</p>'
    '<blockquote><p><b>STANDARD USED THROUGHOUT</b></p>'
    '<p>Every figure carries its source, and anything derived rather than measured says so. The '
    'storage arithmetic is explicitly flagged as arithmetic. Where a solution is rejected, the '
    'reason is a named blocker rather than a preference &mdash; and the kill conditions for the '
    'unconventional sketch are stated in full rather than softened.</p></blockquote>'
    '<hr>'
)

doc = build("Three Sketches, One Commitment — HouseCheck solution design sprint",
            cover + md_to_html(body), "HouseCheck-Solution-Design-Sprint", OUT)
print("wrote %s (%.0f KB)" % (doc, os.path.getsize(doc) / 1024))
