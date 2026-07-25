import type { ReactNode } from "react";
import { StatusPill } from "@/components/StatusPill";
import { SourceLine } from "@/components/SourceLine";

export interface SectionRow {
  label: string;
  value: ReactNode;
  /** right-aligned secondary value rendered above `value` (e.g. "$2,900" over "21% above") */
  hint?: ReactNode;
}

interface SectionCardProps {
  id?: string;
  icon: ReactNode;
  iconTint: string;
  title: string;
  pill?: { text: string; color: string; trend?: "up" | "down" | null };
  /** subtle marker shown beside the title (e.g. "Your priority") */
  badge?: ReactNode;
  /** flex order — used to float priority sections to the top (reorder only) */
  order?: number;
  rows?: SectionRow[];
  children?: ReactNode;
  sentence?: ReactNode;
  footnote?: ReactNode;
  source: { agency: string; date?: string; href?: string };
  onOpenDetail?: () => void;
}

/**
 * Section card — identical template for all four widgets (design-strategy §5):
 * icon chip + title + status pill → 2–4 label:value rows → one plain sentence →
 * divider → source line. Tap the header to open the section detail sheet.
 */
export function SectionCard({
  id,
  icon,
  iconTint,
  title,
  pill,
  badge,
  order,
  rows,
  children,
  sentence,
  footnote,
  source,
  onOpenDetail,
}: SectionCardProps) {
  return (
    <section
      id={id}
      className="hc-card scroll-mt-20 p-4"
      aria-label={title}
      style={order != null ? { order } : undefined}
    >
      <button
        className="flex w-full items-center gap-3 text-left"
        onClick={onOpenDetail}
        aria-label={`${title} details`}
      >
        <span
          className="flex h-11 w-11 shrink-0 items-center justify-center rounded-2xl"
          style={{ background: `${iconTint}1F`, color: iconTint }}
        >
          {icon}
        </span>
        <span className="flex-1 text-[20px]" style={{ color: "var(--hc-ink)" }}>
          {title}
          {badge}
        </span>
        {pill && <StatusPill text={pill.text} color={pill.color} trend={pill.trend ?? null} />}
      </button>

      {rows && rows.length > 0 && (
        <dl className="mt-4 space-y-2.5">
          {rows.map((r) => (
            <div key={r.label} className="flex items-baseline justify-between gap-4">
              <dt className="hc-row-label">{r.label}</dt>
              <dd className="text-right text-[15px] font-medium tabular-nums" style={{ color: "var(--hc-ink)" }}>
                {r.hint != null && (
                  <span className="mr-2 text-[12px] font-normal" style={{ color: "var(--hc-ink-3)" }}>
                    {r.hint}
                  </span>
                )}
                {r.value}
              </dd>
            </div>
          ))}
        </dl>
      )}

      {children}

      {sentence && (
        <p className="mt-4 text-[17px] leading-snug" style={{ color: "var(--hc-ink-2)" }}>
          {sentence}
        </p>
      )}
      {footnote && <div className="mt-3">{footnote}</div>}

      <SourceLine agency={source.agency} date={source.date} href={source.href} />
    </section>
  );
}
