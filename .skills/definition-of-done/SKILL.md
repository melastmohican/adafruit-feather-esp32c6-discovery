---
name: definition-of-done
description: Pre-commit verification checklist to ensure code is ready for commit and will pass CI checks.
---

# Definition of Done Skill

This skill defines the verification steps that must be completed before committing code to the repository. These steps mirror the GitHub Actions CI workflow to catch issues locally.

## Pre-Commit Checklist

Run the following commands in order. All must pass before committing.

### 1. Format Check

Verify that all code is properly formatted according to Rust style guidelines.

```bash
cargo fmt --all -- --check
```

**If this fails:**

```bash
cargo fmt --all
```

### 2. Clippy Lints

Run Clippy to catch common mistakes and enforce best practices. All warnings are treated as errors.

```bash
cargo clippy --workspace --examples --all-features -- -D warnings
```

**If this fails:** Fix the issues reported by Clippy. Common fixes include:

- Removing unused imports
- Removing unused variables (or prefix with `_`)
- Simplifying expressions
- Following idiomatic Rust patterns

### 3. Documentation Check

If there is a `README.md` file in the repository that describes the examples, verify that it is in sync with the documentation inside the example source files.

- Example source files (file headers) are the **source of truth**.
- Ensure pinouts, wiring diagrams, and descriptions in `README.md` match the source code.
- Add or update `README.md` sections if new examples are added or existing ones are modified.

### 4. Security Audit

Check dependencies for known vulnerabilities.

```bash
cargo audit
```

**If this fails:** Update vulnerable dependencies using `cargo update -p <package>`.

## Quick Verification Script

You can run all checks sequentially with:

```bash
cargo fmt --all -- --check && \
cargo clippy --workspace --examples --all-features -- -D warnings && \
cargo build --release && \
cargo audit
```

## 🛑 STOP AND WAIT

**CRITICAL:** Do NOT proceed to commit/push immediately after verification.

1. **Report Results:** Summarize which checks passed.
2. **Show Changes:** Run `git status` and `git diff` so the user can see exactly what will be committed.
3. **ASK FOR PERMISSION:** Explicitly ask: "Checks passed. Ready to commit?"
4. **WAIT** for the user's "Yes" or specific instructions.

## Commit and Push

**ONLY** after receiving explicit user approval in the previous step:

1. **Stage Files:**
   ```bash
   git add <specific-files>
   ```
2. **Commit:**
   ```bash
   git commit -m "Your commit message"
   ```
3. **Push:**
   ```bash
   git push
   ```

**Important:**

- **Never** add files from `.gitignore` (e.g., `target/`, `Cargo.lock` for libraries)
- **Never** add global ignore patterns (e.g., `.claude/`, `.skills/`, `.gemini/`)
- Only commit source code, documentation, and configuration files that are part of the project

## Notes

- These checks mirror the GitHub Actions workflow in `.github/workflows/rust_ci.yml`.
- The CI uses the `riscv32imc-unknown-none-elf` target, which should already be configured in your project.
- If you add new examples, ensure they compile by running `cargo check --example <name>` before committing.
