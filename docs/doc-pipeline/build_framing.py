# Market Framing Notes -> themed .docx
import io
import os
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from docbuild import build, md_to_html, split_front_matter  # noqa: E402

SRC = r"D:\L2 Cycle 4\Housecheck Antonin Idea\docs\classwork\market-framing-notes.md"
OUT = r"D:\L2 Cycle 4\Housecheck Antonin Idea\docs\classwork"

md = io.open(SRC, encoding="utf-8").read()
body = split_front_matter(md)

cover = (
    '<p class="eyebrow">HOUSECHECK &middot; MARKET FRAMING NOTES</p>'
    '<p class="cover-h">Past One User</p>'
    '<p class="lede">Who else has this problem, roughly how many of them there are, and what '
    'they do instead today.</p>'
    '<p class="meta">Aisling &middot; Pursuit L2 Cycle 4 &middot; 9 August 2026</p>'
    '<blockquote><p><b>STANDARD USED THROUGHOUT</b></p>'
    '<p>Every figure carries its source. Where a number is derived rather than sourced, it '
    'says so. Earlier cycles are not covered, because the evidence for them is not in reach '
    'and reconstructing it from memory would not meet the standard the rest of these notes '
    'are held to.</p></blockquote>'
    '<hr>'
)

doc = build("Past One User — HouseCheck market framing notes",
            cover + md_to_html(body), "HouseCheck-Market-Framing-Notes", OUT)
print("wrote %s (%.0f KB)" % (doc, os.path.getsize(doc) / 1024))
