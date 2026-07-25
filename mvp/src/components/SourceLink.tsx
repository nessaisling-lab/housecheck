import type { ReactNode } from "react";

type Props = {
  href: string;
  children: ReactNode;
  compact?: boolean;
};

export function SourceLink({ href, children, compact }: Props) {
  return (
    <a
      href={href}
      target="_blank"
      rel="noopener noreferrer"
      className={
        compact
          ? "text-[0.65rem] font-semibold uppercase tracking-wide text-[var(--teal)] underline-offset-2 hover:underline"
          : "font-medium text-[var(--teal)] underline-offset-2 hover:underline"
      }
    >
      {children}
    </a>
  );
}
