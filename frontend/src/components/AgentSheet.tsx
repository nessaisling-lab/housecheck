import { useEffect, useRef, useState } from "react";
import { Sheet } from "@/components/Sheet";
import { useAgent } from "@/lib/agent-context";
import { getSummary } from "@/lib/api";
import { bandMeta, fmtDistance, fmtMoney, fmtPct } from "@/lib/score";

interface Msg {
  role: "agent" | "user";
  text: string;
  source?: string;
}

const CHIPS = ["Explain this score", "Is it rent stabilized?", "Negotiate the rent?"];

/** Deterministic, data-derived answers — every claim traces to the card. */
function answerChip(chip: string, b: NonNullable<ReturnType<typeof useAgent>["building"]>): Msg {
  const band = bandMeta(b.score);
  if (chip === CHIPS[0]) {
    return {
      role: "agent",
      text: `${b.address} scores ${b.score ?? "—"} — ${band.label}. The total is a plain average of four equal pillars: condition ${b.sub_scores.condition ?? "—"}, legal ${b.sub_scores.legal ?? "—"}, neighborhood ${b.sub_scores.neighborhood ?? "—"}, accessibility ${b.sub_scores.accessibility ?? "—"}. The weakest pillar is where to dig first.`,
      source: "Source: HouseCheck methodology · equal pillar weights",
    };
  }
  if (chip === CHIPS[1]) {
    const s = b.stabilization;
    return {
      role: "agent",
      text:
        s === "likely"
          ? `Public records point to rent stabilization${b.good_cause ? ", and Good Cause eviction coverage also applies" : ""}. Confirm with NYS DHCR before signing — ask for the rent history.`
          : s === "unverified"
            ? "We couldn't verify stabilization from public records. Ask the landlord directly and request the DHCR rent history — it's free."
            : "No stabilization on record for this building. Good Cause protections may still apply depending on the unit.",
      source: "Source: HPD · DHCR-HCR",
    };
  }
  const median = b.rent?.tract_median;
  const pct = b.rent?.pct_vs_median;
  return {
    role: "agent",
    text:
      median != null && pct != null
        ? `The asking pattern here runs ${fmtPct(pct)} the tract median (${fmtMoney(median)}). Concrete levers: cite the ${b.open_violations.c} open Class C violation${b.open_violations.c === 1 ? "" : "s"}, ask for a longer lease in exchange for a lower ask, or negotiate a free month instead of a rent cut.`
        : "We don't have tract rent data for this building, so I can't benchmark the ask. Check the Rent fairness section once tract data is available.",
    source: "Source: US Census B25064 · HUD FMR",
  };
}

function Typing() {
  return (
    <span className="inline-flex gap-1 px-1 py-2" aria-label="Agent is typing">
      {[0, 1, 2].map((i) => (
        <span
          key={i}
          className="h-1.5 w-1.5 rounded-full"
          style={{
            background: "var(--hc-ink-3)",
            animation: `hc-typing 1s ${i * 0.15}s infinite`,
          }}
        />
      ))}
    </span>
  );
}

/**
 * AI agent sheet (Whoop coach sheet, light-mode translation — autopsy #7).
 * Opens from the orb on ANY screen, carrying building context when available.
 */
export function AgentSheet() {
  const { open, closeAgent, building } = useAgent();
  const [msgs, setMsgs] = useState<Msg[]>([]);
  const [busy, setBusy] = useState(false);
  const [input, setInput] = useState("");
  const scrollRef = useRef<HTMLDivElement>(null);
  const loadedFor = useRef<string | null>(null);

  // Seed with the plain-language summary (POST /summary) when context changes
  useEffect(() => {
    if (!open || !building || loadedFor.current === building.bbl) return;
    loadedFor.current = building.bbl;
    setBusy(true);
    getSummary(building.bbl)
      .then(({ data }) =>
        setMsgs([
          {
            role: "agent",
            text: data,
            source: "Source: HPD · DHCR · Census B25064",
          },
        ])
      )
      .catch(() =>
        setMsgs([
          {
            role: "agent",
            text: "The agent couldn't summarize this building — the raw data on the card is still your best source.",
          },
        ])
      )
      .finally(() => setBusy(false));
  }, [open, building]);

  useEffect(() => {
    if (open && !building) {
      loadedFor.current = null;
      setMsgs([
        {
          role: "agent",
          text: "Search a building first and I'll answer questions with its data attached. I can still explain how scores work meanwhile.",
        },
      ]);
    }
  }, [open, building]);

  useEffect(() => {
    scrollRef.current?.scrollTo({ top: scrollRef.current.scrollHeight });
  }, [msgs, busy]);

  const send = (text: string) => {
    const t = text.trim();
    if (!t || busy) return;
    setInput("");
    setMsgs((m) => [...m, { role: "user", text: t }]);
    setBusy(true);
    setTimeout(() => {
      setMsgs((m) => [
        ...m,
        building
          ? answerChip(CHIPS.includes(t) ? t : CHIPS[0], building)
          : {
              role: "agent",
              text: "Each pillar — condition, legal, neighborhood, accessibility — counts equally toward the 0–100 score. Every number links to its public NYC source; unverified means we couldn't confirm it, not that something is wrong.",
              source: "Source: HouseCheck methodology",
            },
      ]);
      setBusy(false);
    }, 700);
  };

  return (
    <Sheet open={open} onClose={closeAgent} labelledBy="agent-title">
      <div className="flex items-center justify-between px-4 pb-2 pt-1">
        <span className="glass-nav flex items-center gap-2 rounded-full px-3 py-1.5">
          <svg width="14" height="14" viewBox="0 0 24 24" fill="var(--hc-ink)" aria-hidden>
            <path d="M12 2l2.1 7.9L22 12l-7.9 2.1L12 22l-2.1-7.9L2 12l7.9-2.1L12 2z" />
          </svg>
          <span id="agent-title" className="text-[14px] font-semibold" style={{ color: "var(--hc-ink)" }}>
            HouseCheck Agent
          </span>
        </span>
        <button onClick={closeAgent} aria-label="Close agent" className="p-2" style={{ color: "var(--hc-ink-3)" }}>
          <svg width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round">
            <path d="M6 6l12 12M18 6L6 18" />
          </svg>
        </button>
      </div>

      {building && (
        <div className="px-4 pb-2">
          <span
            className="inline-flex items-center gap-2 rounded-full px-3 py-1.5 text-[13px]"
            style={{ background: "var(--hc-sunken)", color: "var(--hc-ink-2)" }}
          >
            <svg width="13" height="13" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" aria-hidden>
              <circle cx="11" cy="11" r="7" />
              <path d="M20 20l-3.5-3.5" strokeLinecap="round" />
            </svg>
            Asking about {building.address}
          </span>
        </div>
      )}

      <div ref={scrollRef} className="min-h-[220px] flex-1 space-y-3 overflow-y-auto px-4 py-2">
        {msgs.map((m, i) =>
          m.role === "agent" ? (
            <div
              key={i}
              className="max-w-[88%] rounded-2xl bg-white p-3.5"
              style={{ boxShadow: "0 4px 16px rgba(0,0,0,0.06)" }}
            >
              <p className="text-[15px] leading-snug" style={{ color: "var(--hc-ink)" }}>
                {m.text}
              </p>
              {m.source && (
                <p className="mt-2 text-[11px]" style={{ color: "var(--hc-ink-3)" }}>
                  {m.source}
                </p>
              )}
            </div>
          ) : (
            <div
              key={i}
              className="ml-auto max-w-[80%] rounded-2xl px-3.5 py-2.5 text-[15px] text-white"
              style={{ background: "var(--hc-ink)" }}
            >
              {m.text}
            </div>
          )
        )}
        {busy && (
          <div
            className="inline-block rounded-2xl bg-white px-3.5 py-1.5"
            style={{ boxShadow: "0 4px 16px rgba(0,0,0,0.06)" }}
          >
            <Typing />
          </div>
        )}
      </div>

      <div className="flex flex-wrap gap-2 px-4 pb-2">
        {CHIPS.map((c) => (
          <button
            key={c}
            onClick={() => send(c)}
            disabled={busy}
            className="glass-nav rounded-full px-3.5 py-2 text-[13px] font-medium disabled:opacity-50"
            style={{ color: "var(--hc-ink)" }}
          >
            {c}
          </button>
        ))}
      </div>

      <form
        className="flex items-center gap-2 p-4 pt-1"
        onSubmit={(e) => {
          e.preventDefault();
          send(input);
        }}
      >
        <button
          type="button"
          aria-label="Attach"
          className="flex h-10 w-10 shrink-0 items-center justify-center rounded-full"
          style={{ background: "var(--hc-sunken)", color: "var(--hc-ink-2)" }}
        >
          <svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round">
            <path d="M12 5v14M5 12h14" />
          </svg>
        </button>
        <input
          value={input}
          onChange={(e) => setInput(e.target.value)}
          placeholder={building ? "Ask about this building…" : "Ask about HouseCheck…"}
          className="glass-field h-11 flex-1 rounded-full px-4 text-[15px] outline-none placeholder:text-[15px]"
          style={{ color: "var(--hc-ink)" }}
          aria-label="Message the agent"
        />
        <button
          type="submit"
          aria-label="Send"
          disabled={!input.trim() || busy}
          className="flex h-10 w-10 shrink-0 items-center justify-center rounded-full text-white disabled:opacity-40"
          style={{ background: "var(--hc-ink)" }}
        >
          <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2.4" strokeLinecap="round">
            <path d="M12 19V5M5 12l7-7 7 7" />
          </svg>
        </button>
      </form>
    </Sheet>
  );
}

export { fmtDistance };
