# Product Requirements Document -> themed .docx
import io
import os
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from docbuild import build, md_to_html, split_front_matter  # noqa: E402

SRC = r"D:\L2 Cycle 4\Housecheck Antonin Idea\docs\PRD.md"
OUT = r"D:\L2 Cycle 4\Housecheck Antonin Idea\docs"

md = io.open(SRC, encoding="utf-8").read()
body = split_front_matter(md)

cover = (
    '<p class="eyebrow">HOUSECHECK &middot; PRODUCT REQUIREMENTS DOCUMENT &middot; v2.0</p>'
    '<p class="cover-h">Know the Building Before You Sign</p>'
    '<p class="lede">A building&rsquo;s condition is a matter of public record, and the people '
    'whose safety, money or legal case depends on that record cannot use it at the moment they '
    'need it.</p>'
    '<p class="meta">Aisling Leiva-Davila &middot; Antonin &middot; Jagger &middot; '
    'Pursuit L2 Cycle 4 &middot; 9 August 2026</p>'
    '<blockquote><p><b>WHAT CHANGED IN v2.0</b></p>'
    '<p>The capstone version aimed at renters. This one commits to the housing advocate as the '
    'primary design target and the renter as the audience, because the professional&rsquo;s '
    'requirements are a superset of the renter&rsquo;s and building the harder one first makes '
    'the simpler one a subtraction rather than a rewrite. Rewritten after the industry research, '
    'market framing, problem definition and solution design sprint, all of which are cited '
    'rather than summarised. Every figure carries its source, and anything derived rather than '
    'measured says so.</p></blockquote>'
    '<hr>'
)

doc = build("HouseCheck — Product Requirements Document v2.0",
            cover + md_to_html(body), "HouseCheck-PRD-v2", OUT)
print("wrote %s (%.0f KB)" % (doc, os.path.getsize(doc) / 1024))
