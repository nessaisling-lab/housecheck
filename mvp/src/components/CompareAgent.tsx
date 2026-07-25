"use client";

import { FormEvent, useEffect, useRef, useState } from "react";
import { BUILDINGS, type BuildingRecord } from "@/data/buildings";
import { compareOptions, type RankedOption } from "@/lib/compare";
import { PRIORITIES, type PriorityId, priorityById } from "@/lib/priorities";

type Step =
  | "welcome"
  | "needs"
  | "rank"
  | "budget"
  | "buildings"
  | "rents"
  | "result";

type ChatMessage = {
  id: string;
  role: "agent" | "user";
  text: string;
};

type Props = {
  onBack: () => void;
  onOpenBuilding: (building: BuildingRecord) => void;
};

let msgCounter = 0;
function mid() {
  msgCounter += 1;
  return `m-${msgCounter}`;
}

export function CompareAgent({ onBack, onOpenBuilding }: Props) {
  const [step, setStep] = useState<Step>("welcome");
  const [messages, setMessages] = useState<ChatMessage[]>([
    {
      id: mid(),
      role: "agent",
      text: "I’ll help you pick between a few Brooklyn rentals using HouseCheck’s public-data Building Health Cards. First — what matters most for this lease decision?",
    },
  ]);
  const [selectedNeeds, setSelectedNeeds] = useState<PriorityId[]>([]);
  const [rankedNeeds, setRankedNeeds] = useState<PriorityId[]>([]);
  const [maxRent, setMaxRent] = useState<string>("");
  const [pickedIds, setPickedIds] = useState<string[]>([]);
  const [rentDrafts, setRentDrafts] = useState<Record<string, string>>({});
  const [result, setResult] = useState<RankedOption[] | null>(null);
  const bottomRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    bottomRef.current?.scrollIntoView({ behavior: "smooth", block: "end" });
  }, [messages, step, result]);

  function push(role: "agent" | "user", text: string) {
    setMessages((prev) => [...prev, { id: mid(), role, text }]);
  }

  function toggleNeed(id: PriorityId) {
    setSelectedNeeds((prev) =>
      prev.includes(id) ? prev.filter((x) => x !== id) : [...prev, id],
    );
  }

  function confirmNeeds() {
    if (selectedNeeds.length === 0) return;
    const labels = selectedNeeds.map((id) => priorityById(id).label).join(", ");
    push("user", `I care about: ${labels}.`);
    if (selectedNeeds.length === 1) {
      setRankedNeeds(selectedNeeds);
      push(
        "agent",
        `Got it — ${priorityById(selectedNeeds[0]).label} is the top filter. What’s your monthly rent ceiling? Skip if you’d rather not set one.`,
      );
      setStep("budget");
      return;
    }
    push(
      "agent",
      "Nice. Tap your needs in order — first tap is highest priority, last is lowest.",
    );
    setRankedNeeds([]);
    setStep("rank");
  }

  function tapRank(id: PriorityId) {
    if (rankedNeeds.includes(id)) return;
    const next = [...rankedNeeds, id];
    setRankedNeeds(next);
    if (next.length === selectedNeeds.length) {
      const order = next.map((p, i) => `${i + 1}. ${priorityById(p).short}`).join(" · ");
      push("user", `Priority order: ${order}`);
      push(
        "agent",
        "What’s your monthly rent ceiling? Skip if you want a pure building comparison.",
      );
      setStep("budget");
    }
  }

  function confirmBudget(skip: boolean) {
    if (skip) {
      setMaxRent("");
      push("user", "No hard rent ceiling.");
    } else {
      const n = Number(maxRent.replace(/[,$]/g, ""));
      if (!Number.isFinite(n) || n <= 0) return;
      push("user", `Max rent about $${Math.round(n).toLocaleString()}/mo.`);
    }
    push(
      "agent",
      "Pick 2 or 3 buildings you’re deciding between. These are from the Brooklyn demo set — same cards you’d get from address search.",
    );
    setStep("buildings");
  }

  function toggleBuilding(id: string) {
    setPickedIds((prev) => {
      if (prev.includes(id)) return prev.filter((x) => x !== id);
      if (prev.length >= 3) return prev;
      return [...prev, id];
    });
  }

  function confirmBuildings() {
    if (pickedIds.length < 2) return;
    const names = pickedIds
      .map((id) => BUILDINGS.find((b) => b.id === id)?.address)
      .filter(Boolean)
      .join("; ");
    push("user", `Comparing: ${names}.`);
    push(
      "agent",
      "Optional: enter the quoted monthly rent for each. I’ll use neighborhood medians as a stand-in where you leave a blank.",
    );
    setStep("rents");
  }

  function runCompare(e?: FormEvent) {
    e?.preventDefault();
    const priorities =
      rankedNeeds.length > 0 ? rankedNeeds : selectedNeeds;
    const max =
      maxRent.trim() === ""
        ? null
        : Number(maxRent.replace(/[,$]/g, ""));
    const candidates = pickedIds.map((id) => {
      const building = BUILDINGS.find((b) => b.id === id)!;
      const raw = rentDrafts[id]?.replace(/[,$]/g, "") ?? "";
      const quoted =
        raw.trim() === "" ? null : Number(raw);
      return {
        building,
        quotedRent:
          quoted != null && Number.isFinite(quoted) && quoted > 0
            ? quoted
            : null,
      };
    });

    const comparison = compareOptions(
      candidates,
      priorities,
      max != null && Number.isFinite(max) && max > 0 ? max : null,
    );
    setResult(comparison.ranked);

    const top = comparison.ranked[0];
    push(
      "agent",
      `Here’s how they stack up for your priorities. #1 is ${top.building.address} in ${top.building.neighborhood} — ${top.whyRanked}`,
    );
    setStep("result");
  }

  function restart() {
    msgCounter = 0;
    setStep("welcome");
    setMessages([
      {
        id: mid(),
        role: "agent",
        text: "Fresh start. What matters most for this lease decision?",
      },
    ]);
    setSelectedNeeds([]);
    setRankedNeeds([]);
    setMaxRent("");
    setPickedIds([]);
    setRentDrafts({});
    setResult(null);
  }

  return (
    <div className="relative flex min-h-full flex-1 flex-col">
      <div className="pointer-events-none absolute inset-0 overflow-hidden" aria-hidden>
        <div className="hero-wash absolute inset-0" />
        <div className="hero-grid absolute inset-0 opacity-[0.25]" />
      </div>

      <div className="relative z-10 mx-auto flex w-full max-w-lg flex-1 flex-col px-4 pb-8 pt-6">
        <div className="mb-4 flex items-center justify-between gap-3">
          <button
            type="button"
            onClick={onBack}
            className="text-sm font-medium text-[var(--ink-muted)] transition hover:text-[var(--ink)]"
          >
            ← Home
          </button>
          <p className="font-[family-name:var(--font-display)] text-xs font-semibold uppercase tracking-[0.16em] text-[var(--teal)]">
            Compare agent
          </p>
          <button
            type="button"
            onClick={restart}
            className="text-sm font-medium text-[var(--ink-muted)] transition hover:text-[var(--ink)]"
          >
            Restart
          </button>
        </div>

        <div className="flex flex-1 flex-col gap-3 pb-4">
          {messages.map((m) => (
            <div
              key={m.id}
              className={
                m.role === "agent"
                  ? "animate-fade-up max-w-[92%] self-start rounded-2xl rounded-tl-md bg-white/90 px-4 py-3 text-sm leading-relaxed text-[var(--ink)] shadow-[0_6px_24px_rgba(12,26,36,0.06)]"
                  : "animate-fade-up max-w-[88%] self-end rounded-2xl rounded-tr-md bg-[var(--ink)] px-4 py-3 text-sm leading-relaxed text-[var(--paper)]"
              }
            >
              {m.text}
            </div>
          ))}

          {step === "welcome" || step === "needs" ? (
            <NeedsPicker
              selected={selectedNeeds}
              onToggle={toggleNeed}
              onConfirm={confirmNeeds}
            />
          ) : null}

          {step === "rank" ? (
            <RankPicker
              pool={selectedNeeds}
              ranked={rankedNeeds}
              onTap={tapRank}
            />
          ) : null}

          {step === "budget" ? (
            <BudgetStep
              value={maxRent}
              onChange={setMaxRent}
              onConfirm={() => confirmBudget(false)}
              onSkip={() => confirmBudget(true)}
            />
          ) : null}

          {step === "buildings" ? (
            <BuildingPicker
              pickedIds={pickedIds}
              onToggle={toggleBuilding}
              onConfirm={confirmBuildings}
            />
          ) : null}

          {step === "rents" ? (
            <RentStep
              pickedIds={pickedIds}
              drafts={rentDrafts}
              onChange={(id, v) =>
                setRentDrafts((prev) => ({ ...prev, [id]: v }))
              }
              onSubmit={runCompare}
            />
          ) : null}

          {step === "result" && result ? (
            <ResultPanel
              ranked={result}
              priorities={rankedNeeds.length ? rankedNeeds : selectedNeeds}
              onOpenBuilding={onOpenBuilding}
              onRestart={restart}
            />
          ) : null}

          <div ref={bottomRef} />
        </div>
      </div>
    </div>
  );
}

function NeedsPicker({
  selected,
  onToggle,
  onConfirm,
}: {
  selected: PriorityId[];
  onToggle: (id: PriorityId) => void;
  onConfirm: () => void;
}) {
  return (
    <div className="animate-fade-up mt-1 space-y-3">
      <div className="flex flex-wrap gap-2">
        {PRIORITIES.map((p) => {
          const on = selected.includes(p.id);
          return (
            <button
              key={p.id}
              type="button"
              onClick={() => onToggle(p.id)}
              className={
                on
                  ? "rounded-xl bg-[var(--teal)] px-3 py-2 text-left text-sm font-medium text-white"
                  : "rounded-xl border border-[var(--line-strong)] bg-white/80 px-3 py-2 text-left text-sm font-medium text-[var(--ink)]"
              }
            >
              {p.label}
            </button>
          );
        })}
      </div>
      <button
        type="button"
        disabled={selected.length === 0}
        onClick={onConfirm}
        className="rounded-xl bg-[var(--ink)] px-4 py-3 text-sm font-semibold text-[var(--paper)] disabled:opacity-40"
      >
        Continue with {selected.length || "…"} need{selected.length === 1 ? "" : "s"}
      </button>
    </div>
  );
}

function RankPicker({
  pool,
  ranked,
  onTap,
}: {
  pool: PriorityId[];
  ranked: PriorityId[];
  onTap: (id: PriorityId) => void;
}) {
  return (
    <div className="animate-fade-up space-y-2">
      {pool.map((id) => {
        const idx = ranked.indexOf(id);
        const done = idx >= 0;
        return (
          <button
            key={id}
            type="button"
            disabled={done}
            onClick={() => onTap(id)}
            className={
              done
                ? "flex w-full items-center gap-3 rounded-xl bg-[var(--teal-soft)] px-3 py-3 text-left text-sm text-[var(--ink)]"
                : "flex w-full items-center gap-3 rounded-xl border border-[var(--line-strong)] bg-white/90 px-3 py-3 text-left text-sm font-medium text-[var(--ink)]"
            }
          >
            <span className="flex h-7 w-7 items-center justify-center rounded-lg bg-white font-[family-name:var(--font-display)] text-sm font-semibold text-[var(--teal)]">
              {done ? idx + 1 : "·"}
            </span>
            <span>
              <span className="block font-medium">{priorityById(id).label}</span>
              <span className="block text-xs text-[var(--ink-muted)]">
                {priorityById(id).prompt}
              </span>
            </span>
          </button>
        );
      })}
    </div>
  );
}

function BudgetStep({
  value,
  onChange,
  onConfirm,
  onSkip,
}: {
  value: string;
  onChange: (v: string) => void;
  onConfirm: () => void;
  onSkip: () => void;
}) {
  return (
    <form
      className="animate-fade-up flex flex-col gap-2 sm:flex-row"
      onSubmit={(e) => {
        e.preventDefault();
        onConfirm();
      }}
    >
      <label className="sr-only" htmlFor="max-rent">
        Max monthly rent
      </label>
      <input
        id="max-rent"
        inputMode="numeric"
        placeholder="e.g. 3200"
        value={value}
        onChange={(e) => onChange(e.target.value)}
        className="w-full rounded-xl border border-[var(--line-strong)] bg-white/90 px-4 py-3 text-base outline-none focus:border-[var(--teal)] focus:ring-2 focus:ring-[var(--teal)]/20"
      />
      <div className="flex gap-2">
        <button
          type="submit"
          className="flex-1 rounded-xl bg-[var(--ink)] px-4 py-3 text-sm font-semibold text-[var(--paper)] sm:flex-none"
        >
          Set ceiling
        </button>
        <button
          type="button"
          onClick={onSkip}
          className="flex-1 rounded-xl border border-[var(--line-strong)] bg-white/80 px-4 py-3 text-sm font-medium text-[var(--ink-muted)] sm:flex-none"
        >
          Skip
        </button>
      </div>
    </form>
  );
}

function BuildingPicker({
  pickedIds,
  onToggle,
  onConfirm,
}: {
  pickedIds: string[];
  onToggle: (id: string) => void;
  onConfirm: () => void;
}) {
  return (
    <div className="animate-fade-up space-y-3">
      <ul className="space-y-2">
        {BUILDINGS.map((b) => {
          const on = pickedIds.includes(b.id);
          const full = !on && pickedIds.length >= 3;
          return (
            <li key={b.id}>
              <button
                type="button"
                disabled={full}
                onClick={() => onToggle(b.id)}
                className={
                  on
                    ? "flex w-full items-baseline justify-between gap-3 rounded-xl bg-[var(--ink)] px-3 py-3 text-left text-[var(--paper)]"
                    : "flex w-full items-baseline justify-between gap-3 rounded-xl border border-[var(--line)] bg-white/85 px-3 py-3 text-left text-[var(--ink)] disabled:opacity-40"
                }
              >
                <span>
                  <span className="block text-sm font-medium">{b.address}</span>
                  <span
                    className={
                      on
                        ? "block text-xs text-white/70"
                        : "block text-xs text-[var(--ink-faint)]"
                    }
                  >
                    {b.neighborhood}
                  </span>
                </span>
                <span className="text-xs font-semibold uppercase tracking-wider">
                  {on ? "Selected" : "Add"}
                </span>
              </button>
            </li>
          );
        })}
      </ul>
      <button
        type="button"
        disabled={pickedIds.length < 2}
        onClick={onConfirm}
        className="w-full rounded-xl bg-[var(--teal)] px-4 py-3 text-sm font-semibold text-white disabled:opacity-40"
      >
        Compare {pickedIds.length || "…"} building{pickedIds.length === 1 ? "" : "s"}
      </button>
    </div>
  );
}

function RentStep({
  pickedIds,
  drafts,
  onChange,
  onSubmit,
}: {
  pickedIds: string[];
  drafts: Record<string, string>;
  onChange: (id: string, value: string) => void;
  onSubmit: (e?: FormEvent) => void;
}) {
  return (
    <form className="animate-fade-up space-y-3" onSubmit={onSubmit}>
      {pickedIds.map((id) => {
        const b = BUILDINGS.find((x) => x.id === id)!;
        return (
          <div key={id}>
            <label
              htmlFor={`rent-${id}`}
              className="mb-1 block text-xs font-semibold uppercase tracking-wider text-[var(--ink-faint)]"
            >
              {b.address}
            </label>
            <input
              id={`rent-${id}`}
              inputMode="numeric"
              placeholder={`Optional · median ~$${b.neighborhoodMedianRent.toLocaleString()}`}
              value={drafts[id] ?? ""}
              onChange={(e) => onChange(id, e.target.value)}
              className="w-full rounded-xl border border-[var(--line-strong)] bg-white/90 px-4 py-3 text-base outline-none focus:border-[var(--teal)] focus:ring-2 focus:ring-[var(--teal)]/20"
            />
          </div>
        );
      })}
      <button
        type="submit"
        className="w-full rounded-xl bg-[var(--ink)] px-4 py-3.5 text-sm font-semibold text-[var(--paper)]"
      >
        Rank my options
      </button>
    </form>
  );
}

function ResultPanel({
  ranked,
  priorities,
  onOpenBuilding,
  onRestart,
}: {
  ranked: RankedOption[];
  priorities: PriorityId[];
  onOpenBuilding: (building: BuildingRecord) => void;
  onRestart: () => void;
}) {
  return (
    <div className="animate-rise mt-2 space-y-5">
      <p className="text-xs text-[var(--ink-faint)]">
        Ranked for:{" "}
        {priorities.map((id, i) => (
          <span key={id}>
            {i > 0 ? " → " : ""}
            {priorityById(id).short}
          </span>
        ))}
      </p>

      {ranked.map((option) => (
        <article
          key={option.building.id}
          className="border-b border-[var(--line)] pb-5 last:border-b-0"
        >
          <div className="flex items-start justify-between gap-3">
            <div>
              <p className="font-[family-name:var(--font-display)] text-xs font-semibold uppercase tracking-[0.14em] text-[var(--teal)]">
                #{option.rank} · fit {option.weightedScore}/100
              </p>
              <h2 className="mt-1 font-[family-name:var(--font-display)] text-xl font-semibold tracking-tight text-[var(--ink)]">
                {option.building.address}
              </h2>
              <p className="mt-1 text-sm text-[var(--ink-muted)]">
                {option.building.neighborhood} · health {option.healthScore} (
                {option.healthLabel})
                {option.quotedRent != null
                  ? ` · quoted $${option.quotedRent.toLocaleString()}`
                  : ""}
              </p>
            </div>
          </div>

          <p className="mt-3 text-sm leading-relaxed text-[var(--ink)]">
            {option.whyRanked}
          </p>

          <div className="mt-4 grid gap-4 sm:grid-cols-2">
            <div>
              <p className="text-[0.65rem] font-semibold uppercase tracking-[0.14em] text-[var(--teal)]">
                Pros
              </p>
              <ul className="mt-2 space-y-1.5 text-sm leading-relaxed text-[var(--ink-muted)]">
                {option.pros.map((p) => (
                  <li key={p} className="flex gap-2">
                    <span className="mt-2 h-1.5 w-1.5 shrink-0 rounded-full bg-[var(--teal)]" />
                    <span>{p}</span>
                  </li>
                ))}
              </ul>
            </div>
            <div>
              <p className="text-[0.65rem] font-semibold uppercase tracking-[0.14em] text-[var(--alert)]">
                Cons
              </p>
              <ul className="mt-2 space-y-1.5 text-sm leading-relaxed text-[var(--ink-muted)]">
                {option.cons.map((c) => (
                  <li key={c} className="flex gap-2">
                    <span className="mt-2 h-1.5 w-1.5 shrink-0 rounded-full bg-[var(--alert)]" />
                    <span>{c}</span>
                  </li>
                ))}
              </ul>
            </div>
          </div>

          <button
            type="button"
            onClick={() => onOpenBuilding(option.building)}
            className="mt-4 text-sm font-semibold text-[var(--teal)] transition hover:text-[var(--ink)]"
          >
            Open full Building Health Card →
          </button>
        </article>
      ))}

      <p className="text-xs leading-relaxed text-[var(--ink-faint)]">
        Rankings weight your stated priorities against public records — not a
        recommendation to sign. Confirm stabilization, access, and lease terms
        in person.
      </p>

      <button
        type="button"
        onClick={onRestart}
        className="w-full rounded-xl border border-[var(--line-strong)] bg-white/80 px-4 py-3 text-sm font-medium text-[var(--ink)]"
      >
        Compare another set
      </button>
    </div>
  );
}
