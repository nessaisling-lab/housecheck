import { MiniRing } from "@/components/ScoreRing";
import { bandColor } from "@/lib/score";

interface SubScoreRowProps {
  name: string;
  status: string;
  score: number | null;
  onClick?: () => void;
}

/**
 * Sub-score row (Whoop 3-dials row → 4 equal rows):
 * mini ring + name + one-line status + score-md number + chevron.
 */
export function SubScoreRow({ name, status, score, onClick }: SubScoreRowProps) {
  const color = bandColor(score);
  return (
    <button
      onClick={onClick}
      className="flex min-h-[60px] w-full items-center gap-3 px-4 py-3 text-left transition-colors hover:bg-black/[0.02] active:bg-black/[0.04]"
    >
      <MiniRing score={score} size={34} stroke={4.5} />
      <span className="flex-1">
        <span className="block text-[17px] font-semibold" style={{ color: "var(--hc-ink)" }}>
          {name}
        </span>
        <span className="block text-[13px]" style={{ color: "var(--hc-ink-2)" }}>
          {status}
        </span>
      </span>
      <span className="text-[28px] font-semibold tabular-nums" style={{ color }}>
        {score ?? "—"}
      </span>
      <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="var(--hc-ink-3)" strokeWidth="2.5" strokeLinecap="round" aria-hidden>
        <path d="M9 6l6 6-6 6" />
      </svg>
    </button>
  );
}

interface SubScoreTileProps extends SubScoreRowProps {
  /** hairline dividers inside the 2×2 grid */
  borderR?: boolean;
  borderB?: boolean;
}

/**
 * Sub-score tile for the 2×2 grid layout:
 * Condition · Legal on the first row, Neighborhood · Accessibility beneath.
 */
export function SubScoreTile({ name, status, score, onClick, borderR, borderB }: SubScoreTileProps) {
  const color = bandColor(score);
  return (
    <button
      onClick={onClick}
      className="flex min-h-[92px] flex-col justify-center gap-1 px-4 py-3.5 text-left transition-colors hover:bg-black/[0.02] active:bg-black/[0.04]"
      style={{
        borderRight: borderR ? "0.5px solid rgba(60,60,67,0.1)" : undefined,
        borderBottom: borderB ? "0.5px solid rgba(60,60,67,0.1)" : undefined,
      }}
    >
      <span className="flex items-center gap-2">
        <MiniRing score={score} size={26} stroke={4} />
        <span className="text-[15px] font-semibold" style={{ color: "var(--hc-ink)" }}>
          {name}
        </span>
        <span className="ml-auto text-[24px] font-semibold leading-none tabular-nums" style={{ color }}>
          {score ?? "—"}
        </span>
      </span>
      <span className="block text-[12px] leading-snug" style={{ color: "var(--hc-ink-2)" }}>
        {status}
      </span>
    </button>
  );
}
