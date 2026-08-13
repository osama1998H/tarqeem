---
description: Pick work from the Tarqeem beta Roadmap board — shortlist five if Planning is empty, otherwise start the most impactful planned issue
argument-hint: "[issue number to start directly]"
allowed-tools: Bash(gh:*), Bash(git:*), Read, Grep, Glob
---

# Roadmap

Select work through the project board instead of scanning open issues by hand.

`$ARGUMENTS` — optional issue number. If given, skip selection and start that issue
directly (see **Branch C**).

## Step 0 — Discover the board

Never hardcode IDs; the board is replaced each release cycle (beta → alpha → …).

```bash
gh project list --owner osama1998H --format json
gh project field-list <number> --owner osama1998H --format json
gh project item-list <number> --owner osama1998H --format json --limit 100
```

From these, extract: project id, the `Status` field id, the option id for each status,
and for each item its `id` plus `content.number`.

Statuses are `Todo` → `Planning` → `In Progress` → `Done`. If a future board renames
them, map by position: first = backlog, second = planned, third = active, fourth =
finished. **Never set `Done` by hand** — repository automation does that on issue close.

Moving an item:

```bash
gh project item-edit --id <ITEM_ID> --project-id <PROJECT_ID> --field-id <STATUS_FIELD_ID> --single-select-option-id <OPTION_ID>
```

If any `gh project` call reports `missing required scopes`, stop and tell the user to
run `gh auth refresh -h github.com -s project` in their own terminal. Note that gh's
own hint suggests `read:project`, which is read-only and cannot move items.

## Step 1 — Read the `Planning` column first

Do this before reading any issue body, opening any source file, or forming any opinion
about what to work on.

---

## Branch A — `Planning` is empty → shortlist five, then stop

1. Choose **5** issues from `Todo`.
2. Move each `Todo` → `Planning`.
3. Report them: issue number, one-line title, and **one line of rationale each** naming
   which criterion below drove the pick, plus a suggested order.
4. **Stop.**

**The shortlist is the entire deliverable of this branch.** Do not trace, debug, read
source, plan a fix, or fan out subagents in the same turn — not even for the one you
would rank first, and **not even if the invocation asked to pick the highest-impact
issue and fix it**. That phrasing selects this command; it does not override it. The
point is that the user sees the five and their reasoning before any work begins.

The user then picks. Starting one is a separate invocation.

### Choosing the five

In priority order:

1. **Cluster over spread.** Five issues in one pipeline layer compound — context earned
   on the first makes the rest cheaper. Five scattered across lexer, LSP, and codegen
   do not.
2. **Unblockers first.** An issue other issues depend on outranks a bigger isolated one.
3. **Silent wrong output over cosmetic.** A backend that quietly produces the wrong
   answer is this project's recurring failure mode, and outranks a formatting nit.
4. **Prefer issues with a reproduction** over ones still needing investigation.

Name the criterion in each rationale. "Grouped with #253 — same MethodId naming bug"
is a rationale; "high impact" is not.

---

## Branch B — `Planning` has items → start the most impactful one

The selection was already made and agreed. Do not re-plan and do not re-shortlist.

1. Pick the highest-impact item in `Planning`, using the same criteria.
2. Say which one and why, in one line.
3. Move it `Planning` → `In Progress`.
4. Branch per Gitflow: `git checkout develop && git pull origin develop && git checkout -b <type>/<slug>` — `bugfix/` for a bug, `feature/` for an enhancement.
5. Work it through to a pull request into `develop`, following
   `.claude/rules/00-operating-procedure.md` (explore → plan → implement → verify).
6. Open the PR with `Fixes #<n>` so board automation closes it.

---

## Branch C — an issue number was passed

`$ARGUMENTS` is an explicit instruction and overrides selection. Confirm the issue
exists, move its board item to `In Progress` (adding it to the board first if absent),
then proceed as Branch B from step 4.

---

## Rules that still apply

- **One issue per PR.** If two genuinely must share one, say why in the body and link both.
- **No AI attribution** anywhere — issue bodies, PR bodies, or commit trailers.
- Verify before claiming done: `cargo fmt --check && cargo clippy && cargo test`.
- `.claude/rules/bug-tracking.md` is the authority on anything this command does not cover.
