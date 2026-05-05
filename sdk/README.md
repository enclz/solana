# @enclz/sdk

Anchor IDL, TypeScript types, and program ID for the [Enclz](https://github.com/enclz/solana) on-chain spend-policy enforcement program.

> **AI agent integrators:** if you are building an AI agent that uses Enclz, use the **Agent REST API + MCP server** — not this package. This package is for direct on-chain callers (program composability, auditing tools, and custom backends) that need typed `Program<Enclz>` access.

## Install

```bash
npm install @enclz/sdk @coral-xyz/anchor @solana/web3.js
```

## Usage

```typescript
import { IDL, PROGRAM_ID, type Enclz } from "@enclz/sdk";
import { Program, AnchorProvider } from "@coral-xyz/anchor";

const provider = AnchorProvider.env();
const program = new Program<Enclz>(IDL, provider);

console.log(program.programId.toBase58()); // == PROGRAM_ID
```

## On-chain IDL

The program IDL is also available on-chain via `anchor idl fetch`, so you can construct a typed `Program` without installing this package:

```typescript
import { Program } from "@coral-xyz/anchor";
import { PublicKey } from "@solana/web3.js";

const idl = await Program.fetchIdl(new PublicKey(PROGRAM_ID), provider);
const program = new Program(idl!, provider);
```

## Program ID

```
45PiBcnkKhZbzb5GQDhJ9Rikwiz3DUzyoBwiKHbAFaLW
```

## Publishing

Do not publish directly from `sdk/`. Use the root workspace script, which ensures the IDL and TypeScript types are built from a fresh `anchor build` before publishing:

```bash
npm run publish:sdk   # from repo root
```

## Notes

- `@coral-xyz/anchor` is a peer dependency — install it alongside this package.
- The `version` field of this package is single-sourced from `programs/enclz/Cargo.toml` via the IDL `metadata.version`, synced on every `npm run build:sdk`.
- The upgrade authority for on-chain IDL changes is the program deployer keypair.
