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

## If you are on the web or an iPad, in a cloud container

The container clones this repository fresh every session and is reclaimed after inactivity, so the
clone above is **per-session, not per-machine**: you will do it again next time. Once it is in
place:

```bash
.claude/hooks/cloud_setup.sh <mdube|jsargant>   # your own name, not the other owner's
.claude/checks/cloud_ready.sh                   # non-zero exit if any check fails
```

`cloud_setup.sh` sets the git identity **globally**, turns off the container's own commit signing
(it signs with a key that is neither owner's), builds the venv, installs maturin and builds the
crate. Measured 2026-09-02: 53s cold, 17s on a re-run.

**Run it on a cloud container only, and it enforces that itself**: outside one it refuses and
changes nothing, because setting a global identity and disabling global commit signing on a machine
you own would override your own settings for every repository on it. `cloud_ready.sh` has no such
guard and needs none: it writes nothing anywhere.

**Re-run `cloud_setup.sh` after a resume or a compact, not just once.** The container rewrites its
own name and email back into `~/.gitconfig`, so the identity does not survive; a resume on
2026-09-02 came back as `Claude` and the session brief stopped. Re-running costs nothing when
nothing needs changing.

**Set your own identity, and never copy the other owner's to get past a stop.** The working-docs
skills resolve who you are from `git config user.email`, and a cloud container ships an address the
owner table does not know, so they stop rather than guess. Using someone else's address writes into
their `work/` directory. `cloud_setup.sh` refuses rather than guessing if you do not name an owner.

**Hooks do not run on a cloud session's first startup**, because `settings.json` is inside the
directory that has not been cloned yet. A resume or a compact does fire them, so the session brief
arrives late rather than never. Nothing is watching you in between: save deliberately.

**A refusal to register `.claude/` as a repository root is not a broken clone.** The harness expects
a repository added mid-session to live at `/home/user/get-claude` and rejects any other path, so
registering the clone you just made will fail. Nothing is wrong with it. Read `.claude/CLAUDE.md`
yourself, which is the fallback that refusal names, and carry on: the skills and hooks load from the
directory regardless of what is registered.

## The short version, for a session that cannot clone

- `official_spec_sheet.md` at the repo root is the authority on how this system is designed. Read it
  before changing anything under `get/src/`. It is amended only at a joint meeting of both owners.
- **Every** change to this repository goes on a feature branch and through a pull request — code,
  documentation, the spec sheet, a one-line typo fix, all of it. Never a direct push to `main`, which
  a ruleset now enforces. Widened at the joint meeting of 2026-08-20 from "anything under `get/src/`,
  `Cargo.toml`, `config.example.toml` or the spec sheet".
- Nobody merges their own pull request.
- Don't commit or push unless you were asked to.
