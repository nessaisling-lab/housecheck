// API contract — mirrors the Rust/Axum backend (see design-docs/housecheck-chat-handoff.md)

export interface SearchResult {
  bbl: string;
  label: string;
  in_curated_set: boolean;
}

/**
 * Open HPD violation counts by class.
 *
 * `null` means "the backend did not tell us", which is NOT the same as zero and must never
 * render as one. These were coerced to `0` on the way in, so a building whose payload was
 * missing the field displayed identically to a building with a genuinely clean record — and
 * the card states that case affirmatively ("a clean hazardous-violation record"), which is
 * the strongest claim the product makes about a building. Absence has to survive to the
 * render or the render invents a fact.
 */
export interface ViolationCounts {
  a: number | null;
  b: number | null;
  c: number | null;
  open_since?: number | null;
}

export interface SubScores {
  condition: number | null;
  legal: number | null;
  neighborhood: number | null;
  accessibility: number | null;
}

export type AccessLikelihood = "Higher" | "Mixed" | "Lower" | string;
export type Stabilization =
  | "likely"
  | "none_on_record"
  | "unverified"
  | string;

export interface HudFmr {
  area?: string;
  fiscal_year?: number;
  studio?: number | null;
  one_br?: number | null;
  two_br?: number | null;
  three_br?: number | null;
}

export interface RentContext {
  tract_median?: number | null;
  hud_fmr?: HudFmr | null;
  pct_vs_median?: number | null;
}

export interface BuildingCard {
  bbl: string;
  address: string;
  neighborhood?: string | null;
  year_built?: number | null;
  floors?: number | null;
  units_res?: number | null;
  has_elevator?: boolean | null;
  near_ada_subway_m?: number | null;
  complaints_311?: number | null;
  lat?: number | null;
  long?: number | null;
  score: number | null;
  sub_scores: SubScores;
  open_violations: ViolationCounts;
  access_likelihood?: AccessLikelihood | null;
  stabilization?: Stabilization | null;
  stabilization_message?: string | null;
  good_cause?: boolean | null;
  rent?: RentContext | null;
}

export interface BuildingSummary {
  bbl: string;
  address: string;
  lat: number;
  long: number;
  score: number | null;
}

export interface RentFairnessResult {
  user_rent: number;
  tract_median: number | null;
  pct_vs_median: number | null;
  verdict: string;
  hud_fmr: HudFmr | null;
}

export type DataSource = "live" | "demo";

export interface ApiResult<T> {
  data: T;
  source: DataSource;
}
