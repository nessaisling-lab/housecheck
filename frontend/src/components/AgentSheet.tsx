import { useEffect, useRef, useState, type CSSProperties } from "react";
import ReactMarkdown, { type Components } from "react-markdown";
import remarkGfm from "remark-gfm";
import { Sheet } from "@/components/Sheet";
import { useAgent } from "@/lib/agent-context";
import { getSummary, sendChat, type ChatTurn } from "@/lib/api";
import { bandMeta, fmtMoney, fmtPct } from "@/lib/score";

// ── Markdown rendering for agent replies (spec: SPEC-agent-readability) ──
// Emits real headings/lists/strong so replies are scannable AND screen-reader
// navigable (fixes literal ** and collapsed table pipes). Tables stack into
// blocks so they never force horizontal scroll on a phone.
const mdEyebrow: CSSProperties = {
  fontSize: "0.6875rem",
  fontWeight: 600,
  letterSpacing: "0.08em",
  textTransform: "uppercase",
  color: "var(--hc-ink-3)",
  margin: "12px 0 4px",
};
const mdComponents: Components = {
  h1: ({ children }) => <div style={mdEyebrow}>{children}</div>,
  h2: ({ children }) => <div style={mdEyebrow}>{children}</div>,
  h3: ({ children }) => <div style={mdEyebrow}>{children}</div>,
  p: ({ children }) => (
    <p style={{ color: "var(--hc-ink-2)", lineHeight: 1.6, margin: "0 0 10px" }}>{children}</p>
  ),
  strong: ({ children }) => (
    <strong style={{ color: "var(--hc-ink)", fontWeight: 600 }}>{children}</strong>
  ),
  em: ({ children }) => <em style={{ color: "var(--hc-ink-2)" }}>{children}</em>,
  ul: ({ children }) => (
    <ul style={{ margin: "0 0 10px", paddingLeft: 18, listStyleType: "disc" }}>{children}</ul>
  ),
  ol: ({ children }) => (
    <ol style={{ margin: "0 0 10px", paddingLeft: 18, listStyleType: "decimal" }}>{children}</ol>
  ),
  li: ({ children }) => (
    <li style={{ color: "var(--hc-ink-2)", lineHeight: 1.55, margin: "2px 0" }}>{children}</li>
  ),
  a: ({ href, children }) => (
    <a
      href={href}
      target="_blank"
      rel="noopener noreferrer"
      style={{ color: "var(--hc-strong)", textDecoration: "underline", textUnderlineOffset: 2 }}
    >
      {children} ↗
    </a>
  ),
  hr: () => <div style={{ height: 1, background: "var(--hc-sunken)", margin: "12px 0" }} />,
  code: ({ children }) => (
    <code
      style={{
        fontFamily: "ui-monospace, monospace",
        fontSize: "0.9em",
        background: "rgba(255,255,255,0.08)",
        padding: "1px 5px",
        borderRadius: 4,
      }}
    >
      {children}
    </code>
  ),
  blockquote: ({ children }) => (
    <blockquote
      style={{
        borderLeft: "2px solid var(--hc-sunken)",
        paddingLeft: 10,
        margin: "0 0 10px",
        color: "var(--hc-ink-2)",
      }}
    >
      {children}
    </blockquote>
  ),
  // Task 2 — tables degrade to stacked bordered blocks (never horizontal scroll)
  table: ({ children }) => (
    <div style={{ display: "flex", flexDirection: "column", gap: 8, margin: "4px 0 12px" }}>
      {children}
    </div>
  ),
  thead: ({ children }) => <>{children}</>,
  tbody: ({ children }) => <>{children}</>,
  tr: ({ children }) => (
    <div style={{ border: "0.5px solid rgba(255,255,255,0.14)", borderRadius: 12, padding: "8px 12px" }}>
      {children}
    </div>
  ),
  th: ({ children }) => (
    <div
      style={{
        fontSize: "0.6875rem",
        fontWeight: 600,
        textTransform: "uppercase",
        letterSpacing: "0.06em",
        color: "var(--hc-ink-3)",
        marginBottom: 2,
      }}
    >
      {children}
    </div>
  ),
  td: ({ children }) => (
    <div style={{ color: "var(--hc-ink-2)", fontSize: "0.875rem", lineHeight: 1.5, margin: "2px 0" }}>
      {children}
    </div>
  ),
};

function MarkdownMessage({ text }: { text: string }) {
  return (
    <div className="text-[0.9375rem]">
      <ReactMarkdown remarkPlugins={[remarkGfm]} components={mdComponents}>
        {text}
      </ReactMarkdown>
    </div>
  );
}

interface Msg {
  role: "agent" | "user";
  text: string;
  source?: string;
}

const CHIPS = ["Explain this score", "Is it rent stabilized?", "Negotiate the rent?"];

/**
 * Chips shown before a building is picked.
 *
 * With no building there is nothing to ground a model call in, so /agent/chat
 * is not called at all — these are answered from constants in this file and
 * the shipped dataset. Previously this state offered the building chips and a
 * single "search a building first" reply, so the agent appeared broken to
 * anyone who opened it from the home screen.
 */
const GENERAL_CHIPS = [
  "What does HouseCheck check?",
  "Which buildings are covered?",
  "What can't it tell me?",
];

const GENERAL_ANSWERS: Record<string, Msg> = {
  [GENERAL_CHIPS[0]]: {
    role: "agent",
    text: `Four things, weighted equally into one 0–100 score:

- **Condition** — open HPD violations, weighted by class, plus 311 complaint history
- **Legal** — DHCR rent-stabilization status and Good Cause eviction coverage
- **Neighborhood** — asking rent against the Census tract median
- **Accessibility** — elevator on record, and distance to an ADA subway station

A pillar we can't verify is marked *unverified*. It is never scored as a zero — that would punish a building for a gap in the city's own records.`,
    source: "Source: HouseCheck methodology · equal pillar weights",
  },
  [GENERAL_CHIPS[1]]: {
    role: "agent",
    text: `**250 buildings in Bedford-Stuyvesant**, Brooklyn — the pilot area, on a 2026 data snapshot.

Type any address into the search box. If it's outside the pilot you'll get a map of what we do cover rather than a wrong answer. **About → Covered buildings** lists all 250, sorted by score.

The pipeline is city-wide; the pilot is scoped to Bed-Stuy, not limited to it.`,
    source: "Source: HouseCheck coverage · snapshot 2026",
  },
  [GENERAL_CHIPS[2]]: {
    role: "agent",
    text: `Worth being straight about:

- **It is not legal advice.** It can explain what a statute says and cite it. It can't tell you what to do about your situation, and it won't predict how a case would go.
- **It is not an inspection.** Everything here is the paper record. A building with a clean record can still be badly run.
- **It is a snapshot.** Filings lag reality, and older records predate the city's own geocoding.
- **Absence isn't innocence.** No violations on file can mean a well-kept building, or one nobody has reported.

Every number on a card links to the public source it came from, so you can check the original.`,
    source: "Source: HouseCheck methodology · scope and limits",
  },
};

/** Deterministic, data-derived answers — every claim traces to the card. */
function answerChip(
  chip: string,
  b: NonNullable<ReturnType<typeof useAgent>["building"]>,
  rent: ReturnType<typeof useAgent>["rent"]
): Msg {
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
  // Prefer a rent check the user actually ran (live tract median from
  // POST /rent-fairness); fall back to the demo fixture's embedded rent context.
  const median = rent?.tract_median ?? b.rent?.tract_median;
  const pct = rent?.pct_vs_median ?? b.rent?.pct_vs_median;
  return {
    role: "agent",
    text:
      median != null && pct != null
        ? `The asking pattern here runs ${fmtPct(pct)} the tract median (${fmtMoney(median)}). Concrete levers: ${
            // Only offer the violation lever when there is a count to cite. Interpolating a
            // null here printed "cite the null open Class C violations".
            b.open_violations.c === null
              ? "ask what the building's open HPD violations are"
              : b.open_violations.c > 0
                ? `cite the ${b.open_violations.c} open Class C violation${b.open_violations.c === 1 ? "" : "s"}`
                : "note the clean Class C record as a reason the asking rent should not carry a risk premium"
          }, ask for a longer lease in exchange for a lower ask, or negotiate a free month instead of a rent cut.`
        : "I don't have a tract rent benchmark for this building yet. Enter your rent in the Rent fairness section and I'll compare it against the Census tract median.",
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
  const { open, closeAgent, building, rent } = useAgent();
  const [msgs, setMsgs] = useState<Msg[]>([]);
  /** Index of the answer just copied, so the button can confirm rather than stay silent. */
  const [copiedAt, setCopiedAt] = useState<number | null>(null);
  const [saveNote, setSaveNote] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);

  /**
   * Copy one answer.
   *
   * This is an accessibility fix, not a convenience. Every operating system can already
   * select text — but the people who most need a housing record are elderly tenants and
   * people with low vision, and a text-selection handle on a phone is exactly the
   * interaction they cannot reliably perform. A button removes the whole problem.
   *
   * Falls back to a hidden textarea + execCommand where the clipboard API is unavailable
   * (older Safari, and any non-HTTPS origin), because failing silently on the browsers
   * least-served users are most likely to have would defeat the point.
   */
  const copyAnswer = async (text: string, index: number) => {
    try {
      await navigator.clipboard.writeText(text);
    } catch {
      const ta = document.createElement("textarea");
      ta.value = text;
      ta.setAttribute("readonly", "");
      ta.style.position = "fixed";
      ta.style.opacity = "0";
      document.body.appendChild(ta);
      ta.select();
      try {
        document.execCommand("copy");
      } finally {
        ta.remove();
      }
    }
    setCopiedAt(index);
    setTimeout(() => setCopiedAt((c) => (c === index ? null : c)), 2000);
  };

  /**
   * Save the whole conversation as a plain-text transcript.
   *
   * Plain text rather than a proprietary format, and a download rather than an account:
   * the product has no user records by design, so the only honest place to keep a
   * conversation is on the reader's own machine. The header carries the building and the
   * date, because a transcript that does not say what it is about is not evidence of
   * anything a week later.
   */
  const saveConversation = () => {
    if (!msgs.length) return;
    const when = new Date();
    const head = [
      "HouseCheck — saved conversation",
      building ? `Building: ${building.address} (BBL ${building.bbl})` : "No building selected",
      `Saved: ${when.toISOString().slice(0, 16).replace("T", " ")}`,
      "",
      "This is a transcript of an assistant conversation. It is not legal advice, and the",
      "assistant does not predict case outcomes. Figures come from public NYC records.",
      "",
      "".padEnd(72, "-"),
      "",
    ].join("\n");
    const body = msgs
      .map((m) => `${m.role === "agent" ? "HouseCheck" : "You"}:\n${m.text}\n${m.source ? m.source + "\n" : ""}`)
      .join("\n");
    const blob = new Blob([head + body], { type: "text/plain;charset=utf-8" });
    const url = URL.createObjectURL(blob);
    const a = document.createElement("a");
    a.href = url;
    a.download = `housecheck-conversation-${building?.bbl ?? "session"}-${when
      .toISOString()
      .slice(0, 10)}.txt`;
    document.body.appendChild(a);
    a.click();
    a.remove();
    URL.revokeObjectURL(url);
    setSaveNote("Saved to your downloads");
    setTimeout(() => setSaveNote(null), 2500);
  };
  const [input, setInput] = useState("");
  const scrollRef = useRef<HTMLDivElement>(null);
  const loadedFor = useRef<string | null>(null);

  // Seed with the plain-language summary (POST /summary) when context changes
  useEffect(() => {
    if (!open || !building || loadedFor.current === building.bbl) return;
    loadedFor.current = building.bbl;
    const t = setTimeout(() => {
      setBusy(true);
      getSummary(building.bbl)
        .then(({ data, source }) =>
          setMsgs([
            {
              role: "agent",
              text: data,
              // Honour the demo/live flag. Labelling bundled demo text with a real
              // source line is the one place this app was claiming provenance it
              // did not have — every other surface reports `source` truthfully.
              source:
                source === "demo"
                  ? "Demo data · live summary unavailable"
                  : "Source: HPD · DHCR · Census B25064",
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
    }, 0);
    return () => clearTimeout(t);
  }, [open, building]);

  useEffect(() => {
    if (!open || building) return;
    loadedFor.current = null;
    const t = setTimeout(
      () =>
        setMsgs([
          {
            role: "agent",
            text: `Search an address and I'll answer with that building's own record attached.

Right now I can tell you **what HouseCheck checks**, **which buildings are covered**, and **what it can't tell you**.`,
          },
        ]),
      0
    );
    return () => clearTimeout(t);
  }, [open, building]);

  useEffect(() => {
    scrollRef.current?.scrollTo({ top: scrollRef.current.scrollHeight });
  }, [msgs, busy]);

  /**
   * Deterministic answer used when the agent is unavailable.
   *
   * Kept rather than deleted: every number in it comes from the card, so it is honest,
   * and it is the graceful path when OPENROUTER_API_KEY is unset or the upstream fails.
   * It is always labelled, so a canned answer is never mistaken for a live one.
   */
  const offlineAnswer = (t: string): Msg => {
    if (!building) {
      // Exact chip match first, then a keyword nudge so a typed question lands
      // somewhere useful instead of on the same generic paragraph every time.
      if (GENERAL_ANSWERS[t]) return GENERAL_ANSWERS[t];
      const q = t.toLowerCase();
      if (/cover|area|which building|bed.?stuy|neighborhood|where/.test(q))
        return GENERAL_ANSWERS[GENERAL_CHIPS[1]];
      if (/can'?t|cannot|limit|legal advice|lawyer|accurate|trust|wrong/.test(q))
        return GENERAL_ANSWERS[GENERAL_CHIPS[2]];
      if (/score|work|calculat|pillar|rating|check/.test(q))
        return GENERAL_ANSWERS[GENERAL_CHIPS[0]];
      return {
        role: "agent",
        text: "Search an address and I'll answer with that building's own record attached. Until then I can explain **what HouseCheck checks**, **which buildings are covered**, and **what it can't tell you** — tap one below.",
        source: "Source: HouseCheck methodology",
      };
    }
    const canned = answerChip(CHIPS.includes(t) ? t : CHIPS[0], building, rent);
    return { ...canned, source: `${canned.source ?? "Source: HouseCheck"} · offline answer` };
  };

  const send = async (text: string) => {
    const t = text.trim();
    if (!t || busy) return;
    setInput("");
    const userMsg: Msg = { role: "user", text: t };
    setMsgs((m) => [...m, userMsg]);
    setBusy(true);

    // No building in context → nothing to ground an answer in, so don't spend a call.
    if (!building) {
      setMsgs((m) => [...m, offlineAnswer(t)]);
      setBusy(false);
      return;
    }

    // Send the conversation so far so the agent can follow up. The server keeps only the
    // most recent turns; sending the whole thread lets it decide what to keep.
    const history: ChatTurn[] = [...msgs, userMsg]
      .filter((m) => m.text.trim().length > 0)
      .map((m) => ({ role: m.role === "user" ? "user" : "assistant", content: m.text }));

    try {
      const { answer, citations } = await sendChat(building.bbl, history);
      setMsgs((m) => [
        ...m,
        {
          role: "agent",
          text: answer,
          // Render the sources the server says actually fed the answer, rather than a
          // hardcoded line — the same honesty rule the rest of the app follows.
          source: citations.length ? `Source: ${citations.join(" · ")}` : undefined,
        },
      ]);
    } catch {
      // Say the question went unanswered *before* offering what we can still say.
      //
      // This used to push `offlineAnswer(t)` alone, which for a typed question falls through
      // to `answerChip(CHIPS[0], ...)` — the canned score explanation — no matter what was
      // asked. So "there is no heat in my apartment, what should I do" came back as a
      // paragraph about the score, labelled only "offline answer". Measured on production
      // 2026-08-11 while the upstream was slow: two of three runs of that exact question
      // failed, so this was the common path, not the rare one.
      //
      // A reply that answers a different question is not a degraded answer, it is a wrong
      // one — the same failure as the stale search results and the arbitrary borough, and the
      // worst possible version of it on a page about someone's housing.
      setMsgs((m) => [
        ...m,
        {
          role: "agent",
          text: "I couldn't reach the assistant to answer that one. Here's what I can tell you from this building's record without it:",
        },
        offlineAnswer(t),
      ]);
    } finally {
      setBusy(false);
    }
  };

  return (
    <Sheet open={open} onClose={closeAgent} labelledBy="agent-title">
      <div className="flex items-center justify-between px-4 pb-2 pt-1">
        <span className="glass-nav flex items-center gap-2 rounded-full px-3 py-1.5">
          <svg width="14" height="14" viewBox="0 0 24 24" fill="var(--hc-ink)" aria-hidden>
            <path d="M12 2l2.1 7.9L22 12l-7.9 2.1L12 22l-2.1-7.9L2 12l7.9-2.1L12 2z" />
          </svg>
          <span id="agent-title" className="text-[0.875rem] font-semibold" style={{ color: "var(--hc-ink)" }}>
            HouseCheck Agent
          </span>
        </span>
        <div className="flex items-center gap-1.5">
          {/* Only offered once there is something to save. A disabled button on an empty
              conversation is a control that has to be explained; absence explains itself. */}
          {msgs.length > 0 && (
            <button
              type="button"
              onClick={saveConversation}
              className="rounded-full px-3 py-1.5 text-[0.8125rem] font-semibold"
              style={{ color: "var(--hc-ink)", border: "1px solid rgba(255,255,255,0.18)" }}
            >
              {saveNote ? "Saved" : "Save conversation"}
            </button>
          )}
          <button onClick={closeAgent} aria-label="Close agent" className="p-2" style={{ color: "var(--hc-ink-3)" }}>
            <svg width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round">
              <path d="M6 6l12 12M18 6L6 18" />
            </svg>
          </button>
        </div>
      </div>

      {building && (
        <div className="px-4 pb-2">
          <span
            className="inline-flex items-center gap-2 rounded-full px-3 py-1.5 text-[0.8125rem]"
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

      {/*
        WCAG 2.2 AA 4.1.3 (status messages). An agent reply used to appear with
        no announcement at all, so a screen-reader user had no way to know it
        had arrived short of hunting for it.

        polite, not assertive: the reply is not urgent enough to interrupt what
        the reader is already hearing. atomic=false so only the newly added
        message is read, not the whole thread again on every turn. aria-busy
        marks the wait, which is what the animated dots convey visually.
      */}
      <div
        ref={scrollRef}
        className="min-h-[220px] flex-1 space-y-3 overflow-y-auto px-4 py-2"
        role="log"
        aria-live="polite"
        aria-atomic="false"
        aria-relevant="additions"
        aria-busy={busy}
        aria-label="Conversation with the HouseCheck agent"
      >
        {msgs.map((m, i) =>
          m.role === "agent" ? (
            <div
              key={i}
              className="max-w-[88%] rounded-2xl p-3.5"
              style={{ background: "#48484A", boxShadow: "0 4px 16px rgba(0,0,0,0.2)" }}
            >
              {/* Who is speaking is carried only by bubble colour and side,
                  which is invisible to a screen reader (1.3.1). */}
              <span className="sr-only">Agent said: </span>
              <MarkdownMessage text={m.text} />
              {m.source && (
                <p className="mt-2 text-[0.6875rem]" style={{ color: "var(--hc-ink-3)" }}>
                  {m.source}
                </p>
              )}
              <button
                type="button"
                onClick={() => copyAnswer(m.text, i)}
                className="mt-2 rounded-lg px-2.5 py-1 text-[0.75rem] font-semibold"
                style={{
                  background: "rgba(255,255,255,0.10)",
                  color: "var(--hc-ink, #fff)",
                  border: "1px solid rgba(255,255,255,0.16)",
                }}
              >
                {copiedAt === i ? "Copied" : "Copy answer"}
              </button>
            </div>
          ) : (
            <div
              key={i}
              className="ml-auto max-w-[80%] rounded-2xl px-3.5 py-2.5 text-[0.9375rem]"
              style={{ background: "#F5F5F7", color: "#2C2C2E" }}
            >
              <span className="sr-only">You said: </span>
              {m.text}
            </div>
          )
        )}
        {busy && (
          <div
            className="inline-block rounded-2xl px-3.5 py-1.5"
            style={{ background: "#48484A", boxShadow: "0 4px 16px rgba(0,0,0,0.2)" }}
            // aria-busy on the log already reports the wait; announcing the
            // dots as well would say it twice.
            aria-hidden
          >
            <Typing />
          </div>
        )}
      </div>

      <div className="flex flex-wrap gap-2 px-4 pb-2">
        {(building ? CHIPS : GENERAL_CHIPS).map((c) => (
          <button
            key={c}
            onClick={() => send(c)}
            disabled={busy}
            className="glass-nav rounded-full px-3.5 py-2 text-[0.8125rem] font-medium disabled:opacity-50"
            style={{ color: "#3A3A3C" }}
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
        {/* An "Attach" affordance was here with no handler — it looked interactive,
            focused like a control, and did nothing. Removed rather than stubbed;
            there is no attachment feature to wire it to. */}
        <input
          value={input}
          onChange={(e) => setInput(e.target.value)}
          placeholder={building ? "Ask about this building…" : "Ask about HouseCheck…"}
          className="glass-field h-11 flex-1 rounded-full px-4 text-[0.9375rem] outline-none placeholder:text-[0.9375rem]"
          style={{ color: "var(--hc-ink)" }}
          aria-label="Message the agent"
        />
        <button
          type="submit"
          aria-label="Send"
          disabled={!input.trim() || busy}
          className="flex h-10 w-10 shrink-0 items-center justify-center rounded-full disabled:opacity-40"
          style={{ background: "#F5F5F7", color: "#2C2C2E" }}
        >
          <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2.4" strokeLinecap="round">
            <path d="M12 19V5M5 12l7-7 7 7" />
          </svg>
        </button>
      </form>
    </Sheet>
  );
}
