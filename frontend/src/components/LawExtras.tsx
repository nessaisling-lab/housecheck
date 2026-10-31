/**
 * What sits around an agent answer that used the law (docs/design/law-passages.md §§3, 4, 7, 8):
 * the urgency banner, the laws used with their full text, the lawyer bundle, the full-text
 * confirm step, and the Brief / More / Full toggle.
 *
 * Written for someone in a crisis, on a phone, possibly elderly or with low vision: every
 * level is a word, an icon and a colour together (never colour alone, WCAG 1.4.1); text
 * contrast is above 4.5:1 (1.4.3); buttons are at least 44 px tall and say exactly what they
 * do; the Urgent banner is announced with role="alert".
 */
import { useState } from "react";
import { fetchLawyerBundle, lawTextUrl, type LawDetail, type LawRef, type Urgency } from "@/lib/api";

const LEVELS = {
  urgent: { word: "Urgent", icon: "warning", bg: "#FDE2E1", fg: "#7A1712", edge: "#C0392B" },
  time_sensitive: { word: "Time-sensitive", icon: "clock", bg: "#FFEFD1", fg: "#6B3E00", edge: "#B36B00" },
  coming_up: { word: "Coming up", icon: "calendar", bg: "#FFF8D6", fg: "#4D3F00", edge: "#8C7500" },
} as const;

function Icon({ name }: { name: "warning" | "clock" | "calendar" }) {
  const paths = {
    warning: "M12 3l10 18H2L12 3zm0 6v5m0 3v.5",
    clock: "M12 3a9 9 0 100 18 9 9 0 000-18zm0 4v5l3 2",
    calendar: "M4 6h16v14H4zM4 10h16M8 3v4M16 3v4",
  };
  return (
    <svg width="22" height="22" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2.2"
      strokeLinecap="round" strokeLinejoin="round" aria-hidden>
      <path d={paths[name]} />
    </svg>
  );
}

/** "Tuesday, November 4", from a New York YYYY-MM-DD. Noon, so no time zone shifts the day. */
function longDate(iso: string): string {
  return new Date(`${iso}T12:00:00`).toLocaleDateString("en-US", {
    weekday: "long",
    month: "long",
    day: "numeric",
    timeZone: "America/New_York",
  });
}

function daysWords(days: number): string {
  if (days < -1) return `${-days} days ago`;
  if (days === -1) return "yesterday";
  if (days === 0) return "today";
  if (days === 1) return "tomorrow: 1 day left";
  return `${days} days left`;
}

const BIG_BUTTON =
  "inline-flex min-h-[44px] items-center justify-center gap-2 rounded-xl px-4 text-[1rem] font-semibold";

/** The banner at the top of a time-sensitive answer. Danger leads with a call button. */
export function UrgencyBanner({ urgency }: { urgency: Urgency }) {
  if (urgency.danger) {
    return (
      <div role="alert" className="mb-3 rounded-xl p-3" style={{ background: LEVELS.urgent.bg, color: LEVELS.urgent.fg, border: `2px solid ${LEVELS.urgent.edge}` }}>
        <p className="flex items-center gap-2 text-[1.0625rem] font-bold">
          <Icon name="warning" /> Danger: {urgency.reason}
        </p>
        <a href="tel:911" className={`${BIG_BUTTON} mt-2 w-full`} style={{ background: LEVELS.urgent.edge, color: "#FFFFFF" }}>
          Call 911 now
        </a>
      </div>
    );
  }
  if (!urgency.level) return null;
  const l = LEVELS[urgency.level];
  return (
    <div
      role={urgency.level === "urgent" ? "alert" : "status"}
      className="mb-3 rounded-xl p-3"
      style={{ background: l.bg, color: l.fg, border: `2px solid ${l.edge}` }}
    >
      <p className="flex items-center gap-2 text-[1.0625rem] font-bold">
        <Icon name={l.icon} /> {l.word}: {urgency.reason}
      </p>
      {urgency.date && urgency.days_left !== null && (
        <p className="mt-1 text-[1rem]">
          {longDate(urgency.date)}, {daysWords(urgency.days_left)}.
        </p>
      )}
      <p className="mt-1 text-[0.9375rem]">Counted on {longDate(urgency.counted_on)}, New York time.</p>
    </div>
  );
}

/** The question for a lawyer: the paragraph after the answer's part-4 heading. */
function lawyerQuestion(answer: string): string | undefined {
  const at = answer.toLowerCase().indexOf("a question to ask a legal professional");
  if (at < 0) return undefined;
  const after = answer.slice(at).split("\n").slice(1).join("\n").trim();
  return after.split(/\n\s*\n/)[0]?.trim() || undefined;
}

function save(blob: Blob, filename: string) {
  const url = URL.createObjectURL(blob);
  const a = document.createElement("a");
  a.href = url;
  a.download = filename;
  document.body.appendChild(a);
  a.click();
  a.remove();
  URL.revokeObjectURL(url);
}

/**
 * Share a file through the phone's own share sheet (mail, messages), or save it where the
 * browser can't share files.
 */
async function shareOrSave(blob: Blob, filename: string, title: string): Promise<"shared" | "saved"> {
  const file = new File([blob], filename, { type: "text/plain" });
  if (navigator.canShare?.({ files: [file] })) {
    try {
      await navigator.share({ files: [file], title });
      return "shared";
    } catch {
      // Dismissed, or refused: fall back to a download so the reader still has the file.
    }
  }
  save(blob, filename);
  return "saved";
}

/** "Laws used", each with its full verified text to read, download or share; and the bundle. */
export function LawsUsed({
  laws,
  bbl,
  answer,
  urgency,
}: {
  laws: LawRef[];
  bbl: string;
  answer: string;
  urgency: Urgency | null;
}) {
  const [note, setNote] = useState<string | null>(null);
  const urgent = !!urgency && (urgency.danger || urgency.level !== null);

  const bundle = async (share: boolean) => {
    setNote("Preparing the file…");
    try {
      const { blob, filename } = await fetchLawyerBundle({
        bbl,
        lawIds: laws.map((l) => l.id),
        question: lawyerQuestion(answer),
        urgency,
      });
      const how = share
        ? await shareOrSave(blob, filename, "HouseCheck: papers for your lawyer")
        : (save(blob, filename), "saved");
      setNote(how === "shared" ? "Shared." : `Saved to your downloads: ${filename}`);
    } catch {
      setNote("Couldn't prepare the file. Try again in a moment.");
    }
  };

  const shareLaw = async (l: LawRef) => {
    try {
      const res = await fetch(lawTextUrl(l.id));
      if (!res.ok) throw new Error(String(res.status));
      const how = await shareOrSave(await res.blob(), `HouseCheck-${l.id}.txt`, l.citation);
      setNote(how === "shared" ? "Shared." : `Saved to your downloads: HouseCheck-${l.id}.txt`);
    } catch {
      setNote("Couldn't get that law's text. Try again in a moment.");
    }
  };

  return (
    <section aria-label="Laws used in this answer" className="mt-3 rounded-xl p-3" style={{ background: "rgba(255,255,255,0.08)" }}>
      <div className="flex flex-col gap-2">
        <button
          type="button"
          onClick={() => bundle(false)}
          className={BIG_BUTTON}
          style={urgent ? { background: "#F5F5F7", color: "#1C1C1E" } : { background: "rgba(255,255,255,0.14)", color: "#FFFFFF" }}
        >
          Download everything for your lawyer
        </button>
        <button type="button" onClick={() => bundle(true)} className={BIG_BUTTON} style={{ background: "rgba(255,255,255,0.10)", color: "#FFFFFF", border: "1px solid rgba(255,255,255,0.25)" }}>
          Send it by email or text
        </button>
      </div>
      <p className="mt-3 text-[0.9375rem] font-semibold" style={{ color: "#FFFFFF" }}>
        Laws used
      </p>
      <ul className="mt-1 space-y-3">
        {laws.map((l) => (
          <li key={l.id}>
            <p className="text-[0.9375rem]" style={{ color: "#FFFFFF" }}>
              {l.citation}
              {!l.complete && <span style={{ color: "rgba(255,255,255,0.78)" }}> (the answer used passages)</span>}
            </p>
            <div className="mt-1 flex flex-wrap gap-2">
              <a href={lawTextUrl(l.id)} target="_blank" rel="noopener noreferrer" className={BIG_BUTTON} style={{ background: "rgba(255,255,255,0.10)", color: "#FFFFFF" }}>
                Read the full text
              </a>
              <a href={lawTextUrl(l.id, true)} className={BIG_BUTTON} style={{ background: "rgba(255,255,255,0.10)", color: "#FFFFFF" }}>
                Download
              </a>
              <button type="button" onClick={() => shareLaw(l)} className={BIG_BUTTON} style={{ background: "rgba(255,255,255,0.10)", color: "#FFFFFF" }}>
                Share
              </button>
              <a href={l.url} target="_blank" rel="noopener noreferrer" className={BIG_BUTTON} style={{ background: "transparent", color: "#FFFFFF", textDecoration: "underline" }}>
                Official page
              </a>
            </div>
          </li>
        ))}
      </ul>
      {note && (
        <p role="status" className="mt-2 text-[0.9375rem]" style={{ color: "#FFFFFF" }}>
          {note}
        </p>
      )}
    </section>
  );
}

/** Full was chosen, the law is long, and only a preview was used: ask before sending it all. */
export function FullTextConfirm({ chars, onConfirm }: { chars: number; onConfirm: () => void }) {
  return (
    <div role="status" className="mt-3 rounded-xl p-3" style={{ background: LEVELS.coming_up.bg, color: LEVELS.coming_up.fg, border: `2px solid ${LEVELS.coming_up.edge}` }}>
      <p className="flex items-center gap-2 text-[1rem] font-semibold">
        <Icon name="warning" /> This answer used a preview of the law.
      </p>
      <p className="mt-1 text-[0.9375rem]">
        The full text is {chars.toLocaleString("en-US")} characters, so using all of it makes the answer slower.
      </p>
      <button type="button" onClick={onConfirm} className={`${BIG_BUTTON} mt-2 w-full`} style={{ background: LEVELS.coming_up.fg, color: "#FFFFFF" }}>
        Use the full text
      </button>
    </div>
  );
}

const DETAIL: { value: LawDetail; label: string; hint: string }[] = [
  { value: "brief", label: "Brief", hint: "the parts that matter" },
  { value: "more", label: "More", hint: "more of the law" },
  { value: "full", label: "Full", hint: "the whole law" },
];

/** Law detail: Brief | More | Full. A radio group, so it reads right to a screen reader. */
export function LawDetailToggle({ value, onChange }: { value: LawDetail; onChange: (v: LawDetail) => void }) {
  return (
    <fieldset className="px-4 pb-2">
      <legend className="text-[0.875rem] font-semibold" style={{ color: "var(--hc-ink)" }}>
        How much of the law to use
      </legend>
      <div className="mt-1 grid grid-cols-3 gap-2">
        {DETAIL.map((d) => (
          <label
            key={d.value}
            className="flex min-h-[44px] cursor-pointer flex-col items-center justify-center rounded-xl px-2 py-1 text-center focus-within:outline focus-within:outline-2 focus-within:outline-offset-2 focus-within:outline-white"
            style={
              value === d.value
                ? { background: "#F5F5F7", color: "#1C1C1E", border: "2px solid #F5F5F7" }
                : { background: "transparent", color: "var(--hc-ink)", border: "2px solid rgba(255,255,255,0.35)" }
            }
          >
            <input
              type="radio"
              name="law-detail"
              value={d.value}
              checked={value === d.value}
              onChange={() => onChange(d.value)}
              className="sr-only"
            />
            <span className="text-[0.9375rem] font-semibold">{d.label}</span>
            <span className="text-[0.75rem]">{d.hint}</span>
          </label>
        ))}
      </div>
    </fieldset>
  );
}
