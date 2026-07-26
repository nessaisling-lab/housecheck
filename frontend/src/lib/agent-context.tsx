/* eslint-disable react-refresh/only-export-components -- standard context provider + hook pairing */
import { createContext, useCallback, useContext, useState, type ReactNode } from "react";
import type { BuildingCard, RentFairnessResult } from "@/types/building";

interface AgentCtx {
  /** building the agent should answer about, if any */
  building: BuildingCard | null;
  setBuilding: (b: BuildingCard | null) => void;
  /**
   * Result of a rent-fairness check the user has actually run, if any.
   *
   * The backend's `HealthCard` carries no rent context — `BuildingCard.rent` is
   * always null on live data and only populated by the bundled demo fixtures. The
   * real tract median arrives from `POST /rent-fairness`, which the Health Card
   * calls. Without this the agent could never answer a rent question on live data.
   */
  rent: RentFairnessResult | null;
  setRent: (r: RentFairnessResult | null) => void;
  open: boolean;
  openAgent: () => void;
  closeAgent: () => void;
}

const Ctx = createContext<AgentCtx>({
  building: null,
  setBuilding: () => {},
  rent: null,
  setRent: () => {},
  open: false,
  openAgent: () => {},
  closeAgent: () => {},
});

export function AgentProvider({ children }: { children: ReactNode }) {
  const [building, setBuilding] = useState<BuildingCard | null>(null);
  const [rent, setRent] = useState<RentFairnessResult | null>(null);
  const [open, setOpen] = useState(false);
  const openAgent = useCallback(() => setOpen(true), []);
  const closeAgent = useCallback(() => setOpen(false), []);
  return (
    <Ctx.Provider value={{ building, setBuilding, rent, setRent, open, openAgent, closeAgent }}>
      {children}
    </Ctx.Provider>
  );
}

export function useAgent() {
  return useContext(Ctx);
}
