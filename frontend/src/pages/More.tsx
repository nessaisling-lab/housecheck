import { useEffect, useState } from "react";
import { useNavigate } from "react-router";
import { MiniRing } from "@/components/ScoreRing";
import { Sheet } from "@/components/Sheet";
import { listBuildings } from "@/lib/api";
import { BANDS, scoreCircleColor, type Band } from "@/lib/score";
import type { BuildingSummary, DataSource } from "@/types/building";

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
      <h2 className="text-[20px]" style={{ color: "var(--hc-ink)" }}>
        How scores work
      </h2>
      <p className="mt-2 text-[15px] leading-relaxed" style={{ color: "var(--hc-ink-2)" }}>
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
              <span className="w-24 text-[13px] font-semibold tabular-nums" style={{ color }}>
                {range}
              </span>
              <span className="text-[14px]" style={{ color: "var(--hc-ink-2)" }}>
                {BANDS[band].label}
              </span>
            </div>
          );
        })}
      </div>
      <p className="mt-4 text-[13px] leading-relaxed" style={{ color: "var(--hc-ink-3)" }}>
        HouseCheck is a signal built from public records — not a legal ruling, an inspection, or
        rental advice. Always verify in person and with the agency listed on each source line.
      </p>
    </div>
  );
}

export default function More() {
  const navigate = useNavigate();
  const [buildings, setBuildings] = useState<BuildingSummary[]>([]);
  const [source, setSource] = useState<DataSource>("live");
  const [sortDesc, setSortDesc] = useState(false);
  const [showList, setShowList] = useState(false);

  useEffect(() => {
    listBuildings().then(({ data, source }) => {
      setBuildings(data);
      setSource(source);
    });
  }, []);

  const sorted = [...buildings].sort((a, b) =>
    sortDesc ? (b.score ?? -1) - (a.score ?? -1) : (a.score ?? 999) - (b.score ?? 999)
  );

  return (
    <div className="mx-auto min-h-dvh w-full max-w-md px-5 pb-32 pt-14">
      <h1 className="text-[30px] font-semibold tracking-tight" style={{ color: "var(--hc-canvas-ink)" }}>
        About
      </h1>

      {source === "demo" && (
        <p
          className="mt-4 rounded-full px-4 py-2 text-center text-[12px] font-medium"
          style={{ background: "var(--hc-sunken)", color: "var(--hc-ink-2)" }}
        >
          Backend unreachable — showing bundled demo data
        </p>
      )}

      <div className="mt-6">
        <Methodology />
      </div>

      <button
        onClick={() => setShowList(true)}
        className="hc-card mt-8 flex w-full items-center gap-3 p-4 text-left"
      >
        <span className="flex-1">
          <span className="block text-[17px] font-medium" style={{ color: "var(--hc-ink)" }}>
            Covered buildings
          </span>
          <span className="mt-0.5 block text-[13px]" style={{ color: "var(--hc-ink-2)" }}>
            {buildings.length} in the Bed-Stuy pilot
          </span>
        </span>
        <svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="var(--hc-ink-3)" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round" aria-hidden>
          <path d="M9 6l6 6-6 6" />
        </svg>
      </button>

      <div className="hc-card mt-8 p-5">
        <h2 className="text-[17px] font-semibold" style={{ color: "var(--hc-ink)" }}>
          Data sources
        </h2>
        <p className="mt-2 text-[14px] leading-relaxed" style={{ color: "var(--hc-ink-2)" }}>
          NYC HPD violations · NYC DOB records · 311 complaints · US Census ACS (B25064) · HUD Fair
          Market Rents · NYS DHCR · MTA accessibility. Free, public, and linked from every card.
        </p>
      </div>

      <p className="mt-8 text-center text-[12px]" style={{ color: "var(--hc-canvas-ink-3)" }}>
        HouseCheck · a Pursuit fellowship project · Brooklyn, NY
      </p>

      <Sheet open={showList} onClose={() => setShowList(false)} labelledBy="covered-title">
        <div className="px-5 pb-8 pt-1">
          <div className="flex items-end justify-between">
            <div>
              <h2 id="covered-title" className="text-[22px] font-semibold" style={{ color: "var(--hc-ink)" }}>
                Covered buildings
              </h2>
              <p className="mt-0.5 text-[13px]" style={{ color: "var(--hc-ink-2)" }}>
                {buildings.length} in the Bed-Stuy pilot · links to public NYC data
              </p>
            </div>
            <button
              onClick={() => setSortDesc((v) => !v)}
              className="rounded-full px-3 py-1.5 text-[12px] font-semibold"
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
                className="flex w-full items-center gap-3 rounded-2xl bg-white p-3 text-left"
                style={{ boxShadow: "0 4px 16px rgba(0,0,0,0.06)" }}
              >
                <MiniRing score={b.score} size={36} stroke={4.5} />
                <span className="flex-1 text-[15px] font-medium" style={{ color: "var(--hc-ink)" }}>
                  {b.address}
                </span>
                <span className="text-[15px] font-semibold tabular-nums" style={{ color: scoreCircleColor(b.score) }}>
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
