# program-hardening Specification

## Purpose
TBD - created by archiving change add-devnet-deploy-pipeline. Update Purpose after archive.
## Requirements
### Requirement: CI workflow

The repo SHALL include `.github/workflows/program-ci.yml` that on every push and pull request runs `anchor build`, `cargo test`, `anchor test`, `cargo tarpaulin`, `cargo audit`, and `cargo deny check`.

#### Scenario: PR blocked on test failure
- **WHEN** a PR introduces a failing unit or integration test
- **THEN** CI fails and the PR cannot be merged

#### Scenario: PR blocked on coverage drop
- **WHEN** a PR drops instruction-code coverage below 85% (or `execute_transfer.rs` below 90%)
- **THEN** CI fails

#### Scenario: PR blocked on critical CVE
- **WHEN** `cargo audit` reports a critical advisory in any dependency
- **THEN** CI fails

### Requirement: Security.txt embedded

`programs/enclz/src/lib.rs` SHALL embed a `solana_security_txt!` macro with name, project_url, contacts, source_code, and audit fields.

#### Scenario: Security.txt readable from deployed program
- **WHEN** an auditor runs `query-security-txt` against the deployed program ID
- **THEN** all required fields are returned

### Requirement: Dependency policy

The repo SHALL include `deny.toml` configured to deny disallowed licenses (GPL, AGPL) and warn on duplicate dependency versions.

#### Scenario: New dep with disallowed license rejected
- **WHEN** a PR adds a dependency under a banned license
- **THEN** `cargo deny check` fails in CI
