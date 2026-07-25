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
const TIMEOUT_MS = 8000;

export class ApiError extends Error {
  status?: number;
  constructor(message: string, status?: number) {
    super(message);
    this.status = status;
  }
}

async function req<T>(path: string, init?: RequestInit): Promise<T> {
  const ctrl = new AbortController();
  const t = setTimeout(() => ctrl.abort(), TIMEOUT_MS);
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
      a: v.a ?? v.class_a ?? v.A ?? 0,
      b: v.b ?? v.class_b ?? v.B ?? 0,
      c: v.c ?? v.class_c ?? v.C ?? 0,
      open_since: v.open_since ?? v.since ?? null,
    },
    access_likelihood: raw.access_likelihood ?? null,
    stabilization:
      typeof stab === "string" ? stab : stab?.status ?? null,
    stabilization_message:
      typeof stab === "string" ? raw.stabilization_message ?? null : stab?.message ?? null,
    good_cause: facts.good_cause ?? raw.good_cause ?? null,
    rent: raw.rent ?? null,
  };
}

export async function searchAddress(query: string): Promise<ApiResult<SearchResult[]>> {
  try {
    const raw = await req<SearchResult | SearchResult[]>(
      `/search?address=${encodeURIComponent(query)}`
    );
    return { data: asArray(raw), source: "live" };
  } catch {
    return { data: mockSearch(query), source: "demo" };
  }
}

export async function getBuilding(bbl: string): Promise<ApiResult<BuildingCard>> {
  try {
    const raw = await req<unknown>(`/building/${encodeURIComponent(bbl)}`);
    return { data: normalizeBuilding(raw), source: "live" };
  } catch (e) {
    const demo = mockBuilding(bbl);
    if (demo) return { data: demo, source: "demo" };
    throw e instanceof ApiError && e.status === 404
      ? e
      : new ApiError("Building not found", 404);
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
  } catch {
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
  } catch {
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
  } catch {
    const demo = bbls
      .map(mockBuilding)
      .filter((b): b is BuildingCard => b !== null);
    if (demo.length) return { data: demo, source: "demo" };
    throw new ApiError("Compare unavailable");
  }
}

export async function getSummary(bbl: string): Promise<ApiResult<string>> {
  try {
    const raw = await req<{ summary?: string; text?: string } | string>(`/summary`, {
      method: "POST",
      body: JSON.stringify({ bbl }),
    });
    const text = typeof raw === "string" ? raw : raw.summary ?? raw.text ?? "";
    if (!text) throw new ApiError("Empty summary");
    return { data: text, source: "live" };
  } catch {
    const demo = mockSummary(bbl);
    if (demo) return { data: demo, source: "demo" };
    throw new ApiError("Summary unavailable");
  }
}
