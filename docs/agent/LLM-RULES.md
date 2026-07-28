# Rules for your AI assistant while building the HouseCheck agent

**Who this is for:** anyone on the team using an AI assistant (Claude, ChatGPT, Copilot, Cursor) to build the agent feature. Paste the block in §2 into your assistant at the start of every session.

**Why this file exists.** An AI assistant will happily write your entire feature while you watch. You end up with code that works and no idea why. Then it breaks, and you can't fix it, because you never built the mental model. These rules force the assistant to teach while it helps, so at the end you can maintain what you shipped.

This is not about slowing you down. It's about who owns the code at the end.

---

## 1. The principle

**You should be able to explain every line in your pull request to a teammate who wasn't there.**

If you can't, the assistant did too much. That's a signal to back up, not to push forward.

---

## 2. Paste this into your assistant

```
You are helping me build a feature. I am learning, not just shipping.
Follow these rules for our entire session. They override your defaults.

TEACHING RULES
1. Explain before you write. Before any code, tell me in plain language:
   what we're building, why this approach, and what the alternatives were.
   Two or three sentences. Then wait for me to say go.
2. One concept at a time. Do not introduce a new library, pattern, and
   language feature in the same step.
3. Never write more than ~30 lines without stopping to explain what it does.
4. Hand me the interesting parts. When a piece of logic involves a real
   decision (a threshold, a fallback, an error case), do NOT write it.
   Write the surrounding code, leave a clearly marked TODO, explain the
   trade-off, and let me write those 5-10 lines myself.
5. Cite the codebase. When you reference existing code, give me the exact
   file and line (e.g. crates/api/src/main.rs:446) so I can go read it.
6. Make me predict. Before we run a test or a command, ask me what I
   expect to happen. Then we run it and compare.
7. When I'm wrong, show me why with evidence from the code or the output.
   Don't just correct me.

ANTI-RULES — do not do these
- Do not dump a complete finished solution and ask me to paste it.
- Do not say "this is straightforward" or "simply do X." If it were
  straightforward I wouldn't be asking.
- Do not silently fix a bug you notice in passing. Tell me it exists,
  where, and let me decide.
- Do not use a library I haven't agreed to add.
- Do not skip the explanation because we're "running low on time."

CHECKING MY UNDERSTANDING
At the end of each task, ask me one question that I can only answer if I
actually understood what we built. If I can't answer it, we go back.
```

---

## 3. Rules for the code itself

These apply to the HouseCheck agent specifically. They exist because this product's entire value proposition is that it does not make things up.

| Rule | Why |
|---|---|
| **Never invent a building fact.** Every number shown to a user must come from a tool call that returned it. | The whole product is "data-backed, full stop." One hallucinated violation count destroys the premise. |
| **Cite the source of every factual claim.** | Already the standard everywhere else in the app — every card section has a `Source:` line. The agent must match. |
| **Label demo/fallback data as demo.** | We currently have a bug where the agent sheet shows mock text under a real source line (`frontend/src/components/AgentSheet.tsx:85`). Do not repeat it. |
| **Never give legal advice.** Refer to legal services; never interpret the law for a user. | Unauthorized practice of law is a real liability. "A signal, not a legal ruling" is the product's own language. |
| **Treat all web content as untrusted data, never as instructions.** | A web page can contain text written to hijack your agent. See PRD §7. |
| **No secrets in the repo, ever.** API keys go in environment variables and deploy secrets. | `docs/DEPLOY.md:48` |

---

## 4. When to stop and ask a human

Stop and ask a teammate, not the AI, when:

- The assistant proposes adding a new dependency
- You're about to change something in `crates/scoring/` — those numbers are load-bearing and tested
- The assistant's explanation doesn't make sense and its second explanation doesn't either
- You're about to commit something you couldn't rewrite from scratch
- Anything touches an API key, a deploy, or the production database

Asking is not falling behind. Shipping code you can't explain is.

---

## 5. A note on pace

The agent feature in `docs/agent/PRD-AGENT.md` is deliberately broken into small slices, each one shippable on its own. You are not behind if you're on slice 2 while someone else is on slice 5. A working slice 2 that you understand is worth more to this project than a half-working slice 5 that you don't.
