# Make the deck readable on a phone, and let the reader choose.
#
# THE PROBLEM
# Every slide is `w-screen h-screen overflow-hidden` with no transform anywhere, so on a
# narrow screen the content is not scaled down -- it is CLIPPED. A four-column grid squeezes
# to ~80px columns and everything below the fold is simply gone.
#
# THE APPROACH, AND WHY NOT THE OBVIOUS ONE
# The obvious fix is a CSS transform scaling the 1280x720 stage to fit. It preserves the
# design exactly and can never clip. It is also wrong here: at 390px that is a ~30% scale and
# body text lands near 5px. This deck is hosted for people to *read*, so it reflows.
#
# WHY RAW CSS AND NOT TAILWIND CLASSES
# The bundle ships a fixed precompiled Tailwind set, so a class it does not contain fails
# silently -- that has bitten this deck twice. Hand-written CSS overrides the utilities by
# specificity and does not care what Tailwind compiled.
#
# NAVIGATION
# There is no swipe handler (the touchstart/touchend hits in the source are React's internal
# event registry, not app code). Navigation is two 64px full-height edge overlays plus 6px
# dots. On a phone the overlays eat a third of the screen and swallow taps meant for the
# citation links, and 6px is not a thumb target. So the overlays are re-anchored into a real
# bottom Back/Next bar -- same click handlers, no JS change -- and the dots get an invisible
# enlarged hit area.
#
# THE TOGGLE
# Browser device-emulation is not something to rely on, so the deck ships its own switch.
# It needs to work in BOTH directions -- force the phone layout on a desktop, and force the
# desktop layout on a phone -- and a class alone cannot undo a media query. So the same rule
# set is emitted twice:
#
#   @media (max-width: 860px) { html:not(.hc-force-desktop) <rule> }   <- automatic
#   html.hc-force-mobile <rule>                                        <- forced on
#
# One source of truth (PHONE), mechanically prefixed, so the two can never drift.
import io
import re
import sys

P = r"D:\L2 Cycle 4\Housecheck Antonin Idea\docs\deck\HouseCheck-Presentation.html"
MARK = "hc-mobile-layer"
JSMARK = "hc-view-toggle-script"

# ---------------------------------------------------------------------------------
# The phone rule set. Flat CSS only -- every selector is prefixed programmatically,
# so no nesting and no `html` selectors (use `body`, which prefixes cleanly).
# ---------------------------------------------------------------------------------
PHONE = """
body { overflow-x: hidden !important; }

/* --- the stage stops clipping and starts scrolling --- */
.w-screen.h-screen { height: auto !important; min-height: 100svh; overflow: visible !important; }
.w-screen.h-screen > .w-full.h-full.relative { height: auto !important; }
.relative.w-full.h-full.overflow-hidden { height: auto !important; overflow: visible !important; }
/* Top padding clears the header controls; bottom clears the nav bar. */
.relative.z-10.w-full.h-full { height: auto !important; min-height: 100svh; padding: 84px 16px 128px !important; }
/* Fixed so it covers a document of any height. */
.absolute.inset-0.w-full.h-full.object-cover { position: fixed !important; }

/* --- everything side-by-side becomes stacked --- */
.grid.grid-cols-4 { display: flex !important; flex-direction: column !important; gap: 12px !important; }
.flex.items-stretch { flex-direction: column !important; }
.flex.items-center.gap-8 { flex-direction: column !important; align-items: stretch !important; gap: 14px !important; }
.flex.items-start.gap-16 { flex-wrap: wrap !important; gap: 18px !important; justify-content: center !important; }
.flex.items-center.gap-6 { flex-wrap: wrap !important; gap: 8px 14px !important; }
.flex.items-center.gap-4 { flex-wrap: wrap !important; }
/* Generic collapse: some slides build columns from bare flex-1 children with no
   items-* hook to target. Dots and header controls use gap-2 and are excluded. */
.flex.gap-3, .flex.gap-4, .flex.gap-5, .flex.gap-6, .flex.gap-8, .flex.gap-16 { flex-wrap: wrap !important; }
.flex-1 { flex-basis: 100% !important; min-width: 0 !important; }

/* NOTE: deliberately no global `svg` rule. A blanket svg{height:auto;max-width:100%}
   collapsed a 30px inline icon to 0x0 -- an SVG sized by width/height attributes does
   not survive being told its height is auto. The only wide decorative SVG is the 180px
   closing wordmark, which already fits a 390px screen. */
[style*="maxWidth"], [style*="max-width"] { max-width: 100% !important; }

/* --- type scale --- */
h1 { font-size: 32px !important; line-height: 1.08 !important; }
h2 { font-size: 26px !important; line-height: 1.16 !important; margin-bottom: 12px !important; }
.text-2xl { font-size: 18px !important; }
.text-xl { font-size: 17px !important; }
.text-lg { font-size: 15px !important; }
.text-base { font-size: 14px !important; }
.text-sm { font-size: 12.5px !important; }

/* --- cards tighten --- */
.rounded-2xl { border-radius: 14px !important; }
.p-5 { padding: 14px !important; }
.px-7 { padding-left: 16px !important; padding-right: 16px !important; }
.py-6 { padding-top: 13px !important; padding-bottom: 13px !important; }
.mb-10 { margin-bottom: 14px !important; }
.mb-8, .mb-6 { margin-bottom: 12px !important; }
.mt-6 { margin-top: 12px !important; }
.gap-10 { gap: 16px !important; }
img { max-height: none !important; height: auto !important; max-width: 100% !important; }

/* --- header chrome --- */
.absolute.top-9 { top: 10px !important; }
.absolute.top-9.right-12 { right: 10px !important; }

/* --- navigation: edge overlays become a bottom Back/Next bar --- */
.absolute.left-0.top-0.h-full.w-16, .absolute.right-0.top-0.h-full.w-16 {
  position: fixed !important; top: auto !important; bottom: 0 !important;
  height: 58px !important; width: 50% !important; z-index: 45 !important;
  display: flex !important; align-items: center !important;
  background: rgba(255,255,255,0.90);
  -webkit-backdrop-filter: blur(10px); backdrop-filter: blur(10px);
  border-top: 1px solid rgba(28,28,30,0.10);
  font-size: 15px; font-weight: 600; color: #1D6A53;
}
.absolute.left-0.top-0.h-full.w-16 { left: 0 !important; justify-content: flex-start !important; }
.absolute.right-0.top-0.h-full.w-16 { right: 0 !important; justify-content: flex-end !important; }
.absolute.left-0.top-0.h-full.w-16::after { content: "\\2039  Back"; padding-left: 20px; }
.absolute.right-0.top-0.h-full.w-16::after { content: "Next  \\203A"; padding-right: 20px; }

/* Dots ride above the bar. A 6px dot is not a thumb target, so the hit area is
   enlarged with ::after without changing how it looks. */
.absolute.bottom-5 {
  position: fixed !important; bottom: 70px !important; z-index: 50 !important;
  padding: 7px 11px !important; border-radius: 999px !important;
  background: rgba(255,255,255,0.92);
  -webkit-backdrop-filter: blur(10px); backdrop-filter: blur(10px);
  max-width: calc(100vw - 32px); flex-wrap: wrap !important;
  justify-content: center !important; row-gap: 6px !important;
}
.absolute.bottom-5 button { position: relative; }
.absolute.bottom-5 button::after { content: ""; position: absolute; inset: -11px -4px; }

a { word-break: break-word; }

/* The view switch moves out of the way of the bottom bar. */
#hc-view-toggle { bottom: auto !important; top: 62px !important; left: 12px !important; }
"""

SMALL_PHONE = """
.relative.z-10.w-full.h-full { padding: 78px 13px 124px !important; }
h1 { font-size: 28px !important; }
h2 { font-size: 23px !important; }
.text-lg { font-size: 14.5px !important; }
"""

# Tablets and small laptops keep the multi-column design -- four-across is still right on
# an iPad -- but the fixed-height stage does not fit, so four slides clipped at 1024x768.
# Do not reflow; just stop clipping.
ANTI_CLIP = """
.w-screen.h-screen { height: auto !important; min-height: 100svh; overflow: visible !important; }
.w-screen.h-screen > .w-full.h-full.relative { height: auto !important; }
.relative.w-full.h-full.overflow-hidden { height: auto !important; overflow: visible !important; }
.relative.z-10.w-full.h-full { height: auto !important; min-height: 100svh; padding-bottom: 84px !important; }
.absolute.inset-0.w-full.h-full.object-cover { position: fixed !important; }
.absolute.bottom-5 {
  position: fixed !important; bottom: 16px !important;
  padding: 6px 10px !important; border-radius: 999px !important;
  background: rgba(255,255,255,0.92);
  -webkit-backdrop-filter: blur(10px); backdrop-filter: blur(10px);
}
"""

# The switch itself. Always visible, never inside a media query.
TOGGLE_CHROME = """
#hc-view-toggle {
  position: fixed; left: 16px; bottom: 16px; z-index: 60;
  font: 600 13px/1 Inter, system-ui, sans-serif; letter-spacing: -0.01em;
  padding: 9px 14px; border-radius: 999px; cursor: pointer;
  color: #1D6A53; background: rgba(255,255,255,0.92);
  border: 1px solid rgba(28,28,30,0.12);
  -webkit-backdrop-filter: blur(10px); backdrop-filter: blur(10px);
  box-shadow: 0 2px 10px rgba(0,0,0,0.08);
}
#hc-view-toggle:hover { background: #fff; }
#hc-view-toggle:focus-visible { outline: 2px solid #1D6A53; outline-offset: 2px; }
@media print { #hc-view-toggle { display: none; } }
"""

RULE = re.compile(r"([^{}]+)\{([^{}]*)\}", re.S)


def prefix(css, pre):
    """Prefix every top-level selector. Flat CSS only -- asserted, not assumed."""
    assert "@media" not in css, "prefix() cannot handle nested at-rules"
    out = []
    for sel, decls in RULE.findall(css):
        comments = re.findall(r"/\*.*?\*/", sel, re.S)
        clean = re.sub(r"/\*.*?\*/", "", sel).strip()
        if not clean:
            continue
        parts = [("%s %s" % (pre, p.strip())).strip() for p in clean.split(",")]
        out.append("%s%s { %s }" % ("".join(comments), ", ".join(parts), decls.strip()))
    return "\n".join(out)


def build_css():
    auto = "html:not(.hc-force-desktop)"
    forced = "html.hc-force-mobile"
    return "\n".join([
        "/* ===== HouseCheck responsive layer -- see docs/deck/mobile_layer.py ===== */",
        TOGGLE_CHROME,
        "/* Phones, automatic. Suppressed when the reader forces the desktop view. */",
        "@media (max-width: 860px) {\n%s\n}" % prefix(PHONE, auto),
        "/* Phones, forced from the toggle at any width. */",
        prefix(PHONE, forced),
        # The very-small step stays gated on real width even when forced: someone
        # previewing the phone layout on a 1440px monitor should get the 860px type
        # scale, not the 360px one.
        ("@media (max-width: 400px) {\n%s\n%s\n}"
         % (prefix(SMALL_PHONE, auto), prefix(SMALL_PHONE, forced))),
        "/* Tablets and small laptops: keep the columns, stop the clipping. */",
        "@media (min-width: 861px) and (max-width: 1180px) {\n%s\n}" % prefix(ANTI_CLIP, auto),
        "/* Short viewports at any width -- a desktop window with toolbars open. */",
        "@media (min-width: 1181px) and (max-height: 700px) {\n%s\n}" % prefix(ANTI_CLIP, auto),
    ])


JS = """
(function () {
  var KEY = 'hc-view-mode';           // 'auto' | 'mobile' | 'desktop'
  var root = document.documentElement;

  function paint(mode, btn) {
    root.classList.toggle('hc-force-mobile', mode === 'mobile');
    root.classList.toggle('hc-force-desktop', mode === 'desktop');
    var narrow = window.matchMedia('(max-width: 860px)').matches;
    var showingPhone = mode === 'mobile' || (mode === 'auto' && narrow);
    btn.textContent = showingPhone ? 'Desktop view' : 'Mobile view';
    btn.setAttribute('aria-pressed', showingPhone ? 'true' : 'false');
    btn.title = 'Currently ' + (showingPhone ? 'mobile' : 'desktop') +
                ' layout. Click to switch. (Saved for next visit.)';
  }

  function start() {
    if (document.getElementById('hc-view-toggle')) return;
    var btn = document.createElement('button');
    btn.id = 'hc-view-toggle';
    btn.type = 'button';

    var mode = 'auto';
    try { mode = localStorage.getItem(KEY) || 'auto'; } catch (e) {}

    btn.addEventListener('click', function (e) {
      e.stopPropagation();          // the stage listens for clicks to advance slides
      var narrow = window.matchMedia('(max-width: 860px)').matches;
      var showingPhone = mode === 'mobile' || (mode === 'auto' && narrow);
      mode = showingPhone ? 'desktop' : 'mobile';
      try { localStorage.setItem(KEY, mode); } catch (err) {}
      paint(mode, btn);
      window.scrollTo(0, 0);
    });

    document.body.appendChild(btn);
    paint(mode, btn);
    // Keep the label honest when the window is resized across the breakpoint.
    window.addEventListener('resize', function () { paint(mode, btn); });
  }

  if (document.readyState === 'loading') {
    document.addEventListener('DOMContentLoaded', start);
  } else {
    start();
  }
})();
"""


def main():
    s = io.open(P, encoding="utf-8", errors="replace").read()

    # Idempotent: strip previous blocks so this can be re-run while iterating.
    for tag, mark in (("style", MARK), ("script", JSMARK)):
        open_tag = '<%s id="%s">' % (tag, mark)
        if open_tag in s:
            start = s.find(open_tag)
            end = s.find("</%s>" % tag, start) + len("</%s>" % tag)
            s = s[:start] + s[end:]
            print("  removed previous <%s>" % tag)

    if "</body>" not in s:
        print("  no </body> found")
        sys.exit(1)

    css = build_css()
    block = ('<style id="%s">%s</style>\n<script id="%s">%s</script>\n'
             % (MARK, css, JSMARK, JS))
    s = s.replace("</body>", block + "</body>", 1)

    io.open(P, "w", encoding="utf-8", newline="").write(s)
    print("  injected %.1f KB CSS + toggle script" % (len(css) / 1024))


if __name__ == "__main__":
    main()
