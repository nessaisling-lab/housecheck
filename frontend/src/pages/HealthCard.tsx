import { useCallback, useEffect, useMemo, useState } from "react";
import { useNavigate, useParams } from "react-router";
import { ScoreRing } from "@/components/ScoreRing";
import { SectionCard } from "@/components/SectionCard";
import { Sheet } from "@/components/Sheet";
import { SpectrumTrack } from "@/components/SpectrumTrack";
import { StatusPill } from "@/components/StatusPill";
import { SubScoreTile } from "@/components/SubScoreRow";
import { checkRentFairness, getBuilding } from "@/lib/api";
import { useAgent } from "@/lib/agent-context";
import { bandMeta, fmtDistance, fmtMoney, fmtPct, pctToPosition, scoreWash } from "@/lib/score";
import { store, useIsSaved, useOnboarding, useTray, type Priority } from "@/lib/store";
import type { BuildingCard as Building, DataSource, RentFairnessResult } from "@/types/building";

const DATA_MONTH = "Jul 2026";

type SectionId = "rent" | "condition" | "legal" | "access";

/** Onboarding priority → Health Card section ("neighborhood" lives in Rent fairness) */
const SECTION_OF: Record<Priority, SectionId> = {
  rent: "rent",
  neighborhood: "rent",
  condition: "condition",
  legal: "legal",
  access: "access",
};

const ICONS = {
  rent: (
    <svg width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.8" strokeLinecap="round">
      <rect x="3" y="6" width="18" height="13" rx="2.5" />
      <circle cx="12" cy="12.5" r="3" />
      <path d="M6.5 9.5h.01M17.5 15.5h.01" />
    </svg>
  ),
  condition: (
    <svg width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.8" strokeLinecap="round" strokeLinejoin="round">
      <path d="M14.5 6.5a4 4 0 015.2-3.8l-2.9 2.9.9 2.8 2.8.9 2.9-2.9a4 4 0 01-5.4 5.6L8.5 21.5a2.1 2.1 0 01-3-3L15 9a4 4 0 01-.5-2.5z" />
    </svg>
  ),
  legal: (
    <svg width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.8" strokeLinejoin="round">
      <path d="M12 3l8 3v6c0 5-3.5 8-8 9-4.5-1-8-4-8-9V6l8-3z" />
    </svg>
  ),
  access: (
    <svg width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.8" strokeLinecap="round">
      <circle cx="12" cy="5" r="2" />
      <path d="M5 10h14M12 10v4l-3 7M12 14l3 7" />
    </svg>
  ),
};

function Skeleton({ address }: { address?: string }) {
  return (
    <div className="mx-auto min-h-dvh w-full max-w-md px-5 pb-32 pt-14" aria-label="Loading building health card">
      <div className="h-7 w-48 rounded-lg" style={{ background: "var(--hc-sunken)", animation: "hc-pulse-soft 1.2s infinite" }}>
        <span className="sr-only">{address ?? "Loading"}</span>
      </div>
      <div className="mt-8 flex justify-center">
        <div className="h-[180px] w-[180px] rounded-full" style={{ border: "12px solid var(--hc-sunken)", animation: "hc-pulse-soft 1.2s infinite" }} />
      </div>
      <div className="mt-8 space-y-3">
        {[88, 60, 60, 60, 60].map((h, i) => (
          <div key={i} className="rounded-2xl" style={{ height: h, background: i === 0 ? "var(--hc-sunken)" : "#fff", boxShadow: "0 8px 32px rgba(0,0,0,0.06)", animation: "hc-pulse-soft 1.2s infinite" }} />
        ))}
      </div>
    </div>
  );
}

function subStatus(b: Building, key: "condition" | "legal" | "neighborhood" | "accessibility"): string {
  switch (key) {
    case "condition":
      return b.open_violations.c > 0
        ? `${b.open_violations.c} hazardous violation${b.open_violations.c > 1 ? "s" : ""} open`
        : "No hazardous violations";
    case "legal":
      return b.stabilization === "likely"
        ? `Stabilized${b.good_cause ? " · Good Cause" : ""}`
        : b.stabilization === "unverified"
          ? "Stabilization unverified"
          : "No stabilization on record";
    case "neighborhood":
      return b.rent?.pct_vs_median != null ? `Rent ${fmtPct(b.rent.pct_vs_median)} median` : "Rent data unverified";
    case "accessibility":
      return `${b.has_elevator ? "Elevator" : "Walk-up"} · ADA subway ${fmtDistance(b.near_ada_subway_m)}`;
  }
}

export default function HealthCard() {
  const { bbl = "" } = useParams();
  const navigate = useNavigate();
  const { setBuilding: setAgentBuilding, openAgent } = useAgent();
  const tray = useTray();

  const [building, setBuilding] = useState<Building | null>(null);
  const [source, setSource] = useState<DataSource>("live");
  const [state, setState] = useState<"loading" | "ready" | "notfound" | "error">("loading");
  const [detail, setDetail] = useState<SectionId | null>(null);
  const [toast, setToast] = useState<string | null>(null);

  const [rentInput, setRentInput] = useState("");
  const [rentResult, setRentResult] = useState<RentFairnessResult | null>(null);
  const [rentBusy, setRentBusy] = useState(false);

  const saved = useIsSaved(building?.bbl);
  const { priorities } = useOnboarding();

  // Priority sections float to the top — reorder only, nothing is ever hidden.
  const prioritySections = useMemo(
    () => new Set<SectionId>(priorities.map((p) => SECTION_OF[p])),
    [priorities]
  );
  const sectionOrder = (id: SectionId) => (prioritySections.has(id) ? 0 : 1);
  const priorityBadge = (
    <span
      className="ml-2 inline-block rounded-full px-2 py-0.5 align-middle text-[10px] font-semibold uppercase tracking-wider"
      style={{ background: "var(--hc-sunken)", color: "var(--hc-ink-2)" }}
    >
      Your priority
    </span>
  );

  const load = useCallback(() => {
    setState("loading");
    const started = Date.now();
    getBuilding(bbl)
      .then(({ data, source }) => {
        const wait = Math.max(0, 400 - (Date.now() - started)); // min 400ms, no flash (flow 1b)
        setTimeout(() => {
          setBuilding(data);
          setSource(source);
          setState("ready");
          store.addRecent({ bbl: data.bbl, address: data.address, score: data.score, neighborhood: data.neighborhood });
        }, wait);
      })
      .catch((e) => setState(e?.status === 404 ? "notfound" : "error"));
  }, [bbl]);

  useEffect(load, [load]);

  useEffect(() => {
    setAgentBuilding(building);
    return () => setAgentBuilding(null);
  }, [building, setAgentBuilding]);

  useEffect(() => {
    if (!toast) return;
    const t = setTimeout(() => setToast(null), 2600);
    return () => clearTimeout(t);
  }, [toast]);

  const jump = useCallback((id: string) => {
    document.getElementById(id)?.scrollIntoView({ behavior: "smooth", block: "start" });
  }, []);

  const runRentCheck = async () => {
    const rent = parseInt(rentInput.replace(/[^0-9]/g, ""), 10);
    if (!rent || !building) return;
    setRentBusy(true);
    try {
      const { data } = await checkRentFairness(building.bbl, rent);
      setRentResult(data);
    } catch {
      setToast("Rent check unavailable for this building");
    } finally {
      setRentBusy(false);
    }
  };

  const band = bandMeta(building?.score);
  const wash = useMemo(() => scoreWash(building?.score), [building?.score]);

  if (state === "loading") return <Skeleton />;

  if (state === "notfound")
    return (
      <div className="mx-auto flex min-h-dvh w-full max-w-md flex-col items-center px-6 pb-32 pt-28 text-center">
        <div className="flex h-24 w-24 items-center justify-center rounded-full" style={{ background: "var(--hc-sunken)" }}>
          <svg width="36" height="36" viewBox="0 0 24 24" fill="none" stroke="var(--hc-ink-3)" strokeWidth="1.6" strokeLinejoin="round">
            <path d="M3 11l9-7 9 7" />
            <path d="M6 9.5V20h12V9.5" />
          </svg>
        </div>
        <h1 className="mt-6 text-[24px] font-semibold" style={{ color: "var(--hc-canvas-ink)" }}>
          We don't have this building
        </h1>
        <p className="mt-2 text-[15px] leading-snug" style={{ color: "var(--hc-canvas-ink-2)" }}>
          Our pilot covers ~250 buildings in Bed-Stuy. Double-check the address, or explore the
          covered set.
        </p>
        <button onClick={() => navigate("/")} className="mt-6 rounded-full px-6 py-3 text-[15px] font-semibold text-white" style={{ background: "var(--hc-ink)" }}>
          Back to search
        </button>
      </div>
    );

  if (state === "error" || !building)
    return (
      <div className="mx-auto flex min-h-dvh w-full max-w-md flex-col items-center px-6 pb-32 pt-28 text-center">
        <h1 className="text-[24px] font-semibold" style={{ color: "var(--hc-canvas-ink)" }}>
          Something didn't load
        </h1>
        <p className="mt-2 text-[15px] leading-snug" style={{ color: "var(--hc-canvas-ink-2)" }}>
          We couldn't reach the data service. Nothing about the building changed — this is on us.
        </p>
        <button onClick={load} className="mt-6 rounded-full px-6 py-3 text-[15px] font-semibold text-white" style={{ background: "var(--hc-ink)" }}>
          Try again
        </button>
      </div>
    );

  const v = building.open_violations;
  const stabLabel =
    building.stabilization === "likely" ? "Yes" : building.stabilization === "unverified" ? "Unverified" : "No";
  const stabColor =
    building.stabilization === "likely" ? "var(--hc-strong)" : building.stabilization === "unverified" ? "var(--hc-unverified)" : "var(--hc-ink)";
  const rentPct = rentResult?.pct_vs_median ?? building.rent?.pct_vs_median ?? null;
  const rentColor = rentPct == null ? "var(--hc-unverified)" : rentPct > 10 ? "var(--hc-concern)" : rentPct >= -10 ? "var(--hc-strong)" : "var(--hc-solid)";
  const fmr = rentResult?.hud_fmr ?? building.rent?.hud_fmr ?? null;
  const fmrText =
    fmr == null
      ? "—"
      : `2BR ${fmtMoney(fmr.two_br ?? null)} · 1BR ${fmtMoney(fmr.one_br ?? null)}`;

  const detailContent: Record<SectionId, { title: string; body: React.ReactNode }> = {
    rent: {
      title: "Rent fairness",
      body: (
        <>
          <p className="text-[15px] leading-relaxed" style={{ color: "var(--hc-ink-2)" }}>
            We compare rents against the Census tract median (ACS table B25064) and the HUD Fair
            Market Rent for the area. This is a benchmark, not a judgment of any specific lease.
          </p>
          <dl className="mt-4 space-y-2.5">
            {[
              ["Tract median", fmtMoney(building.rent?.tract_median)],
              ["HUD fair market rent", fmrText],
              ["Asking pattern", rentPct != null ? fmtPct(rentPct) + " median" : "Unverified"],
            ].map(([l, val]) => (
              <div key={l as string} className="flex justify-between">
                <dt className="hc-row-label">{l}</dt>
                <dd className="text-[15px] font-medium tabular-nums" style={{ color: "var(--hc-ink)" }}>{val}</dd>
              </div>
            ))}
          </dl>
        </>
      ),
    },
    condition: {
      title: "Building condition",
      body: (
        <>
          <p className="text-[15px] leading-relaxed" style={{ color: "var(--hc-ink-2)" }}>
            NYC HPD classes: A = non-hazardous, B = hazardous, C = immediately hazardous. Open Class
            C violations are the strongest public signal of neglect — ask the landlord about repair
            timelines and check again before signing.
          </p>
          <dl className="mt-4 space-y-2.5">
            {[
              ["Class C — immediately hazardous", v.c],
              ["Class B — hazardous", v.b],
              ["Class A — non-hazardous", v.a],
              ["Open since", v.open_since ?? "—"],
              ["311 complaints (12 mo)", building.complaints_311 ?? "—"],
            ].map(([l, val]) => (
              <div key={l as string} className="flex justify-between">
                <dt className="hc-row-label">{l}</dt>
                <dd className="text-[15px] font-medium tabular-nums" style={{ color: "var(--hc-ink)" }}>{val}</dd>
              </div>
            ))}
          </dl>
        </>
      ),
    },
    legal: {
      title: "Legal protections",
      body: (
        <>
          <p className="text-[15px] leading-relaxed" style={{ color: "var(--hc-ink-2)" }}>
            {building.stabilization_message ??
              "Stabilization status comes from public HPD/DHCR records."}{" "}
            Stabilization caps rent increases and guarantees lease renewal; Good Cause (2024) adds
            eviction protection for many market-rate units. Always confirm with NYS DHCR — a free
            rent-history request takes minutes.
          </p>
          <dl className="mt-4 space-y-2.5">
            {[
              ["Rent stabilized", stabLabel],
              ["Good Cause covered", building.good_cause == null ? "Unverified" : building.good_cause ? "Yes" : "No"],
            ].map(([l, val]) => (
              <div key={l as string} className="flex justify-between">
                <dt className="hc-row-label">{l}</dt>
                <dd className="text-[15px] font-medium" style={{ color: "var(--hc-ink)" }}>{val}</dd>
              </div>
            ))}
          </dl>
        </>
      ),
    },
    access: {
      title: "Accessibility",
      body: (
        <>
          <p className="text-[15px] leading-relaxed" style={{ color: "var(--hc-ink-2)" }}>
            Access likelihood is inferred from public records — elevator filings (DOB) and distance
            to the nearest ADA-accessible subway (MTA). It is a starting point, never a guarantee:
            verify step-free entry, door widths, and elevator reliability in person.
          </p>
          <dl className="mt-4 space-y-2.5">
            {[
              ["Access likelihood", building.access_likelihood ?? "Unverified"],
              ["Elevator", building.has_elevator == null ? "Unverified" : building.has_elevator ? "Yes" : "None on record"],
              ["ADA subway", fmtDistance(building.near_ada_subway_m)],
              ["Floors", building.floors ?? "—"],
              ["Built", building.year_built ?? "—"],
            ].map(([l, val]) => (
              <div key={l as string} className="flex justify-between">
                <dt className="hc-row-label">{l}</dt>
                <dd className="text-[15px] font-medium" style={{ color: "var(--hc-ink)" }}>{val}</dd>
              </div>
            ))}
          </dl>
        </>
      ),
    },
  };

  return (
    <div className="min-h-dvh" style={{ background: wash }}>
      <div className="mx-auto w-full max-w-md px-5 pb-40 pt-12">
        {/* Header */}
        <header className="flex items-start justify-between gap-3">
          <div>
            <h1 className="text-[24px] font-semibold leading-tight tracking-tight" style={{ color: "var(--hc-canvas-ink)" }}>
              {building.address}
            </h1>
            <p className="mt-0.5 text-[13px]" style={{ color: "var(--hc-canvas-ink-3)" }}>
              {building.neighborhood ?? "Bedford-Stuyvesant"} · BBL {building.bbl}
              {source === "demo" && " · demo data"}
            </p>
          </div>
          <button
            onClick={() =>
              store.toggleSave({ bbl: building.bbl, address: building.address, score: building.score, neighborhood: building.neighborhood })
            }
            aria-label={saved ? "Remove from saved" : "Save building"}
            aria-pressed={saved}
            className="glass-nav mt-1 flex h-10 w-10 shrink-0 items-center justify-center rounded-full"
            style={{ color: saved ? "var(--hc-ink)" : "var(--hc-ink-3)" }}
          >
            <svg width="18" height="18" viewBox="0 0 24 24" fill={saved ? "currentColor" : "none"} stroke="currentColor" strokeWidth="1.8" strokeLinejoin="round">
              <path d="M6 3h12v18l-6-4.5L6 21V3z" />
            </svg>
          </button>
        </header>

        {/* Tier 1 — hero */}
        <div className="flex flex-col items-center pb-6 pt-8">
          <ScoreRing score={building.score} size={184} stroke={12} hero animate />
          <p
            className="hc-eyebrow mt-4"
            style={{ color: "var(--hc-canvas-ink)", letterSpacing: "0.18em" }}
          >
            {band.label}
          </p>
          <p className="mt-2 max-w-[280px] text-center text-[13px]" style={{ color: "var(--hc-canvas-ink-3)" }}>
            Built from public NYC data — a signal, not a legal ruling.
          </p>
        </div>

        {/* AI summary card */}
        <button onClick={openAgent} className="hc-card mb-4 flex w-full items-start gap-3 p-4 text-left">
          <span className="flex h-9 w-9 shrink-0 items-center justify-center rounded-xl" style={{ background: "var(--hc-sunken)" }}>
            <svg width="16" height="16" viewBox="0 0 24 24" fill="var(--hc-ink)" aria-hidden>
              <path d="M12 2l2.1 7.9L22 12l-7.9 2.1L12 22l-2.1-7.9L2 12l7.9-2.1L12 2z" />
            </svg>
          </span>
          <span>
            <span className="block text-[15px] leading-snug" style={{ color: "var(--hc-ink-2)" }}>
              {building.year_built ? `${building.year_built} ` : ""}
              {building.has_elevator ? "elevator building" : "walk-up"}
              {v.c > 0 ? ` with ${v.c} hazardous violation${v.c > 1 ? "s" : ""} open` : " with a clean hazardous-violation record"}
              {building.stabilization === "likely" ? "; likely rent-stabilized" : building.stabilization === "unverified" ? "; stabilization unverified" : ""}
              {building.good_cause ? ", with Good Cause coverage" : ""}.
            </span>
            <span className="mt-1.5 block text-[14px] font-semibold" style={{ color: "var(--hc-ink)" }}>
              Ask the agent ➜
            </span>
          </span>
        </button>

        {/* Tier 2 — sub-scores: 2×2 grid (Condition · Legal / Neighborhood · Accessibility) */}
        <div className="hc-card grid grid-cols-2 overflow-hidden">
          <SubScoreTile name="Condition" status={subStatus(building, "condition")} score={building.sub_scores.condition} onClick={() => jump("section-condition")} borderR borderB />
          <SubScoreTile name="Legal" status={subStatus(building, "legal")} score={building.sub_scores.legal} onClick={() => jump("section-legal")} borderB />
          <SubScoreTile name="Neighborhood" status={subStatus(building, "neighborhood")} score={building.sub_scores.neighborhood} onClick={() => jump("section-rent")} borderR />
          <SubScoreTile name="Accessibility" status={subStatus(building, "accessibility")} score={building.sub_scores.accessibility} onClick={() => jump("section-access")} />
        </div>

        <h2 className="mt-8 text-[28px] tracking-tight" style={{ color: "var(--hc-canvas-ink)" }}>
          Building details
        </h2>

        <div className="mt-4 flex flex-col gap-4">
          {/* Rent fairness */}
          <SectionCard
            id="section-rent"
            icon={ICONS.rent}
            iconTint="var(--hc-mixed)"
            title="Rent fairness"
            badge={prioritySections.has("rent") ? priorityBadge : undefined}
            order={sectionOrder("rent")}
            pill={
              rentPct != null
                ? { text: `${fmtPct(rentPct)} median`, color: rentColor, trend: rentPct > 0 ? "up" : "down" }
                : { text: "Unverified", color: "var(--hc-unverified)" }
            }
            rows={[
              rentResult
                ? { label: "Your rent", value: fmtMoney(rentResult.user_rent) }
                : { label: "Tract median", value: fmtMoney(building.rent?.tract_median) },
              rentResult
                ? { label: "Tract median", value: fmtMoney(rentResult.tract_median) }
                : { label: "HUD fair market", value: fmrText },
              {
                label: "Difference",
                value: rentPct != null ? fmtPct(rentPct) : "—",
                hint: rentResult ? fmtMoney(rentResult.user_rent) : undefined,
              },
            ]}
            sentence={
              rentResult
                ? `${rentResult.verdict.replace(/^./, (c) => c.toUpperCase())} — a benchmark against public records, not a judgment of your lease.${
                    Math.abs(rentResult.pct_vs_median ?? 0) > 40
                      ? " Double-check the number."
                      : ""
                  }`
                : "Asking rents in this tract sit near the city benchmark. Paying rent? Check yours below."
            }
            source={{ agency: "US Census B25064", date: DATA_MONTH, href: "https://data.census.gov/table/ACSDT1Y2023.B25064" }}
            onOpenDetail={() => setDetail("rent")}
          >
            <div className="mt-4">
              <SpectrumTrack
                position={pctToPosition(rentPct)}
                markerLabel={rentPct != null ? `${Math.abs(Math.round(rentPct))}%` : null}
              />
            </div>
            <form
              className="mt-4 flex gap-2"
              onSubmit={(e) => {
                e.preventDefault();
                runRentCheck();
              }}
            >
              <div className="flex h-11 flex-1 items-center rounded-full px-4" style={{ background: "var(--hc-sunken)" }}>
                <span className="text-[15px]" style={{ color: "var(--hc-ink-3)" }}>$</span>
                <input
                  value={rentInput}
                  onChange={(e) => setRentInput(e.target.value)}
                  inputMode="numeric"
                  placeholder="Your monthly rent"
                  aria-label="Your monthly rent"
                  className="w-full bg-transparent px-2 text-[15px] tabular-nums outline-none"
                  style={{ color: "var(--hc-ink)" }}
                />
              </div>
              <button
                type="submit"
                disabled={!rentInput.replace(/[^0-9]/g, "") || rentBusy}
                className="h-11 rounded-full px-5 text-[14px] font-semibold text-white disabled:opacity-40"
                style={{ background: "var(--hc-ink)" }}
              >
                {rentBusy ? "…" : "Check"}
              </button>
            </form>
          </SectionCard>

          {/* Building condition */}
          <SectionCard
            id="section-condition"
            icon={ICONS.condition}
            iconTint="var(--hc-concern)"
            title="Building condition"
            badge={prioritySections.has("condition") ? priorityBadge : undefined}
            order={sectionOrder("condition")}
            pill={
              v.c > 0
                ? { text: `${v.c} Class C`, color: "var(--hc-critical)" }
                : { text: "No Class C", color: "var(--hc-strong)" }
            }
            rows={[
              { label: "Class C — hazardous", value: v.c },
              { label: "Class B", value: v.b },
              { label: "Open since", value: v.open_since ?? "—" },
            ]}
            sentence={
              v.c > 0
                ? "Class C means immediately hazardous — ask what the repair timeline is."
                : "No heat complaints on record."
            }
            source={{ agency: "NYC HPD", date: DATA_MONTH, href: "https://hpdonline.nyc.gov/hpdonline" }}
            onOpenDetail={() => setDetail("condition")}
          />

          {/* Legal protections */}
          <SectionCard
            id="section-legal"
            icon={ICONS.legal}
            iconTint="var(--hc-solid)"
            title="Legal protections"
            badge={prioritySections.has("legal") ? priorityBadge : undefined}
            order={sectionOrder("legal")}
            pill={
              building.stabilization === "likely"
                ? { text: "Protected", color: "var(--hc-strong)" }
                : building.stabilization === "unverified"
                  ? { text: "Unverified", color: "var(--hc-unverified)" }
                  : { text: "Not on record", color: "var(--hc-mixed)" }
            }
            rows={[
              { label: "Rent stabilized", value: <span style={{ color: stabColor }}>{stabLabel}</span> },
              {
                label: "Good Cause covered",
                value: building.good_cause == null ? "Unverified" : building.good_cause ? "Yes" : "No",
              },
            ]}
            sentence={
              building.stabilization_message ?? "Confirm stabilization with NYS DHCR before signing."
            }
            source={{ agency: "HPD · DHCR-HCR", date: DATA_MONTH, href: "https://portal.hcr.ny.gov/app/ask" }}
            onOpenDetail={() => setDetail("legal")}
          />

          {/* Accessibility */}
          <SectionCard
            id="section-access"
            icon={ICONS.access}
            iconTint="var(--hc-unverified)"
            title="Accessibility"
            badge={prioritySections.has("access") ? priorityBadge : undefined}
            order={sectionOrder("access")}
            pill={{
              text: building.access_likelihood ?? "Unverified",
              color:
                building.access_likelihood === "Higher"
                  ? "var(--hc-strong)"
                  : building.access_likelihood === "Mixed"
                    ? "var(--hc-mixed)"
                    : "var(--hc-unverified)",
            }}
            rows={[
              { label: "Step-free access", value: building.access_likelihood ?? "Unverified" },
              { label: "Elevator", value: building.has_elevator == null ? "Unverified" : building.has_elevator ? "Yes" : "None on record" },
              { label: "ADA subway", value: fmtDistance(building.near_ada_subway_m) },
            ]}
            sentence="Access likelihood from public records — verify in person."
            footnote={
              !building.has_elevator && building.floors ? (
                <StatusPill text={`${building.floors}-story walk-up`} color="var(--hc-unverified)" />
              ) : undefined
            }
            source={{ agency: "NYC DOB · MTA", date: DATA_MONTH, href: "https://www.mta.com/accessibility" }}
            onOpenDetail={() => setDetail("access")}
          />
        </div>

        {/* Footer actions */}
        <div className="mt-8 flex items-center justify-between">
          <button
            onClick={() => {
              const r = store.addToTray(building.bbl);
              setToast(r.ok ? (store.inTray(building.bbl) && tray.includes(building.bbl) ? "Already in compare" : "Added to compare") : r.reason ?? "Couldn't add");
            }}
            className="flex items-center gap-2 text-[15px] font-semibold"
            style={{ color: "var(--hc-canvas-ink)" }}
          >
            <svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round">
              <path d="M9 3v18M15 3v18M3 9h6M15 15h6" />
            </svg>
            Compare this building
          </button>
          <button onClick={() => navigate("/more")} className="text-[15px] font-semibold" style={{ color: "var(--hc-canvas-ink)" }}>
            How scores work ➜
          </button>
        </div>

        <p className="mt-10 text-center text-[12px]" style={{ color: "var(--hc-canvas-ink-3)" }}>
          Every number links to a NYC or Census source · Data from {DATA_MONTH}
        </p>
      </div>

      {/* Section detail sheet */}
      <Sheet open={!!detail} onClose={() => setDetail(null)} labelledBy="detail-title">
        {detail && (
          <div className="overflow-y-auto px-6 pb-10 pt-2">
            <h2 id="detail-title" className="text-[24px] font-semibold" style={{ color: "var(--hc-ink)" }}>
              {detailContent[detail].title}
            </h2>
            <div className="mt-3">{detailContent[detail].body}</div>
            <button
              onClick={() => {
                setDetail(null);
                navigate("/more");
              }}
              className="mt-6 text-[15px] font-semibold"
              style={{ color: "var(--hc-ink)" }}
            >
              How we calculate this ➜
            </button>
          </div>
        )}
      </Sheet>

      {/* Toast */}
      {toast && (
        <div
          className="hc-anim glass-dark fixed inset-x-0 bottom-24 z-50 mx-auto w-fit max-w-[calc(100%-3rem)] rounded-full px-5 py-3 text-[14px] font-medium text-white"
          style={{ animation: "hc-fade-in 0.2s ease-out" }}
          role="status"
        >
          {toast}
        </div>
      )}
    </div>
  );
}
