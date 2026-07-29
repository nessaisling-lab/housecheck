import { COVERAGE_POINTS } from "@/lib/coverage-points";

interface CoverageMapProps {
  /** CSS height. The map fills the width of its container. */
  height?: number;
  className?: string;
  /** Caption under the map. Pass null to omit it. */
  caption?: string | null;
}

const W = 320;
const H = 160;
const PAD = 14;

/**
 * The Bed-Stuy pilot footprint, drawn from the real coordinates of all 250
 * covered buildings (`lib/coverage-points.ts`, generated from the shipped DB).
 *
 * This replaces a decorative scatter of ten hardcoded dots. Two reasons it had
 * to go: the dots were `rgba(60,60,67,0.35)` on `--hc-sunken`, which after the
 * theme inversion meant dark grey on dark grey — the box rendered empty. And a
 * fake map sitting under the words "Coverage: Bed-Stuy pilot area" is a claim
 * about our data that the picture wasn't actually making.
 */
export function CoverageMap({ height = 156, className = "", caption = "Coverage: Bed-Stuy pilot area" }: CoverageMapProps) {
  const n = COVERAGE_POINTS.length;
  return (
    <figure className={`mt-5 ${className}`}>
      <div
        className="overflow-hidden rounded-2xl"
        style={{ background: "var(--hc-sunken)", height }}
      >
        <svg
          viewBox={`0 0 ${W} ${H}`}
          width="100%"
          height="100%"
          preserveAspectRatio="xMidYMid meet"
          role="img"
          aria-label={`Map of the HouseCheck pilot area in Bedford-Stuyvesant, showing the ${n} buildings currently covered.`}
        >
          {/* Faint grid — reads as street blocks without pretending to be
              real streets, which we do not have geometry for. */}
          <g stroke="rgba(255,255,255,0.06)" strokeWidth="1">
            {[0, 1, 2, 3, 4, 5].map((i) => (
              <line key={`v${i}`} x1={PAD + i * ((W - PAD * 2) / 5)} y1={PAD} x2={PAD + i * ((W - PAD * 2) / 5)} y2={H - PAD} />
            ))}
            {[0, 1, 2, 3].map((i) => (
              <line key={`h${i}`} x1={PAD} y1={PAD + i * ((H - PAD * 2) / 3)} x2={W - PAD} y2={PAD + i * ((H - PAD * 2) / 3)} />
            ))}
          </g>

          {COVERAGE_POINTS.map(([x, y], i) => (
            <circle
              key={i}
              cx={PAD + x * (W - PAD * 2)}
              cy={PAD + y * (H - PAD * 2)}
              r="2.1"
              fill="var(--hc-strong)"
              opacity="0.85"
            />
          ))}
        </svg>
      </div>
      {caption && (
        <figcaption className="mt-2 text-center text-[0.75rem]" style={{ color: "var(--hc-ink-3)" }}>
          {caption} · {n} buildings
        </figcaption>
      )}
    </figure>
  );
}
