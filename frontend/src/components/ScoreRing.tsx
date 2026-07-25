import { scoreCircleColor } from "@/lib/score";

interface ScoreRingProps {
  score: number | null | undefined;
  size?: number;
  stroke?: number;
  /** show the big number + "of 100" inside (hero mode) */
  hero?: boolean;
  /** animate the arc drawing in */
  animate?: boolean;
  trackColor?: string;
}

/**
 * Thin concentric ring gauge (Whoop recovery dial → HouseCheck hero).
 * Track = surface/sunken, arc = score % in band color, rounded caps.
 */
export function ScoreRing({
  score,
  size = 180,
  stroke = 11,
  hero = false,
  animate = false,
  trackColor = "#EFEFF2",
}: ScoreRingProps) {
  const value = score ?? 0;
  const r = (size - stroke) / 2;
  const c = 2 * Math.PI * r;
  const filled = Math.max(0, Math.min(100, value)) / 100;
  const offset = c * (1 - filled);
  const color = scoreCircleColor(score);

  return (
    <div
      className="relative inline-flex items-center justify-center"
      style={{ width: size, height: size }}
      role="img"
      aria-label={score == null ? "Score unverified" : `Score ${score} of 100`}
    >
      <svg width={size} height={size} className="-rotate-90">
        <circle
          cx={size / 2}
          cy={size / 2}
          r={r}
          fill="none"
          stroke={trackColor}
          strokeWidth={stroke}
        />
        {score != null && (
          <circle
            cx={size / 2}
            cy={size / 2}
            r={r}
            fill="none"
            stroke={color}
            strokeWidth={stroke}
            strokeLinecap="round"
            strokeDasharray={c}
            strokeDashoffset={offset}
            style={
              animate
                ? ({
                    ["--ring-c" as string]: c,
                    ["--ring-o" as string]: offset,
                    animation: "hc-ring-draw 0.9s cubic-bezier(0.22,1,0.36,1) forwards",
                    strokeDashoffset: c,
                  } as React.CSSProperties)
                : undefined
            }
          />
        )}
      </svg>
      {hero && (
        <div className="absolute inset-0 flex flex-col items-center justify-center">
          <span
            className="font-semibold leading-none"
            style={{ fontSize: 72, letterSpacing: "-0.02em", color: "var(--hc-ink)" }}
          >
            {score ?? "—"}
          </span>
          <span className="mt-1 text-[13px]" style={{ color: "var(--hc-ink-3)" }}>
            of 100
          </span>
        </div>
      )}
    </div>
  );
}

/** Mini ring for sub-score rows, sticky strip, lists (Whoop collapsed rings). */
export function MiniRing({
  score,
  size = 32,
  stroke = 4,
}: {
  score: number | null | undefined;
  size?: number;
  stroke?: number;
}) {
  const value = score ?? 0;
  const r = (size - stroke) / 2;
  const c = 2 * Math.PI * r;
  const offset = c * (1 - Math.max(0, Math.min(100, value)) / 100);
  return (
    <svg width={size} height={size} className="-rotate-90 shrink-0" aria-hidden>
      <circle cx={size / 2} cy={size / 2} r={r} fill="none" stroke="#EFEFF2" strokeWidth={stroke} />
      {score != null && (
        <circle
          cx={size / 2}
          cy={size / 2}
          r={r}
          fill="none"
          stroke={scoreCircleColor(score)}
          strokeWidth={stroke}
          strokeLinecap="round"
          strokeDasharray={c}
          strokeDashoffset={offset}
        />
      )}
    </svg>
  );
}
