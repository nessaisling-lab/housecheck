import { createContext, useCallback, useContext, useState, type ReactNode } from "react";
import type { BuildingCard } from "@/types/building";

interface AgentCtx {
  /** building the agent should answer about, if any */
  building: BuildingCard | null;
  setBuilding: (b: BuildingCard | null) => void;
  open: boolean;
  openAgent: () => void;
  closeAgent: () => void;
}

const Ctx = createContext<AgentCtx>({
  building: null,
  setBuilding: () => {},
  open: false,
  openAgent: () => {},
  closeAgent: () => {},
});

export function AgentProvider({ children }: { children: ReactNode }) {
  const [building, setBuilding] = useState<BuildingCard | null>(null);
  const [open, setOpen] = useState(false);
  const openAgent = useCallback(() => setOpen(true), []);
  const closeAgent = useCallback(() => setOpen(false), []);
  return (
    <Ctx.Provider value={{ building, setBuilding, open, openAgent, closeAgent }}>
      {children}
    </Ctx.Provider>
  );
}

export function useAgent() {
  return useContext(Ctx);
}
