export type ViolationClass = "A" | "B" | "C";

export type Violation = {
  id: string;
  class: ViolationClass;
  description: string;
  status: "Open" | "Closed";
  issued: string;
  sourceUrl: string;
};

export type BuildingRecord = {
  id: string;
  address: string;
  borough: "Brooklyn";
  neighborhood: string;
  zip: string;
  bbl: string;
  units: number;
  yearBuilt: number;
  stories: number;
  hasElevator: boolean | null;
  violations: Violation[];
  rentStabilized: boolean | "unknown";
  goodCauseLikely: boolean;
  neighborhoodMedianRent: number;
  medianRentSource: {
    label: string;
    url: string;
    asOf: string;
  };
  hpdProfileUrl: string;
  dataAsOf: string;
  searchAliases: string[];
};

const HPD = "https://hpdonline.nyc.gov/hpdonline";
const DHCR = "https://hcr.ny.gov/rent-stabilization";
const CENSUS = "https://data.census.gov";

export const BUILDINGS: BuildingRecord[] = [
  {
    id: "245-dekalb",
    address: "245 DeKalb Avenue",
    borough: "Brooklyn",
    neighborhood: "Fort Greene",
    zip: "11205",
    bbl: "3019610023",
    units: 24,
    yearBuilt: 1925,
    stories: 6,
    hasElevator: true,
    violations: [
      {
        id: "v1",
        class: "A",
        description: "Paint peeling in public hallway",
        status: "Closed",
        issued: "2024-11-12",
        sourceUrl: HPD,
      },
      {
        id: "v2",
        class: "B",
        description: "Self-closing door defective at roof bulkhead",
        status: "Closed",
        issued: "2023-06-02",
        sourceUrl: HPD,
      },
    ],
    rentStabilized: true,
    goodCauseLikely: true,
    neighborhoodMedianRent: 3200,
    medianRentSource: {
      label: "ACS 5-year median gross rent, Fort Greene PUMA",
      url: CENSUS,
      asOf: "2024-12-01",
    },
    hpdProfileUrl: HPD,
    dataAsOf: "2026-07-01",
    searchAliases: [
      "245 dekalb",
      "245 dekalb ave",
      "245 dekalb avenue",
      "245 dekalb avenue brooklyn",
    ],
  },
  {
    id: "582-gates",
    address: "582 Gates Avenue",
    borough: "Brooklyn",
    neighborhood: "Bedford-Stuyvesant",
    zip: "11221",
    bbl: "3016280041",
    units: 8,
    yearBuilt: 1931,
    stories: 4,
    hasElevator: false,
    violations: [
      {
        id: "v1",
        class: "C",
        description: "Heat not provided — inadequate heat",
        status: "Open",
        issued: "2026-01-18",
        sourceUrl: HPD,
      },
      {
        id: "v2",
        class: "C",
        description: "Lead-based paint hazard — peeling in dwelling unit",
        status: "Open",
        issued: "2025-09-04",
        sourceUrl: HPD,
      },
      {
        id: "v3",
        class: "B",
        description: "Mold condition in bathroom",
        status: "Open",
        issued: "2025-11-22",
        sourceUrl: HPD,
      },
      {
        id: "v4",
        class: "B",
        description: "Broken or defective plastered surfaces",
        status: "Closed",
        issued: "2024-03-14",
        sourceUrl: HPD,
      },
      {
        id: "v5",
        class: "C",
        description: "Hot water not provided",
        status: "Closed",
        issued: "2024-01-09",
        sourceUrl: HPD,
      },
    ],
    rentStabilized: "unknown",
    goodCauseLikely: true,
    neighborhoodMedianRent: 2450,
    medianRentSource: {
      label: "ACS 5-year median gross rent, Bed-Stuy PUMA",
      url: CENSUS,
      asOf: "2024-12-01",
    },
    hpdProfileUrl: HPD,
    dataAsOf: "2026-07-01",
    searchAliases: [
      "582 gates",
      "582 gates ave",
      "582 gates avenue",
      "582 gates avenue brooklyn",
    ],
  },
  {
    id: "91-hicks",
    address: "91 Hicks Street",
    borough: "Brooklyn",
    neighborhood: "Brooklyn Heights",
    zip: "11201",
    bbl: "3002200015",
    units: 42,
    yearBuilt: 1920,
    stories: 8,
    hasElevator: true,
    violations: [
      {
        id: "v1",
        class: "A",
        description: "Missing apartment identification sign",
        status: "Closed",
        issued: "2025-02-20",
        sourceUrl: HPD,
      },
    ],
    rentStabilized: true,
    goodCauseLikely: true,
    neighborhoodMedianRent: 4100,
    medianRentSource: {
      label: "ACS 5-year median gross rent, Brooklyn Heights PUMA",
      url: CENSUS,
      asOf: "2024-12-01",
    },
    hpdProfileUrl: HPD,
    dataAsOf: "2026-07-01",
    searchAliases: [
      "91 hicks",
      "91 hicks st",
      "91 hicks street",
      "91 hicks street brooklyn",
    ],
  },
  {
    id: "1402-mermaid",
    address: "1402 Mermaid Avenue",
    borough: "Brooklyn",
    neighborhood: "Coney Island",
    zip: "11224",
    bbl: "3070240032",
    units: 36,
    yearBuilt: 1963,
    stories: 6,
    hasElevator: true,
    violations: [
      {
        id: "v1",
        class: "B",
        description: "Window guard missing or defective",
        status: "Open",
        issued: "2025-08-11",
        sourceUrl: HPD,
      },
      {
        id: "v2",
        class: "A",
        description: "Litter condition in courtyard",
        status: "Closed",
        issued: "2025-04-03",
        sourceUrl: HPD,
      },
      {
        id: "v3",
        class: "B",
        description: "Plumbing leak — bathroom ceiling",
        status: "Closed",
        issued: "2024-10-19",
        sourceUrl: HPD,
      },
    ],
    rentStabilized: true,
    goodCauseLikely: true,
    neighborhoodMedianRent: 2100,
    medianRentSource: {
      label: "ACS 5-year median gross rent, Coney Island PUMA",
      url: CENSUS,
      asOf: "2024-12-01",
    },
    hpdProfileUrl: HPD,
    dataAsOf: "2026-07-01",
    searchAliases: [
      "1402 mermaid",
      "1402 mermaid ave",
      "1402 mermaid avenue",
      "1402 mermaid avenue brooklyn",
    ],
  },
  {
    id: "318-grand",
    address: "318 Grand Street",
    borough: "Brooklyn",
    neighborhood: "Williamsburg",
    zip: "11211",
    bbl: "3023860018",
    units: 16,
    yearBuilt: 2008,
    stories: 5,
    hasElevator: false,
    violations: [
      {
        id: "v1",
        class: "A",
        description: "Smoke detector missing or defective",
        status: "Closed",
        issued: "2023-09-28",
        sourceUrl: HPD,
      },
    ],
    rentStabilized: false,
    goodCauseLikely: false,
    neighborhoodMedianRent: 4500,
    medianRentSource: {
      label: "ACS 5-year median gross rent, Williamsburg PUMA",
      url: CENSUS,
      asOf: "2024-12-01",
    },
    hpdProfileUrl: HPD,
    dataAsOf: "2026-07-01",
    searchAliases: [
      "318 grand",
      "318 grand st",
      "318 grand street",
      "318 grand street brooklyn",
    ],
  },
];

export const SOURCE_LINKS = {
  hpd: { label: "HPD Online", url: HPD },
  dhcr: { label: "NYS HCR / DHCR", url: DHCR },
  census: { label: "U.S. Census ACS", url: CENSUS },
  goodCause: {
    label: "NYC Good Cause Eviction",
    url: "https://www.nyc.gov/site/hpd/services-and-information/good-cause-eviction.page",
  },
} as const;
