# Room MVP Self-Hydrating Workflow Process

## Project Structure & Key Documents
- `START.txt`: single entry point; points to everything else
- `docs/procs/QUICK_REF.md`: 30-second situational awareness
- `docs/procs/CONTINUE.md`: live session log and handoff notes (update every session)
- `docs/procs/SPRINT.md`: active iteration scope and task ordering
- `docs/procs/TASKS.md`: canonical task backlog and milestone checklist
- `docs/procs/ROADMAP.md`: long-range phase planning
- `docs/procs/BACKLOG.md`: pending tickets coming out of the MVP roadmap
- `docs/procs/archive/`: historical artefacts (prior tasks, notes)
- `docs/ref/`: architecture, strategy, API, and research references
- `.analysis/` + `.eggs/`: consolidated agent wisdom and technical debt summaries

## Context Hydration Checklist
1. Read `START.txt` (ensures you are in the right workflow mindset).
2. Scan `docs/procs/QUICK_REF.md` for current objective, blockers, and commands.
3. Deepen context with `docs/procs/CONTINUE.md` to understand what just happened.
4. Review `docs/procs/SPRINT.md` for the work queue relevant to this iteration.
5. Pull any deeper architectural detail from `docs/ref/strat/LAYOUT_ENGINE_STRATEGY.md`, `docs/ref/strat/RUNTIME_STRATEGY.md`, or the specific reference doc for your task.

## Session Execution Pattern
- **During Work**
  - Work from the top of `docs/procs/SPRINT.md` unless the roadmap dictates otherwise.
  - Keep `docs/procs/TASKS.md` in sync when scope changes; move completed work into `docs/procs/DONE.md` at milestone boundaries.
  - Record meaningful architectural or process insights in the appropriate `docs/ref/` note.
- **End of Session**
  1. Update `docs/procs/CONTINUE.md` with accomplishments, decisions, blockers, and next-step ordering.
  2. Refresh timestamps or content in `docs/procs/QUICK_REF.md` if priorities shifted.
  3. Run `./bin/validate-docs.sh`; resolve any errors before leaving the branch dirty.
  4. Commit code and doc updates together or document intentional divergence in CONTINUE.

## Phase Guide
- **Planning / Alignment**: Focus on `docs/procs/ROADMAP.md` updates, backlog grooming, and capturing triage within `.analysis/`.
- **Implementation**: Pull ready tasks from `docs/procs/SPRINT.md`, reference supporting notes in `docs/ref/`, and keep CONTINUE current.
- **Review / Hardening**: Emphasise validation output, update `docs/ref/LESSONS` style notes, and archive sprint artefacts into `docs/procs/DONE.md` once accepted.

## Agent & Onboarding Rules
- New contributors must prove the system works: follow `START.txt` → `PROCESS.md` → `CONTINUE.md` → `SPRINT.md` before touching code.
- If anything felt missing or stale, file a note in `docs/procs/CONTINUE.md` and create a chore in `docs/procs/TASKS.md`.
- When adding new knowledge, prefer `docs/ref/<topic>.md`; when adding new process, update this file and the validator.

## Reference Shortcuts
- Runtime architecture: `docs/ref/strat/RUNTIME_STRATEGY.md`
- Plugin API details: `docs/ref/PLUGIN_API.md`
- Layout/registry/renderer strategy: `docs/ref/strat/LAYOUT_ENGINE_STRATEGY.md`
- Logging & metrics: `docs/ref/strat/LOGGING_STRATEGY.md`, `docs/ref/FEATURES_RUNTIME_PHASE2.md`
- Token protocols: `docs/ref/METEOR_TOKENS.md`, `docs/ref/strat/SOCKET_STRATEGY.md`

## Validation Discipline
- Run `./bin/validate-docs.sh` before and after major edits.
- Treat warnings on critical docs (`CONTINUE`, `SPRINT`, `QUICK_REF`) as debts that must be cleared before the next session begins.
- Update the validator whenever you add/rename key documents so it remains the single source of truth.

## Validator Maintenance
- **Critical Docs Threshold**: 7 days for docs in `docs/procs/`, `docs/ref/` that represent active workflow state
- **Support Docs Threshold**: 30 days for architectural reference and strategy documents
- **Analysis Docs Threshold**: 14 days for `.analysis/` and `.eggs/` wisdom capture documents
- **Silent-Success Requirement**: Validator must run without warnings for a successful session
- **Maintenance Responsibility**:
  * Always run `./bin/validate-docs.sh` before and after session
  * Update the script when introducing new process or reference files
  * Ensure the validator remains the single source of truth for documentation health
  * Classify new documents when you add them: extend `critical_docs`, `support_docs`, or `analysis_docs` in `bin/validate-docs.sh` instead of leaving ad hoc checks in place.
  * Keep the `ref_docs` allowlist current. When you add a must-exist workshop or strategy note, add its path so the validator protects it.
  * If you intentionally leave a warning (for example, a stale doc awaiting approval), record the rationale in `docs/procs/CONTINUE.md` and link the follow-up task.

## Handoff Requirements Checklist
- CONTINUE entry created with Summary / Status / Next Steps / Blockers sections filled.
- QUICK_REF reflects latest focus and commands.
- Any new insight copied into the appropriate `docs/ref/` note.
- Validator passes without errors.

Following this process keeps the Room project self-hydrating: anyone can regain context in minutes, and no knowledge gets trapped in the heads of whoever worked last.
