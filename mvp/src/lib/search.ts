import { BUILDINGS, type BuildingRecord } from "@/data/buildings";

function normalize(q: string): string {
  return q
    .toLowerCase()
    .replace(/[.,#]/g, " ")
    .replace(/\b(avenue|ave)\b/g, "ave")
    .replace(/\b(street|st)\b/g, "st")
    .replace(/\b(brooklyn|ny|nyc)\b/g, "")
    .replace(/\s+/g, " ")
    .trim();
}

export function searchBuildings(query: string): BuildingRecord[] {
  const q = normalize(query);
  if (!q || q.length < 2) return [];

  return BUILDINGS.filter((b) => {
    const haystack = normalize(
      [b.address, b.neighborhood, b.zip, ...b.searchAliases].join(" "),
    );
    return haystack.includes(q) || q.includes(normalize(b.address));
  });
}

export function findBuilding(query: string): BuildingRecord | null {
  const matches = searchBuildings(query);
  if (matches.length === 1) return matches[0];
  const exact = matches.find((b) =>
    b.searchAliases.some((a) => normalize(a) === normalize(query)),
  );
  return exact ?? matches[0] ?? null;
}

export const EXAMPLE_ADDRESSES = BUILDINGS.map((b) => ({
  id: b.id,
  label: b.address,
  neighborhood: b.neighborhood,
}));
