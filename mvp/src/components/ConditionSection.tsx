import type { BuildingRecord } from "@/data/buildings";
import { SOURCE_LINKS } from "@/data/buildings";
import type { ScoreBreakdown } from "@/lib/score";
import { SourceLink } from "./SourceLink";
import { Section } from "./Section";

type Props = {
  building: BuildingRecord;
  health: ScoreBreakdown;
};

export function ConditionSection({ building, health }: Props) {
  const open = building.violations.filter((v) => v.status === "Open");
  const closed = building.violations.filter((v) => v.status === "Closed");

  return (
    <Section
      eyebrow="01 · Building condition"
      title="Violation & safety history"
      intro="Class C means immediately hazardous. Open Class C is the walk-away signal most renters never see in time."
    >
      <ul className="space-y-3">
        {building.violations.length === 0 ? (
          <li className="text-sm text-[var(--ink-muted)]">
            No violations in the demo dataset for this building.
          </li>
        ) : (
          [...open, ...closed].map((v) => (
            <li
              key={v.id}
              className="grid grid-cols-[auto_1fr_auto] items-start gap-3 border-b border-[var(--line)] pb-3 last:border-0"
            >
              <span
                className={`mt-0.5 inline-flex h-6 min-w-6 items-center justify-center rounded-sm px-1.5 font-[family-name:var(--font-display)] text-xs font-bold ${
                  v.class === "C"
                    ? "bg-[var(--alert-soft)] text-[var(--alert)]"
                    : v.class === "B"
                      ? "bg-[var(--amber-soft)] text-[var(--amber)]"
                      : "bg-[var(--mist)] text-[var(--ink-muted)]"
                }`}
              >
                {v.class}
              </span>
              <div>
                <p className="text-sm font-medium text-[var(--ink)]">
                  {v.description}
                </p>
                <p className="mt-0.5 text-xs text-[var(--ink-faint)]">
                  {v.status} · issued {v.issued}
                </p>
              </div>
              <SourceLink href={v.sourceUrl} compact>
                HPD
              </SourceLink>
            </li>
          ))
        )}
      </ul>

      <p className="mt-4 text-xs text-[var(--ink-faint)]">
        Score basis: {health.openHazardous} open Class C, {health.totalOpen}{" "}
        open total. Source:{" "}
        <SourceLink href={SOURCE_LINKS.hpd.url}>
          {SOURCE_LINKS.hpd.label}
        </SourceLink>
        .
      </p>
    </Section>
  );
}
