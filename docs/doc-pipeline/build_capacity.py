# Capacity & ceilings reflection -> themed .docx
import io, os, sys
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from docbuild import build, md_to_html, split_front_matter  # noqa: E402

SRC = r"D:\L2 Cycle 4\Housecheck Antonin Idea\docs\reflection\capacity-and-ceilings.md"
OUT = r"D:\L2 Cycle 4\Housecheck Antonin Idea\docs\reflection"
md = io.open(SRC, encoding="utf-8").read()
cover = (
    '<p class="eyebrow">L2 CYCLES 1&ndash;4 &middot; CAPACITY GAME THEORY</p>'
    '<p class="cover-h">What Could These<br>Actually Hold?</p>'
    '<p class="lede">If each project got a foothold &mdash; who would the users be, how many, '
    'and what would the code do under them?</p>'
    '<p class="meta">Aisling &middot; 9 August 2026</p>'
    '<blockquote><p><b>THE ONE-LINE FINDING</b></p><p>In three of four projects there is no '
    'capacity ceiling because there is no server. In the fourth there is a server, and capacity '
    'is not its limit &mdash; coverage and cost are, by two orders of magnitude.</p></blockquote>'
    '<hr>'
)
doc = build("What Could These Actually Hold?", cover + md_to_html(split_front_matter(md)),
            "L2-Capacity-And-Ceilings", OUT)
print("wrote %s (%.0f KB)" % (doc, os.path.getsize(doc) / 1024))
