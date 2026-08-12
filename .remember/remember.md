# Handoff

## State
Portfolio repo `nessaisling-lab/pursuit-l2-portfolio` is live (site: https://nessaisling-lab.github.io/pursuit-l2-portfolio/, local clone `D:\L2 Cycle 4\pursuit-l2-portfolio`, branch `main`, HEAD `4f39b82`). All four cycle READMEs done; all four have real screenshots + workflow GIFs captured from running builds. `index.html` is v4: HouseCheck dark ground, 1280px, 2-col grid, WebGL shader hero, mobile verified at 0px overflow.
HouseCheck itself is fully deployed and verified — repair speed, signed export, `/meta` publishing the public key.

## Next
1. Fold research in: HouseCheck lead card **above the fold** (15s screening rule); tighten hero.
2. Per-project card theming — same frame/grid/type scale, only surface colour + accent + display face change per project. Varying the container is what makes it garish.
3. Leptos **SSG** migration (not CSR — CSR ships a WASM bundle and a blank first paint). `cargo leptos` + Rust/UI components, GitHub Action to build+publish, footer line linking to source. Watch https://www.youtube.com/watch?v=GqNmRnaOit0 first.
4. Classroom Step 7: polish root `README.md` — it undersells the now-complete cycle docs.

## Context
- Capture rig in `%TEMP%`: `grabh.ps1` (window by handle, `PW_RENDERFULLCONTENT` — needed for Tauri/WebView2, plain screenshots come back blank), `drive.ps1` (click/type by window-relative coords), `click_grab.ps1`. computer-use screenshots are useless here — the Claude window masks the desktop.
- Resona = `aislingld-pursuit/L2-Clone-Prodject` release `v0.2.0-beta.29`; repo name predates the rename, app installs as Resona. Installed at `%LOCALAPPDATA%\Wisper\wisper.exe`.
- Fly app `housecheck-nessa` is owned by `leiva.stan@gmail.com` — I cannot deploy it; she runs `flyctl deploy`. Vercel I can deploy.
- Never handle keys/tokens. `HOUSECHECK_EXPORT_SIGNING_KEY` and `OPENROUTER_MODEL` are hers to set.
- She wants Rust foregrounded everywhere; do not bulk-install tools from videos without her explicit greenlight.
