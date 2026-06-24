Goal: Build the full solution defined in tsd.md and tbk.md.

Start by reading tsd.md, tbk.md, DEVELOPMENT.md, and recent GitHub commits to determine the current implementation status and remaining gaps. Then create a short plan and execute it checkpoint by checkpoint until the solution is fully implemented.

Completion criteria:
- Every required feature and deliverable described in tsd.md and tbk.md is implemented.
- DEVELOPMENT.md is updated to reflect the true current status, completed work, remaining architecture notes, and any decisions made during implementation.
- The codebase is internally consistent with the latest relevant GitHub commits and does not re-do already completed work.
- All relevant tests pass, and any necessary build/lint/typecheck commands pass cleanly.
- Any unfinished items, blockers, or assumptions are explicitly documented in DEVELOPMENT.md only if they prevent full completion.

Constraints:
- Do not invent scope beyond tsd.md and tbk.md.
- Prefer updating existing code and docs over duplicating files or creating parallel implementations.
- Verify current status from the repository state, Git history, and DEVELOPMENT.md before making changes.
- Work in small checkpoints; after each checkpoint, verify what changed and what remains.
- Only mark the goal complete when the implementation, verification, and documentation updates are all done.
- all agents and subagents should be using kimi-k2.7-code model