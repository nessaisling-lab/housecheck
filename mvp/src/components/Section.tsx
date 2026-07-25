import type { ReactNode } from "react";

type Props = {
  eyebrow: string;
  title: string;
  intro: string;
  children: ReactNode;
};

export function Section({ eyebrow, title, intro, children }: Props) {
  return (
    <section>
      <p className="text-[0.7rem] font-semibold uppercase tracking-[0.16em] text-[var(--teal)]">
        {eyebrow}
      </p>
      <h2 className="mt-1.5 font-[family-name:var(--font-display)] text-xl tracking-tight text-[var(--ink)] sm:text-2xl">
        {title}
      </h2>
      <p className="mt-2 max-w-prose text-sm leading-relaxed text-[var(--ink-muted)]">
        {intro}
      </p>
      <div className="mt-5">{children}</div>
    </section>
  );
}
