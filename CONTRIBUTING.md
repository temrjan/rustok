# Contributing to Rustok

Thank you for your interest in contributing to Rustok.

## License of Contributions

By submitting a pull request, you agree that your contribution will be
distributed under the project's licensing terms:

- **AGPL-3.0-or-later** for source code (see `LICENSE`).

You retain copyright to your contribution. You grant the Rustok project
the right to distribute your contribution under the AGPL-3.0-or-later
open source license **and** under any commercial license the copyright
holder may offer (see `LICENSE-COMMERCIAL.md`), to enable dual-license
distribution.

If you cannot agree to this grant, please do not submit contributions.

---

## Developer Certificate of Origin (DCO)

Rustok uses the **Developer Certificate of Origin** (DCO) version 1.1 to
certify the provenance of contributions. DCO is a lightweight alternative
to a Contributor License Agreement.

Every commit must include a `Signed-off-by:` line.

### What Signing Off Certifies

By signing off a commit, you certify the following (full text at
<https://developercertificate.org>):

1. The contribution was created in whole or in part by you and you have
   the right to submit it under the open source license indicated in
   the file; or
2. The contribution is based upon previous work that, to the best of
   your knowledge, is covered under an appropriate open source license
   and you have the right under that license to submit that work with
   modifications, whether created in whole or in part by you, under the
   same open source license (unless you are permitted to submit under
   a different license), as indicated in the file; or
3. The contribution was provided directly to you by some other person
   who certified (1), (2), or (3) and you have not modified it; and
4. You understand and agree that this project and the contribution are
   public and that a record of the contribution (including all personal
   information you submit with it, including your sign-off) is maintained
   indefinitely and may be redistributed consistent with this project or
   the open source license(s) involved.

### How to Sign Off

Use the `-s` flag with `git commit`:

```bash
git commit -s -m "your commit message"
```

This appends a `Signed-off-by: Your Name <your@email>` line to your
commit message, using `user.name` and `user.email` from your git config.

### Fixing Missing Sign-Off

For your most recent commit:

```bash
git commit --amend -s --no-edit
```

For a series of commits, use an interactive rebase with `--signoff`.

---

## Pull Request Checklist

1. Fork the repository.
2. Create a feature branch: `git checkout -b feature/your-feature`.
3. Make changes with DCO-signed commits (`git commit -s`).
4. `cargo test` — all tests pass.
5. `cargo fmt --all` — formatting clean.
6. `cargo clippy --all-targets -- -D warnings` — no clippy warnings.
7. `cargo deny check` — no new license or advisory violations.
8. Open a pull request against `main` with a clear description of the
   change and its motivation.

---

## Code Quality Expectations

- Ensure all Rust and mobile pre-commit gates pass (`cargo fmt`, `cargo clippy`, `cargo test`, `cargo deny`, and `mobile` `typecheck`/`lint`/`test`).
- New features must include tests.
- Changes to security-critical code (`crates/core/keyring`,
  `crates/txguard`) require extra review.
- Keep changes focused. Avoid mixing unrelated refactors into a feature
  PR.

---

## Security Issues

**Do not open public issues for security vulnerabilities.**

Report privately to <security@rustokwallet.com>.

---

## Questions

If a proposed change is large or its direction is unclear, please open
a GitHub discussion or draft issue before investing significant time.
