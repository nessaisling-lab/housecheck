import { useNavigate } from "react-router";
import { MiniRing } from "@/components/ScoreRing";
import { bandMeta } from "@/lib/score";
import { store, useRecents, useSaved, useTray } from "@/lib/store";

function Row({
  bbl,
  address,
  score,
  saved,
  onToggleSave,
}: {
  bbl: string;
  address: string;
  score: number | null;
  saved: boolean;
  onToggleSave: () => void;
}) {
  const navigate = useNavigate();
  const band = bandMeta(score);
  return (
    <div className="hc-card flex items-center gap-3 p-3.5">
      <button onClick={() => navigate(`/building/${bbl}`)} className="flex flex-1 items-center gap-3 text-left">
        <MiniRing score={score} size={44} stroke={5} />
        <span>
          <span className="block text-[1rem] font-semibold" style={{ color: "var(--hc-ink)" }}>
            {address}
          </span>
          <span className="block text-[0.8125rem]" style={{ color: "var(--hc-ink-2)" }}>
            Bed-Stuy · {band.short}
          </span>
        </span>
      </button>
      <button
        onClick={onToggleSave}
        aria-label={saved ? "Remove from saved" : "Save building"}
        aria-pressed={saved}
        className="p-2"
        style={{ color: saved ? "var(--hc-ink)" : "var(--hc-ink-3)" }}
      >
        <svg width="20" height="20" viewBox="0 0 24 24" fill={saved ? "currentColor" : "none"} stroke="currentColor" strokeWidth="1.8" strokeLinejoin="round">
          <path d="M6 3h12v18l-6-4.5L6 21V3z" />
        </svg>
      </button>
    </div>
  );
}

export default function Saved() {
  const navigate = useNavigate();
  const recents = useRecents();
  const saved = useSaved();
  const tray = useTray();

  return (
    <div className="mx-auto min-h-dvh w-full max-w-md px-5 pb-36 pt-14">
      <h1 className="text-[1.875rem] font-semibold tracking-tight" style={{ color: "var(--hc-canvas-ink)" }}>
        Saved
      </h1>

      <h2 className="hc-eyebrow mt-8" style={{ color: "var(--hc-canvas-ink-3)" }}>
        Recent searches
      </h2>
      <div className="mt-3 space-y-2.5">
        {recents.length === 0 && (
          <p className="rounded-2xl p-4 text-[0.875rem]" style={{ background: "var(--hc-sunken)", color: "var(--hc-ink-2)" }}>
            Buildings you look up will appear here.
          </p>
        )}
        {recents.map((r) => (
          <Row
            key={r.bbl}
            bbl={r.bbl}
            address={r.address}
            score={r.score}
            saved={store.isSaved(r.bbl)}
            onToggleSave={() => store.toggleSave(r)}
          />
        ))}
      </div>

      <h2 className="hc-eyebrow mt-8" style={{ color: "var(--hc-canvas-ink-3)" }}>
        Saved for compare · {tray.length} of 4
      </h2>
      <div className="mt-3 space-y-2.5">
        {saved.length === 0 && (
          <p className="rounded-2xl p-4 text-[0.875rem]" style={{ background: "var(--hc-sunken)", color: "var(--hc-ink-2)" }}>
            Tap the bookmark on any building to save it here — saved buildings feed Compare.
          </p>
        )}
        {saved.map((r) => (
          <Row
            key={r.bbl}
            bbl={r.bbl}
            address={r.address}
            score={r.score}
            saved
            onToggleSave={() => {
              store.toggleSave(r);
              store.removeFromTray(r.bbl);
            }}
          />
        ))}
      </div>

      {tray.length >= 2 && (
        <button
          onClick={() => navigate("/compare")}
          className="hc-anim glass-dark fixed inset-x-0 z-30 mx-auto w-[calc(100%-2.5rem)] max-w-md rounded-full py-4 text-center text-[1rem] font-semibold text-white"
          style={{ bottom: "calc(6rem + var(--hc-safe-bottom))" }}
        >
          Compare {tray.length} buildings →
        </button>
      )}
    </div>
  );
}
