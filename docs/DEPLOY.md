# HouseCheck — Deployment

## Backend → Fly.io

The serving DB is a **read-only artifact baked into the image**, so the running API needs
**no secrets**. (Secrets are only used at ingest time, which happens on your machine.)

### 1. Build the data (once, or whenever you refresh)
```powershell
$env:CENSUS_API_KEY = [Environment]::GetEnvironmentVariable('CENSUS_API_KEY','Machine')
$env:NYC_APP_TOKEN  = [Environment]::GetEnvironmentVariable('NYC_APP_TOKEN','Machine')
cargo run -p ingest -- --real --cd 303 --limit 250 --out data/housecheck.db
```

### 2. Deploy
```bash
fly launch --no-deploy      # first time only — accepts fly.toml, creates the app
fly deploy                  # builds the Docker image (bakes data/housecheck.db) + ships it
```
`fly.toml` scales to zero when idle (≈ free) and auto-starts on the first request. To refresh
the data later: re-run step 1, then `fly deploy` again.

### 3. Verify
```bash
curl https://housecheck.fly.dev/health
curl https://housecheck.fly.dev/buildings | head
curl https://housecheck.fly.dev/building/3015990007
```

## Frontend → Vercel

The Dioxus/WASM app is a static build; point Vercel at the `frontend/` dir. Set the API base
URL (the Fly URL above) as a build env var. Tighten the API's CORS to the Vercel origin before
launch (currently permissive for local dev).

## Cost
Data APIs $0 · Fly.io ~$0–5/mo (scale-to-zero) · map tiles via MapLibre + Protomaps $0.
Well within the $20–50 budget. See PRD Appendix F.

## Secrets (only if you add features that need them)
- `/summary` (optional LLM) → `fly secrets set OPENROUTER_API_KEY=...`
- Re-ingest inside CI/cloud (not needed today) → `CENSUS_API_KEY`, `NYC_APP_TOKEN`
Never commit keys; local dev uses the OS keychain / machine env.
