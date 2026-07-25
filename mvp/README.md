# HouseCheck

Mobile-first web app that turns any Brooklyn address into an instant **Building Health Card** — a 0–100 score with building condition, legal protections, neighborhood context, and rent fairness.

## MVP value props

1. **Vitamin** — Address → violation history + health score in seconds  
2. **Painkiller** — One screen instead of HPD Online + DHCR + Census  
3. **Steroid** — Enter quoted rent → % above/below neighborhood median  

## Run locally

```bash
npm install
npm run dev
```

Open [http://localhost:3000](http://localhost:3000).

## Demo addresses

- 245 DeKalb Avenue (Fort Greene) — strong score, rent-stabilized, elevator  
- 582 Gates Avenue (Bed-Stuy) — open Class C hazards  
- 91 Hicks Street (Brooklyn Heights) — clean + stabilized  
- 1402 Mermaid Avenue (Coney Island) — mixed  
- 318 Grand Street (Williamsburg) — newer, not stabilized  

## Stack

Next.js (App Router) · TypeScript · Tailwind CSS · mock public-data layer for MVP  

Live HPD / DHCR / ACS wiring is the next integration step; every displayed number already links to its government source.
