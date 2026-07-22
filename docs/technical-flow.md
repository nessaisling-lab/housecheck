# HouseCheck — Technical Flow Diagrams

Companion to `docs/superpowers/specs/2026-07-21-housecheck-design.md`. Diagrams render on GitHub / any Mermaid viewer.

Planned repo layout (referenced throughout):

```
housecheck/
├─ Cargo.toml            # Rust workspace
├─ crates/
│  ├─ ingest/            # DuckDB → SQLite/SpatiaLite ETL (binary)
│  ├─ scoring/           # 0–100 Building Health Card engine (lib)
│  └─ api/               # Axum HTTP API (binary: housecheck-api)
├─ data/
│  ├─ raw/               # downloaded open-data CSVs (gitignored)
│  └─ housecheck.db      # built SpatiaLite DB (gitignored; artifact)
├─ frontend/             # Vite + React + TS + Tailwind + Mapbox
└─ .github/workflows/    # ci, security, smoke
```

---

## 1. System / container diagram

```mermaid
flowchart TB
    subgraph SRC["Public data sources (free)"]
        HPD["HPD Violations<br/>wvxf-dwi5"]
        REG["HPD Registrations<br/>tesw-yqqr + feu5-w2e2"]
        C311["311 Requests<br/>erm2-nwe9 (2020+)"]
        REST["DOHMH Restaurants<br/>43nn-pn8j"]
        ACS["Census ACS B25064<br/>tract median rent"]
        PLUTO["PLUTO<br/>BBL + geometry + floors/year"]
        RS["DHCR + JustFix WOW<br/>rent-stabilization"]
        ACC["Accessibility sources<br/>(DOB elevators, MTA, DOT ramps)"]
    end

    subgraph INGEST["crates/ingest (offline, batch)"]
        DUCK["DuckDB ETL<br/>filter → Brooklyn curated set<br/>resolve BBL, join tract, precompute scores"]
    end

    DB[("SpatiaLite DB<br/>buildings·violations·311·<br/>restaurants·acs_rent·rent_stab·<br/>accessibility·scores")]

    subgraph API["crates/api (Axum)"]
        H["GET /health"]
        S["GET /search?address="]
        B["GET /building/{bbl}"]
        RF["POST /rent-fairness"]
        MW["tower: CORS · rate-limit · tracing"]
    end

    SCORE["crates/scoring<br/>weighted 0–100 engine"]

    subgraph FE["frontend (Vite/React/Tailwind/Mapbox)"]
        UI["Address search · Building Health Card ·<br/>rent-fairness input · map/heatmap"]
    end

    GEO["GeoSearch (keyless)<br/>address→BBL, live only"]

    SRC --> DUCK --> DB
    DB --> API
    SCORE -. compiled into .-> API
    S -. live fallback .-> GEO
    API -->|JSON| UI

    subgraph DEPLOY["Deploy"]
        SH["Backend → Shuttle.dev / Fly.io"]
        VE["Frontend → Vercel"]
    end
    API --- SH
    UI --- VE
```

---

## 2. Ingest pipeline (offline, run once + on refresh)

```mermaid
flowchart LR
    A["Download CSVs<br/>Socrata SODA + Census API + PLUTO"] --> B["DuckDB: load raw"]
    B --> C{"Filter to<br/>Brooklyn curated<br/>~50–200 buildings"}
    C --> D["Resolve BBL<br/>(PLUTO join / GeoSearch)"]
    D --> E["Spatial join:<br/>building → census tract<br/>ST_Contains"]
    E --> F["Aggregate per building:<br/>violation counts A/B/C,<br/>311 density, nearby restaurants,<br/>elevator/accessibility flags"]
    F --> G["scoring::compute →<br/>0–100 + sub-scores"]
    G --> H["Write SpatiaLite tables<br/>+ spatial indexes"]
    H --> I["data/housecheck.db<br/>(CI artifact)"]
```

---

## 3. Request flow — address search → Health Card

```mermaid
sequenceDiagram
    actor U as Renter (mobile)
    participant FE as Frontend (Vercel)
    participant API as Axum API
    participant DB as SpatiaLite
    U->>FE: types "123 Example St, Brooklyn"
    FE->>API: GET /search?address=...
    API->>DB: match address → BBL (curated set)
    alt found in curated set
        DB-->>API: BBL + building row
    else not in set (live mode)
        API->>API: GeoSearch → BBL
    end
    API-->>FE: {bbl, address}
    FE->>API: GET /building/{bbl}
    API->>DB: fetch precomputed card + sources
    DB-->>API: score, violations, legal, neighborhood, accessibility
    API-->>FE: Building Health Card JSON
    FE-->>U: animated 0–100 gauge + sections
    U->>FE: enters own rent
    FE->>API: POST /rent-fairness {bbl, monthly_rent}
    API->>DB: tract median (B25064) + HUD FMR
    API-->>FE: "X% above/below neighborhood median"
```

---

## 4. Scoring engine

```mermaid
flowchart TB
    subgraph IN["Inputs (per building)"]
        V["Violations A/B/C<br/>+ issued/closed dates"]
        L["Legal: stabilized? Good Cause?"]
        N["311 density · restaurant grades"]
        AC["Accessibility flags"]
        RENT["User rent (optional)"]
    end
    V --> VS["condition sub-score<br/>severity × recency decay"]
    L --> LS["legal sub-score + badges"]
    N --> NS["neighborhood sub-score"]
    AC --> AS["accessibility sub-score/badge"]
    RENT --> RS["rent-fairness %<br/>vs tract median + HUD FMR"]
    VS --> W["weighted sum → 0–100<br/>(weights documented + user-adjustable)"]
    LS --> W
    NS --> W
    AS --> W
    W --> CARD["Building Health Card<br/>color-coded, 'show the math'"]
    RS -. shown alongside .-> CARD
```

---

## 5. Data model (core tables)

```mermaid
erDiagram
    BUILDINGS ||--o{ VIOLATIONS : has
    BUILDINGS ||--o{ COMPLAINTS_311 : has
    BUILDINGS ||--o{ REGISTRATIONS : has
    BUILDINGS ||--o{ ACCESSIBILITY : has
    BUILDINGS }o--|| ACS_RENT_BY_TRACT : in_tract
    BUILDINGS ||--o| RENT_STAB : may_be
    BUILDINGS ||--|| SCORES : precomputed
    BUILDINGS {
        text bbl PK
        text bin
        text address
        geometry geom
        text tract_geoid FK
        int year_built
        int num_floors
    }
    VIOLATIONS { text bbl FK, text class, date issued, date closed }
    COMPLAINTS_311 { text bbl FK, text complaint_type, date created }
    REGISTRATIONS { text bbl FK, text owner, text agent }
    ACCESSIBILITY { text bbl FK, bool has_elevator, bool near_ada_subway, int curb_ramps_nearby, bool fha_era }
    ACS_RENT_BY_TRACT { text tract_geoid PK, int median_gross_rent }
    RENT_STAB { text bbl FK, text source, text confidence }
    SCORES { text bbl FK, int total, int condition, int legal, int neighborhood, int accessibility }
```

---

## 6. CI / CD pipeline

```mermaid
flowchart LR
    PR["push / PR"] --> CI
    subgraph CI["ci.yml — matrix: ubuntu · macOS · windows"]
        FMT["rustfmt + clippy"] --> BUILD["cargo build --workspace"]
        BUILD --> UT["cargo test + vitest"]
        FEB["frontend build (vite)"]
    end
    PR --> SEC
    subgraph SEC["security.yml"]
        AUD["cargo-audit / cargo-deny"]
        GL["gitleaks (secrets)"]
        NPM["npm audit"]
        CQL["CodeQL (js/ts)"]
    end
    PR --> SMK
    subgraph SMK["smoke.yml"]
        BOOT["boot API on fixture DB"] --> HIT["curl /health + /building/{known bbl}"]
        HIT --> ASSERT["assert 200 + score∈[0,100]"]
        ASSERT --> STAB["stability: repeat 20× no flake"]
    end
    CI --> GATE{"all green?"}
    SEC --> GATE
    SMK --> GATE
    GATE -->|yes| DEPLOY["Shuttle/Fly + Vercel"]
```
