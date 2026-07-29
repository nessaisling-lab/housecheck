import { useEffect, useRef, useState, type RefObject } from "react";
import { MiniRing } from "@/components/ScoreRing";
import type { SubScores } from "@/types/building";

interface StickyStripProps {
  /** ref to the hero block; strip appears when it scrolls out of view */
  watchRef: RefObject<HTMLElement | null>;
  subScores: SubScores;
  onJump: (sectionId: string) => void;
}

const items: { key: keyof SubScores; target: string; label: string }[] = [
  { key: "condition", target: "section-condition", label: "Condition" },
  { key: "legal", target: "section-legal", label: "Legal" },
  { key: "neighborhood", target: "section-rent", label: "Rent" },
  { key: "accessibility", target: "section-access", label: "Access" },
];

/**
 * Collapsed sticky header (Whoop sticky collapse, autopsy #3):
 * when the hero scrolls away, a slim glass strip pins to top with
 * 4 mini-rings + sub-score numbers. Tap → jump to that section.
 */
export function StickyStrip({ watchRef, subScores, onJump }: StickyStripProps) {
  const [visible, setVisible] = useState(false);
  const visibleRef = useRef(false);

  useEffect(() => {
    const el = watchRef.current;
    if (!el) return;
    const obs = new IntersectionObserver(
      ([entry]) => {
        const next = !entry.isIntersecting && entry.boundingClientRect.top < 0;
        if (next !== visibleRef.current) {
          visibleRef.current = next;
          setVisible(next);
        }
      },
      { threshold: 0 }
    );
    obs.observe(el);
    return () => obs.disconnect();
  }, [watchRef]);

  return (
    <div
      className="hc-anim pointer-events-none fixed inset-x-0 top-0 z-30 flex justify-center transition-all duration-300"
      style={{
        opacity: visible ? 1 : 0,
        transform: visible ? "translateY(0)" : "translateY(-110%)",
      }}
      aria-hidden={!visible}
    >
      <div className="glass-nav pointer-events-auto mt-2 flex items-center gap-4 rounded-full px-5 py-2">
        {items.map((it) => (
          <button
            key={it.key}
            onClick={() => onJump(it.target)}
            className="flex items-center gap-1.5"
            aria-label={`Jump to ${it.label}`}
            tabIndex={visible ? 0 : -1}
          >
            <MiniRing score={subScores[it.key]} size={20} stroke={3} />
            <span className="text-[0.8125rem] font-semibold tabular-nums" style={{ color: "var(--hc-ink)" }}>
              {subScores[it.key] ?? "—"}
            </span>
          </button>
        ))}
      </div>
    </div>
  );
}
