export type PriorityId =
  | "condition"
  | "rent"
  | "protections"
  | "access";

export type PriorityDef = {
  id: PriorityId;
  label: string;
  short: string;
  prompt: string;
};

export const PRIORITIES: PriorityDef[] = [
  {
    id: "condition",
    label: "Building condition & safety",
    short: "Condition",
    prompt: "Avoid hazardous buildings and open Class C violations.",
  },
  {
    id: "rent",
    label: "Rent fairness",
    short: "Rent",
    prompt: "Stay near or below the neighborhood median for the area.",
  },
  {
    id: "protections",
    label: "Legal protections",
    short: "Protections",
    prompt: "Prefer rent-stabilized or Good Cause–covered buildings.",
  },
  {
    id: "access",
    label: "Step-free access likelihood",
    short: "Access",
    prompt: "Elevator on record and fewer walk-up barriers.",
  },
];

export function priorityById(id: PriorityId): PriorityDef {
  const found = PRIORITIES.find((p) => p.id === id);
  if (!found) throw new Error(`Unknown priority: ${id}`);
  return found;
}
