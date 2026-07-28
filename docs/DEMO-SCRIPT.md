# HouseCheck — Demo Video Script (2–3 min)

**Goal:** show that HouseCheck turns a blind rental decision into an instant, source-backed score — on *real* data, *live* on the internet.
**Works today** with no built frontend yet — using the case-study page, the live API, and a mockup. Re-shoot the middle beat with the real UI once Anthony's frontend lands.

**You'll need open in tabs:** the [case-study page], the live API (`https://housecheck-nessa.fly.dev`), one mockup (`docs/mockups/nyc-finance.html`), and a terminal.

---

### 0:00–0:20 · Hook
**Show:** the case-study hero — the two gauges, **24 vs 78**.
**Say:** *"Renting in New York means handing over forty thousand dollars for a building you know nothing about. These are two real Brooklyn buildings, a few blocks apart. One scores 24. One scores 78. HouseCheck is how you tell them apart — in seconds."*

### 0:20–0:45 · The problem
**Show:** scroll the case-study problem stats (51.6% rent-burdened, $1,761 gap, 11.1M violations).
**Say:** *"Half of NYC renters are rent-burdened. The data to protect yourself exists — eleven million violation records — but it's buried in three government portals nobody can use. So people sign blind."*

### 0:45–1:30 · The product (the money shot)
**Show:** a mockup card (or the live UI once built) — the Building Health Card for 61 Stuyvesant Ave.
**Say:** *"Type an address, get one honest score across four things: condition, your legal protections, whether you're overpaying, and accessibility. Sixty-one Stuyvesant — score 24. Sixty-five open violations, twelve of them immediately hazardous. And every single number links to its government source."*
**Then show:** 443A Monroe — score 78, zero violations. *"Same neighborhood. Completely different building. That's the point."*

### 1:30–2:00 · It's real and it's live
**Show:** terminal — hit the live URL:
```bash
curl https://housecheck-nessa.fly.dev/building/3018110019
```
**Say:** *"This isn't a mockup — it's deployed, on the public internet, serving 250 real Bed-Stuy buildings. Here's 510 Quincy Street: 192 rent-stabilized units, pulled from city tax records — a protection most renters never even learn they have."*

### 2:00–2:30 · The differentiator + close
**Show:** the case-study "honest data" section.
**Say:** *"Our whole edge is trust. We fact-checked our own pitch and threw out the numbers we couldn't verify. Where the data can't prove a building is stabilized, the card says 'unverified' instead of guessing. Built in about ten days, on nothing but free public data, for zero dollars. HouseCheck — a blind forty-thousand-dollar decision, made in seconds."*

---

**Tips:** screen-record at 1080p, keep it under 3 min, narrate over B-roll (don't film yourself talking the whole time). End on the live URL on screen. Keep energy up in the first 10 seconds — that's what's graded.
