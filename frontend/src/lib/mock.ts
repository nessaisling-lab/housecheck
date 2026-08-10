import type { BuildingCard, BuildingSummary, RentFairnessResult } from "@/types/building";

// Demo data — used only when the backend is unreachable, and labeled "demo data" in the UI.
// Mirrors wireframe 03 (460 Macon St, score 57) and the Saved/Compare wireframes.

export const MOCK_BUILDINGS: BuildingCard[] = [
  {
    bbl: "3018420001",
    address: "460 Macon Street",
    neighborhood: "Bedford-Stuyvesant",
    year_built: 1928,
    floors: 4,
    units_res: 8,
    has_elevator: false,
    near_ada_subway_m: 640,
    complaints_311: 3,
    lat: 40.6812,
    long: -73.9345,
    score: 57,
    sub_scores: { condition: 56, legal: 100, neighborhood: 40, accessibility: 30 },
    open_violations: { a: 0, b: 1, c: 1, open_since: 2025 },
    open_violation_details: [
      {
        class: "C",
        description:
          "§ 27-2005 ADM CODE PROPERLY REPAIR THE BROKEN OR DEFECTIVE MECHANICAL VENTILATION SYSTEM IN THE BATHROOM LOCATED AT APT 3R, 3rd STORY",
        issued_on: "2025-11-02",
        days_open: 280,
      },
      {
        class: "B",
        description: "§ 27-2005 HMC: PROPERLY REPAIR OR REPLACE THE BROKEN OR DEFECTIVE DOOR KNOB IN THE BATHROOM",
        issued_on: "2025-06-18",
        days_open: 417,
      },
    ],
    open_violation_total: 2,
    access_likelihood: "Lower",
    stabilization: "likely",
    stabilization_message: "Confirm stabilization with NYS DHCR before signing.",
    good_cause: true,
    rent: { tract_median: 2400, hud_fmr: { area: "New York, NY HUD Metro FMR Area", fiscal_year: 2026, studio: 2529, one_br: 2655, two_br: 2910, three_br: 3644 }, pct_vs_median: 21 },
  },
  {
    bbl: "3017990028",
    address: "548 Gates Ave",
    neighborhood: "Bedford-Stuyvesant",
    year_built: 1931,
    floors: 6,
    units_res: 24,
    has_elevator: true,
    near_ada_subway_m: 300,
    complaints_311: 1,
    lat: 40.6854,
    long: -73.9412,
    score: 68,
    sub_scores: { condition: 72, legal: 88, neighborhood: 52, accessibility: 44 },
    open_violations: { a: 1, b: 0, c: 0, open_since: 2024 },
    open_violation_details: [
      // No issue date on the wire -- the card must say "age unknown", never "0 days".
      { class: "A", description: "(A) § HMC: FILE ANNUAL BEDBUG REPORT IN ACCORDANCE WITH HPD RULE", issued_on: null, days_open: null },
    ],
    open_violation_total: 1,
    access_likelihood: "Mixed",
    stabilization: "likely",
    stabilization_message: "Confirm stabilization with NYS DHCR before signing.",
    good_cause: true,
    rent: { tract_median: 2400, hud_fmr: { area: "New York, NY HUD Metro FMR Area", fiscal_year: 2026, studio: 2529, one_br: 2655, two_br: 2910, three_br: 3644 }, pct_vs_median: 4 },
  },
  {
    bbl: "3018380040",
    address: "1230 Bedford Ave",
    neighborhood: "Bedford-Stuyvesant",
    year_built: 1901,
    floors: 3,
    units_res: 6,
    has_elevator: false,
    near_ada_subway_m: 1100,
    complaints_311: 11,
    lat: 40.6798,
    long: -73.9496,
    score: 41,
    sub_scores: { condition: 30, legal: 60, neighborhood: 48, accessibility: 35 },
    open_violations: { a: 2, b: 3, c: 4, open_since: 2023 },
    open_violation_details: [
      { class: "C", description: "§ 27-2033 ADM CODE PROVIDE ADEQUATE HEAT AT APT 2F", issued_on: "2023-12-04", days_open: 979 },
      // A record with no text at all: the card falls back rather than rendering blank.
      { class: "B", description: null, issued_on: "2024-02-11", days_open: 910 },
    ],
    // Fewer details than the total, so the "showing N of M" line renders.
    open_violation_total: 9,
    access_likelihood: "Lower",
    stabilization: "unverified",
    stabilization_message:
      "We couldn't verify stabilization from public records — ask the landlord and check with NYS DHCR.",
    good_cause: null,
    rent: { tract_median: 2350, hud_fmr: { area: "New York, NY HUD Metro FMR Area", fiscal_year: 2026, studio: 2529, one_br: 2655, two_br: 2910, three_br: 3644 }, pct_vs_median: -2 },
  },
  {
    bbl: "3018570015",
    address: "921 Fulton Street",
    neighborhood: "Bedford-Stuyvesant",
    year_built: 2009,
    floors: 8,
    units_res: 42,
    has_elevator: true,
    near_ada_subway_m: 180,
    complaints_311: 0,
    lat: 40.6871,
    long: -73.9468,
    score: 82,
    sub_scores: { condition: 88, legal: 78, neighborhood: 66, accessibility: 84 },
    open_violations: { a: 0, b: 0, c: 0, open_since: null },
    open_violation_details: [],
    open_violation_total: 0,
    access_likelihood: "Higher",
    stabilization: "none_on_record",
    stabilization_message: "No stabilization on record — Good Cause protections may still apply.",
    good_cause: true,
    rent: { tract_median: 3100, hud_fmr: { area: "New York, NY HUD Metro FMR Area", fiscal_year: 2026, studio: 2529, one_br: 2655, two_br: 2910, three_br: 3644 }, pct_vs_median: 9 },
  },
];

export function mockSummaries(): BuildingSummary[] {
  return MOCK_BUILDINGS.map((b) => ({
    bbl: b.bbl,
    address: b.address,
    lat: b.lat ?? 0,
    long: b.long ?? 0,
    score: b.score,
  }));
}

export function mockSearch(query: string) {
  const q = query.trim().toLowerCase();
  const hits = MOCK_BUILDINGS.filter((b) => b.address.toLowerCase().includes(q)).map(
    (b) => ({ bbl: b.bbl, label: b.address, in_curated_set: true })
  );
  if (hits.length > 0) return hits;
  // Looks like a NYC address but outside our pilot coverage
  if (/\d/.test(q)) {
    return [
      {
        bbl: "",
        label: query.trim().replace(/\b\w/g, (c) => c.toUpperCase()),
        in_curated_set: false,
      },
    ];
  }
  return [];
}

export function mockBuilding(bbl: string): BuildingCard | null {
  return MOCK_BUILDINGS.find((b) => b.bbl === bbl) ?? null;
}

export function mockRentFairness(bbl: string, rent: number): RentFairnessResult | null {
  const b = mockBuilding(bbl);
  if (!b || !b.rent?.tract_median) return null;
  const median = b.rent.tract_median;
  const pct = Math.round(((rent - median) / median) * 100);
  const verdict =
    pct <= -10 ? "Below typical" : pct <= 10 ? "Near typical" : pct <= 25 ? "Above typical" : "Well above typical";
  return {
    user_rent: rent,
    tract_median: median,
    pct_vs_median: pct,
    verdict,
    hud_fmr: b.rent.hud_fmr ?? null,
  };
}

export function mockSummary(bbl: string): string | null {
  const b = mockBuilding(bbl);
  if (!b) return null;
  const stab =
    b.stabilization === "likely"
      ? "rent stabilized"
      : b.stabilization === "unverified"
        ? "stabilization unverified"
        : "no stabilization on record";
  return `${b.address} is a ${b.year_built ?? "older"} ${b.has_elevator ? "elevator building" : "walk-up"} scoring ${b.score} — a signal built from public records. Legal record: ${stab}${b.good_cause ? " with Good Cause coverage" : ""}. ${b.open_violations.c === null ? "No violation data available." : b.open_violations.c > 0 ? `${b.open_violations.c} hazardous (Class C) violation${b.open_violations.c > 1 ? "s" : ""} open — ask about repairs before signing.` : "No hazardous violations open."} Verify everything in person; this is public data, not a legal ruling.`;
}
