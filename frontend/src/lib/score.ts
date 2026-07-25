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
