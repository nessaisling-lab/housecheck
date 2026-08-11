import { useEffect, useRef, useState } from "react";
import { useNavigate } from "react-router";
import { CoverageMap } from "@/components/CoverageMap";
import { COVERAGE_POINTS } from "@/lib/coverage-points";
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
  /** Whether the reader has asked to look past the pilot. Resets whenever they retype. */
  const [citywide, setCitywide] = useState(false);
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
        const { data } = await searchAddress(query, citywide ? "city" : undefined);
        setSearching(false);
        // Show every match, in-pilot or not, and let the user choose.
        //
        // This used to filter to in-pilot results and open the out-of-coverage
        // sheet automatically. Because it runs on a 300ms debounce, the sheet
        // fired mid-typing: "450" geocodes to 450 Broadway, Manhattan, so the
        // modal interrupted you before you had finished the address. A partial
        // query is not a decision. The sheet now opens only from pick().
        setResults(data.length ? data : null);
        setNotFound(data.length === 0);
      },
      query.length < 3 ? 0 : 300
    );
    return () => {
      if (debounce.current) clearTimeout(debounce.current);
    };
  }, [q, citywide]);

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
            onChange={(e) => {
              // Reset the scope here rather than in an effect on `q`: a new query is a new
              // question, and doing it at the source means the search effect never observes
              // a scope belonging to the previous one.
              setCitywide(false);
              setQ(e.target.value);
            }}
            placeholder="look up a building"
            className="w-full bg-transparent text-[1.0625rem] outline-none placeholder:text-white/40"
            style={{ color: "#F5F5F7" }}
            aria-label="Look up a building by address"
            enterKeyHint="search"
            // Type the address, press Enter, you are in the building. Without
            // this the only way through was to notice the dropdown and tap it.
            onKeyDown={(e) => {
              if (e.key !== "Enter" || !results?.length) return;
              e.preventDefault();
              pick(results[0]);
            }}
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
                style={{ color: r.in_curated_set ? "var(--hc-ink)" : "var(--hc-ink-2)" }}
              >
                <svg
                  width="16"
                  height="16"
                  viewBox="0 0 24 24"
                  fill="none"
                  stroke={r.in_curated_set ? "var(--hc-strong)" : "var(--hc-ink-3)"}
                  strokeWidth="2"
                  strokeLinecap="round"
                  aria-hidden
                >
                  <path d="M12 21s7-5.5 7-11a7 7 0 10-14 0c0 5.5 7 11 7 11z" />
                  <circle cx="12" cy="10" r="2.5" />
                </svg>
                {/* Borough on its own line, always, never only when it looks ambiguous.
                    NYC reuses street names across all five boroughs and a typed address
                    almost never names one, so "869 Park Avenue" is a real building in both
                    Brooklyn and Manhattan. The reader is the only person who knows which one
                    they meant, and this word is what lets them tell before they tap. */}
                <span className="flex-1">
                  <span className="block">{r.label}</span>
                  {r.borough && (
                    <span className="block text-[0.8125rem]" style={{ color: "var(--hc-ink-3)" }}>
                      {r.borough}
                    </span>
                  )}
                </span>
                {/* Say which results we actually hold data for, rather than
                    hiding them and springing a modal on the user. */}
                {!r.in_curated_set && (
                  <span
                    className="shrink-0 rounded-full px-2 py-0.5 text-[0.6875rem] font-semibold"
                    style={{ background: "var(--hc-sunken)", color: "var(--hc-ink-3)" }}
                  >
                    Outside pilot
                  </span>
                )}
              </button>
            ))}
            {/* The other half of the borough problem, and the half a label alone cannot fix.
                Our pilot is one Brooklyn community district, so a match from our own rows is
                always Brooklyn — someone who typed a Manhattan address gets a real building
                back, correctly labelled, that is still not theirs. This is their way out.
                It is a button rather than the default because answering from our own rows
                takes milliseconds and asking the city geocoder takes seconds. */}
            {results.every((r) => r.in_curated_set) && !citywide && (
              <button
                type="button"
                onClick={() => setCitywide(true)}
                disabled={searching}
                className="w-full px-5 py-3 text-left text-[0.8125rem] font-semibold"
                style={{ color: "var(--hc-ink-2)", borderTop: "1px solid var(--hc-hairline, rgba(0,0,0,0.08))" }}
              >
                {searching ? "Searching all five boroughs…" : "Not this one? Search all five boroughs"}
              </button>
            )}
          </div>
        )}

        {notFound && (
          <p className="mt-3 px-2 text-[0.875rem]" style={{ color: "var(--hc-canvas-ink-2)" }}>
            Address not found — try street + house number.
          </p>
        )}
        </div>

        {/* Browsing beats guessing. Without this the only way to reach the
            covered set was to search something outside it and read the modal,
            or to go hunting in About. */}
        <button
          onClick={() => navigate("/more", { state: { openList: true } })}
          className="mt-5 flex items-center gap-2 rounded-full px-5 py-3 text-[0.9375rem] font-semibold"
          style={{ background: "var(--hc-ink)", color: "#1C1C1E" }}
        >
          <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round" aria-hidden>
            <path d="M4 6h16M4 12h16M4 18h10" />
          </svg>
          Browse all {COVERAGE_POINTS.length} covered buildings
        </button>
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
          <CoverageMap />
          <button
            onClick={() => {
              setCoverage(null);
              navigate("/more", { state: { openList: true } });
            }}
            // Was text-white on --hc-ink. The theme inversion turned --hc-ink
            // near-white (#F5F5F7), so the label vanished into the button.
            className="mt-5 w-full rounded-full py-4 text-[1rem] font-semibold"
            style={{ background: "var(--hc-ink)", color: "#1C1C1E" }}
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
