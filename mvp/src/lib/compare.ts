import type { BuildingRecord } from "@/data/buildings";
import { computeHealthScore, rentFairness } from "@/lib/score";
import type { PriorityId } from "@/lib/priorities";
import { priorityById } from "@/lib/priorities";

export type AccessLikelihood = "Higher" | "Mixed" | "Lower";

export type CandidateInput = {
  building: BuildingRecord;
  quotedRent: number | null;
};

export type DimensionScore = {
  id: PriorityId;
  score: number;
  note: string;
};

export type RankedOption = {
  rank: number;
  building: BuildingRecord;
  quotedRent: number | null;
  healthScore: number;
  healthLabel: string;
  weightedScore: number;
  dimensions: DimensionScore[];
  pros: string[];
  cons: string[];
  whyRanked: string;
};

export type ComparisonResult = {
  ranked: RankedOption[];
  priorities: PriorityId[];
  maxRent: number | null;
};

export function accessLikelihood(building: BuildingRecord): AccessLikelihood {
  const elevator = building.hasElevator === true;
  const walkUpLikely =
    building.hasElevator === false &&
    (building.stories >= 3 || building.units >= 3);
  const designEra =
    building.yearBuilt >= 1992 && building.units >= 4
      ? building.yearBuilt >= 2015
        ? "strong"
        : "moderate"
      : "none";

  if (elevator && designEra !== "none") return "Higher";
  if (elevator) return "Higher";
  if (walkUpLikely && designEra === "none") return "Lower";
  if (walkUpLikely) return "Mixed";
  if (designEra !== "none") return "Mixed";
  return "Mixed";
}

function scoreCondition(building: BuildingRecord): DimensionScore {
  const health = computeHealthScore(building);
  return {
    id: "condition",
    score: health.score,
    note:
      health.openHazardous > 0
        ? `${health.openHazardous} open Class C hazard${health.openHazardous === 1 ? "" : "s"}`
        : health.totalOpen > 0
          ? `${health.totalOpen} open violation${health.totalOpen === 1 ? "" : "s"}`
          : "Clean open-violation record",
  };
}

function scoreRent(
  building: BuildingRecord,
  quotedRent: number | null,
  maxRent: number | null,
): DimensionScore {
  const rent = quotedRent ?? building.neighborhoodMedianRent;
  const fairness = rentFairness(rent, building.neighborhoodMedianRent);
  let score = 70;
  if (fairness.verdict === "below") score = 92;
  else if (fairness.verdict === "above") score = Math.max(20, 70 - fairness.pct);
  else score = 78;

  if (maxRent != null && rent > maxRent) {
    score = Math.min(score, 25);
  }

  const basis = quotedRent != null ? "your quoted rent" : "neighborhood median as proxy";
  return {
    id: "rent",
    score,
    note:
      maxRent != null && rent > maxRent
        ? `Above your $${maxRent.toLocaleString()} max (${basis})`
        : `${fairness.message.replace(/\.$/, "")} (${basis})`,
  };
}

function scoreProtections(building: BuildingRecord): DimensionScore {
  let score = 40;
  const bits: string[] = [];

  if (building.rentStabilized === true) {
    score += 40;
    bits.push("likely rent-stabilized");
  } else if (building.rentStabilized === "unknown") {
    score += 15;
    bits.push("stabilization unclear");
  } else {
    bits.push("not indicated as stabilized");
  }

  if (building.goodCauseLikely) {
    score += 20;
    bits.push("Good Cause likely");
  } else {
    bits.push("Good Cause may not apply");
  }

  return {
    id: "protections",
    score: Math.min(100, score),
    note: bits.join(" · "),
  };
}

function scoreAccess(building: BuildingRecord): DimensionScore {
  const likelihood = accessLikelihood(building);
  const score =
    likelihood === "Higher" ? 90 : likelihood === "Mixed" ? 55 : 25;
  const elevator =
    building.hasElevator === true
      ? "elevator on record"
      : building.hasElevator === false
        ? "no elevator indicated"
        : "elevator unknown";

  return {
    id: "access",
    score,
    note: `${likelihood} step-free likelihood · ${elevator} · ${building.stories} stories`,
  };
}

function dimensionScores(
  candidate: CandidateInput,
  maxRent: number | null,
): DimensionScore[] {
  return [
    scoreCondition(candidate.building),
    scoreRent(candidate.building, candidate.quotedRent, maxRent),
    scoreProtections(candidate.building),
    scoreAccess(candidate.building),
  ];
}

function weightedAverage(
  dimensions: DimensionScore[],
  priorities: PriorityId[],
): number {
  if (priorities.length === 0) return 0;
  // Higher rank (earlier in list) gets more weight: n, n-1, ...
  const n = priorities.length;
  let totalWeight = 0;
  let sum = 0;
  priorities.forEach((id, index) => {
    const weight = n - index;
    const dim = dimensions.find((d) => d.id === id);
    if (!dim) return;
    sum += dim.score * weight;
    totalWeight += weight;
  });
  return totalWeight === 0 ? 0 : Math.round(sum / totalWeight);
}

function buildProsCons(
  candidate: CandidateInput,
  dimensions: DimensionScore[],
  priorities: PriorityId[],
): { pros: string[]; cons: string[] } {
  const health = computeHealthScore(candidate.building);
  const b = candidate.building;
  const pros: string[] = [];
  const cons: string[] = [];

  if (health.openHazardous === 0 && health.score >= 85) {
    pros.push(`Strong condition score (${health.score}/100) with no open Class C hazards.`);
  } else if (health.openHazardous === 0 && health.score >= 65) {
    pros.push(`No open Class C hazards; condition score ${health.score}/100.`);
  }
  if (health.openHazardous > 0) {
    cons.push(
      `${health.openHazardous} open Class C (immediately hazardous) violation${health.openHazardous === 1 ? "" : "s"} on record.`,
    );
  } else if (health.totalOpen > 0) {
    cons.push(`${health.totalOpen} open non-Class-C violation${health.totalOpen === 1 ? "" : "s"} still on file.`);
  }

  if (b.rentStabilized === true) {
    pros.push("Likely rent-stabilized (best-available public signal — confirm unit).");
  } else if (b.rentStabilized === false) {
    cons.push("Not indicated as rent-stabilized in public records.");
  } else {
    cons.push("Rent-stabilization status unclear — ask for the apartment rent history.");
  }

  if (b.goodCauseLikely) {
    pros.push("Likely Good Cause eviction coverage.");
  } else {
    cons.push("Good Cause may not apply to this stock.");
  }

  const rentDim = dimensions.find((d) => d.id === "rent");
  if (rentDim) {
    if (rentDim.score >= 85) pros.push(rentDim.note + ".");
    else if (rentDim.score <= 40) cons.push(rentDim.note + ".");
  }

  const access = accessLikelihood(b);
  if (access === "Higher") {
    pros.push(
      "Higher step-free access likelihood from public records (verify in person).",
    );
  } else if (access === "Lower") {
    cons.push(
      "Lower step-free access likelihood — walk-up indicated; verify path to unit.",
    );
  }

  // Bias which bullets surface first toward user priorities
  const prioritySet = new Set(priorities);
  const sortByPriority = (lines: string[]) =>
    [...lines].sort((a, b) => {
      const scoreLine = (line: string) => {
        let s = 0;
        if (prioritySet.has("condition") && /Class C|condition|hazard/i.test(line))
          s += 3;
        if (prioritySet.has("protections") && /stabiliz|Good Cause/i.test(line))
          s += 3;
        if (prioritySet.has("rent") && /rent|median|max/i.test(line)) s += 3;
        if (prioritySet.has("access") && /step-free|walk-up|elevator/i.test(line))
          s += 3;
        return s;
      };
      return scoreLine(b) - scoreLine(a);
    });

  return {
    pros: sortByPriority(pros).slice(0, 3),
    cons: sortByPriority(cons).slice(0, 3),
  };
}

function whyRanked(
  option: Omit<RankedOption, "rank" | "whyRanked" | "pros" | "cons">,
  priorities: PriorityId[],
  rank: number,
): string {
  const top = priorities[0];
  const topDim = option.dimensions.find((d) => d.id === top);
  const topLabel = top ? priorityById(top).short.toLowerCase() : "fit";

  if (rank === 1) {
    return topDim
      ? `Best overall fit for your priorities — strongest on ${topLabel} (${topDim.note}).`
      : "Best overall fit for the priorities you ranked.";
  }
  return topDim
    ? `Ranks here mainly on ${topLabel}: ${topDim.note}.`
    : "Next-best fit given how you ranked needs.";
}

export function compareOptions(
  candidates: CandidateInput[],
  priorities: PriorityId[],
  maxRent: number | null,
): ComparisonResult {
  if (candidates.length < 2 || candidates.length > 3) {
    throw new Error("Compare 2 or 3 rental options.");
  }
  if (priorities.length === 0) {
    throw new Error("At least one priority is required.");
  }

  const scored = candidates.map((candidate) => {
    const dimensions = dimensionScores(candidate, maxRent);
    const health = computeHealthScore(candidate.building);
    const weightedScore = weightedAverage(dimensions, priorities);
    const { pros, cons } = buildProsCons(candidate, dimensions, priorities);

    return {
      building: candidate.building,
      quotedRent: candidate.quotedRent,
      healthScore: health.score,
      healthLabel: health.label,
      weightedScore,
      dimensions: dimensions.filter((d) => priorities.includes(d.id)),
      pros,
      cons,
    };
  });

  scored.sort((a, b) => b.weightedScore - a.weightedScore);

  const ranked: RankedOption[] = scored.map((option, index) => ({
    ...option,
    rank: index + 1,
    whyRanked: whyRanked(option, priorities, index + 1),
  }));

  return { ranked, priorities, maxRent };
}
