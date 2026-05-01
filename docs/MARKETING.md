# Enclz — Marketing & Personas (Agent Wallet Edition)

## Core Positioning

**Enclz gives AI agents a Solana wallet with on-chain spend enforcement.**

Not backend-enforced limits. Not a shared hot wallet. On-chain: the smart contract is the policy — no backend compromise can override it. Agent never sees a private key.

---

## Orchestrator Archetypes

### #1 — Agentic Application Developer *(highest priority)*

Solo developer or small team building a product where agents need to pay for things. LangChain, AutoGen, CrewAI, or custom framework. Deploys to a single cloud environment.

**Pain**: Giving agents a private key means any misconfiguration, hallucination, or prompt injection can drain the wallet. Backend-enforced limits are only as secure as the backend. Key rotation requires redeployment.

**Enclz fit**: Agent gets a scoped API key — never the private key. On-chain limits mean a hallucinating agent can't exceed $1/tx regardless of what the backend does. External service addresses are whitelisted with a time-bound, amount-capped approval — once the budget is consumed the slot auto-voids on-chain, not in a backend. Simulate endpoint lets agents pre-check before committing. Policy templates get them running in under 5 minutes.

**Why #1**: Largest cohort of potential users. Ship to their existing frameworks with zero SDK dependency — REST API + `AGENT_SKILL.md` is all they need. Pain is acute and immediate.

**Unit math**: 1 developer, 10 agents, $10/day each = $100/day under management. Enclz takes 10 bps = $0.10/day. Scale to 1,000 developers = $100/day protocol revenue. Revenue scales with agent activity, not seat count.

**Reach**: LangChain Discord, AutoGen GitHub discussions, CrewAI community, HackerNews Show HN, AI Twitter/X (#agenticAI, #LLMops). Cold pitch at agent hackathons and AI meetups.

**Cold pitch**: *"Your agent just needs an HTTP client and an API key — or install one MCP server. On-chain spend limits mean a hallucinating agent can't drain more than you allow — not because your backend stops it, but because the smart contract does."*

---

### #2 — AI Infrastructure / Platform Builder

Company building an agent orchestration platform where customers run their own agents. Needs to provision wallets at scale and enforce per-customer spending isolation.

**Pain**: Managing one hot wallet per enterprise customer is operationally nightmarish. Per-customer key custody multiplies security surface area. Compliance needs per-customer audit trails.

**Enclz fit**: Orchestrator API enables programmatic group + agent provisioning. Each customer's agents are isolated in their own group with independent on-chain policies. Audit log per agent is natively structured (memo, task_id, timestamp).

**Why #2**: High-value accounts. Each platform customer brings many downstream agents.

**Reach**: a16z infra portfolio companies, YC AI batch founders, LangChain/LlamaIndex core team community, Solana Foundation accelerator cohorts.

**Cold pitch**: *"Provision customer wallets via API. On-chain policy means customer A's agent can never touch customer B's funds — even if your backend is compromised."*

---

### #3 — AI Research Lab / Autonomous Agent Researcher

Academic or independent researcher running multi-agent systems. Agents coordinate, delegate tasks, pay each other for work.

**Pain**: Research budgets are fixed. Runaway agent loops or infinite retry patterns can exhaust a wallet in minutes. Hard to audit which agent spent what and why.

**Enclz fit**: Hourly frequency cap blocks infinite loops on-chain. Per-task `task_id` field enables per-experiment cost accounting. Simulate endpoint lets researchers bound costs before running.

**Why #3**: Research publications drive awareness in the developer community. Credibility multiplier. Revenue secondary.

**Reach**: Twitter/X AI research community, arXiv author networks, EleutherAI Discord, Alignment Forum, LessWrong.

---

### #4 — Crypto-Native Agent Developer *(niche, high intent)*

Developer already building on Solana — DeFi bots, MEV agents, automated market makers, yield optimizers. Already understands wallets; specifically needs policy enforcement.

**Pain**: Existing Solana wallet solutions give full key access. A bug in bot logic or a malicious dependency can drain the entire wallet in one transaction.

**Enclz fit**: Whitelist enforcement means the bot can only interact with pre-approved protocol addresses. Per-tx limit caps single-tx blast radius. Swap and deposit/withdraw flows already integrated with Jupiter and Kamino.

**Reach**: Superteam Discord, Solana Tech Discord #bots channel, @solana_devs Twitter, Anchor framework community.

---

## Regulatory Note

Enclz is developer infrastructure, not a financial product. Orchestrators configure on-chain policies; Enclz executes transfers on instruction. No custody of user funds, no money transmission. Frame as "programmable wallet policy enforcement," not "managed accounts."

---

## Go-to-Market Channels

### Developer Distribution (primary)

| Channel | Tactic |
|---|---|
| GitHub | Open-source `AGENT_SKILL.md`, `openapi.json`, and `@enclz/mcp-server`; agents that use Enclz link back |
| MCP ecosystem | List in MCP server directories; Claude Desktop + Cursor users discover via `npx @enclz/mcp-server` |
| LangChain / AutoGen community | Tutorial: "Give your LangChain agent a Solana wallet in 10 minutes" |
| HackerNews Show HN | Demo: agent that pays for its own API calls with on-chain spend enforcement |
| Solana developer Discord | Announce in #projects, post demo transaction on devnet |
| AI Twitter/X | Thread: "Why giving your agent a private key is the wrong model" |

### Hackathon Demo Strategy

Demo flow takes under 3 minutes:
1. Show landing page — no wallet required; embedded devnet demo, policy template previews visible immediately
2. Show orchestrator creating a group + agent (one curl command or web app wizard)
3. Show orchestrator adding an external address: TTL = 7 days, approved amount = $5
4. Show agent calling `POST /v1/transfer` — succeeds; whitelist `amount_used` increments on-chain
5. Show agent calling `POST /v1/transfer` to same address after budget exhausted — `403 whitelist_amount_exhausted`; Solana Explorer confirms the `WhitelistEntry` PDA is closed
6. Show agent calling `POST /v1/transfer` to unwhitelisted address — `403 whitelist_violation`
7. Show fleet dashboard: amount remaining per entry, TTL countdown, `policy.whitelist_voided` webhook event fired

The on-chain proof is the differentiator. Backend-only solutions can't demonstrate step 5: the whitelist PDA closed by the smart contract, verifiable on Explorer, independent of any backend state.

---

## Customer Development

### Target

3 agentic app developers (Persona #1) before writing more code. Find in LangChain Discord `#showcase`, AutoGen GitHub discussions, CrewAI community — people actively posting agent projects.

### Outreach (DM template)

> Hey — saw your [agent project]. Quick question: how are you handling payments when the agent needs to call a paid API or send funds? I'm building wallet infrastructure for exactly this and would love 20 min to understand how you're solving it today. Happy to share what I'm learning too.

### Interview Script (20 min)

1. What does your agent need to spend money on? (APIs, services, on-chain actions?)
2. How are you handling that today? (Private key in env, backend wallet, nothing yet?)
3. What's the riskiest thing your agent could do with payment access?
4. Have you had a hallucination or prompt injection incident that touched money — or come close?
5. If your backend was compromised, what could an attacker do with your agent's wallet?
6. Would you trust on-chain limits more than your own backend config? Why / why not?
7. What would "done" look like — what would make you feel safe giving an agent a wallet?
8. What frameworks are you using? (LangChain, AutoGen, CrewAI, custom?)
9. Would you pay per-transaction (10 bps) or prefer flat fee?
10. Who else do you know building agents that need to pay for things?

### What to Listen For

- **Green:** mentions hallucination fear, drain risk, "I don't trust the agent with a real key," frustration with backend-only limits → thesis confirmed per interview
- **Kill signal:** "we don't do payments at all and have no plans to" (disqualify, don't pivot)
- **Pivot signal:** consistent alternative framing (e.g., "I just need multi-sig, not per-tx limits") across 3+ interviews

### Signal Thresholds

| Result | Confidence | Action |
|---|---|---|
| 3/3 confirm drain fear + on-chain trust | 0.8+ | Build MVP, full speed |
| 2/3 confirm, 1 neutral | 0.65 | Build MVP, revisit after 3 more |
| 0–1 confirm | 0.3 | Stop, re-examine problem framing |

### Logging Format

One file per interview: date, persona type, framework used, current payment approach, key quotes (verbatim), green/yellow/red signal, follow-up ask.

---

## Competitive Differentiation

| | Enclz | Openfort | lobster.cash | Coinbase AgentKit | Raw Solana wallet |
|---|---|---|---|---|---|
| Enforcement layer | On-chain (Pinocchio) | On-chain (ERC-4337) | Backend | Backend | None |
| Private key exposure | Never (scoped API key) | Never (session keys) | Never | Depends | Full exposure |
| Survives backend compromise | Yes | Yes | No | No | N/A |
| Solana-native | Yes | EVM-first | Yes | Recently added | Yes |
| Per-agent policy | Yes, on-chain | Yes, on-chain | Off-chain limits | Backend config | No |
| TTL + amount-capped external whitelist | Yes, on-chain | No | No | No | No |
| No SDK required | Yes (REST + MCP) | No | Yes | No | No |
| Simulation / dry-run | Yes | No | No | No | No |
| Anomaly alerting | Yes (webhooks) | No | No | No | No |
| MCP server | Yes | No | No | No | No |
| Agent context injection | Yes (AGENT_SKILL.md) | No | No | No | No |

**Lead with**: enforcement survives backend compromise AND no SDK required AND TTL + amount-capped whitelist that auto-voids on-chain. Openfort can match the backend-compromise claim but requires an SDK, is EVM-first, and has no per-address amount ceiling or auto-void mechanic. No competitor ships a simulation endpoint, MCP server, or on-chain amount-exhaustion enforcement.

**Against Openfort specifically**: Solana-native architecture, zero-SDK REST + MCP integration, AGENT_SKILL.md for LLM context injection, and simulation endpoint. Openfort is multi-chain generalist; Enclz is Solana-specialist with agent-first DX.

