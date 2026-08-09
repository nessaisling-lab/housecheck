# Fill Anthony's compiled slide deck with the verified build facts.
# Surgical edits to the minified bundle so his design, assets and nav are untouched.
#
# Slide COUNT is deliberately unchanged. The brief was that the deck reads sparse
# — titles floating in empty space — not that it needs more slides. So the work
# here is density: real screenshots where a claim needs proof, and substance on
# the slides that were carrying a headline and one line of text.
import io, json, sys

SRC = r"C:\Users\Aisling Ld Pursuit\Downloads\HouseCheck-Presentation.html"
OUT = r"C:\Users\Aisling Ld Pursuit\Downloads\HouseCheck-Presentation-filled.html"
LIVE = "https://housecheck-wine.vercel.app"

s = io.open(SRC, encoding="utf-8").read()
orig_len = len(s)

# Screenshots from the 2026-07-29 walkthrough of the deployed app.
_shots = json.load(open("shots/b64.json"))

# Real coordinates of all 250 covered buildings, projected into a 380x300 box.
# Generated from data/housecheck.db — the same source the app's map uses.
_map_pts = json.load(open("map-points.json"))

# Cropped regions of the app screenshots, so a slide can show the one card it is
# talking about instead of a whole phone shrunk to a thumbnail.
_crops = json.load(open("shots/crops.json"))

# Paths lifted from the supplied logo file (Design HouseCheck Logo.svg).
_logo = json.load(open("logo-paths.json"))


def crop(name, alt, width=None):
    st = '{borderRadius:12,border:"1px solid rgba(28,28,30,0.14)",' \
         'boxShadow:"0 8px 24px rgba(0,0,0,0.14)",display:"block",width:"100%"'
    if width:
        st += ',maxWidth:%d' % width
    st += "}"
    return 'K.jsx("img",{src:"data:image/jpeg;base64,%s",alt:"%s",style:%s})' % (
        _crops[name], alt, st)


def logo(width=300):
    """The real HouseCheck lockup — gradient orb, house-check glyph, wordmark.

    The deck was drawing its own approximation of the mark (CS) rather than the
    designed asset. Gradient ids are derived from React's useId so two lockups
    on one slide cannot collide.
    """
    house = ",".join(
        'K.jsx("path",{d:"%s",stroke:aA,strokeWidth:"10.7949",'
        'strokeLinecap:"round",strokeLinejoin:"round"},"h%d")' % (d, i)
        for i, d in enumerate(_logo["strokes"])
    )
    return (
        'K.jsxs("svg",{width:%d,viewBox:"311 200 460 378",fill:"none",'
        'xmlns:"http://www.w3.org/2000/svg",role:"img",'
        '"aria-label":"HouseCheck",children:['
        'K.jsx("defs",{children:'
        'K.jsxs("linearGradient",{id:"hcLogoGrad",x1:"431.657",y1:"230.046",x2:"619.343",'
        'y2:"476.385",gradientUnits:"userSpaceOnUse",children:['
        'K.jsx("stop",{stopColor:"#E49F9F"}),'
        'K.jsx("stop",{offset:"0.5",stopColor:"#EEC095"}),'
        'K.jsx("stop",{offset:"1",stopColor:cA})]})}),'
        'K.jsx("path",{d:"%s",fill:"url(#hcLogoGrad)"}),'
        '%s,'
        'K.jsx("path",{d:"%s",fill:aA})]})'
        % (width, _logo["orb"], house, _logo["wordmark"])
    )


def tech_icon(name, glyph, note):
    """A named technology with its own mark, drawn inline as SVG.

    Her note: "show the icon for React and Vite... SQLite... Rust. Put a crab.
    We love crabs here." Marks are hand-drawn here rather than fetched, because
    the deck is a single self-contained file with no network at present time.
    """
    return (
        'K.jsxs("div",{className:"flex flex-col items-center gap-2",children:['
        'K.jsx("div",{className:"flex items-center justify-center rounded-2xl",'
        'style:{width:78,height:78,backgroundColor:"rgba(255,255,255,0.92)",'
        'border:"1px solid rgba(28,28,30,0.10)"},children:' + glyph + '}),'
        'K.jsx("p",{className:"font-semibold text-base",style:{color:aA},'
        'children:"' + name + '"}),'
        'K.jsx("p",{className:"text-sm",style:{color:WA},'
        'children:"' + note + '"})]},"' + name + '")'
    )


# ── marks ──────────────────────────────────────────────────────────────────
RUST_CRAB = (
    'K.jsxs("svg",{width:"44",height:"44",viewBox:"0 0 48 48",fill:"none",'
    'xmlns:"http://www.w3.org/2000/svg","aria-hidden":!0,children:['
    # shell
    'K.jsx("ellipse",{cx:"24",cy:"26",rx:"13",ry:"9",fill:"#CE422B"}),'
    # eyes on stalks
    'K.jsx("path",{d:"M18 17.5v-3M30 17.5v-3",stroke:"#CE422B",strokeWidth:"2",'
    'strokeLinecap:"round"}),'
    'K.jsx("circle",{cx:"18",cy:"13",r:"2.6",fill:"#1C1C1E"}),'
    'K.jsx("circle",{cx:"30",cy:"13",r:"2.6",fill:"#1C1C1E"}),'
    # claws
    'K.jsx("path",{d:"M11 22c-3.4-.6-5.6.9-6.4 3.2 2.2 1.4 4.6 1.2 6.4-.6",'
    'fill:"#CE422B"}),'
    'K.jsx("path",{d:"M37 22c3.4-.6 5.6.9 6.4 3.2-2.2 1.4-4.6 1.2-6.4-.6",'
    'fill:"#CE422B"}),'
    # legs
    'K.jsx("path",{d:"M13 31l-4 4M18 34l-2 4.5M30 34l2 4.5M35 31l4 4",'
    'stroke:"#CE422B",strokeWidth:"2",strokeLinecap:"round"})]})'
)

REACT_ATOM = (
    'K.jsxs("svg",{width:"44",height:"44",viewBox:"0 0 48 48",fill:"none",'
    'xmlns:"http://www.w3.org/2000/svg","aria-hidden":!0,children:['
    'K.jsx("circle",{cx:"24",cy:"24",r:"3.4",fill:"#0F7C8C"}),'
    'K.jsx("ellipse",{cx:"24",cy:"24",rx:"17",ry:"6.6",stroke:"#0F7C8C",'
    'strokeWidth:"2"}),'
    'K.jsx("ellipse",{cx:"24",cy:"24",rx:"17",ry:"6.6",stroke:"#0F7C8C",'
    'strokeWidth:"2",transform:"rotate(60 24 24)"}),'
    'K.jsx("ellipse",{cx:"24",cy:"24",rx:"17",ry:"6.6",stroke:"#0F7C8C",'
    'strokeWidth:"2",transform:"rotate(120 24 24)"})]})'
)

VITE_BOLT = (
    'K.jsxs("svg",{width:"44",height:"44",viewBox:"0 0 48 48",fill:"none",'
    'xmlns:"http://www.w3.org/2000/svg","aria-hidden":!0,children:['
    'K.jsx("path",{d:"M24 4L42 12 33 42 24 4Z",fill:"#9A6BD6"}),'
    'K.jsx("path",{d:"M24 4L6 12l9 30L24 4Z",fill:"#C9A0F0"}),'
    'K.jsx("path",{d:"M28 15l-11 6 5 1-3 10 11-13-5-1 3-3Z",fill:"#B8860B"})]})'
)

SQLITE_DB = (
    'K.jsxs("svg",{width:"44",height:"44",viewBox:"0 0 48 48",fill:"none",'
    'xmlns:"http://www.w3.org/2000/svg","aria-hidden":!0,children:['
    'K.jsx("ellipse",{cx:"24",cy:"12",rx:"14",ry:"5.5",fill:"#1D6A53"}),'
    'K.jsx("path",{d:"M10 12v24c0 3 6.3 5.5 14 5.5s14-2.5 14-5.5V12",'
    'stroke:"#1D6A53",strokeWidth:"2.6",fill:"none"}),'
    'K.jsx("path",{d:"M10 22c0 3 6.3 5.5 14 5.5s14-2.5 14-5.5",'
    'stroke:"#1D6A53",strokeWidth:"2.2",fill:"none"})]})'
)

AXUM_SERVER = (
    'K.jsxs("svg",{width:"44",height:"44",viewBox:"0 0 48 48",fill:"none",'
    'xmlns:"http://www.w3.org/2000/svg","aria-hidden":!0,children:['
    'K.jsx("rect",{x:"7",y:"9",width:"34",height:"11",rx:"3",stroke:aA,'
    'strokeWidth:"2.4"}),'
    'K.jsx("rect",{x:"7",y:"28",width:"34",height:"11",rx:"3",stroke:aA,'
    'strokeWidth:"2.4"}),'
    'K.jsx("circle",{cx:"14",cy:"14.5",r:"2",fill:"#1D6A53"}),'
    'K.jsx("circle",{cx:"14",cy:"33.5",r:"2",fill:"#1D6A53"})]})'
)


def coverage_map():
    """The pilot footprint, drawn from actual building coordinates.

    Replaces a decorative radial of concentric dashed circles that sat under
    the words "Bed-Stuy today. Five boroughs next." A diagram that carries no
    data is a worse answer than the map we already had the numbers for.
    """
    pts = ",".join("[%g,%g]" % (x, y) for x, y in _map_pts)
    grid = ",".join(
        'K.jsx("line",{x1:%d,y1:18,x2:%d,y2:282,stroke:aA,strokeWidth:"0.5",opacity:"0.10"},"v%d")'
        % (18 + i * 69, 18 + i * 69, i)
        for i in range(6)
    ) + "," + ",".join(
        'K.jsx("line",{x1:18,y1:%d,x2:362,y2:%d,stroke:aA,strokeWidth:"0.5",opacity:"0.10"},"h%d")'
        % (18 + i * 88, 18 + i * 88, i)
        for i in range(4)
    )
    return (
        'K.jsxs("div",{className:"flex-1 flex flex-col items-center justify-center",children:['
        'K.jsxs("svg",{width:"100%%",viewBox:"0 0 380 300",fill:"none",'
        'xmlns:"http://www.w3.org/2000/svg",role:"img",'
        '"aria-label":"Map of the Bedford-Stuyvesant pilot area showing all 250 covered buildings.",'
        'style:{maxWidth:430},children:['
        'K.jsx("g",{children:[' + grid + ']}),'
        'K.jsx("g",{children:[' + pts + '].map((p,i)=>'
        'K.jsx("circle",{cx:p[0],cy:p[1],r:"3",fill:cD,opacity:"0.9"},i))})]}),'
        'K.jsx("p",{className:"text-base font-medium mt-3",style:{color:aA},'
        'children:"250 buildings \\u00b7 every one we hold a full record for"})]})'
    ) % ()


def shot(key):
    return "data:image/jpeg;base64," + _shots[key]


def phone(key, alt, max_h=None):
    """A phone screenshot. Sizing is inline, not utility classes: this bundle's
    Tailwind is precompiled and arbitrary values like max-h-[300px] do not
    exist in it, so they would fail silently."""
    style = ('{borderRadius:14,border:"1px solid rgba(28,28,30,0.14)",'
             'boxShadow:"0 10px 30px rgba(0,0,0,0.16)",objectFit:"contain",'
             'width:"100%%",height:"auto"%s}'
             % (",maxHeight:%d" % max_h if max_h else ""))
    return 'K.jsx("img",{src:"%s",alt:"%s",style:%s})' % (shot(key), alt, style)


def sub(old, new, label):
    global s
    if old not in s:
        sys.exit("MISS: " + label)
    if s.count(old) != 1:
        sys.exit("AMBIGUOUS (%d): %s" % (s.count(old), label))
    s = s.replace(old, new, 1)
    print("  ok  " + label)


# ── 1. title ────────────────────────────────────────────────────────────────
sub(
    "<title>Create HouseCheck Presentation</title>",
    "<title>HouseCheck \u2014 Know the building before you sign</title>",
    "title",
)

# ── 2. team roles (were three literal "Role" placeholders) ──────────────────
sub(
    '{initials:"AN",name:"Antonin",role:"Role",color:"#E49F9F",photo:$i},'
    '{initials:"AI",name:"Aisling",role:"Role",color:"#EEC095",photo:Di},'
    '{initials:"JG",name:"Jagger",role:"Role",color:cA,photo:_i}',
    '{initials:"AN",name:"Antonin",role:"UI & UX",color:"#E49F9F",photo:$i},'
    '{initials:"AI",name:"Aisling",role:"Backend & Data",color:"#EEC095",photo:Di},'
    '{initials:"JG",name:"Jagger",role:"Agent & Research",color:cA,photo:_i}',
    "team roles",
)

# ── 3. slide count ──────────────────────────────────────────────────────────
sub("IS=7", "IS=17", "slide count")

# ── 3a. palette: two greys and two mints ────────────────────────────────────
# Measured on the light canvas, the original single grey and the mint accent
# were at 2.27:1 and 1.38:1 — both far under AA, and the mint carries the
# spoken line on every slide. But the same grey PASSES at 5.22:1 inside the
# dark stat card, so one value cannot serve both. Split them:
#   WA  darker grey, for light surfaces   (4.63:1)
#   WB  the original grey, dark cards only (5.22:1)
#   cD  darker mint, for light surfaces   (4.51:1)
#   cA  the original mint, on dark only   (8.58:1)
# Hue and saturation are unchanged in both mints.
sub(
    'cA="#4BCDA7",aA="#1C1C1E",WA="#8E8E93",ZV="#D7D7D9"',
    'cA="#4BCDA7",cD="#1D6A53",aA="#1C1C1E",WA="#5C5C61",WB="#A8A8AE",ZV="#D7D7D9"',
    "palette: split grey and mint",
)

# The stat card is the one dark surface, so it keeps the bright grey — and its
# 10px label was the smallest type in the deck.
sub(
    'K.jsx("p",{className:"text-[10px] font-semibold tracking-[0.18em] uppercase mb-5",'
    'style:{color:WA},children:R})',
    'K.jsx("p",{className:"font-semibold tracking-[0.18em] uppercase mb-5",'
    'style:{color:WB,fontSize:14},children:R})',
    "stat card label: bright grey, larger",
)
sub(
    'd&&K.jsx("p",{className:"text-xs mt-3 font-medium",style:{color:WA},children:d})',
    'd&&K.jsx("p",{className:"text-base mt-3 font-medium",style:{color:WB},children:d})',
    "stat card sub: bright grey, larger",
)

# ── 3b. the eyebrow was 11px — unreadable from the back of a room ───────────
sub(
    'K.jsx("p",{className:"text-[11px] font-semibold tracking-[0.2em] uppercase",'
    'style:{color:E?"rgba(255,255,255,0.5)":WA},children:R})',
    'K.jsx("p",{className:"font-semibold tracking-[0.2em] uppercase",'
    'style:{color:E?"rgba(255,255,255,0.72)":WA,fontSize:15},children:R})',
    "eyebrow larger",
)

# ── 3c. the animated city, behind every slide instead of just the hero ─────
# One <video> hoisted into the deck shell rather than one per slide. Slides
# unmount as you navigate, so a per-slide element would restart the clip on
# every click and re-decode 11 MB each time. Hoisted, it plays through.
sub(
    'K.jsxs("div",{className:"w-screen h-screen overflow-hidden relative select-none",'
    'style:{fontFamily:"Inter, system-ui, sans-serif",backgroundColor:ZV,cursor:"default"},'
    'onClick:()=>d(1),children:[K.jsx("div",{className:"w-full h-full",children:K.jsx(eA,{})})',
    'K.jsxs("div",{className:"w-screen h-screen overflow-hidden relative select-none",'
    'style:{fontFamily:"Inter, system-ui, sans-serif",backgroundColor:ZV,cursor:"default"},'
    'onClick:()=>d(1),children:['
    'K.jsx("video",{src:gi,autoPlay:!0,muted:!0,loop:!0,playsInline:!0,'
    '"aria-hidden":!0,className:"absolute inset-0 w-full h-full object-cover pointer-events-none"}),'
    'K.jsx("div",{className:"w-full h-full relative",children:K.jsx(eA,{})})',
    "hoist the video behind the whole deck",
)

# Xq now only lays the legibility wash. The hero keeps its dramatic fade so the
# city reads at full strength behind the title; every other slide gets a flat
# wash that guarantees dark ink stays at 5.8:1 even over the video's darkest
# frame. Its own <img>/<video> are gone — the shell owns the footage.
sub(
    'function Xq({video:R=!1}){return K.jsxs("div",{className:"absolute inset-0 overflow-hidden pointer-events-none",children:[R?K.jsx("video",{src:gi,autoPlay:!0,muted:!0,playsInline:!0,className:"absolute inset-0 w-full h-full object-cover"}):K.jsx("img",{src:Bi,alt:"",className:"absolute inset-0 w-full h-full object-cover",style:{opacity:1,objectPosition:"center"}}),K.jsx("div",{className:"absolute inset-0",style:{background:R?`linear-gradient(to bottom,\n                rgba(215,215,217,0.05) 0%,\n                rgba(215,215,217,0.15) 35%,\n                rgba(215,215,217,0.68) 60%,\n                rgba(215,215,217,0.96) 82%,\n                ${ZV} 100%)`:"rgba(215,215,217,0.45)"}})]})}',
    'function Xq({video:R=!1}){return K.jsx("div",{className:"absolute inset-0 overflow-hidden pointer-events-none",children:K.jsx("div",{className:"absolute inset-0",style:{background:R?`linear-gradient(to bottom,\n                rgba(215,215,217,0.02) 0%,\n                rgba(215,215,217,0.12) 34%,\n                rgba(215,215,217,0.62) 58%,\n                rgba(215,215,217,0.94) 80%,\n                ${ZV} 100%)`:"rgba(215,215,217,0.72)"}})})}',
    "Xq: wash only, video comes from the shell",
)

# ── 3b. hero + closing slide: the attribution the template requires ─────────
sub(
    'children:"Built entirely from public NYC records"',
    'children:"Built entirely from public NYC records · Pursuit L2 · '
    'Cycle 4 Capstone"',
    "hero capstone attribution",
)

sub(
    'K.jsx("p",{className:"text-xs mt-1",style:{color:`${WA}80`},'
    'children:"Renters deserve to know what they\'re signing into."})',
    'K.jsx("p",{className:"text-xs mt-1",style:{color:WA},'
    'children:"github.com/nessaisling-lab/housecheck"}),'
    'K.jsx("p",{className:"text-xs mt-3",style:{color:`${WA}80`},'
    'children:"Aisling · Antonin · Jagger  •  Pursuit L2  •  '
    'Cycle 4 Capstone"})',
    "closing links + attribution",
)

# ── 4. new slides, written in the bundle's own component vocabulary ─────────
# In scope here: K (jsx runtime), cA mint, aA ink, WA grey, ZV canvas,
# yl eyebrow, wS stat card, Xq background, CS logo, hS member, PS pill.

# Cards sit over a moving video now, so they have to be near-opaque or their
# body copy loses contrast every time a pale building drifts behind it.
CARD = '{backgroundColor:"rgba(255,255,255,0.9)",border:"1px solid rgba(28,28,30,0.10)"}'
PAD = '{padding:"52px 64px 44px"}'


def shell(body):
    """Standard slide frame: canvas + background wash + padded column."""
    return (
        'K.jsxs("div",{className:"relative w-full h-full overflow-hidden",'
        'style:{backgroundColor:"transparent"},children:[K.jsx(Xq,{}),'
        'K.jsxs("div",{className:"relative z-10 w-full h-full flex flex-col",'
        "style:" + PAD + ",children:[" + body + "]})]})"
    )


def h2(pre, accent, post="", size="clamp(46px, 5.6vw, 68px)", mb=6):
    kids = ['"%s"' % pre, '" "', 'K.jsx("span",{style:{color:cD},children:"%s"})' % accent]
    if post:
        kids.append('" %s"' % post)
    return (
        'K.jsxs("h2",{className:"font-semibold leading-tight tracking-[-0.035em] mt-5 mb-%d",'
        'style:{fontSize:"%s",color:aA},children:[%s]})' % (mb, size, ",".join(kids))
    )


def cards(items, cols):
    """items: list of (title, sub).

    Flex, not grid: the bundle's Tailwind is precompiled and only ships
    `grid-cols-4`, so `grid-cols-2`/`grid-cols-3` would silently collapse
    to a single column. `flex` + `flex-1` gives equal columns at any count.
    """
    kids = ",".join(
        'K.jsxs("div",{className:"flex-1 rounded-2xl p-5 flex flex-col justify-center",'
        'style:%s,children:['
        'K.jsx("p",{className:"font-semibold text-xl",style:{color:aA,marginBottom:8},children:"%s"}),'
        'K.jsx("p",{className:"text-base leading-relaxed",style:{color:WA},children:"%s"})]},"%s")'
        % (CARD, t, sb, t)
        for t, sb in items
    )
    assert cols == len(items)
    # grow: the slide is a flex column, so a fixed-height card row left a dead
    # band between the heading and the bottom quote. Letting the row take the
    # slack makes taller cards with the copy centred, instead of empty space.
    return ('K.jsx("div",{className:"grow flex items-stretch gap-4",children:[%s]})'
            % kids)


def quote(text):
    # Dark bar, bright mint: 8.58:1 instead of the 1.38:1 that mint-on-pale
    # gave. These are the lines the presenter says out loud, so they are the
    # last thing that should be hard to read from the back of a room.
    return (
        'K.jsx("div",{className:"rounded-2xl px-7 py-6 mt-auto",'
        'style:{backgroundColor:aA,borderLeft:`5px solid ${cA}`},'
        'children:K.jsx("p",{className:"font-semibold leading-tight tracking-[-0.02em]",'
        'style:{fontSize:33,color:cA},children:"%s"})})' % text
    )


def chips(items):
    kids = ",".join(
        'K.jsx("span",{className:"px-5 py-2 rounded-full text-base font-semibold",'
        'style:{border:"1px solid rgba(28,28,30,0.15)",color:WA,'
        'backgroundColor:"rgba(255,255,255,0.35)"},children:"%s"},"%s")' % (c, c)
        for c in items
    )
    return 'K.jsx("div",{className:"flex flex-wrap items-center gap-2.5",children:[%s]})' % kids


# ── N1  Provenance ──────────────────────────────────────────────────────────
n1 = shell(
    'K.jsx(yl,{children:"Provenance"}),'
    + h2("Named datasets.", "Dated snapshots.")
    + ","
    + cards(
        [
            ("HPD Violations", "wvxf-dwi5 \u00b7 native 10-digit BBL, read directly"),
            ("311 Service Requests", "erm2-nwe9 \u00b7 2020\u2013present"),
            ("PLUTO", "64uk-42ks \u00b7 tabular, not MapPLUTO"),
            ("DHCR Registrations", "data.ny.gov \u00b7 rent-stabilized status"),
        ],
        4,
    )
    # NOTE: this paragraph is the search target of the `provenance` patch below
    # (targets/provenance.txt holds it verbatim). Editing the text here silently
    # breaks that patch — change the replacement at the patch instead.
    + ',K.jsx("p",{className:"text-lg font-medium mt-6",style:{color:aA},'
    'children:"Snapshot year 2026. Every score decomposes back into the records behind it."}),'
    + quote(
        "250 buildings. 26,306 violation records. All public."
    )
)

# ── N2  Under the hood ──────────────────────────────────────────────────────
n2 = shell(
    'K.jsx(yl,{children:"Under the Hood"}),'
    + h2("A Rust API and a", "read-only", "database.")
    + ","
    + cards(
        [
            ("Rust + Axum", "Five crates: model, scoring, store, ingest, api"),
            ("SQLite, baked in", "Shipped inside the image \u00b7 read-only \u00b7 no DB server to breach"),
            ("React 19 + Vite", "TypeScript, Tailwind \u00b7 deployed on Vercel"),
            ("Fly.io", "Scale-to-zero container \u00b7 no secrets in the image"),
        ],
        4,
    )
    + ',K.jsxs("div",{className:"flex items-end gap-5 mt-auto",children:['
    'K.jsx(wS,{label:"Tests Passing",value:"106",sub:"fmt + clippy clean"}),'
    'K.jsx(wS,{label:"API Endpoints",value:"9",sub:"all public data"})]})'
)

# ── N3  The assistant ───────────────────────────────────────────────────────
n3 = shell(
    'K.jsx(yl,{children:"Grounded by Construction"}),'
    + h2("The model never touches the", "database.")
    + ','
    'K.jsx("p",{className:"text-lg leading-relaxed",style:{color:WA,maxWidth:640},'
    'children:"It asks. Our code answers. You ask a question \u2192 the model emits a tool call '
    "\u2192 our Rust code runs the query \u2192 the result comes back as data. Grounding is "
    'enforced by the architecture, not requested in a prompt."}),'
    'K.jsx("div",{className:"mt-8",children:'
    + chips(
        [
            "get_building",
            "get_open_violations",
            "check_rent_fairness",
            "search_address",
            "rank_by_priorities",
            "legal_context",
            "find_legal_help",
            "search_law",
        ]
    )
    + "}),"
    + quote(
        "Injection-tested: a planted instruction inside a violation record did not move it."
    )
)

# ── N4  Guardrails ──────────────────────────────────────────────────────────
n4 = shell(
    'K.jsx(yl,{children:"Guardrails"}),'
    + h2("It explains the law. It does not", "practice it.")
    + ","
    + cards(
        [
            (
                "No legal advice",
                "NY Judiciary Law \u00a7\u00a7 478 and 484. Every legal answer carries a "
                "disclaimer and a citation to its statute.",
            ),
            (
                "No promised outcomes",
                "FTC v. DoNotPay settled at $193,000 over unevidenced capability claims.",
            ),
            (
                "Real humans, verified",
                "Every referral was checked against the organization's own page. "
                "Three errors found and fixed.",
            ),
        ],
        3,
    )
    + ","
    + quote("A signal \u2014 not a legal ruling.")
)

# ── N5  Integrity ───────────────────────────────────────────────────────────
n5 = shell(
    'K.jsx(yl,{children:"Integrity"}),'
    + h2("Two of our own claims were wrong. We", "published the corrections.")
    + ","
    + cards(
        [
            (
                "We deleted a real statistic.",
                "Our own fact-check flagged REBNY's 761,352-building analysis as fabricated. "
                "The report post-dates the check. Verified firsthand, restored, cited.",
            ),
            (
                "We claimed work we never needed to do.",
                "HPD's dataset has a native BBL column at schema position 40. Our "
                "reconstruction was redundant, not wrong. Retracted.",
            ),
            (
                "We checked the things nobody checks.",
                "Every tenant hotline in the referral directory, against that "
                "organisation's own page. Three were wrong: bad hours, a dead "
                "domain. Fixed.",
            ),
        ],
        3,
    )
    + ","
    + quote("No incorrect data shipped. The claim did.")
)

# ═══ Slides required by the Pursuit MVP deck template ══════════════════════
# The template's six beats: title / THE SITUATION / THE CHANGE / THE EVIDENCE /
# WHAT IT MAKES POSSIBLE / thank-you. Each carries an uppercase eyebrow, a
# headline, and a spoken line. Every statistic below is from the claim
# verification dossier; nothing here is estimated.


def stats(items):
    """items: list of (label, value, sub) -> a row of the deck's stat cards."""
    kids = ",".join(
        'K.jsx(wS,{label:"%s",value:"%s",sub:"%s"},"%s")' % (l, v, sb, l)
        for l, v, sb in items
    )
    return 'K.jsx("div",{className:"grow flex items-stretch gap-5",children:[%s]})' % kids


def column(head, lines):
    """One side of the Before / After comparison."""
    rows = ",".join(
        'K.jsx("p",{className:"text-xl leading-relaxed",style:{color:aA},'
        'children:"%s"},"%s")' % (t, t)
        for t in lines
    )
    return (
        'K.jsxs("div",{className:"flex-1 rounded-2xl p-7",style:%s,children:['
        'K.jsx("p",{className:"text-base font-semibold tracking-[0.2em] uppercase mb-5",'
        'style:{color:WA},children:"%s"}),'
        'K.jsxs("div",{className:"flex flex-col gap-3",children:[%s]})]})'
        % (CARD, head, rows)
    )


def steps(items):
    """Numbered live-demo steps."""
    kids = ",".join(
        'K.jsxs("div",{className:"flex-1 rounded-2xl p-5",style:%s,children:['
        'K.jsx("p",{className:"font-bold leading-none tabular-nums mb-4",'
        'style:{color:cD,fontSize:40},children:"%d"}),'
        'K.jsx("p",{className:"font-semibold text-xl",style:{color:aA,marginBottom:8},children:"%s"}),'
        'K.jsx("p",{className:"text-base leading-relaxed",style:{color:WA},children:"%s"})]},"%s")'
        % (CARD, i + 1, t, sb, t)
        for i, (t, sb) in enumerate(items)
    )
    return 'K.jsx("div",{className:"grow flex items-stretch gap-4",children:[%s]})' % kids


# ── THE SITUATION ───────────────────────────────────────────────────────────
sit = shell(
    'K.jsx(yl,{children:"The Situation"}),'
    + h2("Renters are asked to decide", "blind.")
    + ","
    + stats(
        [
            ("Rent-Burdened", "51.6%", "of NYC renter households"),
            ("At Least One", "~11%", "of buildings, hazardous violation"),
            ("On The Record", "11.1M", "HPD violations, all public"),
        ]
    )
    + ',K.jsx("p",{className:"text-lg leading-relaxed mt-4",style:{color:aA,maxWidth:900},'
    'children:"51.6% of NYC renter households pay 30%+ of income in rent, 28.8% pay 50%+ '
    "(NYC Rent Guidelines Board, 2026). REBNY analyzed 761,352 residential buildings: 89% "
    "had no HPD Class C — immediately hazardous — violation over 24 months, so "
    'roughly 11% had at least one. The records exist. They are unreadable in the moment."}),'
    + quote("You get 15 minutes and a signature. The building has a 40-year record.")
)

# ── THE CHANGE ──────────────────────────────────────────────────────────────
chg = shell(
    'K.jsx(yl,{children:"The Change"}),'
    + h2("Before you sign.", "Not after.")
    + ','
    'K.jsxs("div",{className:"grow flex items-stretch gap-6",children:['
    + column(
        "Before",
        [
            "Six agencies, six portals, six vocabularies.",
            "Violation codes with no plain-English meaning.",
            "No way to tell a fair rent from a high one.",
        ],
    )
    + ","
    + column(
        "After",
        [
            "One address in.",
            "A 0–100 Building Health Card out.",
            "Every number opens to the record behind it.",
        ],
    )
    + "]}),"
    + quote("Type an address. Get an answer while you are still standing in the apartment.")
)

# ── THE EVIDENCE ────────────────────────────────────────────────────────────
dem = shell(
    'K.jsx(yl,{children:"The Evidence"}),'
    + h2("Live", "demo.")
    + ","
    + steps(
        [
            ("Search", "Type any address in the Bed-Stuy pilot"),
            ("Score", "A 0–100 Building Health Card in seconds"),
            ("Open it up", "Rent, condition, legal, accessibility — each traced to its source"),
            ("Ask", "The assistant answers from the building's own record, with citations"),
        ]
    )
    + ',K.jsx("p",{className:"text-lg font-medium mt-6",style:{color:aA},'
    'children:"housecheck-wine.vercel.app — running live on 250 real Bed-Stuy buildings."}),'
    + quote("Switch to the live app and run it.")
)

# ── WHAT WE'D BUILD NEXT ────────────────────────────────────────────────────
nxt = shell(
    'K.jsx(yl,{children:"What It Makes Possible"}),'
    + h2("Give renters the record", "before the signature.")
    + ","
    + cards(
        [
            (
                "All five boroughs",
                "The ingest pipeline is city-wide already. The pilot is scoped to "
                "Bed-Stuy, not limited to it.",
            ),
            (
                "Read it any way you need",
                "Screen-reader announcements and an in-app text-size control, so a "
                "violation history is readable on a phone in a hallway.",
            ),
            (
                "Live data, not a snapshot",
                "Scheduled refresh from HPD and DHCR instead of a dated bundled "
                "database.",
            ),
        ],
        3,
    )
    + ","
    + quote("Renters deserve to know what they are signing into.")
)

# ── N10  Market ─────────────────────────────────────────────────────────────
# Researched rather than asserted (docs/MARKET-RESEARCH.md). Deliberately does
# not quote the $40-47B proptech figure: five research firms disagree by ~17% on
# the same year, and the category bundles virtual tours and building management.
# Naming the number we refused is the more credible move in front of anyone who
# knows the space, and it is the same discipline as the Integrity slide.
mkt = shell(
    'K.jsx(yl,{children:"Market"}),'
    + h2("We looked up the", "ceiling.")
    + ","
    + cards(
        [
            (
                "Rentlogic · since 2013",
                "Grades every NYC building A–F from city data. Free to renters; "
                "landlords pay $99–$1,499 a building for the badge. Raised $2.4M. "
                "Twelve years on, it is nine people.",
            ),
            (
                "Openigloo · since 2020",
                "Reached 3M+ NYC renters on reviews and violation records. In 2025 it "
                "pivoted to brokerage — screening tenants and guaranteeing rent, at "
                "about half a month's rent per placement.",
            ),
            (
                "The number we did not quote",
                "Global proptech is put at $40–$47B for 2025 — five research firms, "
                "17% apart on the same year. It bundles virtual tours and building "
                "management. It is not our market.",
            ),
        ],
        3,
    )
    + ',K.jsx("p",{className:"text-lg font-medium mt-5",style:{color:aA},'
    'children:"Sized bottom-up instead, against the 91,918 NYC multifamily buildings we '
    'had already verified: roughly $1.8M a year at 5% adoption. A real business, not a '
    'venture-scale one — which is exactly what twelve years of Rentlogic shows."}),'
    + quote("Two teams already built this. Neither one sold the data.")
)

NEW = (
    "function nA1(){return " + n1 + "}"
    "function nA2(){return " + n2 + "}"
    "function nA3(){return " + n3 + "}"
    "function nA4(){return " + n4 + "}"
    "function nA5(){return " + n5 + "}"
    "function nB1(){return " + sit + "}"
    "function nB2(){return " + chg + "}"
    "function nB3(){return " + dem + "}"
    "function nB4(){return " + nxt + "}"
    "function nB5(){return " + mkt + "}"
)

sub(
    "const OS=[AL,VL,qL,lL,pL,uL,rL];",
    NEW
    + "const OS=[AL,VL,nB1,nB2,qL,nB3,lL,pL,nA1,nA2,nA3,nA4,nA5,uL,nB4,nB5,rL];",
    "insert 10 slides",
)

# ═══ Density pass on the slides that were carrying a title and little else ══


def notes(items):
    """A column of label/detail rows to sit beside a screenshot."""
    rows = ",".join(
        'K.jsxs("div",{children:['
        'K.jsx("p",{className:"font-semibold text-xl",style:{color:aA},children:"%s"}),'
        'K.jsx("p",{className:"text-base leading-relaxed mt-1",style:{color:WA},children:"%s"})]},"%s")'
        % (t, d, t)
        for t, d in items
    )
    return 'K.jsx("div",{className:"flex flex-col gap-5",children:[%s]})' % rows


# ── Slide 7: the App Screenshot placeholder becomes the actual card ─────────
sub(
    'K.jsxs("div",{className:"flex-1 rounded-3xl flex flex-col items-center justify-center",'
    'style:{border:"2px dashed rgba(28,28,30,0.18)",backgroundColor:"rgba(255,255,255,0.18)",maxHeight:320},'
    'children:[K.jsx("p",{className:"font-semibold text-xs tracking-[0.2em] uppercase",style:{color:WA},'
    'children:"App Screenshot"}),K.jsx("p",{className:"text-xs mt-2",style:{color:`${WA}80`},'
    'children:"Drop in a screenshot here, or switch to the live app"})]})',
    'K.jsxs("div",{className:"flex-1 flex items-center gap-10",children:['
    'K.jsx("div",{className:"shrink-0",style:{width:214},children:'
    + phone("03-health-card",
            "Building Health Card for 35 Skillman Street scoring 42 of 100, labelled Mixed "
            "Signals, with condition, legal, neighborhood and accessibility sub-scores.",
            max_h=430)
    + '}),K.jsx("div",{className:"flex-1 min-w-0",children:'
    + notes([
        ("42 of 100 — mixed signals",
         "One number, four pillars scored separately, so you can see which one drags it down."),
        ("It shows its work",
         "Every pillar carries the reason underneath: no hazardous violations, elevator, "
         "ADA subway 1.2 km."),
        ("“Unverified” is a real answer",
         "Stabilization unverified, not guessed. A wrong yes on a legal-rights tool is worse "
         "than an honest blank."),
        ("Your rent, in context",
         "$2,600 measured against the Census tract median for this block."),
      ])
    + "})]})",
    "slide 7: real Health Card screenshot + annotations",
)

# ── Slide 11: put the agent's own output next to the claim about it ─────────
sub(
    'K.jsx("p",{className:"text-lg leading-relaxed",style:{color:WA,maxWidth:640},'
    'children:"It asks. Our code answers. You ask a question → the model emits a tool call '
    "→ our Rust code runs the query → the result comes back as data. Grounding is "
    'enforced by the architecture, not requested in a prompt."}),'
    'K.jsx("div",{className:"mt-8",children:',
    'K.jsxs("div",{className:"flex-1 flex items-start gap-10",children:['
    'K.jsxs("div",{className:"flex-1 min-w-0",children:['
    'K.jsx("p",{className:"text-xl leading-relaxed",style:{color:aA},'
    'children:"It asks. Our code answers. You ask a question → the model emits a tool call '
    "→ our Rust code runs the query → the result comes back as data. Grounding is "
    'enforced by the architecture, not requested in a prompt."}),'
    'K.jsx("p",{className:"text-lg leading-relaxed mt-4",style:{color:aA},'
    'children:"Right: a real answer about an electrical fault. It cites § 235-b and the HPD '
    'violation classes with links you can open, then lists what to document as evidence. '
    'It does not tell you whether you would win."}),'
    'K.jsx("div",{className:"mt-6",children:',
    "slide 11: split layout, text left",
)

sub(
    'K.jsx("div",{className:"rounded-2xl px-7 py-6 mt-auto",'
    'style:{backgroundColor:aA,borderLeft:`5px solid ${cA}`},'
    'children:K.jsx("p",{className:"font-semibold leading-tight tracking-[-0.02em]",'
    'style:{fontSize:33,color:cA},children:"Injection-tested: a planted instruction inside a '
    'violation record did not move it."})})',
    'K.jsx("div",{className:"rounded-2xl px-7 py-5 mt-6",'
    'style:{backgroundColor:aA,borderLeft:`5px solid ${cA}`},'
    'children:K.jsx("p",{className:"font-semibold leading-tight tracking-[-0.02em]",'
    'style:{fontSize:24,color:cA},children:"Injection-tested: a planted instruction inside a '
    'violation record did not move it."})})]}),'
    'K.jsx("div",{className:"shrink-0",style:{width:232},children:'
    + phone("05-agent-statutes",
            "The agent citing NY Real Property Law 235-b and HPD violation classes with working "
            "links, followed by a checklist of what to document.",
            max_h=470)
    + "})]})",
    "slide 11: agent screenshot right",
)

# ── Slide 5: the one-liner slide was a logo and three chips ─────────────────
sub(
    'K.jsx("p",{className:"text-xl font-normal text-center",style:{color:WA},'
    'children:"One address in. One honest Building Health Card out."}),'
    'K.jsx("div",{className:"flex items-center gap-3 mt-2",children:R.map(E=>',
    'K.jsx("p",{className:"text-xl font-normal text-center",style:{color:WA},'
    'children:"One address in. One honest Building Health Card out."}),'
    + cards([
        ("Condition", "Open HPD violations by class, weighted by severity"),
        ("Legal", "Rent-stabilization and Good Cause coverage"),
        ("Neighborhood", "311 complaint density near the building, on a log curve"),
        ("Accessibility", "Elevator on record · distance to an ADA subway"),
      ], 4)
    + ','
    'K.jsx("p",{className:"text-lg font-medium",style:{color:aA},'
    'children:"Four pillars, weighted equally. A missing pillar is marked unverified — '
    'never scored as a zero."}),'
    'K.jsx("div",{className:"flex items-center gap-3",children:R.map(E=>',
    "slide 5: add the four pillars",
)

# ── Slide 2: the hook slide had a headline and a single paragraph ───────────
sub(
    'K.jsx("p",{className:"text-lg font-normal leading-relaxed mt-8",style:{color:WA,maxWidth:560},'
    "children:\"The building's real record — violations, rent status, repairs — is public. "
    "It's just scattered, jargon-filled, and impossible to use when you're deciding on the spot.\"})",
    'K.jsx("p",{className:"text-lg font-normal leading-relaxed mt-8",style:{color:WA,maxWidth:560},'
    "children:\"The building's real record — violations, rent status, repairs — is public. "
    "It's just scattered, jargon-filled, and impossible to use when you're deciding on the spot.\"}),"
    'K.jsx("div",{className:"mt-8",style:{maxWidth:760},children:'
    + cards([
        ("HPD Online", "Violations, by building — in violation-code shorthand"),
        ("DHCR / HCR", "Stabilization status — by written request, not lookup"),
        ("Census + HUD", "What rent is normal here — in a data table"),
      ], 3)
    + '}),'
    'K.jsx("p",{className:"text-lg font-medium mt-5",style:{color:aA},'
    'children:"Three agencies, three vocabularies, none of them talking to each other — '
    'while you are standing in the apartment."})',
    "slide 2: name the three portals",
)

# ── Slide 2: stop centring it, and give it the slide's full height ──────────
# It was vertically centred, which read as a hole once the neighbouring slides
# filled out. Top-aligned like the rest, with the portal cards taking the
# slack and the spoken line in a bar at the bottom.
sub(
    'K.jsxs("div",{className:"relative z-10 w-full h-full flex flex-col justify-center",'
    'style:{padding:"60px 80px"},children:[K.jsx(yl,{children:"The Blind Sign"})',
    'K.jsxs("div",{className:"relative z-10 w-full h-full flex flex-col",'
    'style:{padding:"52px 64px 44px"},children:[K.jsx(yl,{children:"The Blind Sign"})',
    "slide 2: top-aligned, full height",
)

sub(
    'K.jsx("p",{className:"text-lg font-medium mt-5",style:{color:aA},'
    'children:"Three agencies, three vocabularies, none of them talking to each other — '
    'while you are standing in the apartment."})',
    'K.jsx("p",{className:"text-lg font-medium mt-5",style:{color:aA},'
    'children:"Three agencies, three vocabularies, none of them talking to each other — '
    'while you are standing in the apartment."}),'
    + quote("A year of your life, decided on a fifteen-minute walkthrough."),
    "slide 2: spoken line at the bottom",
)

# ── Slide 8 (Anthony's): four pillars named but not explained ───────────────
# His own grid, so it never picked up the grow treatment, and each card carried
# a title plus a two-word source. 36% of the slide was empty.
sub(
    'const R=[{title:"Condition",source:"HPD violations"},'
    '{title:"Legal",source:"DHCR stabilization"},'
    '{title:"Neighborhood",source:"Census rent"},'
    '{title:"Accessibility",source:"Subway + elevator data"}];',
    'const R=[{title:"Condition",source:"HPD violations · 311",'
    'detail:"Open violations by class. Class C means no heat, '
    'no hot water, exposed wiring."},'
    '{title:"Legal",source:"DHCR · Good Cause",'
    'detail:"Stabilization and Good Cause. Where the record cannot '
    'support a claim, it reads unverified."},'
    '{title:"Neighborhood",source:"NYC 311 · erm2-nwe9",'
    'detail:"311 complaint density near the address, on a log curve so '
    'dense blocks still separate. Not rent — that is a separate check."},'
    '{title:"Accessibility",source:"DOB · MTA ADA",'
    'detail:"Elevator on record, and metres to a step-free subway."}];',
    "slide 8: real detail per pillar",
)

sub(
    'K.jsx("div",{className:"grid grid-cols-4 gap-4 mb-10",children:R.map(E=>'
    'K.jsxs("div",{className:"rounded-2xl p-5",'
    'style:{backgroundColor:"rgba(255,255,255,0.42)",border:"1px solid rgba(28,28,30,0.08)"},'
    'children:[K.jsx("p",{className:"font-semibold text-base mb-1",style:{color:aA},children:E.title}),'
    'K.jsx("p",{className:"text-xs",style:{color:WA},children:E.source})]},E.title))})',
    'K.jsx("div",{className:"grow grid grid-cols-4 gap-4 mb-8",children:R.map(E=>'
    'K.jsxs("div",{className:"rounded-2xl p-5 flex flex-col justify-center",'
    'style:{backgroundColor:"rgba(255,255,255,0.42)",border:"1px solid rgba(28,28,30,0.08)"},'
    'children:[K.jsx("p",{className:"font-semibold text-base mb-1",style:{color:aA},children:E.title}),'
    'K.jsx("p",{className:"text-xs font-semibold",style:{color:cA},children:E.source}),'
    'K.jsx("p",{className:"text-lg leading-relaxed mt-3",style:{color:WA},children:E.detail})]},'
    'E.title))})',
    "slide 8: grow + render detail",
)


# ── 3d. nothing smaller than 14px anywhere ─────────────────────────────────
# The floor was 10px: the team-member roles under the hero, and a couple of
# footer lines. On a projector that is not small type, it is absent type.
sub(
    'const d=E==="lg"?64:40,eA=E==="lg"?"text-sm":"text-[11px]",'
    'o=E==="lg"?"text-xs":"text-[10px]"',
    'const d=E==="lg"?124:40,eA=E==="lg"?25:15,o=E==="lg"?19:14',
    "team member: lg photo 64 -> 124px, name 25px, role 19px",
)
sub(
    'K.jsx("p",{className:`font-semibold ${eA}`,style:{color:aA},children:R.name})',
    'K.jsx("p",{className:"font-semibold",style:{color:aA,fontSize:eA},children:R.name})',
    "team name size",
)
sub(
    'K.jsx("p",{className:`${o}`,style:{color:WA},children:R.role})',
    'K.jsx("p",{style:{color:WA,fontSize:o},children:R.role})',
    "team role size",
)

# Anthony's pillar slide: the dataset line under each title was 12px.
sub(
    'K.jsx("p",{className:"text-xs font-semibold",style:{color:cA},children:E.source})',
    'K.jsx("p",{className:"text-lg font-semibold",style:{color:cD},children:E.source})',
    "pillar source line: 12 -> 16px, readable mint",
)

# Closing slide: the repo link and the capstone attribution.
sub(
    'K.jsx("p",{className:"text-xs mt-1",style:{color:WA},'
    'children:"github.com/nessaisling-lab/housecheck"})',
    'K.jsx("p",{className:"text-base mt-2",style:{color:aA},'
    'children:"github.com/nessaisling-lab/housecheck"})',
    "closing repo link larger",
)
sub(
    'K.jsx("p",{className:"text-xs mt-3",style:{color:`${WA}80`},'
    'children:"Aisling · Antonin · Jagger  •  Pursuit L2  •  '
    'Cycle 4 Capstone"})',
    'K.jsx("p",{className:"text-base mt-4",style:{color:WA},'
    'children:"Aisling · Antonin · Jagger  •  Pursuit L2  •  '
    'Cycle 4 Capstone"})',
    "closing attribution larger",
)


sub(
    'K.jsx("p",{className:"text-xs font-medium",style:{color:WA},'
    'children:"Built entirely from public NYC records · Pursuit L2 · '
    'Cycle 4 Capstone"})',
    'K.jsx("p",{className:"text-base font-medium",style:{color:aA},'
    'children:"Built entirely from public NYC records · Pursuit L2 · '
    'Cycle 4 Capstone"})',
    "hero attribution larger",
)
sub(
    'K.jsx("span",{className:"text-white text-xs font-medium",children:"Explore"})',
    'K.jsx("span",{className:"text-white text-sm font-medium",children:"Explore"})',
    "Explore chip larger",
)


# ── 3e. THE REGRESSION: every slide painted over the video ─────────────────
# Hoisting the <video> into the shell put it BEHIND the slide, and each slide
# root carries an opaque `backgroundColor: ZV`. So the city was rendering the
# whole time, with 16 sheets of solid canvas on top of it. Make the slide roots
# transparent and let Xq's wash supply the tint; the shell keeps a solid canvas
# underneath so nothing shows through if the video fails to decode.
def sub_all(old, new, label, expect):
    """Like sub(), but for a pattern that legitimately repeats."""
    global s
    n = s.count(old)
    if n != expect:
        sys.exit("COUNT %d, expected %d: %s" % (n, expect, label))
    s = s.replace(old, new)
    print("  ok  %s (x%d)" % (label, n))


# Anthony's slide roots, plus the one on his centred Carfax slide.
sub_all(
    'style:{backgroundColor:ZV}',
    'style:{backgroundColor:"transparent"}',
    "slide roots -> transparent",
    7,
)


# ── 3f. Anthony's headline accents were still at 1.38:1 ────────────────────
# The dark-mint fix landed in my h2() helper, but his seven slides hardcode the
# accent word, so "building", "Carfax", "Health Card", "Five boroughs",
# "15 minutes." and "public source." kept the unreadable bright mint. The dark
# stat card uses `children:E`, not a literal, so it keeps the bright mint that
# is correct against near-black.
sub_all(
    'style:{color:cA},children:"',
    'style:{color:cD},children:"',
    "Anthony's headline accents -> dark mint",
    7,
)


# ── "Get a map. We have map." ───────────────────────────────────────────────
# The "Bed-Stuy today / Five boroughs next" slide showed concentric dashed
# circles with a "Bed-Stuy" label and left the bottom 40% of the slide empty.
# We already hold the coordinates of every covered building, so plot them.
sub(
    io.open("radial-block.txt", encoding="utf-8").read(),
    coverage_map(),
    "boroughs slide: real coverage map replaces the decorative radial",
)

# ── 5. wire the dead "See it live" button ───────────────────────────────────
sub(
    'onClick:o=>o.stopPropagation(),children:"See it live \u2197"',
    'onClick:o=>{o.stopPropagation();window.open("__LIVE__","_blank","noopener")},'
    'children:"See it live \u2197"',
    "See it live link",
)


# ── the +/- type-size control, next to "See it live" ───────────────────────
# Her suggestion, and the right one: rather than me guessing a single size that
# suits every room, let the presenter scale the whole deck live. Applied as a
# CSS zoom on the slide layer so every size, gap and image scales together and
# the layout keeps its proportions -- bumping font-size alone would reflow the
# cards and reintroduce the overflow that was just measured out.
sub(
    'function tL(){const[R,E]=op.useState(0),',
    'function tL(){const[zm,setZm]=op.useState(1),[R,E]=op.useState(0),',
    "zoom state",
)

# Its own capsule, positioned to the left of "See it live" rather than
# restructuring that button — less to go wrong in a minified bundle.
sub(
    'K.jsx("button",{className:"absolute top-9 right-12 z-50 rounded-full px-5 py-2.5 ',
    'K.jsxs("div",{className:"absolute top-9 z-50 flex items-center gap-2 rounded-full px-2 py-1",'
    'style:{right:212,backgroundColor:"rgba(255,255,255,0.92)",'
    'border:"1px solid rgba(28,28,30,0.16)"},'
    'onClick:o=>o.stopPropagation(),children:['
    'K.jsx("button",{"aria-label":"Smaller text",className:"rounded-full font-bold",'
    'style:{width:34,height:34,color:aA,fontSize:22,lineHeight:1},'
    'onClick:()=>setZm(v=>Math.max(0.8,Math.round((v-0.1)*10)/10)),children:"−"}),'
    'K.jsx("span",{className:"font-semibold tabular-nums",'
    'style:{color:aA,fontSize:15,minWidth:46,textAlign:"center"},'
    'children:Math.round(zm*100)+"%"}),'
    'K.jsx("button",{"aria-label":"Larger text",className:"rounded-full font-bold",'
    'style:{width:34,height:34,color:aA,fontSize:22,lineHeight:1},'
    'onClick:()=>setZm(v=>Math.min(1.6,Math.round((v+0.1)*10)/10)),children:"+"})]}),'
    'K.jsx("button",{className:"absolute top-9 right-12 z-50 rounded-full px-5 py-2.5 ',
    "type-size capsule beside See it live",
)

# Apply the zoom to the slide layer only, so the chrome stays put.
sub(
    'K.jsx("div",{className:"w-full h-full relative",children:K.jsx(eA,{})})',
    'K.jsx("div",{className:"w-full h-full relative",'
    'style:{zoom:zm},children:K.jsx(eA,{})})',
    "apply zoom to the slide layer",
)

# Keyboard: +/- as well as the buttons, since a presenter's hands are on the
# arrow keys already.
sub(
    'x.key==="ArrowRight"||x.key===" "?(x.preventDefault(),d(1)):'
    'x.key==="ArrowLeft"&&(x.preventDefault(),d(-1))',
    'x.key==="ArrowRight"||x.key===" "?(x.preventDefault(),d(1)):'
    'x.key==="ArrowLeft"?(x.preventDefault(),d(-1)):'
    '(x.key==="+"||x.key==="=")?(x.preventDefault(),'
    'setZm(v=>Math.min(1.6,Math.round((v+0.1)*10)/10))):'
    '(x.key==="-"||x.key==="_")&&(x.preventDefault(),'
    'setZm(v=>Math.max(0.8,Math.round((v-0.1)*10)/10)))',
    "keyboard +/- zoom",
)


# Anthony's own quote bar was the last 1.38:1 offender — same pale-mint-on-pale
# treatment my helper replaced. Give it the dark bar so all 16 spoken lines read
# the same way from the back of a room.
sub(
    io.open("anthony-quote.txt", encoding="utf-8").read(),
    'K.jsx("div",{className:"rounded-2xl px-7 py-6",'
    'style:{backgroundColor:aA,borderLeft:`5px solid ${cA}`},'
    'children:K.jsx("p",{className:"font-semibold leading-tight tracking-[-0.02em]",'
    'style:{fontSize:33,color:cA},'
    + 'children:\'"A signal \\u2014 not a legal ruling."\'})})',
    "Anthony's quote bar -> dark bar",
)


# ═══ review pass: "show things", "back it up with sources", "put a crab" ═══




# ═══ review pass ═══════════════════════════════════════════════════════════

def source_link(label, url):
    """A dataset chip that is an actual link. Opens in a new tab; the deck is a
    local file, so window.open beats an <a> that would navigate the slide away."""
    return (
        'K.jsx("button",{className:"px-5 py-2 rounded-full text-base font-semibold",'
        'style:{border:"1px solid rgba(28,28,30,0.22)",color:cD,'
        'backgroundColor:"rgba(255,255,255,0.9)"},'
        'onClick:o=>{o.stopPropagation();window.open("' + url + '","_blank","noopener")},'
        'children:"' + label + ' \u2197"},"' + label + '")'
    )


SOURCE_LINKS = ",".join([
    source_link("HPD violations",
                "https://data.cityofnewyork.us/Housing-Development/"
                "Housing-Maintenance-Code-Violations/wvxf-dwi5"),
    source_link("311 requests",
                "https://data.cityofnewyork.us/d/erm2-nwe9"),
    source_link("PLUTO",
                "https://data.cityofnewyork.us/d/64uk-42ks"),
    source_link("DHCR registrations",
                "https://data.ny.gov/Housing/Rent-Stabilized-Buildings/39hk-dx4f"),
    source_link("Census B25064",
                "https://data.census.gov/table?q=B25064"),
])

TECH_ICONS = ",".join([
    tech_icon("Rust", RUST_CRAB, "5 crates"),
    tech_icon("Axum", AXUM_SERVER, "9 endpoints"),
    tech_icon("SQLite", SQLITE_DB, "baked in"),
    tech_icon("React", REACT_ATOM, "19"),
    tech_icon("Vite", VITE_BOLT, "7"),
])


def pipeline():
    """Ingest once, serve forever — drawn rather than described.

    "Give us the pipeline... where's the snapshot?" The point of the picture is
    that the geospatial work happens at ingest, so the serving artifact is a
    read-only file with no credentials in it.
    """
    steps = [
        ("NYC Open Data", "HPD \u00b7 311 \u00b7 PLUTO \u00b7 DHCR \u00b7 Census"),
        ("Ingest", "geocode, join, score \u2014 once"),
        ("housecheck.db", "read-only snapshot, 2026"),
        ("Rust API", "9 endpoints, zero secrets"),
    ]
    out = []
    for i, (t, d) in enumerate(steps):
        if i:
            out.append(
                'K.jsx("span",{style:{color:cD,fontSize:26,fontWeight:700},'
                'children:"\u2192"},"a%d")' % i)
        out.append(
            'K.jsxs("div",{className:"flex-1 rounded-2xl p-4",'
            'style:{backgroundColor:"rgba(255,255,255,0.9)",'
            'border:"1px solid rgba(28,28,30,0.10)"},children:['
            'K.jsx("p",{className:"font-semibold text-base",style:{color:aA},'
            'children:"' + t + '"}),'
            'K.jsx("p",{className:"text-sm leading-relaxed mt-1",style:{color:WA},'
            'children:"' + d + '"})]},"s%d")' % i)
    return ('K.jsx("div",{className:"flex items-center gap-3 mt-5",children:[%s]})'
            % ",".join(out))


CROP_PILLARS = crop(
    "pillars",
    "The four sub-scores as the app draws them: condition 1, legal 60, "
    "neighborhood 75, accessibility 90, each with its reason underneath.")
CROP_SEARCH = crop(
    "search",
    "The HouseCheck search field with a browse-all-250-buildings button below it.")
LOGO_LOCKUP = logo(320)
PIPELINE = pipeline()


# ── the four pillars, as the app actually draws them ───────────────────────
sub(
    io.open("targets/quotebar.txt", encoding="utf-8").read(),
    'K.jsxs("div",{className:"flex items-center gap-8 mt-6",children:['
    'K.jsx("div",{className:"flex-1 min-w-0",children:' + CROP_PILLARS + '}),'
    + io.open("targets/quotebar.txt", encoding="utf-8").read().replace("px-7 py-6", "px-7 py-6").replace("fontSize:33", "fontSize:29")
      .replace('borderLeft:`5px solid ${cA}`}', 'borderLeft:`5px solid ${cA}`,maxWidth:400}')
    + ']})',
    "slide 8: the real pillar card beside the quote",
)

# ── the search field, on the slide about typing an address ─────────────────
sub(
    io.open("targets/evidence.txt", encoding="utf-8").read(),
    'K.jsxs("div",{className:"flex items-center gap-8 mt-6",children:['
    'K.jsx("div",{className:"flex-1 min-w-0",children:' + CROP_SEARCH + '}),'
    'K.jsx("p",{className:"text-lg font-medium",style:{color:aA,maxWidth:420},'
    'children:"One field. Covered buildings surface as you type; anything '
    'outside the pilot is labelled, never guessed. Live on 250 real '
    'Bed-Stuy buildings."})]})',
    "evidence slide: the actual search field",
)

# ── sources you can open ───────────────────────────────────────────────────
sub(
    io.open("targets/provenance.txt", encoding="utf-8").read(),
    'K.jsxs("div",{className:"mt-5",children:['
    'K.jsx("p",{className:"text-lg font-medium",style:{color:aA},'
    'children:"Snapshot year 2026. Every score decomposes back into the '
    'records behind it — open any of these and check us. In August that check '
    "caught us: the HPD pull was requesting 50,000 records against 134,837 that "
    "matched its own query, so we held half the violations — and because the score "
    "starts at 100 and subtracts, every building read too high. Paged it. Fleet mean "
    "69.5 to 63.0, and 72 of 250 buildings changed band. The database now carries its "
    "own provenance, and the card prints what it excludes: HPD class I, 753 records, "
    'not scored."}),'
    'K.jsx("div",{className:"flex flex-wrap items-center gap-2.5 mt-4",children:['
    + SOURCE_LINKS + ']})]})',
    "provenance: openable source links",
)

# ── the stack, with its own marks ──────────────────────────────────────────
sub(
    io.open("targets/hoodstats.txt", encoding="utf-8").read(),
    'K.jsxs("div",{className:"flex items-end justify-between gap-8 mt-auto",children:['
    'K.jsxs("div",{className:"flex items-start gap-6",children:[' + TECH_ICONS + ']}),'
    'K.jsx(wS,{label:"Tests Passing",value:"106",sub:"fmt + clippy clean"})]})',
    "under the hood: tech marks beside the stats",
)

# ── the pipeline, drawn ────────────────────────────────────────────────────
sub(
    'K.jsx(yl,{children:"Provenance"}),',
    'K.jsx(yl,{children:"Provenance"}),' + PIPELINE + ',',
    "provenance: the pipeline as a picture",
)

# ── the designed logo on the closing slide ─────────────────────────────────
sub(
    io.open("targets/closelock.txt", encoding="utf-8").read(),
    'K.jsxs("div",{className:"flex flex-col items-center gap-3 mt-4",children:['
    + LOGO_LOCKUP + ',',
    "closing slide: the designed logo lockup",
)



# ── the Carfax slide was rendering washed out ──────────────────────────────
# "so this is a little strange so that needs to be fixed". Every other slide
# wraps its content in `relative z-10`; this one laid its children directly on
# the root as static siblings of Xq. An absolutely-positioned element paints
# above static in-flow content, so the legibility wash was covering the slide
# instead of sitting behind it. Give it the same wrapper the others have.
sub(
    'className:"relative w-full h-full overflow-hidden flex flex-col items-center '
    'justify-center gap-8",style:{backgroundColor:"transparent"},children:[K.jsx(Xq,{}),',
    'className:"relative w-full h-full overflow-hidden",'
    'style:{backgroundColor:"transparent"},children:[K.jsx(Xq,{}),'
    'K.jsxs("div",{className:"relative z-10 w-full h-full flex flex-col '
    'items-center justify-center gap-8",children:[',
    "carfax slide: content above the wash",
)
sub(
    'backgroundColor:"rgba(255,255,255,0.35)"},children:E},E))})]})}',
    'backgroundColor:"rgba(255,255,255,0.35)"},children:E},E))})]})]})}',
    "carfax slide: close the wrapper",
)

# ── the team, prominent instead of tucked into a corner ────────────────────
# "the profile pics name and title this could be moved to this empty spot...
#  I want it to be prominent and shown and visible." The hero's whole upper
# band was empty while the three of them were crammed against the Explore pill.
# The empty middle of the hero is the spot — centred, not tucked into a
# corner. That band was a bare flex-1 spacer doing nothing but hold air.
sub(
    'K.jsx("div",{className:"flex-1"})',
    'K.jsx("div",{className:"flex-1 flex items-center justify-center",'
    'children:K.jsx("div",{className:"flex items-start gap-16",'
    'children:jS.map(R=>K.jsx(hS,{member:R,size:"lg"},R.name))})})',
    "hero: team centred in the empty middle",
)
sub(
    ',K.jsx(PS,{}),K.jsx("div",{className:"flex items-end gap-5",'
    'children:jS.map(R=>K.jsx(hS,{member:R,size:"sm"},R.name))})',
    ',K.jsx(PS,{})',
    "hero: drop the cramped corner copy of the team",
)


# ── 6. closing slide: unregistered domain -> the URL that actually serves ───
sub(
    'children:"housechecknyc.com"',
    'children:"housecheck-wine.vercel.app"',
    "closing URL",
)

# One place resolves the live URL placeholder used above.
s = s.replace("__LIVE__", LIVE)

io.open(OUT, "w", encoding="utf-8").write(s)
print("\nwrote %s\n  %d -> %d chars (+%d)" % (OUT, orig_len, len(s), len(s) - orig_len))
