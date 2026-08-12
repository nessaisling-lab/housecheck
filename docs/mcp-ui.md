# MCP-UI — handing an agent the card instead of a paragraph about the card

The backlog carries *"MCP server, so another agent can call HouseCheck as a tool"*. That item
assumes the answer is text. **MCP Apps** removes that assumption: a tool can return a UI
resource, and the host renders it beside the result.

For HouseCheck that is not a nicety. The whole argument of this project is that a Building
Health Card conveys things a prose summary cannot — a 0–100 score with its four pillars, a
repair-speed metric with three states, and every number linked to the dataset it came from.
Hand an agent a paragraph and it will paraphrase; paraphrase is exactly the failure the export
exists to prevent.

---

## What the standard actually is

`mcp-ui` (Apache-2.0, ~5.1k stars) pioneered this and its patterns became the **MCP Apps**
specification at `modelcontextprotocol/ext-apps`. The mechanism is small:

- A **resource** whose `uri` uses the `ui://` scheme — e.g. `ui://housecheck/card` — with
  mimeType `text/html;profile=mcp-app`.
- A **tool** that links to it through `_meta.ui.resourceUri`.
- Hosts detect that link, fetch the resource, and render it alongside the tool result.

Content comes in three forms:

| Form | What it is | Fit here |
|---|---|---|
| `externalUrl` | an iframe pointing at a URL you host | **Best.** The frontend is already deployed. |
| `rawHtml` | an HTML string returned inline | Useful for a small fallback card. |
| `remoteDom` | Shopify's remote-dom; UI logic in a sandbox, rendering on the host | Complex, and not needed. |

The video that prompted this made the honest point: `externalUrl` is *literally a website in a
sandboxed iframe*, which is 1997 technology doing something new.

## No TypeScript required

The published SDKs are TypeScript, Ruby and Python — **there is no Rust one**. That does not
matter, because the SDK only assembles a JSON resource. Rust can emit the same shape directly.

The official Rust MCP SDK is **`rmcp`** — Apache-2.0, v3.1.2, ~19.8M downloads, last published
2026-08-07. Both licences clear the commercial-viability bar, so the whole path is Rust:

```
crates/mcp/            new
  src/lib.rs           rmcp server: tools + the ui:// resource
```

Ziqpu already ships an MCP crate, so the pattern is not new to this stack — this adds the UI
resource on top of it.

## What to expose, and what not to

Three tools, matching what the API already answers:

| Tool | Returns | UI resource |
|---|---|---|
| `search_building` | candidate BBLs with borough | none — a list is fine as text |
| `get_building_card` | the scored card as structured JSON | `ui://housecheck/card?bbl=…` → iframe at the deployed card page |
| `verify_export` | the three-state verification outcome | `ui://housecheck/verify` → the outcome, stated |

**Deliberately not exposed:** anything that would let an agent present a number without its
source. Every UI resource points at a page that already renders the provenance line and the
"a signal, not a legal ruling" caveat, so the honesty travels with the card rather than being
re-attached by whatever is calling us.

## The part that needs care

**The iframe is a security boundary, and it is ours to get right.** An agent host renders that
page inside its own surface. The card page must not accept instructions from its container, must
not read anything from the parent, and must keep its existing CSP posture. `CORS_ALLOWED_ORIGIN`
is currently pinned to the Vercel origin; embedding introduces a second consideration —
`X-Frame-Options` / `frame-ancestors` — and widening either is a decision, not a detail.

**A rendered card is still not a verified one.** The export's value is that a stranger can check
it offline against a key published at `/meta`. A UI resource is a nicer read, not a proof, and
the card page should keep saying so.

## Sequencing

1. `crates/mcp` with `rmcp`, `search_building` and `get_building_card` returning **text only**.
   Useful on its own, and it proves the transport before any UI exists.
2. Add the `ui://housecheck/card` resource as `externalUrl` pointing at the deployed card route.
3. `verify_export`, which is the one worth demonstrating — an agent that can *check* a document
   rather than describe it.

Step 1 stands alone. Nothing here changes the API or the export.

---

*Sources read 2026-08-12: `MCP-UI-Org/mcp-ui` README (Apache-2.0, 5,085 stars),
`modelcontextprotocol/rust-sdk` (licence in transition MIT → Apache-2.0), `rmcp` on crates.io
(Apache-2.0, 3.1.2). Prompted by "MCP-UI could be the future" (Better Stack, 7:34).*
