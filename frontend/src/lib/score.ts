// Score bands — locked design decision #4 (design-strategy §1.3)

export type Band =
  | "strong"
  | "solid"
  | "mixed"
  | "concern"
  | "critical"
  | "unverified";

export interface BandMeta {
  label: string;
  short: string;
  color: string;
}

export const BANDS: Record<Band, BandMeta> = {
  strong: { label: "Strong record", short: "Strong", color: "#248A3D" },
  solid: { label: "Generally solid", short: "Solid", color: "#7DA629" },
  mixed: { label: "Mixed signals", short: "Mixed", color: "#B7791F" },
  concern: { label: "Real concerns", short: "Caution", color: "#D04A1E" },
  critical: { label: "Serious red flags", short: "Red flags", color: "#C7272B" },
  unverified: { label: "Unverified", short: "Unverified", color: "#8E8E93" },
};

const SCALE_START = [228, 159, 159] as const;
const SCALE_MID = [238, 192, 149] as const;
const SCALE_END = [75, 205, 167] as const;

const SCORE_CIRCLE_COLORS: Record<Band, string> = {
  strong: "rgb(75, 205, 167)",
  solid: "rgb(75, 205, 167)",
  mixed: "rgb(238, 192, 149)",
  concern: "rgb(228, 159, 159)",
  critical: "rgb(228, 159, 159)",
  unverified: "#717182",
};

export function bandFor(score: number | null | undefined): Band {
  if (score === null || score === undefined || Number.isNaN(score))
    return "unverified";
  if (score >= 80) return "strong";
  if (score >= 60) return "solid";
  if (score >= 40) return "mixed";
  if (score >= 20) return "concern";
  return "critical";
}

export function bandMeta(score: number | null | undefined): BandMeta {
  return BANDS[bandFor(score)];
}

export function bandColor(score: number | null | undefined): string {
  return bandMeta(score).color;
}

export function scoreCircleColor(score: number | null | undefined): string {
  return SCORE_CIRCLE_COLORS[bandFor(score)];
}

export function gradientScaleColor(position: number): string {
  const t = Math.max(0, Math.min(1, position));
  const start = t < 0.5 ? SCALE_START : SCALE_MID;
  const end = t < 0.5 ? SCALE_MID : SCALE_END;
  const local = t < 0.5 ? t / 0.5 : (t - 0.5) / 0.5;
  const [r, g, b] = start.map((channel, i) =>
    Math.round(channel + (end[i] - channel) * local)
  );
  return `rgb(${r}, ${g}, ${b})`;
}

/** 8% opacity wash of band color → transparent, for the top of the Health Card */
export function scoreWash(score: number | null | undefined): string {
  const c = bandColor(score);
  return `linear-gradient(180deg, ${c}14 0%, ${c}08 55%, transparent 100%)`;
}

export function fmtDistance(m: number | null | undefined): string {
  if (m === null || m === undefined) return "Unverified";
  if (m >= 1000) return `${(m / 1000).toFixed(1)} km`;
  return `${Math.round(m)} m`;
}

export function fmtMoney(n: number | null | undefined): string {
  if (n === null || n === undefined) return "—";
  return `$${Math.round(n).toLocaleString("en-US")}`;
}

export function fmtPct(p: number | null | undefined): string {
  if (p === null || p === undefined) return "—";
  const v = Math.round(p);
  return `${Math.abs(v)}% ${v >= 0 ? "above" : "below"}`;
}

/** Map a pct-vs-median value (-50%..+60%) onto 0..1 for the rent spectrum track. */
export function pctToPosition(pct: number | null | undefined): number | null {
  if (pct == null) return null;
  const clamped = Math.max(-50, Math.min(60, pct));
  return (clamped + 50) / 110;
}
