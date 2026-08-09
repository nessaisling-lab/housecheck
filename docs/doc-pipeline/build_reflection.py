# L2 arc reflection -> themed .docx
import io
import os
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from docbuild import build, md_to_html, split_front_matter  # noqa: E402

SRC = r"D:\L2 Cycle 4\Housecheck Antonin Idea\docs\reflection\l2-arc.md"
OUT = r"D:\L2 Cycle 4\Housecheck Antonin Idea\docs\reflection"

md = io.open(SRC, encoding="utf-8").read()
body = split_front_matter(md)

cover = (
    '<p class="eyebrow">L2 CYCLES 1&ndash;4 &middot; A LOOK BACK AT THE CODE</p>'
    '<p class="cover-h">Four Cycles,<br>One Missing Piece</p>'
    '<p class="lede">Resona, SiteAssure, Ziqpu, HouseCheck &mdash; read as one arc, from the '
    'source rather than from memory.</p>'
    '<p class="meta">Aisling &middot; 9 August 2026</p>'
    '<blockquote><p><b>HOW THIS WAS PRODUCED</b></p>'
    '<p>Each of the four codebases was profiled independently against the same questions: who '
    'is this built for, and what in the code proves it. The arc was traced afterwards. Every '
    'claim is grounded in a file, a line, or a measured count, and where the evidence did not '
    'support a tidy story it says so &mdash; the reversal in Cycle 2 is real and is not '
    'smoothed over.</p></blockquote>'
    '<hr>'
)

doc = build("Four Cycles, One Missing Piece", cover + md_to_html(body), "L2-Arc-Reflection", OUT)
print("wrote %s (%.0f KB)" % (doc, os.path.getsize(doc) / 1024))
