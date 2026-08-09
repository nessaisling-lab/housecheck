# Industry Research Notes -> themed .docx
import io
import os
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from docbuild import build, md_to_html  # noqa: E402

SRC = r"D:\L2 Cycle 4\Housecheck Antonin Idea\docs\classwork\industry-research-notes.md"
OUT = r"D:\L2 Cycle 4\Housecheck Antonin Idea\docs\classwork"

md = io.open(SRC, encoding="utf-8").read()
# Drop the H1 and lead block; the cover below replaces them.
body = md.split("---", 1)[1] if md.startswith("#") and "---" in md else md

cover = (
    '<p class="eyebrow">HOUSECHECK &middot; INDUSTRY RESEARCH NOTES</p>'
    '<p class="cover-h">Residential Rental<br>Transparency</p>'
    '<p class="lede">Property data pointed at the tenant, not the owner &mdash; what exists, '
    'who it serves, and the gaps a renter falls through.</p>'
    '<p class="meta">Researched 29 July 2026 &middot; extended 8 August 2026</p>'
    '<p class="meta">Living document. The open questions in &sect;7 are the current edge, '
    'not a closing summary.</p>'
    '<blockquote><p><b>STANDARD USED THROUGHOUT</b></p>'
    '<p>Every figure carries its source, and confidence is stated where a source is weak. '
    'Nothing here is estimated and then presented as measured. Claims from our own build are '
    'marked and can be checked against a live URL.</p></blockquote>'
    '<hr>'
)

doc = build("Residential Rental Transparency — Research Notes",
            cover + md_to_html(body), "HouseCheck-Industry-Research-Notes", OUT)
print("wrote %s (%.0f KB)" % (doc, os.path.getsize(doc) / 1024))
