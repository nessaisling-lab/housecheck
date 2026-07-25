import { useEffect, useRef, useState } from "react";
import { useNavigate } from "react-router";
import { Sheet } from "@/components/Sheet";
import { searchAddress } from "@/lib/api";
import { useRecents } from "@/lib/store";
import type { SearchResult } from "@/types/building";

// Real buildings in the ~250-building Bed-Stuy pilot set (verified in /buildings).
// Chosen to tell a story: one strong record, one middling, one to avoid.
const DEFAULT_SUGGESTIONS = ["1024 Gates Avenue", "633 Marcy Avenue", "1754 Fulton Street"];

export default function Home() {
  const navigate = useNavigate();
  const recents = useRecents();
  const [q, setQ] = useState("");
  const [results, setResults] = useState<SearchResult[] | null>(null);
  const [notFound, setNotFound] = useState(false);
  const [coverage, setCoverage] = useState<SearchResult | null>(null);
  const [searching, setSearching] = useState(false);
  const debounce = useRef<ReturnType<typeof setTimeout>>(null);

  useEffect(() => {
    if (debounce.current) clearTimeout(debounce.current);
    const query = q.trim();
    // Always defer to a timeout: resets and searches both sync with the
    // external system (the API), not with render (react-hooks/set-state-in-effect).
    debounce.current = setTimeout(
      async () => {
        if (query.length < 3) {
          setResults(null);
          setNotFound(false);
          return;
        }
        setSearching(true);
        const { data } = await searchAddress(query);
        setSearching(false);
        const inSet = data.filter((r) => r.in_curated_set);
        const outSet = data.find((r) => !r.in_curated_set);
        setResults(inSet.length ? inSet : null);
        setNotFound(!inSet.length && !outSet);
        if (!inSet.length && outSet) setCoverage(outSet);
      },
      query.length < 3 ? 0 : 300
    );
    return () => {
      if (debounce.current) clearTimeout(debounce.current);
    };
  }, [q]);

  const pick = (r: SearchResult) => {
    if (!r.in_curated_set) {
      setCoverage(r);
      return;
    }
    navigate(`/building/${r.bbl}`);
  };

  const chips = recents.length
    ? recents.slice(0, 3).map((r) => ({ label: r.address, bbl: r.bbl }))
    : DEFAULT_SUGGESTIONS.map((label) => ({ label, bbl: null as string | null }));

  return (
    <div className="mx-auto flex min-h-dvh w-full max-w-md flex-col items-center px-6 pb-32 pt-14 text-center">
      <div className="flex w-full flex-1 flex-col items-center justify-center pb-16">
        <h1 className="text-[24px] font-semibold tracking-tight" style={{ color: "var(--hc-ink)" }}>
          HouseCheck
        </h1>

        <div
          className="glass-orb mt-8 flex h-24 w-24 items-center justify-center rounded-full"
          aria-hidden
        >
          <svg
            width="44"
            height="44"
            viewBox="0 0 24 24"
            fill="none"
            stroke="var(--hc-ink)"
            strokeWidth="1.8"
            strokeLinecap="round"
            strokeLinejoin="round"
          >
            <path d="M3 11l9-7 9 7" />
            <path d="M6 9.5V20h12V9.5" />
            <path d="M9.5 14.5l2.5 2.5 4.5-4.5" />
          </svg>
        </div>

        <p
          className="mt-8 text-[34px] font-semibold leading-[1.12] tracking-tight"
          style={{ color: "var(--hc-ink)" }}
        >
          Know the building
          <br />
          before you sign.
        </p>

        <div className="relative mt-8 w-full">
        <div
          className="glass-field flex h-14 items-center gap-3 rounded-full px-5"
        >
          <svg width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="var(--hc-ink-3)" strokeWidth="2" strokeLinecap="round" aria-hidden>
            <circle cx="11" cy="11" r="7" />
            <path d="M20 20l-3.5-3.5" />
          </svg>
          <input
            value={q}
            onChange={(e) => setQ(e.target.value)}
            placeholder="look up a building"
            className="w-full bg-transparent text-[17px] outline-none"
            style={{ color: "var(--hc-ink)" }}
            aria-label="Look up a building by address"
            enterKeyHint="search"
          />
          {searching && (
            <span
              className="h-4 w-4 shrink-0 animate-spin rounded-full border-2 border-t-transparent"
              style={{ borderColor: "var(--hc-ink-3)", borderTopColor: "transparent" }}
            />
          )}
        </div>

        {results && (
          <div className="hc-card absolute inset-x-0 top-16 z-20 overflow-hidden" role="listbox">
            {results.map((r) => (
              <button
                key={r.bbl}
                role="option"
                aria-selected="false"
                onClick={() => pick(r)}
                className="flex w-full items-center gap-3 px-5 py-3.5 text-left text-[16px] hover:bg-black/[0.03]"
                style={{ color: "var(--hc-ink)" }}
              >
                <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="var(--hc-ink-3)" strokeWidth="2" strokeLinecap="round" aria-hidden>
                  <path d="M12 21s7-5.5 7-11a7 7 0 10-14 0c0 5.5 7 11 7 11z" />
                  <circle cx="12" cy="10" r="2.5" />
                </svg>
                {r.label}
              </button>
            ))}
          </div>
        )}

        {notFound && (
          <p className="mt-3 px-2 text-[14px]" style={{ color: "var(--hc-ink-2)" }}>
            Address not found — try street + house number.
          </p>
        )}
        </div>

        <div className="mt-6 flex flex-col items-center gap-2.5">
          {chips.map((c) => (
            <button
              key={c.label}
              onClick={() => (c.bbl ? navigate(`/building/${c.bbl}`) : setQ(c.label))}
              className="rounded-full px-4 py-2 text-[14px] font-medium"
              style={{ background: "var(--hc-sunken)", color: "var(--hc-ink-2)" }}
            >
              {c.label}
            </button>
          ))}
        </div>
      </div>

      {/* Out-of-coverage sheet (flow 1 edge state) */}
      <Sheet open={!!coverage} onClose={() => setCoverage(null)} labelledBy="coverage-title">
        <div className="px-6 pb-10 pt-2">
          <h2 id="coverage-title" className="text-[24px] font-semibold" style={{ color: "var(--hc-ink)" }}>
            We're not there yet
          </h2>
          <p className="mt-2 text-[15px] leading-snug" style={{ color: "var(--hc-ink-2)" }}>
            HouseCheck currently covers ~250 buildings in Bedford-Stuyvesant for our pilot.
            {coverage?.label ? ` “${coverage.label}” is outside that area.` : ""}
          </p>
          <div
            className="relative mt-5 h-32 overflow-hidden rounded-2xl"
            style={{ background: "var(--hc-sunken)" }}
            aria-hidden
          >
            {[
              [18, 40], [32, 66], [46, 30], [58, 58], [70, 36], [82, 62],
              [26, 78], [64, 82], [88, 30], [40, 50],
            ].map(([x, y], i) => (
              <span
                key={i}
                className="absolute h-2 w-2 rounded-full"
                style={{ left: `${x}%`, top: `${y}%`, background: "rgba(60,60,67,0.35)" }}
              />
            ))}
          </div>
          <p className="mt-2 text-center text-[12px]" style={{ color: "var(--hc-ink-3)" }}>
            Coverage: Bed-Stuy pilot area
          </p>
          <button
            onClick={() => {
              setCoverage(null);
              navigate("/more");
            }}
            className="mt-5 w-full rounded-full py-4 text-[16px] font-semibold text-white"
            style={{ background: "var(--hc-ink)" }}
          >
            Explore covered buildings
          </button>
          <button
            className="mt-4 w-full text-center text-[15px] font-medium"
            style={{ color: "var(--hc-ink)" }}
            onClick={() => setCoverage(null)}
          >
            Get notified when we expand →
          </button>
        </div>
      </Sheet>
    </div>
  );
}
