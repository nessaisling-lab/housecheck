interface StatusPillProps {
  text: string;
  color?: string;
  /** show ▲ / ▼ glyph before text */
  trend?: "up" | "down" | null;
}

/** Capsule status chip — semantic color at 15% bg, full-strength text (Whoop pills). */
export function StatusPill({ text, color = "#8E8E93", trend = null }: StatusPillProps) {
  return (
    <span
      className="inline-flex items-center gap-1 rounded-full px-2.5 py-1 text-[13px] font-medium whitespace-nowrap"
      style={{ backgroundColor: `${color}26`, color }}
    >
      {trend === "up" && <span aria-hidden>▲</span>}
      {trend === "down" && <span aria-hidden>▼</span>}
      {text}
    </span>
  );
}
