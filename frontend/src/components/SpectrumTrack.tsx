import { bandColor } from "@/lib/score";

interface SpectrumTrackProps {
  /** marker position 0..1 (null = no marker yet) */
  position: number | null;
  markerColor?: string;
  markerLabel?: string | null;
  /** optional HUD FMR reference line position 0..1 */
  reference?: number | null;
  referenceLabel?: string;
  leftLabel?: string;
  rightLabel?: string;
}

/**
 * Rent fairness track — "Pace of Aging" tick spectrum (Whoop autopsy #7).
 * Ruler of vertical ticks on surface/sunken; marker = 4px rounded bar in verdict color.
 */
export function SpectrumTrack({
  position,
  markerColor = "#1C1C1E",
  markerLabel,
  reference = null,
  referenceLabel,
  leftLabel = "Below median",
  rightLabel = "Above median",
}: SpectrumTrackProps) {
  const TICKS = 41;
  return (
    <div className="w-full">
      <div className="relative">
        {markerLabel != null && position != null && (
          <div
            className="absolute -top-7 -translate-x-1/2 whitespace-nowrap text-[12px] font-semibold"
            style={{ left: `${position * 100}%`, color: markerColor }}
          >
            {markerLabel}
          </div>
        )}
        <div
          className="relative flex h-14 items-center justify-between rounded-xl px-3"
          style={{ background: "var(--hc-sunken)" }}
        >
          {Array.from({ length: TICKS }).map((_, i) => (
            <span
              key={i}
              className="w-[2px] rounded-full"
              style={{
                height: i % 5 === 0 ? 26 : 14,
                background: "rgba(60,60,67,0.22)",
              }}
            />
          ))}
          {reference != null && (
            <div
              className="absolute top-1 bottom-1 w-px"
              style={{
                left: `calc(${reference * 100}% )`,
                background: "var(--hc-ink-3)",
              }}
              title={referenceLabel}
            />
          )}
          {position != null && (
            <div
              className="hc-anim absolute -top-1 -bottom-1 w-[4px] rounded-full transition-[left] duration-500"
              style={{ left: `calc(${position * 100}% - 2px)`, background: markerColor }}
            />
          )}
        </div>
      </div>
      <div className="mt-1.5 flex justify-between text-[11px]" style={{ color: "var(--hc-ink-3)" }}>
        <span>{leftLabel}</span>
        {referenceLabel && reference != null && <span>{referenceLabel}</span>}
        <span>{rightLabel}</span>
      </div>
    </div>
  );
}

/** Map a pct-vs-median value (-50%..+60%) onto 0..1 for the track. */
export function pctToPosition(pct: number | null | undefined): number | null {
  if (pct == null) return null;
  const clamped = Math.max(-50, Math.min(60, pct));
  return (clamped + 50) / 110;
}

export { bandColor };
