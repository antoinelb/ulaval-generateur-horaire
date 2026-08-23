# AGENTS.md

This file provides guidance to Codex when working with code in this repository.

## Shared project instructions

Before planning, reviewing, or changing this project, read `CLAUDE.md` completely.
It remains the shared source of truth while Claude Code and Codex are supported in parallel.

Apply every project fact, architectural constraint, verification requirement, and coding rule in `CLAUDE.md` to Codex.
Where that file says “Claude”, read “the active coding agent”, including Codex.
Where it requires `.claude/dioxus.md`, use the repository skill `dioxus-0-7`; the skill loads that shared reference before any Dioxus work.

Codex-specific configuration lives under `.codex/` and repository skills live under `.agents/skills/`.
Do not remove or rewrite the parallel Claude configuration unless the user explicitly asks to end that compatibility period.
