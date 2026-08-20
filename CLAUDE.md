# GraphEvolutionTool

**If you are reading this and `.claude/` does not exist, that is the problem — fix it first.**

```bash
git clone https://github.com/md12ol/GET-claude.git .claude
# or, if your account is set up for SSH:
git clone git@github.com:md12ol/GET-claude.git .claude
```

`.claude/` is a separate private repository (`md12ol/GET-claude`) cloned into place as that
directory. It holds the working documents, the skills and the hooks — everything about *how* the two
owners work on this project. It is deliberately not tracked here, so that neither a package user nor
a contributor to the Rust ever receives it, and so it never appears in a built wheel or in a release
tag. Agreed at the joint meeting of 2026-08-13; GitHub #74. (This said "on a release branch" until
2026-08-20, when the joint meeting made GET trunk-plus-tags: `main` is the only long-lived branch and
a release is an annotated tag on it, so there is no release branch to keep anything off.)

**This file exists for exactly one failure.** When `.claude/` is missing, `settings.json` is missing
with it, so no hook runs and no skill loads — a session in that state has no conventions at all and
nothing to say so. This is the only file that still loads, so it is the only place the warning can
live. Once `.claude/` is cloned, `.claude/CLAUDE.md` loads too and carries the real rules; there is
no need to repeat them here, and a second copy would only drift.

## The short version, for a session that cannot clone

- `official_spec_sheet.md` at the repo root is the authority on how this system is designed. Read it
  before changing anything under `get/src/`. It is amended only at a joint meeting of both owners.
- **Every** change to this repository goes on a feature branch and through a pull request — code,
  documentation, the spec sheet, a one-line typo fix, all of it. Never a direct push to `main`, which
  a ruleset now enforces. Widened at the joint meeting of 2026-08-20 from "anything under `get/src/`,
  `Cargo.toml`, `config.example.toml` or the spec sheet".
- Nobody merges their own pull request.
- Don't commit or push unless you were asked to.
