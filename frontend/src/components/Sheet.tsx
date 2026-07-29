import { useEffect, useRef, type ReactNode } from "react";

const FOCUSABLE =
  'a[href],button:not([disabled]),input:not([disabled]),select:not([disabled]),' +
  'textarea:not([disabled]),[tabindex]:not([tabindex="-1"])';

interface SheetProps {
  open: boolean;
  onClose: () => void;
  children: ReactNode;
  /** extra classes on the sheet panel */
  className?: string;
  labelledBy?: string;
}

/**
 * Bottom sheet — glass/sheet material, rounded top 24, drag handle,
 * 35% dimming layer behind (Apple hard rules, design-strategy §3.2).
 */
export function Sheet({ open, onClose, children, className = "", labelledBy }: SheetProps) {
  const panelRef = useRef<HTMLDivElement>(null);

  // WCAG 2.2 AA 2.4.3 (focus order) + 2.1.2 (no keyboard trap outside the
  // dialog). aria-modal alone hides the page from a screen reader but does
  // nothing for the Tab key, so a keyboard user could tab out of an open
  // sheet into content they cannot see. Three things are needed:
  // move focus in, cycle it inside, and give it back on close.
  useEffect(() => {
    if (!open) return;

    const opener = document.activeElement as HTMLElement | null;

    // Prefer the first text input (the agent sheet's composer) so the user can
    // simply start typing; otherwise the first control in the panel.
    //
    // Done synchronously: the DOM is committed by the time an effect runs, so
    // there is nothing to wait for. An earlier version deferred this to
    // requestAnimationFrame to let the slide-up animation settle, which meant
    // focus silently never moved whenever rAF was throttled — a background
    // tab, or a browser that isn't compositing.
    const panel = panelRef.current;
    if (panel) {
      const items = [...panel.querySelectorAll<HTMLElement>(FOCUSABLE)];
      (items.find((el) => el instanceof HTMLInputElement) ?? items[0])?.focus();
    }

    const onKey = (e: KeyboardEvent) => {
      if (e.key === "Escape") {
        onClose();
        return;
      }
      if (e.key !== "Tab") return;
      const panel = panelRef.current;
      if (!panel) return;
      const items = [...panel.querySelectorAll<HTMLElement>(FOCUSABLE)].filter(
        (el) => el.offsetParent !== null
      );
      if (items.length === 0) return;
      const first = items[0];
      const last = items[items.length - 1];
      const active = document.activeElement;
      // Wrap at both ends, and pull focus back in if it has escaped the panel.
      if (e.shiftKey && (active === first || !panel.contains(active))) {
        e.preventDefault();
        last.focus();
      } else if (!e.shiftKey && (active === last || !panel.contains(active))) {
        e.preventDefault();
        first.focus();
      }
    };

    document.addEventListener("keydown", onKey);
    document.body.style.overflow = "hidden";
    return () => {
      document.removeEventListener("keydown", onKey);
      document.body.style.overflow = "";
      // Return focus to whatever opened the sheet (the orb, a card, a button).
      // isConnected guards the case where that element unmounted meanwhile.
      if (opener?.isConnected) opener.focus();
    };
  }, [open, onClose]);

  if (!open) return null;
  return (
    <div className="fixed inset-0 z-50 hc-anim" role="dialog" aria-modal="true" aria-labelledby={labelledBy}>
      <div
        className="absolute inset-0"
        style={{ background: "rgba(0,0,0,0.35)", animation: "hc-fade-in 0.2s ease-out" }}
        onClick={onClose}
      />
      <div
        ref={panelRef}
        className={`glass-sheet absolute inset-x-0 bottom-0 mx-auto flex max-h-[92dvh] w-full max-w-md flex-col overflow-hidden rounded-t-3xl ${className}`}
        style={{ animation: "hc-sheet-up 0.32s cubic-bezier(0.22,1,0.36,1)" }}
      >
        <button
          className="mx-auto mt-2.5 h-1.5 w-10 shrink-0 rounded-full"
          style={{ background: "rgba(60,60,67,0.25)" }}
          onClick={onClose}
          aria-label="Close sheet"
        />
        {children}
      </div>
    </div>
  );
}
