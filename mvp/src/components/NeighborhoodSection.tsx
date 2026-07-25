import type { ReactNode } from "react";
import type { BuildingRecord } from "@/data/buildings";
import { SourceLink } from "./SourceLink";
import { Section } from "./Section";

type Props = {
  building: BuildingRecord;
};

export function NeighborhoodSection({ building }: Props) {
  return (
    <Section
      eyebrow="03 · Neighborhood context"
      title="What you’re comparing against"
      intro="Median rent is the negotiation baseline. Pair it with building condition — cheap in a Class C building isn’t a win."
    >
      <dl className="grid grid-cols-2 gap-x-4 gap-y-5">
        <Item label="Neighborhood" value={building.neighborhood} />
        <Item label="Units in building" value={String(building.units)} />
        <Item label="Year built" value={String(building.yearBuilt)} />
        <Item label="Stories" value={String(building.stories)} />
        <Item
          label="Median gross rent"
          value={`$${building.neighborhoodMedianRent.toLocaleString()}`}
          hint={
            <>
              {building.medianRentSource.label}.{" "}
              <SourceLink href={building.medianRentSource.url}>
                Census ACS
              </SourceLink>
              {" · "}
              data from {building.medianRentSource.asOf}
            </>
          }
        />
      </dl>
    </Section>
  );
}

function Item({
  label,
  value,
  hint,
}: {
  label: string;
  value: string;
  hint?: ReactNode;
}) {
  return (
    <div className={hint ? "col-span-2" : undefined}>
      <dt className="text-[0.65rem] uppercase tracking-wider text-[var(--ink-faint)]">
        {label}
      </dt>
      <dd className="mt-1 font-[family-name:var(--font-display)] text-xl font-semibold text-[var(--ink)]">
        {value}
      </dd>
      {hint ? (
        <p className="mt-1 text-xs leading-relaxed text-[var(--ink-faint)]">
          {hint}
        </p>
      ) : null}
    </div>
  );
}
