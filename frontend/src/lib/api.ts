import { asStabilization } from "@/types/building";
import type {
  ApiResult,
  BuildingCard,
  BuildingSummary,
  RentFairnessResult,
  SearchResult,
} from "@/types/building";
import {
  mockBuilding,
  mockRentFairness,
  mockSearch,
  mockSummaries,
  mockSummary,
} from "@/lib/mock";

/**
 * API client for the Rust/Axum backend.
 *
 * The frontend calls relative `/api/*` paths. In dev, Vite proxies `/api` to
 * the backend (see vite.config.ts → server.proxy, target = VITE_BACKEND_URL,
 * default http://localhost:8080). For production, set VITE_API_URL to the
 * backend origin, e.g. VITE_API_URL=https://api.example.com (no trailing /api).
 *
 * If the backend is unreachable, calls fall back to bundled demo data and the
 * result is flagged `source: "demo"` so the UI can label it honestly.
 */

const BASE =
  (import.meta.env.VITE_API_URL as string | undefined) ??
  "https://housecheck-nessa.fly.dev";
/**
 * Whether this build may substitute demo data when the API cannot be reached.
 *
 * **Off in production, deliberately.** Falling back to fixtures is right for a laptop with
 * no backend running and indefensible on a live site: someone checking a real address
 * before signing a lease could be shown a *fabricated building*, and the only signal was
 * the words "demo data" in small text under the score. A person who cannot reach the
 * record is far better served by being told so.
 *
 * Local `npm run dev` keeps the fixtures, so the UI is still workable offline. A preview
 * build can opt in with `VITE_ALLOW_DEMO_DATA=true` when demoing without a backend.
 */
export const DEMO_DATA_ALLOWED =
  import.meta.env.DEV || import.meta.env.VITE_ALLOW_DEMO_DATA === "true";

/** Default budget for the fast, DB-backed endpoints. */
const TIMEOUT_MS = 8000;
/**
 * The LLM-backed endpoints need their own budget, and it must exceed the server's.
 *
 * The backend allows the model 30s per attempt and retries once on a transient
 * failure, so a worst-case round trip is roughly 60s. An 8s client abort would kill
 * a slow but *successful* answer and silently swap in demo text — letting the client
 * decide an outcome the server was still working on. Measured live: legal answers
 * land in 12-27s, with an occasional retry pushing past 60s.
 */
const LLM_TIMEOUT_MS = 70000;

export class ApiError extends Error {
  status?: number;
  constructor(message: string, status?: number) {
    super(message);
    this.status = status;
  }
}

async function req<T>(path: string, init?: RequestInit, timeoutMs = TIMEOUT_MS): Promise<T> {
  const ctrl = new AbortController();
  const t = setTimeout(() => ctrl.abort(), timeoutMs);
  try {
    const res = await fetch(`${BASE}${path}`, {
      ...init,
      signal: ctrl.signal,
      headers: { "Content-Type": "application/json", ...(init?.headers ?? {}) },
    });
    if (!res.ok) throw new ApiError(`API ${res.status}`, res.status);
    return (await res.json()) as T;
  } finally {
    clearTimeout(t);
  }
}

function asArray<T>(v: T | T[]): T[] {
  return Array.isArray(v) ? v : v ? [v] : [];
}

/**
 * Normalize the backend's building payload into our BuildingCard shape.
 * Live shape (verified against https://housecheck-nessa.fly.dev):
 *   { building: { bbl, address, year_built, num_floors, units_res, has_elevator,
 *                 near_ada_subway_m, complaints_311, latitude, longitude, ... },
 *     score: { total, condition, legal, neighborhood, accessibility },
 *     open_violations: { a, b, c },
 *     access_likelihood: "Higher"|"Mixed"|"Lower",
 *     stabilization: { status, message } }
 */
/**
 * What the deployed artifact says about itself (`GET /meta`).
 *
 * Everything here used to be either hardcoded in the UI or not stated at all. `data_month`
 * in particular was a literal in HealthCard.tsx — a claim about the backend's data that the
 * backend could not confirm and that no re-ingest would update.
 */
export interface ArtifactMeta {
  data_month?: string | null;
  buildings?: string | null;
  violations?: string | null;
  violation_classes?: string | null;
  violation_classes_excluded?: string | null;
  sources?: string | null;
  snapshot_year?: string | null;
}

let metaCache: Promise<ArtifactMeta | null> | null = null;

/** Fetched once per page load; the artifact cannot change while the process is up. */
export function getMeta(): Promise<ArtifactMeta | null> {
  if (!metaCache) {
    metaCache = req<ArtifactMeta>("/meta").catch(() => null);
  }
  return metaCache;
}

/** A finite number, or null. Anything else the backend sends is "we don't know". */
function numOrNull(x: unknown): number | null {
  return typeof x === "number" && Number.isFinite(x) ? x : null;
}

// eslint-disable-next-line @typescript-eslint/no-explicit-any
function normalizeBuilding(raw: any): BuildingCard {
  const facts = raw.building ?? raw;
  const score = raw.score ?? {};
  const v = raw.open_violations ?? raw.violations ?? {};
  const stab = raw.stabilization;
  return {
    bbl: facts.bbl ?? raw.bbl,
    address: facts.address ?? raw.address ?? "Unknown address",
    neighborhood: facts.neighborhood ?? raw.neighborhood ?? null,
    year_built: facts.year_built ?? null,
    floors: facts.num_floors ?? facts.floors ?? null,
    units_res: facts.units_res ?? null,
    has_elevator: facts.has_elevator ?? null,
    near_ada_subway_m: facts.near_ada_subway_m ?? null,
    complaints_311: facts.complaints_311 ?? null,
    lat: facts.latitude ?? facts.lat ?? null,
    long: facts.longitude ?? facts.long ?? null,
    score: score.total ?? raw.score_total ?? (typeof raw.score === "number" ? raw.score : null),
    sub_scores: {
      condition: score.condition ?? null,
      legal: score.legal ?? null,
      neighborhood: score.neighborhood ?? null,
      accessibility: score.accessibility ?? null,
    },
    open_violations: {
      // `?? 0` here is what let a missing field render as "a clean hazardous-violation
      // record". The spelling fallbacks stay — the backend has used all three — but the
      // final fallback is null, so absence stays absent all the way to the card.
      a: numOrNull(v.a ?? v.class_a ?? v.A),
      b: numOrNull(v.b ?? v.class_b ?? v.B),
      c: numOrNull(v.c ?? v.class_c ?? v.C),
      open_since: v.open_since ?? v.since ?? null,
    },
    // Absent on an older API build, so default to an empty list rather than undefined:
    // the card checks `.length`, and a crash here would take out the whole page.
    open_violation_details: Array.isArray(raw.open_violation_details)
      ? // eslint-disable-next-line @typescript-eslint/no-explicit-any
        raw.open_violation_details.map((d: any) => ({
          class: String(d.class ?? ""),
          description: d.description ?? null,
          issued_on: d.issued_on ?? null,
          days_open: numOrNull(d.days_open),
        }))
      : [],
    open_violation_total: numOrNull(raw.open_violation_total),
    access_likelihood: raw.access_likelihood ?? null,
    stabilization:
      // Narrowed rather than cast: the union is closed now, so an unrecognised value from a
      // future backend becomes null (and renders as unverified) instead of quietly widening
      // the type and defeating the ten comparisons downstream.
      asStabilization(typeof stab === "string" ? stab : stab?.status),
    stabilization_message:
      typeof stab === "string" ? raw.stabilization_message ?? null : stab?.message ?? null,
    good_cause: facts.good_cause ?? raw.good_cause ?? null,
    rent: raw.rent ?? null,
  };
}

/**
 * @param scope `"city"` widens past the pilot to all five boroughs.
 *
 * Left off by default on purpose. A curated hit answers in milliseconds because it never
 * leaves the server; the citywide path calls NYC GeoSearch and takes seconds. The reader
 * opens the wider door only when the fast answer was not the building they meant.
 */
export async function searchAddress(
  query: string,
  scope?: "city"
): Promise<ApiResult<SearchResult[]>> {
  try {
    const raw = await req<SearchResult | SearchResult[]>(
      `/search?address=${encodeURIComponent(query)}${scope ? `&scope=${scope}` : ""}`
    );
    return { data: asArray(raw), source: "live" };
  } catch (e) {
    if (DEMO_DATA_ALLOWED) return { data: mockSearch(query), source: "demo" };
    throw e;
  }
}

export async function getBuilding(bbl: string): Promise<ApiResult<BuildingCard>> {
  try {
    const raw = await req<unknown>(`/building/${encodeURIComponent(bbl)}`);
    return { data: normalizeBuilding(raw), source: "live" };
  } catch (e) {
    if (DEMO_DATA_ALLOWED) {
      const demo = mockBuilding(bbl);
      if (demo) return { data: demo, source: "demo" };
    }
    // Two different failures that used to look identical. A 404 means the city has the
    // building and our pilot does not cover it; anything else means we could not reach the
    // record at all. Collapsing them into "Building not found" told people their address
    // was outside coverage when the truth was that the server was down — a wrong answer
    // dressed as a definite one.
    throw e instanceof ApiError && e.status === 404
      ? e
      : new ApiError("Could not reach the building record", 503);
  }
}

/**
 * Fetch the verifiable export document for a building.
 *
 * Deliberately does NOT fall back to demo data the way `getBuilding` does. Every other
 * endpoint degrades to a mock so the UI keeps working offline, and that is right for a
 * score someone is browsing. It would be indefensible here: this document is meant to be
 * handed to a court, and a fabricated exhibit that looks real is far worse than an error
 * message. A failure has to surface as a failure.
 *
 * Returns the raw text rather than a parsed object, because the bytes are what the hash
 * chain covers — re-serialising through JS could reorder keys and invalidate the signature
 * for whoever verifies it later.
 */
export async function exportRecord(
  bbl: string,
  format: "json" | "text" = "json",
): Promise<{ text: string; filename: string }> {
  const ctrl = new AbortController();
  const t = setTimeout(() => ctrl.abort(), TIMEOUT_MS);
  try {
    const qs = format === "text" ? "?format=text" : "";
    const res = await fetch(`${BASE}/building/${encodeURIComponent(bbl)}/export${qs}`, {
      signal: ctrl.signal,
    });
    if (!res.ok) throw new ApiError(`Export failed (${res.status})`, res.status);
    const text = await res.text();
    const today = new Date().toISOString().slice(0, 10);
    const ext = format === "text" ? "txt" : "json";
    return { text, filename: `housecheck-${bbl}-${today}.${ext}` };
  } finally {
    clearTimeout(t);
  }
}

export async function listBuildings(): Promise<ApiResult<BuildingSummary[]>> {
  try {
    // eslint-disable-next-line @typescript-eslint/no-explicit-any
    const raw = await req<any[]>(`/buildings`);
    // live shape: latitude/longitude (not lat/long)
    const data: BuildingSummary[] = raw.map((b) => ({
      bbl: b.bbl,
      address: b.address || "—",
      lat: b.latitude ?? b.lat ?? 0,
      long: b.longitude ?? b.long ?? 0,
      score: typeof b.score === "number" ? b.score : b.score?.total ?? null,
    }));
    return { data, source: "live" };
  } catch (e) {
    if (!DEMO_DATA_ALLOWED) throw e;
    return { data: mockSummaries(), source: "demo" };
  }
}

export async function checkRentFairness(
  bbl: string,
  rent: number
): Promise<ApiResult<RentFairnessResult>> {
  try {
    const raw = await req<RentFairnessResult>(`/rent-fairness`, {
      method: "POST",
      body: JSON.stringify({ bbl, monthly_rent: rent }),
    });
    return { data: raw, source: "live" };
  } catch (e) {
    if (!DEMO_DATA_ALLOWED) throw e;
    const demo = mockRentFairness(bbl, rent);
    if (demo) return { data: demo, source: "demo" };
    throw new ApiError("Rent fairness unavailable for this building");
  }
}

export async function compareBuildings(bbls: string[]): Promise<ApiResult<BuildingCard[]>> {
  try {
    const raw = await req<unknown>(`/compare?bbls=${bbls.map(encodeURIComponent).join(",")}`);
    // live shape: { buildings: [...] }
    // eslint-disable-next-line @typescript-eslint/no-explicit-any
    const wrapped = (raw as any)?.buildings;
    const list = asArray(wrapped ?? (raw as object | object[]));
    return { data: list.map(normalizeBuilding), source: "live" };
  } catch (e) {
    const demo = bbls
      .map(mockBuilding)
      .filter((b): b is BuildingCard => b !== null);
    if (!DEMO_DATA_ALLOWED) throw e;
    if (demo.length) return { data: demo, source: "demo" };
    throw new ApiError("Compare unavailable");
  }
}

export async function getSummary(bbl: string): Promise<ApiResult<string>> {
  try {
    const raw = await req<{ summary?: string; text?: string } | string>(
      `/summary`,
      { method: "POST", body: JSON.stringify({ bbl }) },
      LLM_TIMEOUT_MS
    );
    const text = typeof raw === "string" ? raw : raw.summary ?? raw.text ?? "";
    if (!text) throw new ApiError("Empty summary");
    return { data: text, source: "live" };
  } catch (e) {
    if (!DEMO_DATA_ALLOWED) throw e;
    const demo = mockSummary(bbl);
    if (demo) return { data: demo, source: "demo" };
    throw new ApiError("Summary unavailable");
  }
}

/** One turn of an agent conversation, in the shape POST /agent/chat expects. */
export interface ChatTurn {
  role: "user" | "assistant";
  content: string;
}

export interface ChatReply {
  answer: string;
  /** Sources that actually fed the answer — render these, never a hardcoded line. */
  citations: string[];
}

/**
 * Grounded multi-turn Q&A about one building (POST /agent/chat).
 *
 * Unlike the other calls this has no demo fallback: a fabricated "agent answer" is
 * exactly the thing this product must not produce. When the agent is unavailable the
 * caller falls back to the deterministic canned answers, which are grounded in the
 * card and honest about being canned.
 */
export async function sendChat(bbl: string, messages: ChatTurn[]): Promise<ChatReply> {
  const raw = await req<{ answer?: string; citations?: string[] }>(
    "/agent/chat",
    { method: "POST", body: JSON.stringify({ bbl, messages }) },
    LLM_TIMEOUT_MS
  );
  const answer = (raw.answer ?? "").trim();
  if (!answer) throw new ApiError("Empty agent answer");
  return { answer, citations: raw.citations ?? [] };
}

export interface RankedBuilding {
  bbl: string;
  address: string;
  weighted_score: number;
  card_score: number | null;
  sub_scores: {
    condition: number | null;
    legal: number | null;
    neighborhood: number | null;
    accessibility: number | null;
  };
}

/**
 * Rank buildings by the renter's stated priorities (GET /rank).
 *
 * The weighting deliberately lives on the server, shared with the agent's
 * rank_by_priorities tool. Computing it here would be a second scoring engine,
 * and a compare view that disagrees with the Health Card it links to is the
 * exact defect this replaces.
 */
export async function rankByPriorities(
  bbls: string[],
  priorities: string[]
): Promise<RankedBuilding[]> {
  const qs = new URLSearchParams({ bbls: bbls.join(",") });
  if (priorities.length) qs.set("priorities", priorities.join(","));
  const raw = await req<{ ranked?: RankedBuilding[] }>("/rank?" + qs.toString());
  return raw.ranked ?? [];
}
