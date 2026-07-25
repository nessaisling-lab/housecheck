import type { BuildingRecord, Violation } from "@/data/buildings";

export type ScoreBreakdown = {
  score: number;
  openHazardous: number;
  openNonHazardous: number;
  closedHazardousLast2Y: number;
  totalOpen: number;
  label: "Strong" | "Fair" | "Caution" | "Hazardous";
  summary: string;
};

function yearsSince(dateStr: string): number {
  const then = new Date(dateStr).getTime();
  const now = new Date("2026-07-21").getTime();
  return (now - then) / (1000 * 60 * 60 * 24 * 365.25);
}

function isRecent(v: Violation, years = 2): boolean {
  return yearsSince(v.issued) <= years;
}

export function computeHealthScore(building: BuildingRecord): ScoreBreakdown {
  let score = 100;
  const open = building.violations.filter((v) => v.status === "Open");
  const openHazardous = open.filter((v) => v.class === "C").length;
  const openNonHazardous = open.filter((v) => v.class !== "C").length;
  const closedHazardousLast2Y = building.violations.filter(
    (v) => v.status === "Closed" && v.class === "C" && isRecent(v, 2),
  ).length;

  for (const v of building.violations) {
    const recent = isRecent(v, 2);
    if (v.status === "Open") {
      if (v.class === "C") score -= 28;
      else if (v.class === "B") score -= 12;
      else score -= 4;
    } else if (recent) {
      if (v.class === "C") score -= 10;
      else if (v.class === "B") score -= 4;
      else score -= 1;
    }
  }

  score = Math.max(0, Math.min(100, Math.round(score)));

  let label: ScoreBreakdown["label"];
  let summary: string;

  if (openHazardous > 0 || score < 40) {
    label = "Hazardous";
    summary =
      openHazardous > 0
        ? `${openHazardous} open Class C (immediately hazardous) violation${openHazardous === 1 ? "" : "s"} on record.`
        : "Recent serious violation history pulls this building into the danger zone.";
  } else if (score < 65) {
    label = "Caution";
    summary =
      "Open or recent violations suggest you should dig into HPD details before signing.";
  } else if (score < 85) {
    label = "Fair";
    summary =
      "Some history, but nothing currently screaming walk-away. Still verify open items.";
  } else {
    label = "Strong";
    summary =
      "Clean recent record relative to typical Brooklyn stock — a solid baseline for condition.";
  }

  return {
    score,
    openHazardous,
    openNonHazardous,
    closedHazardousLast2Y,
    totalOpen: open.length,
    label,
    summary,
  };
}

export function rentFairness(
  quoted: number,
  median: number,
): { pct: number; verdict: "below" | "near" | "above"; message: string } {
  const pct = Math.round(((quoted - median) / median) * 100);
  if (pct <= -8) {
    return {
      pct,
      verdict: "below",
      message: `${Math.abs(pct)}% below neighborhood median — a relative bargain on paper.`,
    };
  }
  if (pct >= 8) {
    return {
      pct,
      verdict: "above",
      message: `${pct}% above neighborhood median — use this when you negotiate.`,
    };
  }
  return {
    pct,
    verdict: "near",
    message: "Roughly in line with the neighborhood median for this area.",
  };
}
