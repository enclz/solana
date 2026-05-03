# Security Policy

## Reporting a Vulnerability

If you believe you have found a security vulnerability in the Enclz program,
please report it privately to **security@enclz.dev** rather than opening a
public GitHub issue. Include:

- A description of the vulnerability and the impact you believe it has.
- Steps to reproduce, or a proof-of-concept transaction / instruction.
- The deployed program ID / cluster you tested against.
- Any suggested mitigations.

We will acknowledge receipt within 72 hours and aim to validate the report and
respond with a remediation timeline within 14 days.

## Supported Versions

Until the program is mainnet-deployed and audited, only `main` is supported.
Devnet deployments track `main` via `npm run deploy:devnet`.

## Disclosure Policy

We follow coordinated disclosure: please give us a reasonable window to deploy a
fix before publishing details. Once a fix is live on every supported cluster
and the upgrade authority has been rotated (mainnet only), full details may be
published.

## Bug Bounty

Pre-mainnet there is no formal bounty. A bounty may be announced after mainnet
deploy and external audit; check the README for the current status.

## Upgrade Authority

On mainnet, the upgrade authority MUST be a Squads multisig before any user
funds touch the program. The deploy script (`migrations/deploy.ts`) refuses to
publish to mainnet with a single-sig wallet unless `--force-mainnet` is passed.
