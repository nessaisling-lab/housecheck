# Make the deck readable on a phone.
#
# THE PROBLEM
# Every slide is `w-screen h-screen overflow-hidden` with no transform anywhere, so on a
# narrow screen the content is not scaled down -- it is CLIPPED. A four-column grid squeezes
# to ~80px columns and everything below the fold is simply gone.
#
# THE APPROACH, AND WHY NOT THE OBVIOUS ONE
# The obvious fix is a CSS transform that scales the 1280x720 stage to fit. It preserves the
# design exactly and can never clip. It is also wrong here: at 390px wide that is a ~30% scale
# and the body text lands around 5px. This deck is going to be hosted for people to *read*,
# so it has to reflow, not shrink.
#
# So: below 860px the slide stops being a fixed viewport and becomes a scrollable column.
# Grids stack, rows stack, type steps down, and the chrome is rebuilt for thumbs.
#
# WHY THIS IS PLAIN CSS AND NOT TAILWIND CLASSES
# The bundle ships a fixed, precompiled Tailwind set, so a class it does not contain fails
# silently -- that constraint has bitten this deck twice. Authoring raw CSS sidesteps it
# entirely: these rules override the utility classes by specificity and !important, and
# nothing depends on what Tailwind happened to compile.
#
# NAVIGATION
# There is no swipe handler in the bundle (the touchstart/touchend hits in the source are
# React's internal event registry, not app code). Navigation is the two 64px full-height
# edge overlays plus a row of 6px dots. On a phone the overlays would eat a third of the
# screen and swallow taps meant for the new citation links, and 6px is not a thumb target.
# So the overlays are re-anchored into a real bottom Back/Next bar -- same click handlers,
# no JS change -- and the dots get an invisible enlarged hit area via ::after.
import io
import sys

P = r"D:\L2 Cycle 4\Housecheck Antonin Idea\docs\deck\HouseCheck-Presentation.html"
MARK = "hc-mobile-layer"

CSS = """
/* ================= HouseCheck mobile layer =================
   Phones and small tablets. Desktop is untouched: every rule is inside this
   media query. See docs/deck/mobile_layer.py for the reasoning. */
@media (max-width: 860px) {

  html, body { overflow-x: hidden !important; }

  /* --- the stage stops clipping and starts scrolling --------------------- */
  .w-screen.h-screen {
    height: auto !important;
    min-height: 100svh;
    overflow: visible !important;
  }
  .w-screen.h-screen > .w-full.h-full.relative { height: auto !important; }
  .relative.w-full.h-full.overflow-hidden {
    height: auto !important;
    overflow: visible !important;
  }
  /* Top padding clears the fixed header controls; bottom clears the nav bar. */
  .relative.z-10.w-full.h-full {
    height: auto !important;
    min-height: 100svh;
    padding: 84px 16px 128px !important;
  }

  /* The page background is fixed so it covers a document of any height. */
  .absolute.inset-0.w-full.h-full.object-cover { position: fixed !important; }

  /* --- layout: everything side-by-side becomes stacked ------------------- */
  .grid.grid-cols-4 {
    display: flex !important;
    flex-direction: column !important;
    gap: 12px !important;
  }
  .flex.items-stretch { flex-direction: column !important; }
  .flex.items-center.gap-8 {
    flex-direction: column !important;
    align-items: stretch !important;
    gap: 14px !important;
  }
  /* Team avatars and the citation strips wrap rather than overflow. */
  .flex.items-start.gap-16 {
    flex-wrap: wrap !important;
    gap: 18px !important;
    justify-content: center !important;
  }
  .flex.items-center.gap-6 { flex-wrap: wrap !important; gap: 8px 14px !important; }
  .flex.items-center.gap-4 { flex-wrap: wrap !important; }

  /* Generic collapse. Some slides build their columns with bare flex-1 children and no
     items-stretch/items-center hook to target, so rather than chase each one: let content
     rows wrap, and make every flex-1 child claim a full row. The dots and the header
     controls use gap-2 and are deliberately excluded. */
  .flex.gap-3, .flex.gap-4, .flex.gap-5,
  .flex.gap-6, .flex.gap-8, .flex.gap-16 { flex-wrap: wrap !important; }
  .flex-1 { flex-basis: 100% !important; min-width: 0 !important; }

  /* NOTE: deliberately no global `svg` rule. A blanket
     `svg { height:auto; max-width:100% }` collapsed a 30px inline icon inside an
     inline-flex to 0x0 -- SVGs sized by width/height attributes do not survive being
     told their height is auto. The only wide decorative SVG is the closing wordmark at
     180px, which already fits a 390px screen, so nothing needs clamping.
     Overhanging inner geometry on the title slide is clipped by an ancestor and causes
     no horizontal scroll, which is verified rather than assumed. */

  /* Anything with an explicit max-width should not force a horizontal scroll. */
  [style*="maxWidth"], [style*="max-width"] { max-width: 100% !important; }

  /* --- type scale -------------------------------------------------------- */
  h1 { font-size: 32px !important; line-height: 1.08 !important; }
  h2 { font-size: 26px !important; line-height: 1.16 !important; margin-bottom: 12px !important; }
  .text-2xl { font-size: 18px !important; }
  .text-xl  { font-size: 17px !important; }
  .text-lg  { font-size: 15px !important; }
  .text-base{ font-size: 14px !important; }
  .text-sm  { font-size: 12.5px !important; }

  /* --- cards tighten up -------------------------------------------------- */
  .rounded-2xl { border-radius: 14px !important; }
  .p-5  { padding: 14px !important; }
  .px-7 { padding-left: 16px !important; padding-right: 16px !important; }
  .py-6 { padding-top: 13px !important; padding-bottom: 13px !important; }
  .mb-10 { margin-bottom: 14px !important; }
  .mb-8, .mb-6 { margin-bottom: 12px !important; }
  .mt-6 { margin-top: 12px !important; }
  .gap-10 { gap: 16px !important; }

  /* Screenshots: let them use the full column, never force overflow. */
  img { max-height: none !important; height: auto !important; max-width: 100% !important; }

  /* --- header chrome ----------------------------------------------------- */
  .absolute.top-9 { top: 10px !important; }
  .absolute.top-9.right-12 { right: 10px !important; }

  /* --- navigation: the edge overlays become a bottom Back/Next bar -------- */
  .absolute.left-0.top-0.h-full.w-16,
  .absolute.right-0.top-0.h-full.w-16 {
    position: fixed !important;
    top: auto !important;
    bottom: 0 !important;
    height: 58px !important;
    width: 50% !important;
    z-index: 45 !important;
    display: flex !important;
    align-items: center !important;
    background: rgba(255,255,255,0.90);
    -webkit-backdrop-filter: blur(10px);
    backdrop-filter: blur(10px);
    border-top: 1px solid rgba(28,28,30,0.10);
    font-size: 15px;
    font-weight: 600;
    color: #1D6A53;
  }
  .absolute.left-0.top-0.h-full.w-16  { left: 0 !important;  justify-content: flex-start !important; }
  .absolute.right-0.top-0.h-full.w-16 { right: 0 !important; justify-content: flex-end !important; }
  .absolute.left-0.top-0.h-full.w-16::after  { content: "\\2039  Back"; padding-left: 20px; }
  .absolute.right-0.top-0.h-full.w-16::after { content: "Next  \\203A"; padding-right: 20px; }

  /* Dots ride above the bar, on their own readable pill. */
  .absolute.bottom-5 {
    position: fixed !important;
    bottom: 70px !important;
    z-index: 50 !important;
    padding: 7px 11px !important;
    border-radius: 999px !important;
    background: rgba(255,255,255,0.92);
    -webkit-backdrop-filter: blur(10px);
    backdrop-filter: blur(10px);
    max-width: calc(100vw - 32px);
    flex-wrap: wrap !important;
    justify-content: center !important;
    row-gap: 6px !important;
  }
  /* A 6px dot is not a thumb target. Enlarge the hit area without changing the look. */
  .absolute.bottom-5 button { position: relative; }
  .absolute.bottom-5 button::after {
    content: "";
    position: absolute;
    inset: -11px -4px;
  }

  /* Links stay obviously tappable. */
  a { word-break: break-word; }
}

/* ---- Middle tier: tablets and small laptops -------------------------------
   861-1180px keeps the multi-column design -- a four-across grid is still right on
   an iPad -- but the fixed-height stage does not fit, so four slides clipped at
   1024x768. Rather than reflow, just stop clipping: the slide grows and the page
   scrolls. Same trick as the phone layer, none of the restyling. */
@media (min-width: 861px) and (max-width: 1180px) {
  .w-screen.h-screen {
    height: auto !important;
    min-height: 100svh;
    overflow: visible !important;
  }
  .w-screen.h-screen > .w-full.h-full.relative { height: auto !important; }
  .relative.w-full.h-full.overflow-hidden {
    height: auto !important;
    overflow: visible !important;
  }
  .relative.z-10.w-full.h-full {
    height: auto !important;
    min-height: 100svh;
    padding-bottom: 84px !important;
  }
  .absolute.inset-0.w-full.h-full.object-cover { position: fixed !important; }
  .absolute.bottom-5 {
    position: fixed !important;
    bottom: 16px !important;
    padding: 6px 10px !important;
    border-radius: 999px !important;
    background: rgba(255,255,255,0.92);
    -webkit-backdrop-filter: blur(10px);
    backdrop-filter: blur(10px);
  }
}

/* Short viewports at any width -- a 1280x640 browser window with toolbars open --
   hit the same wall for the same reason. */
@media (min-width: 1181px) and (max-height: 700px) {
  .w-screen.h-screen { height: auto !important; min-height: 100svh; overflow: visible !important; }
  .w-screen.h-screen > .w-full.h-full.relative { height: auto !important; }
  .relative.w-full.h-full.overflow-hidden { height: auto !important; overflow: visible !important; }
  .relative.z-10.w-full.h-full { height: auto !important; min-height: 100svh; padding-bottom: 84px !important; }
  .absolute.inset-0.w-full.h-full.object-cover { position: fixed !important; }
  .absolute.bottom-5 { position: fixed !important; bottom: 14px !important; }
}

/* Very small phones: one more step down. */
@media (max-width: 400px) {
  .relative.z-10.w-full.h-full { padding: 78px 13px 124px !important; }
  h1 { font-size: 28px !important; }
  h2 { font-size: 23px !important; }
  .text-lg { font-size: 14.5px !important; }
}
"""


def main():
    s = io.open(P, encoding="utf-8", errors="replace").read()

    if MARK in s:
        # Idempotent: strip the previous layer so this can be re-run while iterating.
        start = s.find("<style id=\"%s\">" % MARK)
        end = s.find("</style>", start) + len("</style>")
        s = s[:start] + s[end:]
        print("  removed previous mobile layer")

    anchor = "</body>"
    if anchor not in s:
        print("  no </body> found")
        sys.exit(1)

    block = '<style id="%s">%s</style>\n' % (MARK, CSS)
    s = s.replace(anchor, block + anchor, 1)

    io.open(P, "w", encoding="utf-8", newline="").write(s)
    print("  mobile layer injected (%.1f KB of CSS)" % (len(CSS) / 1024))


if __name__ == "__main__":
    main()
