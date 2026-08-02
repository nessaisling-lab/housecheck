# Standalone portable SVG: one file, no script, no external references.
# Committed light palette with an opaque background so it drops predictably into slides,
# email, a README, or a PDF. Text is real <text>, so it stays selectable and scales.
#
# Layout is three columns — annotations | isometric stack | per-layer labels — because
# every slab's left corner sits at x = -W*cos30 and margin notes placed naively land on
# the geometry. Column x positions are derived from the geometry, not guessed.
import io
import json
import os

HERE = os.path.dirname(os.path.abspath(__file__))
G = json.load(io.open(os.path.join(HERE, "iso_geom.json"), encoding="utf-8"))
OUT = os.path.join(HERE, "housecheck-architecture.svg")

INK, SLATE, ACC, PAPER = "#14161C", "#5C6472", "#C2321B", "#F7F6F3"
FILL = {"top": "#FFFFFF", "left": "#E7E5E0", "right": "#D6D3CC"}
HOT = {"top": "#FDEEEA", "left": "#F5D8D0", "right": "#EAC3B8"}
SANS = "ui-sans-serif,system-ui,'Segoe UI',Helvetica,Arial,sans-serif"
MONO = "ui-monospace,Menlo,Consolas,'Courier New',monospace"


def esc(s):
    return s.replace("&", "&amp;").replace("<", "&lt;").replace(">", "&gt;")


def txt(x, y, s, size=12, fill=INK, fam=None, weight=None, anchor=None, ls=None):
    a = ['x="%.1f" y="%.1f" font-size="%s"' % (x, y, size), 'fill="%s"' % fill]
    if fam:
        a.append('font-family="%s"' % fam)
    if weight:
        a.append('font-weight="%s"' % weight)
    if anchor:
        a.append('text-anchor="%s"' % anchor)
    if ls:
        a.append('letter-spacing="%s"' % ls)
    return "<text %s>%s</text>" % (" ".join(a), esc(s))


vb = [float(v) for v in G["vb"].split()]
VW, VH = vb[2], vb[3]
L_COL = vb[0] + 36                       # annotation column, clear of the stack
STACK_LEFT = min(float(pt.split(",")[0])
                 for Lr in G["layers"] for pt in Lr["left"].split())
COL_W = STACK_LEFT - L_COL - 46          # usable width before the geometry starts

p = ['<svg xmlns="http://www.w3.org/2000/svg" viewBox="%s" width="%d" height="%d" '
     'font-family="Georgia,\'Times New Roman\',serif" role="img" aria-labelledby="t d">'
     % (G["vb"], int(VW), int(VH)),
     '<title id="t">HouseCheck architecture: six layers from municipal sources to a React '
     'client</title>',
     '<desc id="d">An isometric stack of six layers. Eight municipal data sources feed a '
     'one-time ingest that runs on a laptop. The ingest writes a 1,240 KB SQLite artifact. '
     'The artifact is baked into a 29 MB Docker image. The image runs as an axum API on '
     'Fly.io with scale-to-zero. A React client on Vercel reads it. Everything above the '
     'artifact happens once and touches the network; everything below is deterministic and '
     'ships.</desc>',
     '<rect x="%.1f" y="%.1f" width="%.1f" height="%.1f" fill="%s"/>'
     % (vb[0], vb[1], VW, VH, PAPER)]

# ── masthead, top of the left column ───────────────────────────────────────
my = vb[1] + 54
p += [txt(L_COL, my, "HOUSECHECK · BUILD ARCHITECTURE", 11.5, SLATE, SANS, ls="2.2"),
      txt(L_COL, my + 36, "One artifact,", 30, INK, weight="600"),
      txt(L_COL, my + 68, "six layers.", 30, INK, weight="600"),
      txt(L_COL, my + 96, "Nothing here is fast, because", 14, SLATE),
      txt(L_COL, my + 114, "nothing needed to be.", 14, SLATE)]

# ── connectors first, so the slabs sit on top ──────────────────────────────
p.append('<g stroke="%s" stroke-width="2" stroke-dasharray="3 4" opacity=".45">' % SLATE)
for c in G["conns"]:
    p.append('<line x1="%.1f" y1="%.1f" x2="%.1f" y2="%.1f"/>'
             % (c["x1"], c["y1"], c["x2"], c["y2"]))
p.append("</g>")

# ── the stack, with its labels ─────────────────────────────────────────────
for Lr in G["layers"]:
    f = HOT if Lr["hot"] else FILL
    st = ACC if Lr["hot"] else INK
    op = ".55" if Lr["hot"] else ".22"
    p.append('<g stroke="%s" stroke-opacity="%s" stroke-width="1.25" stroke-linejoin="round">'
             % (st, op))
    p.append('<polygon points="%s" fill="%s"/>' % (Lr["right"], f["right"]))
    p.append('<polygon points="%s" fill="%s"/>' % (Lr["left"], f["left"]))
    p.append('<polygon points="%s" fill="%s"/>' % (Lr["top"], f["top"]))
    p.append("</g>")

    lx, ly = Lr["lx"], Lr["ly"]
    p.append('<line x1="%.1f" y1="%.1f" x2="%.1f" y2="%.1f" stroke="%s" stroke-opacity=".35" '
             'stroke-width="1"/>' % (lx - 22, ly, lx - 7, ly, SLATE))
    p.append(txt(lx, ly + 1, Lr["title"], 17, ACC if Lr["hot"] else INK, weight="600"))
    p.append(txt(lx, ly + 18, Lr["sub"], 10.5, SLATE, MONO))
    for k, d in enumerate(Lr["det"]):
        p.append(txt(lx, ly + 37 + k * 15, d, 11.5, INK))

by = G["layers"][0]                       # sources
ing = G["layers"][1]                      # ingest
art = G["layers"][2]                      # artifact
rt = G["layers"][4]                       # runtime

# ── the boundary that matters, in the left column beside ingest/artifact ───
mid = (ing["cy"] + art["cy"]) / 2
p.append('<line x1="%.1f" y1="%.1f" x2="%.1f" y2="%.1f" stroke="%s" stroke-width="1.5" '
         'stroke-dasharray="7 5" opacity=".8"/>'
         % (L_COL, mid + 34, STACK_LEFT + 158, mid - 46, ACC))
p += [txt(L_COL, mid + 60, "THE LINE THE DESIGN TURNS ON", 11, ACC, SANS, "600", ls="1.5"),
      txt(L_COL, mid + 82, "Above — runs once, on a laptop,", 12.5, INK),
      txt(L_COL, mid + 99, "over the network. Not reproducible.", 12.5, INK),
      txt(L_COL, mid + 122, "Below — deterministic, offline,", 12.5, INK),
      txt(L_COL, mid + 139, "and shipped. Same answer in 2030.", 12.5, INK)]

# ── the two pure crates ────────────────────────────────────────────────────
py = rt["cy"] - 6
p.append('<rect x="%.1f" y="%.1f" width="%.1f" height="92" rx="7" fill="#FFFFFF" '
         'stroke="%s" stroke-opacity=".2"/>' % (L_COL, py, COL_W, INK))
p.append(txt(L_COL + 15, py + 24, "NO I/O · NO RESULT · NO CLOCK", 9.5, SLATE, SANS, ls="1.5"))
for k, (name, meta) in enumerate(G["pure"].items()):
    p.append(txt(L_COL + 15, py + 48 + k * 21, name, 12.5, INK, MONO, "600"))
    p.append(txt(L_COL + 86, py + 48 + k * 21, meta, 11, SLATE))
p.append(txt(L_COL, py + 116, "The two crates the whole score", 12, SLATE))
p.append(txt(L_COL, py + 133, "is computed in.", 12, SLATE))

# ── the one runtime egress, drawn off the stack's left face ────────────────
oy = rt["cy"] + 4
p.append('<line x1="%.1f" y1="%.1f" x2="%.1f" y2="%.1f" stroke="%s" stroke-width="1.5" '
         'stroke-dasharray="4 4" opacity=".55"/>'
         % (rt["cx"] + 210, oy, rt["cx"] + 286, oy, SLATE))
p.append(txt(rt["cx"] + 294, oy + 1, "OpenRouter", 12.5, INK, weight="600"))
p.append(txt(rt["cx"] + 294, oy + 16, "the only call the runtime makes", 10.5, SLATE))

p.append(txt(L_COL, vb[1] + VH - 26,
             "250 buildings · 26,306 violations · snapshot 2026 · measured August 2026",
             10, SLATE, MONO))
p.append("</svg>")

svg = "\n".join(p)
io.open(OUT, "w", encoding="utf-8").write(svg)
print("wrote %s  (%.1f KB)" % (os.path.basename(OUT), len(svg) / 1024))
print("  viewBox %s" % G["vb"])
print("  left column x=%.0f width=%.0f   stack starts x=%.0f" % (L_COL, COL_W, STACK_LEFT))
