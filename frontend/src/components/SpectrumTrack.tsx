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
 * Rent fairness track — compact score-style gradient with a slim marker.
 */
export function SpectrumTrack({
  position,
  markerColor,
  markerLabel,
  reference = null,
  referenceLabel,
  leftLabel = "Below median",
  rightLabel = "Above median",
}: SpectrumTrackProps) {
  return (
    <div className="w-full">
      <div className="relative">
        <div
          className="relative h-9 overflow-visible rounded-full"
          style={{
            backgroundImage:
              "linear-gradient(90.5315deg, rgb(228, 159, 159) 2.7086%, rgb(238, 192, 149) 48.566%, rgb(75, 205, 167) 98.504%)",
          }}
          aria-label={markerLabel ? `Rent difference ${markerLabel}` : "Rent difference"}
        >
          {reference != null && (
            <div
              className="absolute -top-1 -bottom-1 w-px"
              style={{
                left: `calc(${reference * 100}% )`,
                background: "rgba(28, 28, 30, 0.35)",
              }}
              title={referenceLabel}
            />
          )}
          {position != null && (
            <div
              className="hc-anim absolute -top-1.5 -bottom-1.5 w-[4px] rounded-full transition-[left] duration-500"
              style={{
                left: `calc(${position * 100}% - 2px)`,
                background: markerColor ?? "#FFFFFF",
                boxShadow: "0 1px 4px rgba(0,0,0,0.25)",
              }}
            />
          )}
        </div>
      </div>
      <div className="mt-2 flex justify-between text-[11px] font-medium" style={{ color: "var(--hc-ink-2)" }}>
        <span>{leftLabel}</span>
        {referenceLabel && reference != null && <span>{referenceLabel}</span>}
        <span>{rightLabel}</span>
      </div>
    </div>
  );
}
