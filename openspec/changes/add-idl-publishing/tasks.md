## 1. SDK package scaffold

- [ ] 1.1 Create `sdk/package.json` with `name: "@enclz/sdk"`, `version: "0.1.0"` (will be overwritten by build), `main: "dist/index.js"`, `types: "dist/index.d.ts"`, `files: ["dist"]`, `peerDependencies: { "@coral-xyz/anchor": "^0.30.1" }`, `repository`, `license`, `description`
- [ ] 1.2 Create `sdk/tsconfig.json` matching root targets (`target: "es2020"`, `module: "commonjs"`, `lib: ["es2020"]`) plus `declaration: true`, `outDir: "./dist"`, `rootDir: "./src"`, `resolveJsonModule: true`, `esModuleInterop: true`, `strict: true`, `include: ["src"]`
- [ ] 1.3 Create `sdk/src/index.ts` re-exporting `IDL`, `PROGRAM_ID`, and `type Enclz` per design.md "Re-export shape" decision
- [ ] 1.4 Create `sdk/README.md` with install snippet, usage example (`new Program<Enclz>(IDL, provider)`), and explicit positioning paragraph routing AI agents to the Agent REST API + MCP server

## 2. Build pipeline

- [ ] 2.1 Create `scripts/build-sdk.mjs` that: (a) runs `anchor build` if `target/idl/enclz.json` or `target/types/enclz.ts` is missing, (b) copies both into `sdk/src/`, (c) syncs `sdk/package.json` `version` from IDL `metadata.version`, (d) runs `tsc` in `sdk/`
- [ ] 2.2 Add root `package.json` script `"build:sdk": "node scripts/build-sdk.mjs"`
- [ ] 2.3 Add root `package.json` script `"publish:sdk": "npm run build:sdk && cd sdk && npm publish --access public"`
- [ ] 2.4 Update root `.gitignore` to add `sdk/dist/`, `sdk/src/enclz.ts`, `sdk/src/enclz.json`, `sdk/node_modules/`
- [ ] 2.5 Decide and implement `prepublishOnly` hook in `sdk/package.json` (runs `node ../scripts/build-sdk.mjs`) — resolves design.md open question

## 3. On-chain IDL scripts

- [ ] 3.1 Add root `package.json` script `"idl:init:devnet": "dotenv -- anchor idl init --provider.cluster ${QUICKNODE_DEVNET_RPC_URL} --filepath target/idl/enclz.json 67i3uY4gZaidynKa8XbNW569qACSVCebwKnLpNYVtWjj"`
- [ ] 3.2 Add `"idl:upgrade:devnet"` (same shape, `idl upgrade`)
- [ ] 3.3 Add `"idl:init:mainnet"` and `"idl:upgrade:mainnet"` using `${MAINNET_RPC_URL}`
- [ ] 3.4 Document required env vars and upgrade-authority requirement in root `README.md` distribution subsection

## 4. Documentation (root repo)

- [ ] 4.1 Add "Distribution" subsection to root `README.md` covering `npm run publish:sdk` for npm and `npm run idl:upgrade:<cluster>` for on-chain, with the manual-publish caveat
- [ ] 4.2 Cross-link the SDK from the Architecture / Backend integration sections of root `README.md`

## 5. Documentation (`docs/` submodule, in `enclz/.github`)

- [ ] 5.1 In the docs submodule, add a "Program Integration Resources" section to `docs/REQUIREMENTS.md` alongside the existing "Agent Integration Resources" section, naming `@enclz/sdk` and the on-chain IDL channel
- [ ] 5.2 In the docs submodule, add a footnote on the "No SDK required" row of the competitive comparison in `docs/MARKETING.md` clarifying the agent-vs-program scope of the claim
- [ ] 5.3 Optional: add an `@enclz/sdk` row to the components table in `docs/SPECIFICATION.md` and a parallel bullet alongside `@enclz/mcp-server` in `docs/profile/README.md`
- [ ] 5.4 Open + merge a PR in `enclz/.github` for the above
- [ ] 5.5 In this repo, run `git submodule update --remote --merge docs && git add docs && git commit -m "chore(docs): sync after add-idl-publishing"`

## 6. Pre-publish gating

- [ ] 6.1 Verify `@enclz` npm org is owned by the publishing account; claim it if not. (Fall back to `enclz-sdk` unscoped only if blocked.)
- [ ] 6.2 Confirm 2FA is enabled on the publishing npm account
- [ ] 6.3 Run `npm run build:sdk` and inspect `sdk/dist/` for expected files (`index.js`, `index.d.ts`, `enclz.js`, `enclz.d.ts`, `enclz.json`)
- [ ] 6.4 From `sdk/`, run `npm publish --dry-run --access public`; confirm tarball contents = `dist/` only and version matches `target/idl/enclz.json` `metadata.version`

## 7. First publish + on-chain init

- [ ] 7.1 `npm run publish:sdk` — first public release of `@enclz/sdk`
- [ ] 7.2 Smoke-install in a throwaway directory: `npm install @enclz/sdk @coral-xyz/anchor @solana/web3.js`, then verify `require("@enclz/sdk").PROGRAM_ID` matches the deployed program ID
- [ ] 7.3 (After devnet deploy lands via `add-devnet-deploy-pipeline`) `npm run idl:init:devnet`; verify `anchor idl fetch` returns the same JSON as `target/idl/enclz.json`
- [ ] 7.4 (After mainnet deploy) `npm run idl:init:mainnet`

## 8. Verification

- [ ] 8.1 `npm run build:sdk` completes successfully on a fresh checkout
- [ ] 8.2 Running `npm run build:sdk` twice in a row produces byte-identical `sdk/dist/`
- [ ] 8.3 Manually editing `sdk/package.json` `version` to a different value and re-running `npm run build:sdk` restores it to `metadata.version`
- [ ] 8.4 A throwaway consumer can `import { IDL, type Enclz } from "@enclz/sdk"` and construct `new Program<Enclz>(IDL, provider)` without TS errors
- [ ] 8.5 `anchor idl fetch` against devnet returns identical JSON to `target/idl/enclz.json`
- [ ] 8.6 `sdk/README.md` mentions the Agent REST API + MCP server within the first two paragraphs
- [ ] 8.7 `docs/REQUIREMENTS.md` and `docs/MARKETING.md` reflect the program-vs-agent split (verified after submodule SHA bump in 5.5)
