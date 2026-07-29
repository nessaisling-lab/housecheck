interface StatusPillProps {
  text: string;
  color?: string;
  /** show ▲ / ▼ glyph before text */
  trend?: "up" | "down" | null;
}

/** Capsule status chip — semantic color at 15% bg, full-strength text (Whoop pills). */
export function StatusPill({ text, color = "#6C6C70", trend = null }: StatusPillProps) {
  return (
    <span
      className="inline-flex items-center gap-1 rounded-full px-2.5 py-1 text-[0.8125rem] font-medium whitespace-nowrap"
      style={{ backgroundColor: `${color}26`, color }}
    >
      {trend === "up" && <span aria-hidden>▲</span>}
      {trend === "down" && <span aria-hidden>▼</span>}
      {text}
    </span>
  );
}
