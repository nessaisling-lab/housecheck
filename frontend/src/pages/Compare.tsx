import { useEffect, useMemo, useState } from "react";
import { useNavigate } from "react-router";
import { MiniRing } from "@/components/ScoreRing";
import { compareBuildings } from "@/lib/api";
import { fmtDistance } from "@/lib/score";
import { store, useTray } from "@/lib/store";
import type { BuildingCard } from "@/types/building";

interface RowDef {
  label: string;
  get: (b: BuildingCard) => string | number;
  raw: (b: BuildingCard) => number | null | undefined;
  /** higher raw value = better */
  higherIsBetter: boolean;
}

const ROWS: RowDef[] = [
  { label: "Total", get: (b) => b.score ?? "—", raw: (b) => b.score, higherIsBetter: true },
  { label: "Condition", get: (b) => b.sub_scores.condition ?? "—", raw: (b) => b.sub_scores.condition, higherIsBetter: true },
  { label: "Legal", get: (b) => b.sub_scores.legal ?? "—", raw: (b) => b.sub_scores.legal, higherIsBetter: true },
  { label: "Neighborhood", get: (b) => b.sub_scores.neighborhood ?? "—", raw: (b) => b.sub_scores.neighborhood, higherIsBetter: true },
  { label: "Accessibility", get: (b) => b.sub_scores.accessibility ?? "—", raw: (b) => b.sub_scores.accessibility, higherIsBetter: true },
  { label: "Class C", get: (b) => b.open_violations.c, raw: (b) => b.open_violations.c, higherIsBetter: false },
  {
    label: "Stabilized",
    get: (b) => (b.stabilization === "likely" ? "Yes" : b.stabilization === "unverified" ? "Unverified" : "No"),
    raw: (b) => (b.stabilization === "likely" ? 1 : b.stabilization === "none_on_record" ? 0 : null),
    higherIsBetter: true,
  },
  {
    label: "Elevator",
    get: (b) => (b.has_elevator == null ? "Unverified" : b.has_elevator ? "Yes" : "No"),
    raw: (b) => (b.has_elevator == null ? null : b.has_elevator ? 1 : 0),
    higherIsBetter: true,
  },
  {
    label: "ADA subway",
    get: (b) => fmtDistance(b.near_ada_subway_m),
    raw: (b) => b.near_ada_subway_m,
    higherIsBetter: false,
  },
];

export default function Compare() {
  const navigate = useNavigate();
  const tray = useTray();
  // Outcome keyed by tray signature — loading/error/cards are all derived,
  // so the effect only reports results (no sync setState in the effect body).
  const [outcome, setOutcome] = useState<
    { key: string; cards: BuildingCard[]; error?: boolean } | null
  >(null);
  const key = tray.join(",");

  useEffect(() => {
    if (tray.length < 2) return;
    let cancelled = false;
    compareBuildings(tray)
      .then(({ data }) => {
        if (cancelled) return;
        const cards = tray
          .map((bbl) => data.find((d) => d.bbl === bbl))
          .filter((b): b is BuildingCard => !!b);
        setOutcome({ key: tray.join(","), cards });
      })
      .catch(() => {
        if (!cancelled) setOutcome({ key: tray.join(","), cards: [], error: true });
      });
    return () => {
      cancelled = true;
    };
  }, [tray]);

  const current = useMemo(() => (outcome?.key === key ? outcome : null), [outcome, key]);
  const cards = useMemo(() => current?.cards ?? [], [current]);
  const loading = tray.length >= 2 && !current;
  const error = !!current?.error;

  const best = useMemo(() => {
    return ROWS.map((row) => {
      const vals = cards.map((c) => row.raw(c));
      const valid = vals.filter((v): v is number => v != null);
      if (valid.length < 2) return null;
      return row.higherIsBetter ? Math.max(...valid) : Math.min(...valid);
    });
  }, [cards]);

  // Plain-language takeaway: who leads overall, and any notable trade-off.
  const summary = useMemo(() => {
    const short = (a: string) => a.split(",")[0];
    const scored = cards.filter((c) => c.score != null);
    if (scored.length < 2) return null;
    const leader = scored.reduce((a, b) => ((b.score ?? 0) > (a.score ?? 0) ? b : a));
    const parts = [`${short(leader.address)} leads overall at ${leader.score}/100.`];
    const cond = cards.filter((c) => c.sub_scores.condition != null);
    if (cond.length >= 2) {
      const bestCond = cond.reduce((a, b) =>
        ((b.sub_scores.condition ?? 0) > (a.sub_scores.condition ?? 0) ? b : a)
      );
      if (bestCond.bbl !== leader.bbl) {
        parts.push(`${short(bestCond.address)} has the best building condition.`);
      }
    }
    return parts.join(" ");
  }, [cards]);

  if (tray.length < 2) {
    return (
      <div className="mx-auto flex min-h-dvh w-full max-w-md flex-col px-5 pb-32 pt-14">
        <h1 className="text-[30px] font-semibold tracking-tight" style={{ color: "var(--hc-ink)" }}>
          Compare
        </h1>
        <div className="mt-10 rounded-2xl p-5" style={{ background: "var(--hc-sunken)" }}>
          <p className="text-[16px] font-medium" style={{ color: "var(--hc-ink)" }}>
            {tray.length === 0 ? "Your compare tray is empty" : "Add one more building"}
          </p>
          <p className="mt-1.5 text-[14px] leading-snug" style={{ color: "var(--hc-ink-2)" }}>
            Add 2–4 buildings from Search or Saved, then compare scores, violations, and access side by side.
          </p>
          <div className="mt-4 flex flex-wrap gap-2.5">
            <button
              onClick={() => navigate("/")}
              className="rounded-full px-5 py-2.5 text-[14px] font-semibold text-white"
              style={{ background: "var(--hc-ink)" }}
            >
              Search a new building
            </button>
            <button
              onClick={() => navigate("/saved")}
              className="rounded-full px-5 py-2.5 text-[14px] font-semibold"
              style={{ background: "var(--hc-card)", color: "var(--hc-ink)", border: "0.5px solid var(--hc-ink-3)" }}
            >
              Pick from Saved
            </button>
          </div>
        </div>
      </div>
    );
  }

  return (
    <div className="mx-auto min-h-dvh w-full max-w-md px-5 pb-32 pt-14">
      <h1 className="text-[30px] font-semibold tracking-tight" style={{ color: "var(--hc-ink)" }}>
        Compare
      </h1>
      <p className="mt-1 text-[14px]" style={{ color: "var(--hc-ink-2)" }}>
        {tray.length} buildings
      </p>

      {loading && (
        <div className="mt-8 space-y-2.5" aria-label="Loading comparison">
          {[0, 1, 2, 3].map((i) => (
            <div key={i} className="h-11 rounded-xl" style={{ background: "var(--hc-sunken)", animation: "hc-pulse-soft 1.2s infinite" }} />
          ))}
        </div>
      )}

      {error && (
        <div className="hc-card mt-8 p-5">
          <p className="text-[16px] font-medium" style={{ color: "var(--hc-ink)" }}>
            We couldn't load the comparison
          </p>
          <p className="mt-1 text-[14px]" style={{ color: "var(--hc-ink-2)" }}>
            Check your connection and try again — your tray is kept.
          </p>
        </div>
      )}

      {!loading && !error && cards.length > 0 && (
        <>
          {summary && (
            <div className="hc-card mt-6 p-4">
              <p className="text-[12px] font-semibold uppercase tracking-wide" style={{ color: "var(--hc-ink-3)" }}>
                Summary
              </p>
              <p className="mt-1 text-[15px] leading-snug" style={{ color: "var(--hc-ink)" }}>
                {summary}
              </p>
            </div>
          )}
          <div className="mt-4 overflow-x-auto">
          <table className="w-full border-collapse" style={{ minWidth: 120 + cards.length * 88 }}>
            <thead>
              <tr>
                <th className="w-[104px]" />
                {cards.map((c) => (
                  <th key={c.bbl} className="relative px-1 pb-3 align-top">
                    <button
                      onClick={() => {
                        store.removeFromTray(c.bbl);
                      }}
                      aria-label={`Remove ${c.address} from compare`}
                      className="absolute -top-1 right-0 p-1"
                      style={{ color: "var(--hc-ink-3)" }}
                    >
                      <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round">
                        <path d="M6 6l12 12M18 6L6 18" />
                      </svg>
                    </button>
                    <button onClick={() => navigate(`/building/${c.bbl}`)} className="flex flex-col items-center gap-1.5">
                      <MiniRing score={c.score} size={52} stroke={6} />
                      <span className="max-w-[84px] text-center text-[12px] font-medium leading-tight" style={{ color: "var(--hc-ink)" }}>
                        {c.address}
                      </span>
                    </button>
                  </th>
                ))}
              </tr>
            </thead>
            <tbody>
              {ROWS.map((row, ri) => (
                <tr key={row.label} style={{ background: ri % 2 ? "transparent" : "rgba(60,60,67,0.045)" }}>
                  <th
                    scope="row"
                    className="hc-row-label px-2 py-2.5 text-left align-middle font-semibold"
                    style={{ maxWidth: 104 }}
                  >
                    {row.label}
                  </th>
                  {cards.map((c) => {
                    const raw = row.raw(c);
                    const isBest = best[ri] != null && raw != null && raw === best[ri];
                    const isUnverified = raw == null;
                    return (
                      <td
                        key={c.bbl}
                        className="px-1 py-2.5 text-center text-[15px] tabular-nums"
                        style={{
                          color: isUnverified ? "var(--hc-ink-3)" : "var(--hc-ink)",
                          fontWeight: isBest ? 700 : 400,
                        }}
                      >
                        {row.get(c)}
                      </td>
                    );
                  })}
                </tr>
              ))}
            </tbody>
          </table>
          <p className="mt-5 text-center text-[13px]" style={{ color: "var(--hc-ink-3)" }}>
            Best value per row in bold · tap a column for the full card
          </p>
          </div>
        </>
      )}
    </div>
  );
}
