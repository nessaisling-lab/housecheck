import type { BuildingRecord } from "@/data/buildings";
import { computeHealthScore } from "@/lib/score";
import { ScoreGauge } from "./ScoreGauge";
import { ConditionSection } from "./ConditionSection";
import { ProtectionsSection } from "./ProtectionsSection";
import { NeighborhoodSection } from "./NeighborhoodSection";
import { RentFairnessPanel } from "./RentFairnessPanel";
import { SourceLink } from "./SourceLink";

type Props = {
  building: BuildingRecord;
  onBack: () => void;
};

export function BuildingHealthCard({ building, onBack }: Props) {
  const health = computeHealthScore(building);

  return (
    <article className="animate-rise mx-auto w-full max-w-lg px-4 pb-16 pt-6">
      <button
        type="button"
        onClick={onBack}
        className="mb-6 text-sm font-medium text-[var(--ink-muted)] transition hover:text-[var(--ink)]"
      >
        ← New address
      </button>

      <header className="mb-8">
        <p className="font-[family-name:var(--font-display)] text-xs font-semibold uppercase tracking-[0.18em] text-[var(--teal)]">
          Building Health Card
        </p>
        <h1 className="mt-2 font-[family-name:var(--font-display)] text-[1.85rem] leading-[1.1] tracking-tight text-[var(--ink)] sm:text-[2.15rem]">
          {building.address}
        </h1>
        <p className="mt-2 text-[var(--ink-muted)]">
          {building.neighborhood}, Brooklyn {building.zip}
        </p>
        <p className="mt-1 text-xs text-[var(--ink-faint)]">
          Data from {formatDate(building.dataAsOf)} · BBL {building.bbl}
        </p>
      </header>

      <div className="mb-10 flex flex-col items-center gap-5 sm:flex-row sm:items-start sm:gap-8">
        <ScoreGauge score={health.score} label={health.label} />
        <div className="flex-1 text-center sm:pt-3 sm:text-left">
          <p className="text-lg font-medium leading-snug text-[var(--ink)]">
            {health.summary}
          </p>
          <dl className="mt-4 grid grid-cols-3 gap-2 text-center sm:text-left">
            <Stat
              value={health.openHazardous}
              label="Open Class C"
              tone={health.openHazardous > 0 ? "bad" : "ok"}
            />
            <Stat value={health.totalOpen} label="Open total" />
            <Stat
              value={health.closedHazardousLast2Y}
              label="Closed C · 2y"
            />
          </dl>
        </div>
      </div>

      <div className="space-y-10">
        <ConditionSection building={building} health={health} />
        <ProtectionsSection building={building} />
        <NeighborhoodSection building={building} />
        <RentFairnessPanel building={building} />
      </div>

      <footer className="mt-12 border-t border-[var(--line)] pt-6 text-xs leading-relaxed text-[var(--ink-faint)]">
        HouseCheck summarizes public records for research only — not legal advice.
        Always confirm with{" "}
        <SourceLink href={building.hpdProfileUrl}>HPD Online</SourceLink> and
        your lease before signing.
      </footer>
    </article>
  );
}

function Stat({
  value,
  label,
  tone = "neutral",
}: {
  value: number;
  label: string;
  tone?: "neutral" | "bad" | "ok";
}) {
  const color =
    tone === "bad"
      ? "text-[var(--alert)]"
      : tone === "ok"
        ? "text-[var(--teal)]"
        : "text-[var(--ink)]";

  return (
    <div>
      <dt className="text-[0.65rem] uppercase tracking-wider text-[var(--ink-faint)]">
        {label}
      </dt>
      <dd className={`font-[family-name:var(--font-display)] text-xl font-semibold ${color}`}>
        {value}
      </dd>
    </div>
  );
}

function formatDate(iso: string): string {
  return new Date(iso + "T12:00:00").toLocaleDateString("en-US", {
    month: "short",
    day: "numeric",
    year: "numeric",
  });
}
