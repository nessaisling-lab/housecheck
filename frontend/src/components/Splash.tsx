import { useEffect, useState } from "react";

/**
 * Splash — ring draws to 100% over the wordmark, plays < 1s (wireframe 01).
 * Honored: prefers-reduced-motion via .hc-anim reset in index.css.
 */
export function Splash({ onDone }: { onDone: () => void }) {
  const [fading, setFading] = useState(false);

  useEffect(() => {
    const t1 = setTimeout(() => setFading(true), 900);
    const t2 = setTimeout(onDone, 1200);
    return () => {
      clearTimeout(t1);
      clearTimeout(t2);
    };
  }, [onDone]);

  const size = 132;
  const stroke = 9;
  const r = (size - stroke) / 2;
  const c = 2 * Math.PI * r;

  return (
    <div
      className="hc-anim fixed inset-0 z-[60] flex flex-col items-center justify-center transition-opacity duration-300"
      style={{ background: "var(--hc-canvas)", opacity: fading ? 0 : 1 }}
      aria-hidden
    >
      <div className="relative" style={{ width: size, height: size }}>
        <svg width={size} height={size} className="-rotate-90">
          <circle cx={size / 2} cy={size / 2} r={r} fill="none" stroke="rgba(255,255,255,0.16)" strokeWidth={stroke} />
          <circle
            cx={size / 2}
            cy={size / 2}
            r={r}
            fill="none"
            stroke="rgba(255,255,255,0.94)"
            strokeWidth={stroke}
            strokeLinecap="round"
            strokeDasharray={c}
            style={{
              ["--ring-c" as string]: c,
              ["--ring-o" as string]: 0,
              strokeDashoffset: c,
              animation: "hc-ring-draw 0.8s cubic-bezier(0.22,1,0.36,1) forwards",
            }}
          />
        </svg>
        <svg
          className="absolute inset-0 m-auto"
          width="40"
          height="40"
          viewBox="0 0 24 24"
          fill="none"
          stroke="rgba(255,255,255,0.94)"
          strokeWidth="1.8"
          strokeLinejoin="round"
        >
          <path d="M3 11l9-7 9 7" />
          <path d="M6 9.5V20h12V9.5" />
          <path d="M9.5 14.5l2.5 2.5 4.5-4.5" strokeLinecap="round" />
        </svg>
      </div>
      <h1 className="mt-5 text-[1.75rem] font-semibold tracking-tight" style={{ color: "var(--hc-canvas-ink)" }}>
        HouseCheck
      </h1>
      <p className="absolute bottom-16 text-[0.9375rem]" style={{ color: "var(--hc-canvas-ink-2)" }}>
        Public data. Honest signals.
      </p>
    </div>
  );
}
