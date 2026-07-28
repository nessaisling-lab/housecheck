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

  if (hero) {
    const segments = 40;
    const filledSegments = score == null ? 0 : Math.round(filled * segments);
    const cx = size / 2;
    const cy = size * 0.68;
    const outerR = size * 0.46;
    const innerR = outerR - stroke * 2.6;
    const arcStart = 180;
    const arcEnd = 360;
    const gradientId = "hero-score-gradient";

    return (
      <div
        className="relative inline-flex items-center justify-center"
        style={{ width: size, height: size }}
        role="img"
        aria-label={score == null ? "Score unverified" : `Score ${score} of 100`}
      >
        <svg width={size} height={size} aria-hidden>
          <defs>
            <linearGradient id={gradientId} x1={0} y1={0} x2={size} y2={0} gradientUnits="userSpaceOnUse">
              <stop offset="2.7086%" stopColor="rgb(228, 159, 159)" />
              <stop offset="48.566%" stopColor="rgb(238, 192, 149)" />
              <stop offset="98.504%" stopColor="rgb(75, 205, 167)" />
            </linearGradient>
          </defs>
          {Array.from({ length: segments }).map((_, i) => {
            const t = i / (segments - 1);
            const angle = (arcStart + (arcEnd - arcStart) * t) * (Math.PI / 180);
            const isFilled = i < filledSegments;
            return (
              <line
                key={i}
                x1={cx + innerR * Math.cos(angle)}
                y1={cy + innerR * Math.sin(angle)}
                x2={cx + outerR * Math.cos(angle)}
                y2={cy + outerR * Math.sin(angle)}
                stroke={isFilled ? `url(#${gradientId})` : "rgba(60,60,67,0.16)"}
                strokeWidth={Math.max(2.5, stroke * 0.3)}
                strokeLinecap="round"
                style={
                  animate
                    ? ({
                        opacity: isFilled ? 0 : 1,
                        animation: isFilled
                          ? `hc-fade-in 0.28s ease-out ${i * 18}ms forwards`
                          : undefined,
                      } as React.CSSProperties)
                    : undefined
                }
              />
            );
          })}
        </svg>
        <div className="absolute inset-x-0 top-[42%] flex flex-col items-center">
          <span
            className="font-semibold leading-none"
            style={{ fontSize: size * 0.275, letterSpacing: 0, color: "var(--hc-canvas-ink)" }}
          >
            {score ?? "—"}
          </span>
          <span className="mt-1 text-[13px]" style={{ color: "var(--hc-canvas-ink-3)" }}>
            of 100
          </span>
        </div>
      </div>
    );
  }

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
