"use client";

import { FormEvent, useMemo, useState } from "react";
import { findBuilding } from "@/lib/search";
import { EXAMPLE_ADDRESSES } from "@/lib/search";
import type { BuildingRecord } from "@/data/buildings";
import { BuildingHealthCard } from "./BuildingHealthCard";
import { CompareAgent } from "./CompareAgent";

type Mode = "home" | "card" | "compare";

export function HouseCheckApp() {
  const [mode, setMode] = useState<Mode>("home");
  const [query, setQuery] = useState("");
  const [building, setBuilding] = useState<BuildingRecord | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [submitted, setSubmitted] = useState(false);

  const suggestions = useMemo(() => EXAMPLE_ADDRESSES, []);

  function lookup(raw: string) {
    const trimmed = raw.trim();
    if (!trimmed) {
      setError("Enter a Brooklyn address to check.");
      setBuilding(null);
      setSubmitted(true);
      return;
    }

    const match = findBuilding(trimmed);
    setSubmitted(true);
    if (!match) {
      setBuilding(null);
      setError(
        "No match in the demo set. Try one of the sample Brooklyn addresses below.",
      );
      return;
    }

    setError(null);
    setBuilding(match);
    setQuery(match.address);
    setMode("card");
  }

  function onSubmit(e: FormEvent) {
    e.preventDefault();
    lookup(query);
  }

  function goHome() {
    setMode("home");
    setBuilding(null);
    setError(null);
    setSubmitted(false);
    setQuery("");
  }

  if (mode === "compare") {
    return (
      <CompareAgent
        onBack={goHome}
        onOpenBuilding={(b) => {
          setBuilding(b);
          setMode("card");
        }}
      />
    );
  }

  if (mode === "card" && building) {
    return (
      <BuildingHealthCard
        building={building}
        onBack={goHome}
      />
    );
  }

  return (
    <div className="relative flex min-h-full flex-1 flex-col">
      <div className="pointer-events-none absolute inset-0 overflow-hidden" aria-hidden>
        <div className="hero-wash absolute inset-0" />
        <div className="hero-grid absolute inset-0 opacity-[0.35]" />
        <div className="absolute -right-16 top-24 h-72 w-72 rounded-full bg-[var(--teal)]/10 blur-3xl animate-drift" />
        <div className="absolute -left-20 bottom-10 h-64 w-64 rounded-full bg-[var(--alert)]/8 blur-3xl animate-drift-slow" />
      </div>

      <main className="relative z-10 mx-auto flex w-full max-w-lg flex-1 flex-col justify-center px-4 pb-10 pt-14 sm:pt-20">
        <p className="animate-fade-up font-[family-name:var(--font-display)] text-[clamp(2.75rem,12vw,4.25rem)] font-semibold leading-[0.92] tracking-[-0.03em] text-[var(--ink)]">
          HouseCheck
        </p>
        <h1 className="animate-fade-up mt-5 max-w-[18ch] text-[1.35rem] font-medium leading-snug text-[var(--ink)] [animation-delay:80ms] sm:text-2xl">
          Know the building before you sign the lease.
        </h1>
        <p className="animate-fade-up mt-3 max-w-[34ch] text-[0.95rem] leading-relaxed text-[var(--ink-muted)] [animation-delay:140ms]">
          Instant Building Health Card from HPD, DHCR, and Census data — condition,
          protections, and rent fairness in under a minute.
        </p>

        <form
          onSubmit={onSubmit}
          className="animate-fade-up mt-8 [animation-delay:220ms]"
        >
          <label htmlFor="address" className="sr-only">
            Brooklyn address
          </label>
          <div className="flex flex-col gap-2 sm:flex-row sm:items-stretch">
            <input
              id="address"
              value={query}
              onChange={(e) => {
                setQuery(e.target.value);
                setError(null);
              }}
              placeholder="Type a Brooklyn address"
              autoComplete="street-address"
              className="w-full rounded-xl border border-[var(--line-strong)] bg-white/90 px-4 py-3.5 text-base text-[var(--ink)] shadow-[0_8px_30px_rgba(12,26,36,0.06)] outline-none backdrop-blur placeholder:text-[var(--ink-faint)] focus:border-[var(--teal)] focus:ring-2 focus:ring-[var(--teal)]/20"
            />
            <button
              type="submit"
              className="rounded-xl bg-[var(--ink)] px-5 py-3.5 text-sm font-semibold tracking-wide text-[var(--paper)] transition hover:bg-[var(--ink-soft)] active:scale-[0.98] sm:shrink-0"
            >
              Check building
            </button>
          </div>
          {submitted && error ? (
            <p className="mt-3 text-sm text-[var(--alert)]" role="alert">
              {error}
            </p>
          ) : (
            <p className="mt-3 text-xs text-[var(--ink-faint)]">
              Brooklyn demo · every number links to its government source
            </p>
          )}
        </form>

        <button
          type="button"
          onClick={() => setMode("compare")}
          className="animate-fade-up mt-5 w-full rounded-xl border border-[var(--line-strong)] bg-white/70 px-4 py-3.5 text-left transition hover:border-[var(--teal)] hover:bg-white/95 [animation-delay:260ms]"
        >
          <span className="block text-sm font-semibold text-[var(--ink)]">
            Comparing a few places?
          </span>
          <span className="mt-0.5 block text-xs leading-relaxed text-[var(--ink-muted)]">
            Tell the compare agent what matters — get pros, cons, and a ranked shortlist.
          </span>
        </button>

        <div className="animate-fade-up mt-10 [animation-delay:320ms]">
          <p className="text-[0.65rem] font-semibold uppercase tracking-[0.16em] text-[var(--ink-faint)]">
            Try a sample
          </p>
          <ul className="mt-3 space-y-2">
            {suggestions.map((s) => (
              <li key={s.id}>
                <button
                  type="button"
                  onClick={() => lookup(s.label)}
                  className="group flex w-full items-baseline justify-between gap-3 border-b border-[var(--line)] py-2.5 text-left transition hover:border-[var(--teal)]"
                >
                  <span className="text-sm font-medium text-[var(--ink)] group-hover:text-[var(--teal)]">
                    {s.label}
                  </span>
                  <span className="shrink-0 text-xs text-[var(--ink-faint)]">
                    {s.neighborhood}
                  </span>
                </button>
              </li>
            ))}
          </ul>
        </div>
      </main>
    </div>
  );
}
