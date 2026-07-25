"use client";

import { useMemo, useState } from "react";
import type { BuildingRecord } from "@/data/buildings";
import { rentFairness } from "@/lib/score";
import { SourceLink } from "./SourceLink";
import { Section } from "./Section";

type Props = {
  building: BuildingRecord;
};

export function RentFairnessPanel({ building }: Props) {
  const [rent, setRent] = useState("");
  const quoted = Number(rent.replace(/[^0-9.]/g, ""));
  const result = useMemo(() => {
    if (!quoted || quoted < 100) return null;
    return rentFairness(quoted, building.neighborhoodMedianRent);
  }, [quoted, building.neighborhoodMedianRent]);

  return (
    <Section
      eyebrow="04 · Rent fairness"
      title="Enter your rent. Get a verdict."
      intro="The steroid moment — turn a quoted price into a negotiation lever backed by neighborhood data."
    >
      <div className="rounded-xl border border-[var(--line)] bg-white/70 p-4 shadow-[0_1px_0_rgba(12,26,36,0.04)] backdrop-blur-sm">
        <label
          htmlFor="quoted-rent"
          className="text-xs font-semibold uppercase tracking-wider text-[var(--ink-muted)]"
        >
          Quoted monthly rent
        </label>
        <div className="mt-2 flex items-center gap-2">
          <span className="font-[family-name:var(--font-display)] text-2xl text-[var(--ink-muted)]">
            $
          </span>
          <input
            id="quoted-rent"
            inputMode="numeric"
            placeholder="e.g. 2800"
            value={rent}
            onChange={(e) => setRent(e.target.value)}
            className="w-full bg-transparent font-[family-name:var(--font-display)] text-3xl font-semibold tracking-tight text-[var(--ink)] outline-none placeholder:text-[var(--line-strong)]"
          />
        </div>

        <div
          className={`mt-4 overflow-hidden transition-all duration-500 ${
            result ? "max-h-40 opacity-100" : "max-h-0 opacity-0"
          }`}
        >
          {result ? (
            <div
              className={`rounded-lg px-3 py-3 ${
                result.verdict === "above"
                  ? "bg-[var(--alert-soft)]"
                  : result.verdict === "below"
                    ? "bg-[var(--teal-soft)]"
                    : "bg-[var(--mist)]"
              }`}
            >
              <p className="font-[family-name:var(--font-display)] text-2xl font-semibold text-[var(--ink)]">
                {result.pct > 0 ? "+" : ""}
                {result.pct}%{" "}
                <span className="text-base font-medium text-[var(--ink-muted)]">
                  vs median
                </span>
              </p>
              <p className="mt-1 text-sm text-[var(--ink-muted)]">
                {result.message}
              </p>
            </div>
          ) : null}
        </div>

        <p className="mt-3 text-xs text-[var(--ink-faint)]">
          Median ${building.neighborhoodMedianRent.toLocaleString()} ·{" "}
          <SourceLink href={building.medianRentSource.url}>
            {building.medianRentSource.label}
          </SourceLink>{" "}
          · data from {building.medianRentSource.asOf}
        </p>
      </div>
    </Section>
  );
}
