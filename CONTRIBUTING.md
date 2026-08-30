# Contributing to Rsact

Thanks for your interest in contributing. This document explains how to set
up your environment, and — most importantly — the commit and pull request
naming rules that CI enforces on every PR.

## Development setup

1. Install a recent stable Rust toolchain (`rustup install stable`).
2. Fork the repository and clone your fork.
3. Create a branch off `main` for your change.
4. Build and test locally before opening a PR:

   ```sh
   cargo build --workspace
   cargo test --workspace
   cargo fmt --all -- --check
   cargo clippy --workspace --all-targets -- -D warnings
   ```

## Commit message convention (enforced by CI)

Every commit message must follow [Conventional Commits](https://www.conventionalcommits.org/):

```
<type>(<optional scope>): <short summary>

[optional body]

[optional footer(s)]
```

Rules, all checked automatically by the `Commit Messages` workflow
(commitlint) on every pull request:

- `<type>` must be one of:

  | type       | use for                                              |
  |------------|-------------------------------------------------------|
  | `feat`     | a new feature                                          |
  | `fix`      | a bug fix                                              |
  | `docs`     | documentation only                                     |
  | `style`    | formatting, whitespace — no code behavior change       |
  | `refactor` | code change that neither fixes a bug nor adds a feature|
  | `perf`     | a performance improvement                              |
  | `test`     | adding or correcting tests                             |
  | `build`    | build system or dependency changes                     |
  | `ci`       | CI configuration changes                               |
  | `chore`    | anything else that doesn't touch `src`                 |
  | `revert`   | reverts a previous commit                              |

- The summary (the part after `type:`) starts with a **lowercase letter**,
  uses the imperative mood ("add", not "added"/"adds"), and has **no
  trailing period**.
- The full header line (`type(scope): summary`) must be **72 characters or
  fewer**.
- `scope` is optional — use it to name the crate or module the commit
  touches, e.g. `feat(rsact-core): add cell diffing engine`.

Examples:

```
feat(rsact-core): add run-length-encoded buffer diff
fix(rsact-ffi): null-check handle before rendering
docs: describe the render pipeline in the README
ci: add commit message linting
```

If you're unfamiliar with the format, the CI failure message on your PR
will point to the exact commit and rule that failed. You can also lint
locally before pushing:

```sh
npx --yes commitlint --from=main --to=HEAD
```

## Pull request title convention (enforced by CI)

PR titles follow the **same Conventional Commits format** as commit
messages (`type(scope): summary`), checked automatically by the `PR Title`
workflow. This keeps the PR title usable as-is as the squash-merge commit
message.

Examples of valid PR titles:

```
feat(rsact-core): add cell diffing engine
fix: restore terminal state on panic
docs(contributing): add commit convention
```

If the PR title doesn't match, the `PR Title` check fails with an
explanation; edit the title (no need to force-push) and the check re-runs
automatically.

## Pull request checklist

Before requesting review, make sure:

- [ ] The PR title follows the convention above.
- [ ] Every commit in the PR follows the convention above (squash locally
      with `git rebase -i` if you accumulated messy WIP commits).
- [ ] `cargo build --workspace`, `cargo test --workspace`, `cargo fmt --all
      -- --check`, and `cargo clippy` all pass.
- [ ] New public APIs have a short doc comment explaining *why*, not just
      *what* (the signature already says what).

## License

By contributing, you agree that your contributions will be licensed under
the project's [MIT License](LICENSE).
