## Summary

<!-- What does this PR do, and why? One or two sentences. -->

## Related issue

<!-- Closes #123 — or "N/A" -->

## Changes

-

## Test plan

- [ ] `cargo build --workspace`
- [ ] `cargo test --workspace`
- [ ] `cargo fmt --all -- --check`
- [ ] `cargo clippy --workspace --all-targets -- -D warnings`

## Checklist

- [ ] PR title follows [Conventional Commits](../CONTRIBUTING.md#pull-request-title-convention-enforced-by-ci)
      (`type(scope): summary`, lowercase, no trailing period), e.g.
      `feat(rsact-core): add cell diffing engine`
- [ ] Every commit in this PR follows the
      [same convention](../CONTRIBUTING.md#commit-message-convention-enforced-by-ci)
      (squash/rebase locally with `git rebase -i` if you have WIP commits)
- [ ] Branch name follows `type/description` or `type/<issue-number>-description`
      (e.g. `fix/4-cursor-position-on-exit`)
- [ ] New public APIs have a short doc comment explaining *why*, not just *what*
- [ ] README/docs updated if behavior changed
