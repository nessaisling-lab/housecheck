import { useEffect, type ReactNode } from "react";

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
  useEffect(() => {
    if (!open) return;
    const onKey = (e: KeyboardEvent) => e.key === "Escape" && onClose();
    document.addEventListener("keydown", onKey);
    document.body.style.overflow = "hidden";
    return () => {
      document.removeEventListener("keydown", onKey);
      document.body.style.overflow = "";
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
