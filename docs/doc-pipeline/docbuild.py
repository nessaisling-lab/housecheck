# Shared HouseCheck document pipeline: markdown -> themed HTML -> .docx via LibreOffice.
#
# One theme, one converter, every document. The palette is derived from the app's own tokens
# by theme.py rather than eyeballed, and the band colours are re-solved for a light ground
# because a colour is only accessible against a stated surface.
import html
import io
import os
import re
import subprocess

from theme import APP, INK, PAPER, darken_to, hexs

SOFFICE = r"C:\Program Files\LibreOffice\program\soffice.exe"
HERE = os.path.dirname(os.path.abspath(__file__))

# Single font names: a CSS fallback list survives the conversion literally as "Georgia;serif".
SERIF, SANS, MONO = "Georgia", "Segoe UI", "Consolas"

PRINT = {k: hexs(darken_to(APP[k], PAPER)) for k in
         ("strong", "solid", "mixed", "concern", "critical", "unverified")}

CSS = """
@page {{ size: Letter; margin: 2.0cm 2.2cm 2.2cm 2.2cm; }}
body   {{ font-family: {serif}; font-size: 10.5pt; color: {ink}; background: {paper};
          line-height: 1.44; }}
h1     {{ font-family: {sans}; font-size: 20pt; color: {ink}; margin: 0 0 4pt 0;
          page-break-before: always; page-break-after: avoid; }}
h1.first {{ page-break-before: avoid; }}
h2     {{ font-family: {sans}; font-size: 13.5pt; color: {card}; margin: 16pt 0 4pt 0;
          page-break-after: avoid; }}
h3     {{ font-family: {sans}; font-size: 11.5pt; color: {card}; margin: 12pt 0 3pt 0;
          page-break-after: avoid; }}
p      {{ margin: 0 0 7pt 0; }}
ul, ol {{ margin: 0 0 8pt 0; padding-left: 16pt; }}
li     {{ margin-bottom: 3pt; }}
a      {{ color: {accent}; }}
/* No background chip. LibreOffice collapses every `code` into one character style,
   SourceText, with a hard-coded light fill -- which inside a dark table header renders
   near-white on near-white (~1.09:1). A `th code` selector cannot override it because the
   character style is shared. Consolas alone distinguishes code on paper. */
code   {{ font-family: {mono}; font-size: 9pt; }}
/* Light, not the dark card. LibreOffice applies a block background to the paragraph but
   does not push the block's `color` down into the runs, so a dark code block renders
   dark-on-dark and unreadable. Light ground also survives photocopying and does not eat
   toner across 77 pages. The dark card stays on table headers, where shading works. */
pre    {{ font-family: {mono}; font-size: 8.5pt; background: {codebg}; color: {ink};
          padding: 7pt 9pt; margin: 8pt 0; line-height: 1.35;
          border-left: 3pt solid {accent}; }}
pre code {{ background: transparent; color: {ink}; font-size: 8.5pt; }}
table  {{ border-collapse: collapse; width: 100%; margin: 8pt 0; font-family: {sans};
          font-size: 9pt; }}
th     {{ background: {card}; color: {cardink}; text-align: left; padding: 5pt 7pt;
          font-weight: bold; }}
td     {{ border-bottom: 0.5pt solid {rule}; padding: 4pt 7pt; vertical-align: top; }}
blockquote {{ margin: 8pt 0 8pt 0; padding: 6pt 12pt; background: {canvas};
              border-left: 3pt solid {accent}; font-style: normal; }}
blockquote p {{ margin: 0 0 4pt 0; }}
hr     {{ border: 0; border-top: 0.5pt solid {rule}; margin: 14pt 0; }}
.eyebrow {{ font-family: {sans}; font-size: 8pt; color: {muted};
            letter-spacing: 1.4pt; margin: 0 0 6pt 0; }}
.lede    {{ font-size: 12pt; color: {card}; margin: 0 0 14pt 0; }}
.cover-h {{ font-family: {sans}; font-size: 30pt; color: {ink}; margin: 0 0 8pt 0;
            page-break-before: avoid; }}
.pagebreak {{ page-break-before: always; }}
.meta    {{ font-family: {sans}; font-size: 8.5pt; color: {muted}; margin: 0 0 3pt 0; }}
.callout {{ background: {codebg}; color: {ink}; padding: 9pt 12pt; margin: 10pt 0;
            border-left: 3pt solid {accent}; }}
.callout p {{ margin: 0 0 5pt 0; color: {ink}; }}
.callout .k {{ font-family: {sans}; font-size: 8pt; letter-spacing: 1.2pt;
               color: {accent}; margin: 0 0 4pt 0; }}
.b-strong {{ color: {p_strong}; font-weight: bold; }}
.b-mixed  {{ color: {p_mixed}; font-weight: bold; }}
.b-concern{{ color: {p_concern}; font-weight: bold; }}
.b-crit   {{ color: {p_critical}; font-weight: bold; }}
.b-unver  {{ color: {p_unverified}; font-weight: bold; }}
""".format(
    serif=SERIF, sans=SANS, mono=MONO,
    ink=hexs(INK), paper=hexs(PAPER), card=hexs(APP["card"]), cardink=hexs(APP["ink"]),
    canvas=hexs(APP["canvas"]), rule="#E3E3E6", muted="#6A6A72", codebg="#F0F0F2",
    accent=PRINT["strong"], strong=hexs(APP["strong"]),
    p_strong=PRINT["strong"], p_mixed=PRINT["mixed"], p_concern=PRINT["concern"],
    p_critical=PRINT["critical"], p_unverified=PRINT["unverified"],
)


def inline(s):
    spans = []

    def stash(m):
        spans.append(m.group(1))
        return "\x00%d\x00" % (len(spans) - 1)

    s = re.sub(r"`([^`]+)`", stash, s)
    s = html.escape(s, quote=False)
    s = re.sub(r"\[([^\]]+)\]\(([^)]+)\)",
               lambda m: '<a href="%s">%s</a>' % (html.escape(m.group(2), quote=True), m.group(1)), s)
    s = re.sub(r"\*\*([^*]+)\*\*", r"<b>\1</b>", s)
    s = re.sub(r"(?<![\w*])\*([^*\n]+)\*(?![\w*])", r"<i>\1</i>", s)
    s = re.sub(r"\x00(\d+)\x00",
               lambda m: "<code>%s</code>" % html.escape(spans[int(m.group(1))], quote=False), s)
    return s


def cells(row):
    # Split on unescaped pipes only. A markdown-escaped `\|` inside a cell (the ledger row in
    # chapter 14 uses one) otherwise produced an extra column, shifting every later cell right
    # and leaving unbalanced ** and ` as literal text.
    parts = re.split(r"(?<!\\)\|", row.strip().strip("|"))
    return [c.strip().replace("\\|", "|") for c in parts]


def md_to_html(md):
    lines = md.split("\n")
    out, i = [], 0
    while i < len(lines):
        ln = lines[i]
        if ln.startswith("```"):
            i += 1
            buf = []
            while i < len(lines) and not lines[i].startswith("```"):
                buf.append(lines[i])
                i += 1
            i += 1
            out.append("<pre><code>%s</code></pre>" % html.escape("\n".join(buf), quote=False))
            continue
        if re.match(r"^\s*(---|\*\*\*)\s*$", ln):
            out.append("<hr>")
            i += 1
            continue
        m = re.match(r"^(#{1,4})\s+(.*)$", ln)
        if m:
            lv = len(m.group(1))
            out.append("<h%d>%s</h%d>" % (lv, inline(m.group(2)), lv))
            i += 1
            continue
        if ln.startswith(">"):
            buf = []
            while i < len(lines) and lines[i].startswith(">"):
                buf.append(lines[i].lstrip(">").strip())
                i += 1
            paras = [p for p in re.split(r"\n\s*\n", "\n".join(buf).strip()) if p.strip()]
            out.append("<blockquote>%s</blockquote>"
                       % "".join("<p>%s</p>" % inline(p.replace("\n", " ")) for p in paras))
            continue
        if ln.strip().startswith("|") and i + 1 < len(lines) and re.match(
                r"^\s*\|[\s:|-]+\|\s*$", lines[i + 1]):
            head = cells(ln)
            i += 2
            body = []
            while i < len(lines) and lines[i].strip().startswith("|"):
                body.append(cells(lines[i]))
                i += 1
            th = "".join("<th>%s</th>" % inline(c) for c in head)
            tr = "".join("<tr>%s</tr>" % "".join("<td>%s</td>" % inline(c) for c in r) for r in body)
            out.append('<table width="100%%" cellpadding="6" cellspacing="0">'
                       '<tr>%s</tr>%s</table>' % (th, tr))
            continue
        m = re.match(r"^(\s*)([-*]|\d+\.)\s+(.*)$", ln)
        if m:
            ordered = m.group(2) not in ("-", "*")
            items, cur = [], None
            while i < len(lines):
                mm = re.match(r"^(\s*)([-*]|\d+\.)\s+(.*)$", lines[i])
                if mm:
                    if cur is not None:
                        items.append(cur)
                    cur = mm.group(3)
                    i += 1
                elif lines[i].strip() and lines[i].startswith(("  ", "\t")) and cur is not None:
                    cur += " " + lines[i].strip()
                    i += 1
                else:
                    break
            if cur is not None:
                items.append(cur)
            tag = "ol" if ordered else "ul"
            out.append("<%s>%s</%s>" % (tag, "".join("<li>%s</li>" % inline(x) for x in items), tag))
            continue
        if not ln.strip():
            i += 1
            continue
        buf = []
        while i < len(lines) and lines[i].strip() and not re.match(
                r"^\s*(#{1,4}\s|>|```|\||[-*]\s|\d+\.\s|---\s*$)", lines[i]):
            buf.append(lines[i].strip())
            i += 1
        if buf:
            out.append("<p>%s</p>" % inline(" ".join(buf)))
    return "\n".join(out)


def page(title, body_html):
    return ('<html><head><meta charset="utf-8"><title>%s</title><style>%s</style></head>'
            '<body>%s</body></html>' % (html.escape(title), CSS, body_html))


def to_docx(html_path, outdir):
    r = subprocess.run(
        [SOFFICE, "--headless", "--norestore", "--infilter=HTML (StarWriter)",
         "--convert-to", "docx", "--outdir", outdir, html_path],
        capture_output=True, text=True, timeout=900)
    out = os.path.join(outdir, os.path.splitext(os.path.basename(html_path))[0] + ".docx")
    if not os.path.exists(out):
        raise RuntimeError("conversion failed: %s %s" % (r.stdout[-400:], r.stderr[-400:]))
    return out


def build(title, body_html, stem, outdir):
    hp = os.path.join(outdir, stem + ".html")
    io.open(hp, "w", encoding="utf-8").write(page(title, body_html))
    return to_docx(hp, outdir)
