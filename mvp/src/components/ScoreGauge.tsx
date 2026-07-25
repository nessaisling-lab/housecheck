"use client";

import { useEffect, useState } from "react";

type Props = {
  score: number;
  label: string;
};

export function ScoreGauge({ score, label }: Props) {
  const [shown, setShown] = useState(0);
  const radius = 54;
  const circumference = 2 * Math.PI * radius;
  const progress = (shown / 100) * circumference;

  useEffect(() => {
    const start = performance.now();
    const duration = 900;
    let frame: number;

    const tick = (now: number) => {
      const t = Math.min(1, (now - start) / duration);
      const eased = 1 - Math.pow(1 - t, 3);
      setShown(Math.round(score * eased));
      if (t < 1) frame = requestAnimationFrame(tick);
    };

    frame = requestAnimationFrame(tick);
    return () => cancelAnimationFrame(frame);
  }, [score]);

  const tone =
    score >= 85
      ? "var(--teal)"
      : score >= 65
        ? "var(--amber)"
        : score >= 40
          ? "var(--alert)"
          : "var(--alert)";

  return (
    <div className="relative h-[140px] w-[140px] shrink-0">
      <svg viewBox="0 0 128 128" className="h-full w-full -rotate-90">
        <circle
          cx="64"
          cy="64"
          r={radius}
          fill="none"
          stroke="var(--line)"
          strokeWidth="10"
        />
        <circle
          cx="64"
          cy="64"
          r={radius}
          fill="none"
          stroke={tone}
          strokeWidth="10"
          strokeLinecap="round"
          strokeDasharray={`${progress} ${circumference}`}
          className="transition-[stroke-dasharray] duration-75"
        />
      </svg>
      <div className="absolute inset-0 flex rotate-0 flex-col items-center justify-center">
        <span className="font-[family-name:var(--font-display)] text-4xl font-semibold tabular-nums leading-none text-[var(--ink)]">
          {shown}
        </span>
        <span className="mt-1 text-[0.65rem] font-semibold uppercase tracking-[0.14em] text-[var(--ink-muted)]">
          {label}
        </span>
      </div>
    </div>
  );
}
