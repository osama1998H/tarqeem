# Vendored Skill — Provenance

This skill is third-party source copied into the repository, not original work.

| | |
|---|---|
| Upstream | https://github.com/WH-2099/mermaid-skill |
| License | MIT (see `LICENSE`) |
| Pinned commit | `982d9035231312c527197501a882cf6c8f4394cc` |
| Vendored on | 2026-08-13 |

## Why vendored

The skill ships no plugin manifest, so it cannot be installed with `/plugin install`.
Committing it keeps `.claude/rules/diagrams.md` self-contained: every contributor and
agent gets the same syntax reference without a per-machine setup step.

It was chosen over the alternatives because it has **no runtime dependencies** — it is
`SKILL.md` plus per-diagram-type reference files, loaded on demand. The other candidates
require npm / `mermaid-cli` / headless Chrome to render PNG or SVG, which this project
does not need: GitHub renders ```mermaid fences natively, and diagrams are validated with
the `mermaidchart` MCP tool.

## Refreshing

Upstream syncs its references from `mermaid-js/mermaid` weekly. To update:

```bash
git clone --depth 1 https://github.com/WH-2099/mermaid-skill /tmp/mermaid-skill
rm -rf .claude/skills/mermaid/references
cp -r /tmp/mermaid-skill/.claude/skills/mermaid/references .claude/skills/mermaid/references
cp /tmp/mermaid-skill/.claude/skills/mermaid/SKILL.md .claude/skills/mermaid/SKILL.md
```

Then update the pinned commit above. Refresh only when a diagram type is missing or its
syntax has changed — this is reference material, not a dependency that needs tracking.

## Scope

The skill governs *how to write mermaid syntax correctly*. It does **not** govern when a
diagram is appropriate or how large it may be — `.claude/rules/diagrams.md` decides that,
and its limits override the skill's richer defaults (theming, styling, elaborate output).
