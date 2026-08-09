# Derives the print theme from the app's own tokens.
#
# The band colours in frontend/src/index.css were tuned to clear WCAG 4.5:1 against the DARK
# card (--hc-sunken #48484A). Moving them onto a light page changes the surface, and a colour
# is only "accessible" against a stated surface — that is the whole argument of chapter 11.
# So: keep hue and saturation, re-derive lightness for the new ground, exactly the method the
# app's CSS comment describes. Same identity, correct on paper.
import colorsys

APP = {
    "canvas": (215, 215, 217),
    "card": (0x3A, 0x3A, 0x3C),
    "sunken": (0x48, 0x48, 0x4A),
    "ink": (0xF5, 0xF5, 0xF7),
    "strong": (0x5E, 0xCC, 0x79),
    "solid": (0xA6, 0xD6, 0x5E),
    "mixed": (0xE2, 0xAE, 0x4A),
    "concern": (0xF5, 0xA3, 0x7F),
    "critical": (0xF6, 0x9F, 0xA1),
    "unverified": (0xB6, 0xB6, 0xBA),
}

PAPER = (0xFA, 0xFA, 0xFB)      # near-white, carrying the canvas's faint blue-grey cast
INK = (0x1C, 0x1C, 0x1E)        # --hc-canvas-ink at full opacity


def lin(c):
    c /= 255
    return c / 12.92 if c <= 0.03928 else ((c + 0.055) / 1.055) ** 2.4


def lum(rgb):
    return 0.2126 * lin(rgb[0]) + 0.7152 * lin(rgb[1]) + 0.0722 * lin(rgb[2])


def contrast(a, b):
    la, lb = lum(a), lum(b)
    hi, lo = max(la, lb), min(la, lb)
    return (hi + 0.05) / (lo + 0.05)


def darken_to(rgb, ground, target=4.5):
    """Lower HLS lightness until the colour clears `target` on `ground`. Hue/sat untouched."""
    h, l, s = colorsys.rgb_to_hls(*[c / 255 for c in rgb])
    for i in range(1000):
        cand = tuple(round(c * 255) for c in colorsys.hls_to_rgb(h, max(0.0, l - i * 0.001), s))
        if contrast(cand, ground) >= target:
            return cand
    return (0, 0, 0)


def hexs(rgb):
    return "#%02X%02X%02X" % rgb


if __name__ == "__main__":
    print("  band colours, app value -> print value (target 4.5:1 on paper %s)\n" % hexs(PAPER))
    print("  %-12s %-9s %-7s   %-9s %-7s" % ("token", "app", "on paper", "print", "on paper"))
    for k in ("strong", "solid", "mixed", "concern", "critical", "unverified"):
        app = APP[k]
        pr = darken_to(app, PAPER)
        print("  %-12s %-9s %5.2f:1   %-9s %5.2f:1  %s"
              % (k, hexs(app), contrast(app, PAPER), hexs(pr), contrast(pr, PAPER),
                 "unchanged" if pr == app else ""))
    print("\n  body ink %s on paper: %.2f:1" % (hexs(INK), contrast(INK, PAPER)))
    print("  app ink %s on card %s: %.2f:1  (callouts keep the app's own pairing)"
          % (hexs(APP["ink"]), hexs(APP["card"]), contrast(APP["ink"], APP["card"])))
