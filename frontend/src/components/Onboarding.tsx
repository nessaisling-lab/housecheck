import { useState } from "react";
import { Sheet } from "@/components/Sheet";
import { MAX_PRIORITIES, store, useOnboarding, type Priority } from "@/lib/store";

const OPTIONS: { id: Priority; label: string; hint: string; icon: React.ReactNode }[] = [
  {
    id: "rent",
    label: "Rent fairness",
    hint: "Is the asking rent reasonable here?",
    icon: (
      <svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.8" strokeLinecap="round">
        <rect x="3" y="6" width="18" height="13" rx="2.5" />
        <circle cx="12" cy="12.5" r="3" />
      </svg>
    ),
  },
  {
    id: "condition",
    label: "Building condition",
    hint: "Violations, repairs, 311 history",
    icon: (
      <svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.8" strokeLinecap="round" strokeLinejoin="round">
        <path d="M14.5 6.5a4 4 0 015.2-3.8l-2.9 2.9.9 2.8 2.8.9 2.9-2.9a4 4 0 01-5.4 5.6L8.5 21.5a2.1 2.1 0 01-3-3L15 9a4 4 0 01-.5-2.5z" />
      </svg>
    ),
  },
  {
    id: "legal",
    label: "Legal protection",
    hint: "Stabilization and Good Cause",
    icon: (
      <svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.8" strokeLinejoin="round">
        <path d="M12 3l8 3v6c0 5-3.5 8-8 9-4.5-1-8-4-8-9V6l8-3z" />
      </svg>
    ),
  },
  {
    id: "access",
    label: "Accessibility",
    hint: "Elevator, step-free entry, ADA subway",
    icon: (
      <svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.8" strokeLinecap="round">
        <circle cx="12" cy="5" r="2" />
        <path d="M5 10h14M12 10v4l-3 7M12 14l3 7" />
      </svg>
    ),
  },
  {
    id: "neighborhood",
    label: "Neighborhood",
    hint: "Local rents and area context",
    icon: (
      <svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.8" strokeLinecap="round" strokeLinejoin="round">
        <path d="M3 21h18M5 21V8l4-3 4 3v13M13 21V11l4-2 4 2v10" />
      </svg>
    ),
  },
];

/**
 * First-launch onboarding (P1) — one question, pick up to 2 or Skip.
 * Fully skippable; persists to the localStorage store; never blocks search.
 */
export function Onboarding() {
  const ob = useOnboarding();
  const [selected, setSelected] = useState<Priority[]>([]);

  if (ob.done) return null;

  const toggle = (id: Priority) => {
    setSelected((s) =>
      s.includes(id) ? s.filter((x) => x !== id) : s.length < MAX_PRIORITIES ? [...s, id] : s
    );
  };

  const skip = () => store.completeOnboarding(null);
  const confirm = () => {
    if (selected.length > 0) store.completeOnboarding(selected);
  };

  return (
    <Sheet open onClose={skip} labelledBy="onboarding-title">
      <div className="px-6 pb-8 pt-2">
        <p className="hc-eyebrow">One question · optional</p>
        <h2
          id="onboarding-title"
          className="mt-2 text-[26px] font-semibold tracking-tight"
          style={{ color: "var(--hc-ink)" }}
        >
          What matters most to you?
        </h2>
        <p className="mt-1.5 text-[14px] leading-snug" style={{ color: "var(--hc-ink-2)" }}>
          Pick up to {MAX_PRIORITIES} — those sections move to the top of every Building Health
          Card. Nothing is ever hidden.
        </p>

        <div className="mt-5 space-y-2" role="group" aria-label="Priorities">
          {OPTIONS.map((o) => {
            const active = selected.includes(o.id);
            return (
              <button
                key={o.id}
                onClick={() => toggle(o.id)}
                aria-pressed={active}
                className={`flex w-full items-center gap-3 rounded-2xl px-4 py-3 text-left transition-all ${
                  active ? "glass-nav" : ""
                }`}
                style={
                  active
                    ? { outline: "1.5px solid var(--hc-ink)", outlineOffset: -1 }
                    : { background: "rgba(255,255,255,0.55)" }
                }
              >
                <span style={{ color: active ? "var(--hc-ink)" : "var(--hc-ink-3)" }}>{o.icon}</span>
                <span className="flex-1">
                  <span className="block text-[16px] font-semibold" style={{ color: "var(--hc-ink)" }}>
                    {o.label}
                  </span>
                  <span className="block text-[12px]" style={{ color: "var(--hc-ink-2)" }}>
                    {o.hint}
                  </span>
                </span>
                <span
                  className="flex h-5 w-5 items-center justify-center rounded-full text-[11px] font-bold"
                  style={
                    active
                      ? { background: "var(--hc-ink)", color: "#fff" }
                      : { border: "1.5px solid rgba(60,60,67,0.25)", color: "transparent" }
                  }
                  aria-hidden
                >
                  ✓
                </span>
              </button>
            );
          })}
        </div>

        <button
          onClick={confirm}
          disabled={selected.length === 0}
          className="glass-dark mt-6 w-full rounded-full py-4 text-[16px] font-semibold text-white disabled:opacity-40"
        >
          Continue{selected.length > 0 ? ` (${selected.length})` : ""}
        </button>
        <button
          onClick={skip}
          className="mt-3 w-full text-center text-[15px] font-medium"
          style={{ color: "var(--hc-ink-2)" }}
        >
          Skip for now
        </button>
      </div>
    </Sheet>
  );
}
