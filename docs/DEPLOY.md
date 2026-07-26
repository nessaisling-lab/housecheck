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
curl https://housecheck-nessa.fly.dev/health
curl https://housecheck-nessa.fly.dev/buildings | head
curl https://housecheck-nessa.fly.dev/building/3014800023
```

> **Note:** the binary is `flyctl`, not `fly` — `fly` is not on PATH on Windows, and
> `flyctl apps resume` is deprecated in favour of `flyctl scale count`.

## Frontend → Vercel

**Live:** https://housecheck-wine.vercel.app

The React/Vite app is a static build; point Vercel at the `frontend/` dir. Vercel is **not**
connected to git here, so pushing to `main` does not redeploy. Ship it explicitly:
```bash
cd frontend && npx vercel --prod
```
`vite.config.ts` must keep `base: '/'`. With `base: './'` the emitted asset paths are relative,
so on a nested route like `/building/{bbl}` the browser requests `/building/assets/*.js`, no
static file matches, the SPA rewrite returns `index.html`, and the deep link renders blank with
a MIME-type error.

Tighten the API's CORS to the Vercel origin at launch by setting `CORS_ALLOWED_ORIGIN` on the
backend to the exact Vercel URL — no code change needed:
```bash
flyctl secrets set CORS_ALLOWED_ORIGIN=https://housecheck-wine.vercel.app -a housecheck-nessa
```
When set, the API allows only that origin (GET+POST, JSON `content-type`); when unset it falls
back to permissive for local dev. The active mode is logged at startup.

Because only one origin is allowed, `localhost` dev falls back to bundled demo data. Route dev
through the Vite proxy instead, via `frontend/.env.local`:
```
VITE_API_URL=/api
VITE_BACKEND_URL=https://housecheck-nessa.fly.dev
```

## Continuous deployment (currently blocked)

`.github/workflows/fly-deploy.yml` exists and authenticates correctly, but **cannot build**:
`Dockerfile:18` does `COPY data/housecheck.db`, and `.gitignore:4` excludes `/data/*.db`. A
clean CI checkout has no database, so the image build fails at that step. Deploys currently
have to run from a machine where `data/housecheck.db` exists on disk. Resolve by either
generating the DB in CI before deploying, or committing it — note that committing publishes
derived rent-stabilization data to a public repo, which is an open question in the IP audit.

## Cost
Data APIs $0 · Fly.io ~$0–5/mo (scale-to-zero) · map tiles via MapLibre + Protomaps $0.
Well within the $20–50 budget. See PRD Appendix F.

## Secrets (only if you add features that need them)
- `/summary` (optional LLM) → `flyctl secrets set OPENROUTER_API_KEY=... -a housecheck-nessa`
  - Optionally `OPENROUTER_MODEL=<slug>`; defaults to `anthropic/claude-haiku-4.5`.
  - A `:free` model is fine here. OpenRouter logs free-tier prompts, but the grounding facts
    are public NYC building data and the user's own rent never reaches an LLM. Switch to a paid
    zero-data-retention model before collecting personal data. The server warns at startup when
    the model ends in `:free`.
- Re-ingest inside CI/cloud (not needed today) → `CENSUS_API_KEY`, `NYC_APP_TOKEN`
Never commit keys; local dev uses the OS keychain / machine env.
