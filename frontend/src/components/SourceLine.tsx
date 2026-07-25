interface SourceLineProps {
  agency: string;
  date?: string;
  href?: string;
}

/**
 * "Source: NYC HPD · Jul 2026 ↗" — a component, not text (design-strategy §5).
 * Every number links to its public source.
 */
export function SourceLine({ agency, date, href }: SourceLineProps) {
  const body = (
    <>
      <span>
        Source: {agency}
        {date ? ` · ${date}` : ""}
      </span>
      {href && (
        <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" aria-hidden>
          <path d="M7 17L17 7M9 7h8v8" strokeLinecap="round" strokeLinejoin="round" />
        </svg>
      )}
    </>
  );
  const cls =
    "mt-3 flex items-center justify-between border-t pt-2.5 text-[12px]";
  const style = { color: "var(--hc-ink-3)", borderColor: "rgba(60,60,67,0.12)" };
  if (href) {
    return (
      <a href={href} target="_blank" rel="noreferrer" className={`${cls} hover:underline`} style={style}>
        {body}
      </a>
    );
  }
  return (
    <div className={cls} style={style}>
      {body}
    </div>
  );
}
