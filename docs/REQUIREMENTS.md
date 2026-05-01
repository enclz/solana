# Enclz — Requirements (Agent Wallet Edition)

> **Enclz** (enclz.com) — one enclave per agent, one policy ceiling enforced on-chain.

## Vision

Give autonomous AI agents a dedicated Solana wallet with on-chain spend enforcement — so they can pay for things without a developer handing them a real private key.

## Problem

AI agents need to spend money: API calls, micro-services, bounties, tips, on-chain actions. Current options are:

- **Full private key in environment** — catastrophic if agent is compromised, hallucinates, or config leaks
- **Backend-enforced limits** — bypassable if the backend itself is compromised
- **No wallet at all** — agent can't operate autonomously

Enclz takes a fourth path: agents hold a dedicated wallet governed by on-chain spend policy. Limits and whitelist enforcement live in the smart contract — no backend compromise can override them.

## Target Users

### Orchestrator (Person A2)

Developer or researcher running one or more autonomous AI agents.

- Builds agents using LangChain, AutoGen, CrewAI, Eliza, or custom frameworks
- Agents perform real work: research, content, task coordination, API orchestration, on-chain workflows
- Agents need to pay for things: API calls, micro-services, contractor bounties, tipping
- Primary fear: agent hallucinates a large transfer, prompt injection drains wallet, or agent config is exfiltrated exposing a private key
- Manages the fleet from a web dashboard or via the orchestrator REST API

### AI Agent (Person B2)

Autonomous software process, not a human.

- Never sees or holds a private key — authenticated via scoped API key only
- Interacts via REST API with structured JSON requests (no natural language)
- Subject to on-chain whitelist and spend limit enforcement
- Receives confirmations via synchronous response or webhook callback

---

## Product Channels

| Channel | Who uses it | Purpose |
|---|---|---|
| **Web App** | Orchestrator (A2) | Group setup, agent provisioning, whitelist management, policy config, fleet dashboard, audit log |
| **Orchestrator REST API** | Orchestrator (A2) | Programmatic agent provisioning — create groups, add agents, configure policies, issue invite codes |
| **Agent REST API** | AI Agent (B2) | Transfer, swap, deposit, withdraw, balance/limit queries, webhook registration, simulation |
| **MCP Server** | AI Agent (B2) | Native tool integration for MCP-enabled runtimes (Claude, Cursor, any MCP client) — same operations as Agent REST API, zero HTTP client code required |

---

## Core Model

Each agent gets a dedicated wallet on Solana. The wallet is governed by a spend policy — daily limit, per-transaction limit, hourly frequency cap — and a whitelist of approved recipient addresses and protocol addresses. These rules are enforced on-chain: no backend configuration or compromise can override them.

External addresses are whitelisted with a **TTL** (expiry timestamp) and an **approved amount cap**. Once the full approved amount is transferred to an address, the whitelist entry is automatically voided on-chain — the orchestrator must explicitly re-approve it. Intra-group agent addresses and protocol addresses (DEX router, lending pools) are permanent and unlimited.

The orchestrator controls group setup and policy via the web app or orchestrator API. The agent interacts only through the agent REST API using a scoped API key — no private key, no signing, no crypto knowledge required.

---

## User Flows

### Flow A — Group Setup

1. Orchestrator opens the web app (no wallet required to browse or preview) and connects their Solana wallet, OR calls `POST /v1/orchestrator/groups` via the orchestrator API
2. A `GroupConfig` is recorded on-chain. If the on-chain instruction fails, the web app shows an error with a retry button — no partial state is persisted.
3. Orchestrator adds agent members — each gets a dedicated wallet PDA on-chain
4. Orchestrator configures per-agent policy: whitelist of approved service endpoints, per-tx/daily/hourly limits
5. Orchestrator optionally selects a policy template as a starting point
6. For each external service address added to the whitelist, orchestrator sets a TTL (e.g., 30 days) and an approved amount cap (e.g., $50 total). Intra-group agent addresses are added automatically with no TTL and no cap.

### Flow B — Agent Registration

1. Orchestrator clicks **Add Agent** in the web app or calls `POST /v1/orchestrator/groups/:id/agents`
2. System generates a one-time invitation code — shown once, expires in 24 hours
3. Agent (or orchestrator on agent's behalf) calls `POST /v1/register` with the invitation code
4. Backend creates a scoped API key, returns it **once** — never stored or shown again
5. Agent stores the API key in its secret manager or environment
6. Invitation code is invalidated immediately — cannot be replayed
7. Agent is now active: can transfer, swap, query balance, all subject to on-chain policy

If the API key is compromised: orchestrator revokes it in the web app, issues a new invite code, agent re-registers.

### Flow C — Agent Transfer

1. Agent calls `POST /v1/transfer` with `to`, `amount`, `token`, optional `memo` and `task_id`, and optional idempotency key
2. Backend authenticates via scoped API key, resolves agent's wallet
3. Backend pre-flight checks limits (mirrors on-chain state)
4. Transfer executes — whitelist and spend limits enforced on-chain
5. Response returns `tx_sig`, `status`, remaining daily and hourly headroom
6. If webhook registered: async `transfer.confirmed` event fires to callback URL

If recipient is not whitelisted, the whitelist entry has expired, the approved amount is exhausted, or a spend limit is exceeded: `403` with structured error code, no transaction submitted. On RPC timeout or transient failure: `503` with `retry_after` field — agent retries with same idempotency key.

### Flow D — Agent Swap

1. Agent calls `POST /v1/swap` with `from_token`, `to_token`, `amount`
2. Backend fetches Jupiter quote, executes swap via whitelisted DEX router
3. Response returns `tx_sig`, `received_amount`, `rate`

### Flow E — Deposit / Withdraw (Yield)

Orchestrator whitelists a lending protocol (e.g., Kamino). Agents can then deposit and withdraw.

Deposit:
1. Agent calls `POST /v1/deposit` with `token`, `amount`, optional `protocol` label
2. Backend routes to the whitelisted lending protocol, executes deposit
3. Response returns `tx_sig`, `deposited_amount`, current APY

Withdrawal:
1. Agent calls `POST /v1/withdraw` with `token`, `amount`, optional `protocol` label
2. Backend redeems from the protocol, returns funds to agent wallet
3. Response returns `tx_sig`, `received_amount`, `yield_earned`

### Flow F — Balance, Limits, History

Agent queries current state without executing any transaction:
- `GET /v1/balance` — token balances, daily/hourly headroom
- `GET /v1/limits` — full spend policy, whitelist
- `GET /v1/history` — paginated transaction log

### Flow G — Simulation (Dry Run)

1. Agent calls `POST /v1/intents/simulate` with same body as a transfer or swap
2. System checks whitelist and current limit state without submitting any transaction
3. Response returns `{ "would_succeed": true/false, "reason": "daily_limit_exceeded" | ... }`

Agents pre-check before committing, especially in limit-sensitive workflows. Eliminates unnecessary failed-tx fees.

### Flow H — Incoming Payment Notification

Any incoming transfer to an agent wallet triggers a `payment.received` event to the agent's registered webhook URL.

### Flow I — Anomaly Alerts

Orchestrator registers a webhook for policy events across their fleet:
- `policy.limit_threshold` — agent reaches 80% of daily limit
- `policy.limit_exceeded_attempt` — agent request rejected for exceeding limit
- `policy.whitelist_violation` — agent attempted transfer to non-whitelisted address
- `policy.whitelist_expiring` — a whitelist entry TTL expires within 24 hours (fires once)
- `policy.whitelist_amount_threshold` — a whitelist entry has consumed 80% of its approved amount
- `policy.whitelist_voided` — a whitelist entry was auto-voided (approved amount fully drained)

Allows orchestrators to detect runaway agents or injection attacks without polling. Expiry and amount alerts give orchestrators time to re-approve before agents are blocked.

---

## Security Model

The actual threat model is backend abuse and agent misbehavior, not key theft. On-chain policies address this directly:

| Threat | Protection |
|---|---|
| Backend hacked, attacker attempts to drain funds | Blocked — only whitelisted recipients, capped amounts |
| Backend operator turns malicious | Blocked — same ceiling applies |
| Agent hallucinates a large transfer | Blocked — per-tx limit caps damage; whitelist blocks arbitrary recipients |
| Prompt injection tricks agent into draining wallet | Contained — whitelist + frequency cap + approved amount cap limit blast radius; even a fully exploited agent can only drain the pre-approved amount to pre-approved addresses |
| Agent API key exfiltrated | Contained — API key has no key custody; attacker can only move funds within on-chain policy ceiling |
| Agent enters infinite spend loop | Blocked — hourly frequency cap enforced on-chain |
| Compromised orchestrator account | Partial — limits still apply post-compromise; emergency withdraw available |

### Spend Policy

Each agent wallet has independently configurable limits. Defaults are tight to bound hallucination and loop damage:

- **Daily spend limit** — default $10/day
- **Per-transaction limit** — default $1/tx
- **Hourly frequency cap** — default 5 tx/hour

Orchestrator can raise limits per-agent. Upper bounds are enforced on-chain — no backend call can exceed what the smart contract allows.

### Whitelist

Transfers are only allowed to addresses explicitly approved by the orchestrator. Three categories with different lifecycle rules:

**Intra-group addresses** — other agent wallets within the same group:
- Added automatically when an agent is created
- Permanent, no TTL, no amount cap
- Intra-group transfers always allowed (within per-tx and daily limits)

**External recipient addresses** — wallets the agent is allowed to pay:
- External service addresses (APIs, contractors, bounty recipients)
- Exchange deposit addresses
- Require: **TTL** (expiry timestamp) + **approved amount cap** (total USDC the agent may send to this address)
- On expiry: whitelist entry is void — transfer to this address returns `whitelist_expired`
- On amount exhaustion: whitelist entry is auto-closed on-chain — transfer returns `whitelist_amount_exhausted`
- Orchestrator re-approves by creating a new whitelist entry with fresh TTL and amount

**Protocol addresses** — DeFi integrations:
- DEX swap router (whitelisted at group initialization, permanent)
- Lending protocol pools (e.g., Kamino) — added explicitly by orchestrator, permanent

No address outside the whitelist can receive funds regardless of backend state. TTL and amount exhaustion checks are enforced on-chain in `execute_transfer`.

### Authentication

**Agent API key** — scoped credential issued at registration. Never stored in plaintext (bcrypt hash only). Revocable instantly. Agent never sees or holds the underlying wallet private key.

**Orchestrator auth** — Solana wallet signature (web app) or orchestrator API key (programmatic provisioning). Separate credential tier with group admin capabilities.

### Limitations

Enclz does not protect against:
- Orchestrator intentionally draining agent wallets (mitigated by design: orchestrator funds wallets, doesn't withdraw from them)
- Smart contract bugs (mitigated by audit before mainnet)
- Solana network-level issues (outside scope)

---

## Policy Templates

Orchestrators select a template when creating an agent; any field can be overridden.

| Template | per-tx limit | daily limit | hourly cap | Whitelist preset |
|---|---|---|---|---|
| `research-agent` | $0.10 | $1.00 | 10 tx | Known data API endpoints |
| `micro-payment-agent` | $1.00 | $10.00 | 5 tx | Specified at creation |
| `payment-agent` | $10.00 | $100.00 | 20 tx | Exchange addresses + specified |
| `custom` | Orchestrator-defined | Orchestrator-defined | Orchestrator-defined | Fully manual |

Templates are advisory — the orchestrator can override any field. The on-chain policy is what matters.

---

## Agent Integration Resources

Rather than maintaining language-specific SDKs, Enclz ships two integration artifacts:

**`openapi.json`** — Machine-readable OpenAPI 3.1 spec covering all agent REST endpoints. Consumed by code generators, API clients, and AI assistants.

**`AGENT_SKILL.md`** — Markdown file designed to be injected into an agent's system prompt or context. Describes all available operations, parameter formats, error codes, and policy constraints in a format optimized for LLM consumption. Drop-in compatible with LangChain tool context, AutoGen skill description, and plain system-prompt injection.

**MCP Server** — Model Context Protocol server wrapping the Agent REST API. Exposes Enclz operations as native MCP tools — no HTTP client code, no SDK. Compatible with any MCP runtime: Claude Desktop, Cursor, Claude Code, or custom agents built with the MCP SDK. Configured with a single env var (`ENCLZ_API_KEY`); the agent API key is already issued at registration. Each tool maps 1:1 to an agent REST endpoint and returns structured JSON that MCP runtimes can reason over directly.

---

## Monetization

**Protocol fee** — Enclz deducts a flat 10 basis points (0.1%) from every outbound transfer and swap at execution time. Collected on-chain. No separate billing, no off-chain settlement.

The fee is charged to the sending agent's wallet at execution time, deducted from the transfer amount. It counts against the agent's daily spend limit (like any other transfer).

---

## Web App Features (Orchestrator)

- **Landing page (no wallet required)**: product overview, live devnet demo (transfer blocked by whitelist shown on Solana Explorer), policy template previews. Wallet connection only required to take action.
- Connect Solana wallet, create groups
- Add agents — policy template selection, limit override, invitation code generation
- Manage whitelist:
  - Intra-group agents listed as permanent entries (read-only)
  - Add external address: set label, TTL (expiry date), approved amount cap
  - Renew / top up: extend TTL or increase approved amount for an existing entry
  - Remove: close whitelist entry before expiry
  - Protocol addresses (DEX router, lending pools): permanent, no cap
- Configure per-agent spend limits
- Per-agent spend audit log (timestamp, amount, recipient, memo, task_id, agent ID)
- Whitelist approval dashboard: shows each external entry's TTL countdown, amount used vs. cap, status (active / expiring soon / voided)
- Revoke agent API key — immediately invalidates credential
- Re-invite agent — issues fresh invitation code after revocation
- Agent fleet dashboard — daily spend vs. limit, hourly tx rate, remaining headroom per agent
- Anomaly alert configuration — set webhook URL for policy events across the fleet
- Error recovery: all failed on-chain operations surface a human-readable error with a retry action; no silent failures

---

## Deferred

- Programmatic key rotation API (currently: web app only)
- Budget pool — orchestrator sets shared budget ceiling across agent fleet
- Multi-sig for high-value operations (require orchestrator co-sign above threshold)
- Human custody model (SMS / Telegram / WhatsApp channels, NLP intent parsing)
- Fiat on/off-ramp integration
- Cross-chain transactions

---

## Future Scope

- Per-task budget allocation: agent requests ephemeral sub-limit for a specific task, orchestrator approves
