import { useEffect, useState } from "react";
import { useLocation, useNavigate } from "react-router";
import { CoverageMap } from "@/components/CoverageMap";
import { COVERAGE_POINTS } from "@/lib/coverage-points";
import { MiniRing } from "@/components/ScoreRing";
import { Sheet } from "@/components/Sheet";
import { listBuildings } from "@/lib/api";
import { BANDS, scoreCircleColor, type Band } from "@/lib/score";
import {
  applyTextSize,
  store,
  useOnboarding,
  useTextSize,
  TEXT_SIZE_LABEL,
  TEXT_SCALE,
  type Priority,
  type TextSize,
} from "@/lib/store";
import type { BuildingSummary, DataSource } from "@/types/building";

const SIZES = Object.keys(TEXT_SCALE) as TextSize[];

const PRIORITY_LABEL: Record<Priority, string> = {
  rent: "Rent fairness",
  condition: "Building condition",
  legal: "Legal protection",
  access: "Accessibility",
  neighborhood: "Neighborhood",
};

/**
 * Current priorities, and a way back into the picker.
 *
 * Onboarding was a one-shot: once answered or skipped it never returned, so
 * there was no way to change your mind, and no way to replay the intro before
 * a demo without clearing site data in devtools. Reset puts the sheet back up
 * immediately — App renders it whenever onboarding is not done.
 */
function PrioritiesCard() {
  const { priorities, skipped } = useOnboarding();

  return (
    <div className="hc-card mt-8 p-5">
      <h2 className="text-[1.0625rem] font-semibold" style={{ color: "var(--hc-ink)" }}>
        What matters to you
      </h2>
      <p className="mt-2 text-[0.875rem] leading-relaxed" style={{ color: "var(--hc-ink-2)" }}>
        These sections move to the top of every Building Health Card. Nothing is ever hidden.
      </p>

      {priorities.length > 0 ? (
        <div className="mt-4 flex flex-wrap gap-2">
          {priorities.map((p) => (
            <span
              key={p}
              className="rounded-full px-3 py-1.5 text-[0.8125rem] font-semibold"
              style={{ background: "var(--hc-ink)", color: "#1C1C1E" }}
            >
              {PRIORITY_LABEL[p]}
            </span>
          ))}
        </div>
      ) : (
        <p className="mt-4 text-[0.875rem]" style={{ color: "var(--hc-ink-3)" }}>
          {skipped ? "Skipped — every section is shown in its default order." : "None picked yet."}
        </p>
      )}

      <button
        onClick={() => store.resetOnboarding()}
        className="mt-5 w-full rounded-full py-3.5 text-[0.9375rem] font-semibold"
        style={{ background: "var(--hc-ink)", color: "#1C1C1E" }}
      >
        {priorities.length > 0 ? "Change priorities" : "Choose priorities"}
      </button>
    </div>
  );
}

/**
 * Reader text size (WCAG 2.2 AA 1.4.4).
 *
 * Browser zoom already satisfies the criterion on paper. This exists because
 * the people using HouseCheck are reading a violation history on a phone in a
 * hallway, and "pinch to zoom, then scroll sideways" is not a real answer.
 * Scaling the root font size keeps the layout intact instead.
 */
function TextSizeControl() {
  const size = useTextSize();

  const choose = (s: TextSize) => {
    store.setTextSize(s);
    applyTextSize(s);
  };

  return (
    <div className="hc-card mt-8 p-5">
      <h2 className="text-[1.0625rem] font-semibold" style={{ color: "var(--hc-ink)" }}>
        Text size
      </h2>
      <p className="mt-2 text-[0.875rem] leading-relaxed" style={{ color: "var(--hc-ink-2)" }}>
        Applies everywhere in HouseCheck and is remembered on this device.
      </p>
      <div
        className="mt-4 flex gap-2"
        role="radiogroup"
        aria-label="Text size"
      >
        {SIZES.map((s) => {
          const active = s === size;
          return (
            <button
              key={s}
              role="radio"
              aria-checked={active}
              onClick={() => choose(s)}
              className="flex-1 rounded-2xl px-2 py-3 font-semibold"
              style={{
                background: active ? "var(--hc-ink)" : "var(--hc-sunken)",
                color: active ? "#1C1C1E" : "var(--hc-ink-2)",
                // Deliberately fixed px, not rem: these are previews of the
                // sizes, so they must NOT resize with the setting they set.
                fontSize: `${13 * TEXT_SCALE[s]}px`,
              }}
            >
              {TEXT_SIZE_LABEL[s]}
            </button>
          );
        })}
      </div>
    </div>
  );
}

function Methodology() {
  const legend: { band: Band; range: string; sample: number | null }[] = [
    { band: "strong", range: "80–100", sample: 90 },
    { band: "solid", range: "60–79", sample: 70 },
    { band: "mixed", range: "40–59", sample: 50 },
    { band: "concern", range: "20–39", sample: 30 },
    { band: "critical", range: "0–19", sample: 10 },
    { band: "unverified", range: "no data", sample: null },
  ];

  return (
    <div className="hc-card p-5">
      <h2 className="text-[1.25rem]" style={{ color: "var(--hc-ink)" }}>
        How scores work
      </h2>
      <p className="mt-2 text-[0.9375rem] leading-relaxed" style={{ color: "var(--hc-ink-2)" }}>
        The Building Health Score runs 0–100 and is a plain average of four pillars —{" "}
        <strong style={{ color: "var(--hc-ink)" }}>each pillar counts equally</strong>: condition
        (HPD violations, 311), legal protections (DHCR stabilization, Good Cause), neighborhood
        rents (Census, HUD), and accessibility (DOB elevators, MTA ADA stations). A missing pillar
        is marked <em>unverified</em> — never treated as a zero.
      </p>
      <div className="mt-4 space-y-2">
        {legend.map(({ band, range, sample }) => {
          const color = scoreCircleColor(sample);
          return (
            <div key={band} className="flex items-center gap-3">
              <span className="h-3 w-3 rounded-full" style={{ background: color }} />
              <span className="w-24 text-[0.8125rem] font-semibold tabular-nums" style={{ color }}>
                {range}
              </span>
              <span className="text-[0.875rem]" style={{ color: "var(--hc-ink-2)" }}>
                {BANDS[band].label}
              </span>
            </div>
          );
        })}
      </div>
      <p className="mt-4 text-[0.8125rem] leading-relaxed" style={{ color: "var(--hc-ink-3)" }}>
        HouseCheck is a signal built from public records — not a legal ruling, an inspection, or
        rental advice. Always verify in person and with the agency listed on each source line.
      </p>
    </div>
  );
}

export default function More() {
  const navigate = useNavigate();
  const location = useLocation();
  // Home and the out-of-coverage sheet both deep-link straight to the list.
  const openListOnArrival = (location.state as { openList?: boolean } | null)?.openList === true;
  const [buildings, setBuildings] = useState<BuildingSummary[]>([]);
  /** The list could not be fetched, as distinct from being fetched and empty. */
  const [loadFailed, setLoadFailed] = useState(false);
  const [source, setSource] = useState<DataSource>("live");
  const [sortDesc, setSortDesc] = useState(false);
  const [showList, setShowList] = useState(openListOnArrival);

  useEffect(() => {
    listBuildings()
      .then(({ data, source }) => {
        setBuildings(data);
        setSource(source);
        setLoadFailed(false);
      })
      // Same unguarded-promise class as the search box, and this is the sheet the
      // out-of-coverage path sends people to. With no catch, a failed fetch left the header
      // reading "Showing 0 of 250" over an empty list -- which looks like an answer about
      // the pilot rather than a failure to load it.
      .catch(() => setLoadFailed(true));
  }, []);

  const sorted = [...buildings].sort((a, b) =>
    sortDesc ? (b.score ?? -1) - (a.score ?? -1) : (a.score ?? 999) - (b.score ?? 999)
  );

  return (
    <div className="mx-auto min-h-dvh w-full max-w-md px-5 pb-32 pt-14">
      <h1 className="text-[1.875rem] font-semibold tracking-tight" style={{ color: "var(--hc-canvas-ink)" }}>
        About
      </h1>

      {source === "demo" && (
        <p
          className="mt-4 rounded-full px-4 py-2 text-center text-[0.75rem] font-medium"
          style={{ background: "var(--hc-sunken)", color: "var(--hc-ink-2)" }}
        >
          Backend unreachable — showing bundled demo data
        </p>
      )}

      <div className="hc-card mt-6 p-5">
        <h2 className="text-[1.0625rem] font-semibold" style={{ color: "var(--hc-ink)" }}>
          Covered buildings
        </h2>
        <p className="mt-2 text-[0.875rem] leading-relaxed" style={{ color: "var(--hc-ink-2)" }}>
          Every address HouseCheck holds a full record for. Outside this area we say so
          rather than guess.
        </p>
        <CoverageMap height={132} caption={null} />
        <button
          onClick={() => setShowList(true)}
          className="mt-4 w-full rounded-full py-3.5 text-[0.9375rem] font-semibold"
          style={{ background: "var(--hc-ink)", color: "#1C1C1E" }}
        >
          {/* The pilot size comes from the shipped dataset, not from whatever
              the list call returned. If the API is unreachable the list falls
              back to a 4-building fixture, and "Browse all 4" sitting under a
              250-point map would misstate our coverage. The demo banner above
              already says the data is stale; the coverage claim stays true. */}
          Browse all {COVERAGE_POINTS.length} buildings
        </button>
      </div>

      <div className="mt-6">
        <Methodology />
      </div>

      <PrioritiesCard />

      <TextSizeControl />

      <div className="hc-card mt-8 p-5">
        <h2 className="text-[1.0625rem] font-semibold" style={{ color: "var(--hc-ink)" }}>
          Data sources
        </h2>
        <p className="mt-2 text-[0.875rem] leading-relaxed" style={{ color: "var(--hc-ink-2)" }}>
          NYC HPD violations · NYC DOB records · 311 complaints · US Census ACS (B25064) · HUD Fair
          Market Rents · NYS DHCR · MTA accessibility. Free, public, and linked from every card.
        </p>
      </div>

      <p className="mt-8 text-center text-[0.75rem]" style={{ color: "var(--hc-canvas-ink-3)" }}>
        HouseCheck · a Pursuit fellowship project · Brooklyn, NY
      </p>

      <Sheet open={showList} onClose={() => setShowList(false)} labelledBy="covered-title">
        <div className="px-5 pb-8 pt-1">
          <div className="flex items-end justify-between">
            <div>
              <h2 id="covered-title" className="text-[1.375rem] font-semibold" style={{ color: "var(--hc-ink)" }}>
                Covered buildings
              </h2>
              {/* Both numbers, so the line stays true when the API is
                  unreachable and the list is the 4-building fixture. */}
              <p className="mt-0.5 text-[0.8125rem]" style={{ color: "var(--hc-ink-2)" }}>
                {loadFailed
                  ? "We couldn't load the building list. Check your connection and reload."
                  : buildings.length === COVERAGE_POINTS.length
                    ? `${buildings.length} in the Bed-Stuy pilot · links to public NYC data`
                    : `Showing ${buildings.length} of ${COVERAGE_POINTS.length} in the Bed-Stuy pilot · links to public NYC data`}
              </p>
            </div>
            <button
              onClick={() => setSortDesc((v) => !v)}
              className="rounded-full px-3 py-1.5 text-[0.75rem] font-semibold"
              style={{ background: "var(--hc-sunken)", color: "var(--hc-ink-2)" }}
              aria-label="Toggle sort order"
            >
              Score {sortDesc ? "↓" : "↑"}
            </button>
          </div>
          <div className="mt-3 max-h-[60vh] space-y-2 overflow-y-auto">
            {sorted.map((b) => (
              <button
                key={b.bbl}
                onClick={() => navigate(`/building/${b.bbl}`)}
                className="flex w-full items-center gap-3 rounded-2xl p-3 text-left"
                style={{ background: "#48484A", boxShadow: "0 4px 16px rgba(0,0,0,0.2)" }}
              >
                <MiniRing score={b.score} size={36} stroke={4.5} />
                <span className="flex-1 text-[0.9375rem] font-medium" style={{ color: "var(--hc-ink)" }}>
                  {b.address}
                </span>
                <span className="text-[0.9375rem] font-semibold tabular-nums" style={{ color: scoreCircleColor(b.score) }}>
                  {b.score ?? "—"}
                </span>
              </button>
            ))}
          </div>
        </div>
      </Sheet>
    </div>
  );
}
