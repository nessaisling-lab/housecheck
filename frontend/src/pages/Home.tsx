import { useEffect, useRef, useState } from "react";
import { useNavigate } from "react-router";
import { Sheet } from "@/components/Sheet";
import { searchAddress } from "@/lib/api";
import type { SearchResult } from "@/types/building";

export default function Home() {
  const navigate = useNavigate();
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

  // Only surface the user's own recent lookups — no canned sample addresses.

  return (
    <div
      className="relative mx-auto flex min-h-dvh w-full max-w-md flex-col items-center px-6 pb-32 pt-14 text-center"
      style={{ background: "rgb(215, 215, 217)" }}
    >
      <img
        src="/city-hero.jpg"
        alt=""
        aria-hidden="true"
        className="pointer-events-none absolute inset-x-0 top-0 h-[52dvh] w-full object-cover"
      />
      {/* Fade the city into the page canvas so text above stays readable */}
      <div
        aria-hidden="true"
        className="pointer-events-none absolute inset-x-0 top-0 h-[52dvh]"
        style={{ background: "linear-gradient(to bottom, transparent 44%, rgb(215, 215, 217) 94%)" }}
      />
      <div className="relative z-10 flex w-full flex-1 flex-col items-center justify-center pb-16">
        <h1 className="sr-only">HouseCheck</h1>
        <img src="/housecheck-logo.svg" alt="HouseCheck" className="h-auto w-52" />

        <p
          className="mt-8 text-[2.125rem] font-semibold leading-[1.12] tracking-tight"
          style={{ color: "var(--hc-canvas-ink)" }}
        >
          Know the building
          <br />
          before you sign.
        </p>

        <div className="relative mt-8 w-full">
        <div
          className="flex h-14 items-center gap-3 rounded-full px-5"
          style={{ background: "#3A3A3C" }}
        >
          <svg width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="rgba(255,255,255,0.55)" strokeWidth="2" strokeLinecap="round" aria-hidden>
            <circle cx="11" cy="11" r="7" />
            <path d="M20 20l-3.5-3.5" />
          </svg>
          <input
            value={q}
            onChange={(e) => setQ(e.target.value)}
            placeholder="look up a building"
            className="w-full bg-transparent text-[1.0625rem] outline-none placeholder:text-white/40"
            style={{ color: "#F5F5F7" }}
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
                className="flex w-full items-center gap-3 px-5 py-3.5 text-left text-[1rem] hover:bg-black/[0.03]"
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
          <p className="mt-3 px-2 text-[0.875rem]" style={{ color: "var(--hc-canvas-ink-2)" }}>
            Address not found — try street + house number.
          </p>
        )}
        </div>
      </div>

      {/* Out-of-coverage sheet (flow 1 edge state) */}
      <Sheet open={!!coverage} onClose={() => setCoverage(null)} labelledBy="coverage-title">
        <div className="px-6 pb-10 pt-2">
          <h2 id="coverage-title" className="text-[1.5rem] font-semibold" style={{ color: "var(--hc-ink)" }}>
            We're not there yet
          </h2>
          <p className="mt-2 text-[0.9375rem] leading-snug" style={{ color: "var(--hc-ink-2)" }}>
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
          <p className="mt-2 text-center text-[0.75rem]" style={{ color: "var(--hc-ink-3)" }}>
            Coverage: Bed-Stuy pilot area
          </p>
          <button
            onClick={() => {
              setCoverage(null);
              navigate("/more");
            }}
            className="mt-5 w-full rounded-full py-4 text-[1rem] font-semibold text-white"
            style={{ background: "var(--hc-ink)" }}
          >
            Explore covered buildings
          </button>
          <button
            className="mt-4 w-full text-center text-[0.9375rem] font-medium"
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
