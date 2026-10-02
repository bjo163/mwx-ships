## Summary
<!-- Provide a concise summary of the changes in this pull request. -->

## Linked Issue
<!-- Link the GitHub issue this PR resolves (e.g. Closes #12). Every PR must resolve an issue. -->
Closes #

## Testing Performed
<!-- Describe the tests executed to verify this change. Include commands run (cargo test, clippy, etc.). -->
- [ ] Unit & service tests passed (`cargo test`)
- [ ] Formatting verified (`cargo fmt --check`)
- [ ] Strict clippy check passed (`cargo clippy --all-targets --all-features -- -D warnings`)

## SQLite Impact
<!-- Describe any impact on SQLite query performance, locking, transaction boundaries, or WAL mode. -->

## Migration Impact
<!-- Does this PR add or alter SeaORM migrations? Has it been tested on a fresh SQLite database? -->
- [ ] No migration changes
- [ ] Migration added and tested on empty database

## Security Impact
<!-- Are secrets masked? Are shell inputs structured? Is crypto key handling secure? -->
- [ ] No secrets exposed in logs or API responses
- [ ] No unescaped shell concatenation
- [ ] Cryptographic inputs validated

## Documentation Impact
<!-- Have docs/ or ADRs been added or updated? -->
- [ ] Relevant documentation updated in `docs/`
- [ ] ADR added in `docs/adr/` if architectural decision was made

## Breaking Changes
<!-- List any breaking API changes or schema modifications. -->
- [ ] No breaking changes
