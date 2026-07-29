import { useSyncExternalStore } from "react";

// localStorage-backed Saved / Recent / Compare tray / Onboarding (no accounts — design decision #3)

export interface SavedEntry {
  bbl: string;
  address: string;
  score: number | null;
  neighborhood?: string | null;
  at: number;
}

/** Onboarding priorities — map to Health Card sections */
export type Priority = "rent" | "condition" | "legal" | "access" | "neighborhood";

export interface OnboardingState {
  done: boolean;
  skipped: boolean;
  priorities: Priority[];
}

const KEYS = {
  saved: "hc.saved.v1",
  recent: "hc.recent.v1",
  tray: "hc.compareTray.v1",
  onboarding: "hc.onboarding.v1",
  priorityCounts: "hc.priorityCounts.v1",
  textSize: "hc.textSize.v1",
} as const;

/** Reader text size (WCAG 2.2 AA 1.4.4). Scales the root font size; every
 *  type size in the app is rem-based, so the whole UI grows with it. */
export type TextSize = "sm" | "md" | "lg" | "xl";

export const TEXT_SCALE: Record<TextSize, number> = {
  sm: 0.875,
  md: 1,
  lg: 1.125,
  xl: 1.25,
};

export const TEXT_SIZE_LABEL: Record<TextSize, string> = {
  sm: "Small",
  md: "Default",
  lg: "Large",
  xl: "Larger",
};

const MAX_RECENT = 8;
export const MAX_COMPARE = 4;
export const MAX_PRIORITIES = 2;

const ONBOARDING_EMPTY: OnboardingState = {
  done: false,
  skipped: false,
  priorities: [],
};

// ── Cache layer ─────────────────────────────────────────────────────────
// getSnapshot for useSyncExternalStore must return a stable reference
// between writes, so parsed values are cached and invalidated on write
// (or on a "storage" event from another tab).
const cache = new Map<string, unknown>();

const listeners = new Set<() => void>();
function emit() {
  listeners.forEach((l) => l());
}
function subscribe(cb: () => void) {
  listeners.add(cb);
  const onStorage = (e: StorageEvent) => {
    if (e.key) cache.delete(e.key);
    cb();
  };
  window.addEventListener("storage", onStorage);
  return () => {
    listeners.delete(cb);
    window.removeEventListener("storage", onStorage);
  };
}

function read<T>(key: string, fallback: T): T {
  if (cache.has(key)) return cache.get(key) as T;
  let value = fallback;
  try {
    const raw = localStorage.getItem(key);
    if (raw) value = JSON.parse(raw) as T;
  } catch {
    /* corrupted entry → fallback */
  }
  cache.set(key, value);
  return value;
}
function write(key: string, value: unknown) {
  try {
    localStorage.setItem(key, JSON.stringify(value));
  } catch {
    /* storage full/blocked → keep in-memory only */
  }
  cache.set(key, value);
  emit();
}

// ── Store ───────────────────────────────────────────────────────────────
export const store = {
  saved(): SavedEntry[] {
    return read<SavedEntry[]>(KEYS.saved, []);
  },
  recents(): SavedEntry[] {
    return read<SavedEntry[]>(KEYS.recent, []);
  },
  tray(): string[] {
    return read<string[]>(KEYS.tray, []);
  },
  isSaved(bbl: string) {
    return store.saved().some((e) => e.bbl === bbl);
  },
  toggleSave(entry: Omit<SavedEntry, "at">) {
    const list = store.saved();
    if (list.some((e) => e.bbl === entry.bbl)) {
      write(KEYS.saved, list.filter((e) => e.bbl !== entry.bbl));
      return false;
    }
    write(KEYS.saved, [{ ...entry, at: Date.now() }, ...list]);
    return true;
  },
  addRecent(entry: Omit<SavedEntry, "at">) {
    const list = store.recents().filter((e) => e.bbl !== entry.bbl);
    write(KEYS.recent, [{ ...entry, at: Date.now() }, ...list].slice(0, MAX_RECENT));
  },
  inTray(bbl: string) {
    return store.tray().includes(bbl);
  },
  addToTray(bbl: string): { ok: boolean; reason?: string } {
    const t = store.tray();
    if (t.includes(bbl)) return { ok: true };
    if (t.length >= MAX_COMPARE)
      return { ok: false, reason: `Compare holds ${MAX_COMPARE} — remove one first.` };
    write(KEYS.tray, [...t, bbl]);
    return { ok: true };
  },
  removeFromTray(bbl: string) {
    write(KEYS.tray, store.tray().filter((x) => x !== bbl));
  },
  clearTray() {
    write(KEYS.tray, []);
  },

  // ── Onboarding (P1) ──────────────────────────────────────────────────
  onboarding(): OnboardingState {
    return read<OnboardingState>(KEYS.onboarding, ONBOARDING_EMPTY);
  },
  /**
   * Finish onboarding. `null` = Skip (no priorities, still marks done).
   * Each picked priority increments a local aggregate counter.
   */
  completeOnboarding(priorities: Priority[] | null) {
    const picked = priorities?.slice(0, MAX_PRIORITIES) ?? [];
    write(KEYS.onboarding, {
      done: true,
      skipped: priorities === null,
      priorities: picked,
    } satisfies OnboardingState);
    if (picked.length > 0) {
      const counts = { ...store.priorityCounts() };
      picked.forEach((p) => {
        counts[p] = (counts[p] ?? 0) + 1;
      });
      write(KEYS.priorityCounts, counts);
    }
  },
  /** Local aggregate of priority picks (this device only). */
  priorityCounts(): Partial<Record<Priority, number>> {
    return read<Partial<Record<Priority, number>>>(KEYS.priorityCounts, {});
  },

  // ── Reader text size (WCAG 1.4.4) ────────────────────────────────────
  textSize(): TextSize {
    const v = read<TextSize>(KEYS.textSize, "md");
    return v in TEXT_SCALE ? v : "md";
  },
  setTextSize(size: TextSize) {
    write(KEYS.textSize, size);
  },
};

/**
 * Apply the stored size to the document root.
 *
 * Browsers let the user set a default font size, and 1.4.4 is about honouring
 * it — so this multiplies that preference rather than replacing it. Writing a
 * hard `16px * scale` here would override the very setting the criterion
 * exists to protect.
 */
export function applyTextSize(size: TextSize = store.textSize()) {
  document.documentElement.style.fontSize = `${TEXT_SCALE[size] * 100}%`;
}

// ── React bindings ──────────────────────────────────────────────────────
function useStore<T>(pick: () => T): T {
  return useSyncExternalStore(subscribe, pick, pick);
}

export function useSaved(): SavedEntry[] {
  return useStore(store.saved);
}
export function useRecents(): SavedEntry[] {
  return useStore(store.recents);
}
export function useTray(): string[] {
  return useStore(store.tray);
}
export function useIsSaved(bbl: string | undefined): boolean {
  const saved = useSaved();
  return !!bbl && saved.some((e) => e.bbl === bbl);
}
export function useOnboarding(): OnboardingState {
  return useStore(store.onboarding);
}
export function usePriorityCounts(): Partial<Record<Priority, number>> {
  return useStore(store.priorityCounts);
}
export function useTextSize(): TextSize {
  return useStore(store.textSize);
}
