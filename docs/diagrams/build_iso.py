# Generates the isometric architecture diagram as SVG geometry.
# Written as code rather than hand-authored path data: the projection has to be exact,
# and the same geometry feeds both the standalone .svg and the interactive artifact.
import io
import json
import math
import os

OUT = os.path.dirname(os.path.abspath(__file__))

C30, S30 = math.cos(math.radians(30)), math.sin(math.radians(30))
W, D = 300.0, 190.0          # plane footprint in model units
GAP = 86.0                   # vertical separation between layers
TH = 13.0                    # slab thickness


def iso(x, y, z):
    return ((x - y) * C30, (x + y) * S30 - z)


def slab(z):
    """Top face + two visible side faces of a slab at height z."""
    t = [iso(0, 0, z), iso(W, 0, z), iso(W, D, z), iso(0, D, z)]
    lo = [iso(0, 0, z - TH), iso(W, 0, z - TH), iso(W, D, z - TH), iso(0, D, z - TH)]
    top = " ".join("%.1f,%.1f" % p for p in t)
    left = " ".join("%.1f,%.1f" % p for p in (t[0], t[3], lo[3], lo[0]))
    right = " ".join("%.1f,%.1f" % p for p in (t[3], t[2], lo[2], lo[3]))
    return top, left, right


# id, title, subtitle, detail lines, accent?, where the audit bit
LAYERS = [
    ("sources", "Eight municipal sources", "Socrata · Census · JustFix · MTA",
     ["HPD wvxf-dwi5 · 311 erm2-nwe9 · PLUTO 64uk-42ks",
      "DOB e5aq-a4j2 · DOHMH 43nn-pn8j · ACS B25064"], False,
     "Live datasets. No as-of parameter, so a re-run is never the same run."),
    ("ingest", "ingest", "1,322 lines · blocking reqwest · runs once, on a laptop",
     ["Paged on $order=:id — 134,837 HPD rows over 3 requests",
      "219,692 311 points over 5. Truncation now fails loudly."], True,
     "The bug lived here: 50,000 requested against 134,837 matching."),
    ("artifact", "housecheck.db", "1,240 KB · 310 pages · read-only in practice",
     ["250 buildings · 26,306 violations · 41 tract medians",
      "meta: snapshot_year=2026 — the whole provenance table"], False,
     "Not in git. Regenerable, but not reproducible byte-for-byte."),
    ("image", "Docker image", "29 MB · debian-slim + one static binary",
     ["COPY data/housecheck.db — the database ships inside",
      "No database URL. No password. No secret in the image."], False,
     "Compromising the image yields public NYC data."),
    ("runtime", "api on Fly.io", "3,028 lines · axum · 256 MB · scale-to-zero",
     ["824 lines REST · 1,258 agent · 947 tests",
      "167 ms cold start · 2.2 ms per card · 21 ms for all 250"], False,
     "Whole DB fits SQLite's 2 MB page cache — no I/O when warm."),
    ("client", "React on Vercel", "4,303 lines of app code",
     ["useSyncExternalStore · BrowserRouter · no UI framework",
      "Falls back to demo data, and says so on screen"], False,
     "Renders a missing count as 0 — the last place absence gains confidence."),
]

PURE = {"model": "192 lines · serde only", "scoring": "275 lines · model only"}

layers = []
for i, (lid, title, sub, det, hot, note) in enumerate(LAYERS):
    z = (len(LAYERS) - 1 - i) * GAP
    top, left, right = slab(z)
    cx, cy = iso(W / 2, D / 2, z)
    lx, ly = iso(W, 0, z)          # right-hand corner: label anchor
    layers.append({
        "id": lid, "title": title, "sub": sub, "det": det, "hot": hot, "note": note,
        "top": top, "left": left, "right": right,
        "cx": round(cx, 1), "cy": round(cy, 1),
        "lx": round(lx + 26, 1), "ly": round(ly, 1),
        "z": z,
    })

# connector: down the middle of the stack, from each plane to the one below
conns = []
for i in range(len(LAYERS) - 1):
    a = iso(W / 2, D / 2, (len(LAYERS) - 1 - i) * GAP - TH)
    b = iso(W / 2, D / 2, (len(LAYERS) - 2 - i) * GAP)
    conns.append({"x1": round(a[0], 1), "y1": round(a[1], 1),
                  "x2": round(b[0], 1), "y2": round(b[1], 1)})

xs, ys = [], []
for L in layers:
    for poly in (L["top"], L["left"], L["right"]):
        for pt in poly.split():
            x, y = pt.split(",")
            xs.append(float(x))
            ys.append(float(y))
# Three columns: annotations left of the stack, the stack itself, labels right.
# Every slab's left corner sits at x = -W*cos30, so the left pad must clear it or the
# margin notes land on top of the geometry.
PAD_L, PAD_R, PAD_T, PAD_B = 350, 480, 62, 54
vb = (min(xs) - PAD_L, min(ys) - PAD_T,
      (max(xs) - min(xs)) + PAD_L + PAD_R, (max(ys) - min(ys)) + PAD_T + PAD_B)

io.open(os.path.join(OUT, "iso_geom.json"), "w", encoding="utf-8").write(
    json.dumps({"layers": layers, "conns": conns,
                "vb": " ".join("%.1f" % v for v in vb), "pure": PURE}, indent=1))
print("layers: %d   viewBox: %s" % (len(layers), " ".join("%.0f" % v for v in vb)))
for L in layers:
    print("  %-9s top-corner (%7.1f,%7.1f)  label x=%.0f" % (L["id"], L["cx"], L["cy"], L["lx"]))
