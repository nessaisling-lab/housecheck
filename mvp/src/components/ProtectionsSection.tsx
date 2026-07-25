import type { BuildingRecord } from "@/data/buildings";
import { SOURCE_LINKS } from "@/data/buildings";
import { SourceLink } from "./SourceLink";
import { Section } from "./Section";

type Props = {
  building: BuildingRecord;
};

export function ProtectionsSection({ building }: Props) {
  const stab =
    building.rentStabilized === true
      ? {
          title: "Likely rent-stabilized",
          body: "Public indicators suggest rent stabilization may apply. Confirm unit status before signing — landlords rarely volunteer this.",
          tone: "good" as const,
        }
      : building.rentStabilized === false
        ? {
            title: "Not indicated as rent-stabilized",
            body: "No stabilization signal in demo records. You may still have other protections — check Good Cause below.",
            tone: "neutral" as const,
          }
        : {
            title: "Stabilization status unclear",
            body: "DHCR coverage isn’t conclusive for this building in our demo data. Ask for the apartment’s rent history.",
            tone: "caution" as const,
          };

  return (
    <Section
      eyebrow="02 · Legal protections"
      title="Rights the listing won’t mention"
      intro="Stabilization and Good Cause can be worth thousands. Surface them before you commit."
    >
      <div className="space-y-4">
        <ProtectionRow
          title={stab.title}
          body={stab.body}
          tone={stab.tone}
          source={SOURCE_LINKS.dhcr}
        />
        <ProtectionRow
          title={
            building.goodCauseLikely
              ? "Likely Good Cause–covered"
              : "Good Cause may not apply"
          }
          body={
            building.goodCauseLikely
              ? "Based on unit count and ownership patterns typical for this stock, Good Cause eviction limits may apply."
              : "Newer or exempt stock may fall outside Good Cause. Verify with counsel or tenant hotline."
          }
          tone={building.goodCauseLikely ? "good" : "neutral"}
          source={SOURCE_LINKS.goodCause}
        />
        <ProtectionRow
          title={
            building.hasElevator === true
              ? "Elevator on record"
              : building.hasElevator === false
                ? "No elevator indicated"
                : "Accessibility unknown"
          }
          body={
            building.hasElevator === true
              ? `${building.stories}-story building with elevator — stronger signal for step-free access (still verify lobby and unit).`
              : building.hasElevator === false
                ? `${building.stories}-story walk-up indicated. Listings often omit this.`
                : "Elevator status not in demo records."
          }
          tone={building.hasElevator === true ? "good" : "neutral"}
          source={SOURCE_LINKS.hpd}
        />
      </div>
    </Section>
  );
}

function ProtectionRow({
  title,
  body,
  tone,
  source,
}: {
  title: string;
  body: string;
  tone: "good" | "caution" | "neutral";
  source: { label: string; url: string };
}) {
  const dot =
    tone === "good"
      ? "bg-[var(--teal)]"
      : tone === "caution"
        ? "bg-[var(--amber)]"
        : "bg-[var(--ink-faint)]";

  return (
    <div className="flex gap-3">
      <span className={`mt-1.5 h-2.5 w-2.5 shrink-0 rounded-full ${dot}`} />
      <div>
        <p className="font-medium text-[var(--ink)]">{title}</p>
        <p className="mt-1 text-sm leading-relaxed text-[var(--ink-muted)]">
          {body}
        </p>
        <p className="mt-1.5 text-xs text-[var(--ink-faint)]">
          Source: <SourceLink href={source.url}>{source.label}</SourceLink>
        </p>
      </div>
    </div>
  );
}
