import { NavLink, useLocation } from "react-router";
import { useEffect, useRef, useState } from "react";
import { useTray } from "@/lib/store";

const tabs = [
  {
    to: "/",
    label: "Home",
    icon: (active: boolean) => (
      <svg width="22" height="22" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth={active ? 2.4 : 1.8} strokeLinecap="round" strokeLinejoin="round">
        <path d="M3 10.5 12 3l9 7.5" />
        <path d="M5 9.5V20h4.5v-6h5v6H19V9.5" />
      </svg>
    ),
  },
  {
    to: "/saved",
    label: "Saved",
    icon: (active: boolean) => (
      <svg width="22" height="22" viewBox="0 0 24 24" fill={active ? "currentColor" : "none"} stroke="currentColor" strokeWidth={active ? 2 : 1.8} strokeLinejoin="round">
        <path d="M6 3h12v18l-6-4.5L6 21V3z" />
      </svg>
    ),
  },
  {
    to: "/compare",
    label: "Compare",
    icon: (active: boolean) => (
      <svg width="22" height="22" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth={active ? 2.4 : 1.8} strokeLinecap="round">
        <path d="M9 3v18M15 3v18M3 9h6M15 15h6" />
      </svg>
    ),
  },
  {
    to: "/more",
    label: "About",
    icon: (active: boolean) => (
      <svg width="22" height="22" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth={active ? 2.4 : 1.8} strokeLinecap="round">
        <path d="M4 7h16M4 12h16M4 17h16" />
      </svg>
    ),
  },
];

/**
 * Floating bottom chrome = TWO detached elements (Whoop pattern, locked decision):
 * glass capsule tab bar + separate circular agent orb. The agent is NOT a tab.
 * Hides on scroll down, returns on scroll up (iOS convention).
 */
export function NavChrome({ onOpenAgent }: { onOpenAgent: () => void }) {
  const [hidden, setHidden] = useState(false);
  const lastY = useRef(0);
  const tray = useTray();
  const location = useLocation();

  useEffect(() => {
    lastY.current = window.scrollY;
    const onScroll = () => {
      const y = window.scrollY;
      const dy = y - lastY.current;
      if (y < 24) setHidden(false);
      else if (dy > 8) setHidden(true);
      else if (dy < -8) setHidden(false);
      lastY.current = y;
    };
    window.addEventListener("scroll", onScroll, { passive: true });
    return () => window.removeEventListener("scroll", onScroll);
  }, []);

  return (
    <nav
      aria-label="Primary"
      className="hc-anim pointer-events-none fixed inset-x-0 bottom-4 z-40 flex items-center justify-center gap-2.5 px-4 transition-transform duration-300"
      style={{ transform: hidden ? "translateY(120%)" : "translateY(0)" }}
    >
      <div className="glass-nav pointer-events-auto flex h-16 items-stretch rounded-full px-2">
        {tabs.map((t) => (
          <NavLink
            key={t.to}
            to={t.to}
            end={t.to === "/"}
            className="relative flex w-[68px] flex-col items-center justify-center gap-0.5 rounded-full"
          >
            {({ isActive }) => (
              <>
                <span style={{ color: isActive ? "#3A3A3C" : "rgba(58, 58, 60, 0.5)" }}>
                  {t.icon(isActive)}
                </span>
                <span
                  className="text-[11px] font-medium"
                  style={{ color: isActive ? "#3A3A3C" : "rgba(58, 58, 60, 0.5)" }}
                >
                  {t.label}
                </span>
                {t.to === "/compare" && tray.length > 0 && (
                  <span
                    className="absolute right-2.5 top-2 flex h-4 min-w-4 items-center justify-center rounded-full px-1 text-[10px] font-semibold text-white"
                    style={{ background: "var(--hc-ink)" }}
                  >
                    {tray.length}
                  </span>
                )}
              </>
            )}
          </NavLink>
        ))}
      </div>

      <button
        onClick={onOpenAgent}
        aria-label="Open HouseCheck agent"
        className="glass-orb pointer-events-auto flex h-14 w-14 items-center justify-center rounded-full"
      >
        <svg width="24" height="24" viewBox="0 0 24 24" fill="#3A3A3C" aria-hidden>
          <path d="M12 2l2.1 7.9L22 12l-7.9 2.1L12 22l-2.1-7.9L2 12l7.9-2.1L12 2z" />
        </svg>
      </button>
      <span className="sr-only">Current: {location.pathname}</span>
    </nav>
  );
}
