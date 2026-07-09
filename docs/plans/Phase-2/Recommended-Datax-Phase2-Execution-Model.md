# Recommended Datax Phase 2 Execution Model

## Purpose

This document records the execution model for Phase 2. Phase 2 starts after the
Phase 1 migration baseline and introduces Datax-owned architecture around a
downstream Codex app-server integration. Phase 2 must keep Datax product state,
public APIs, and data-engineering concepts separate from Codex runtime concepts.

The product direction is captured in `docs/plans/Phase-2/platform.md`.

## Phase 2 Principle

Phase 2 is not a broad product-feature phase. It is the architecture and adapter
phase that makes future data-engineering product features possible.

Terminology in this document is intentionally explicit:

- `Datax app-server` means the app-server in this repository that serves Datax
  clients and owns the Datax public protocol.
- `downstream Codex app-server` means the external or separately managed Codex
  runtime app-server that Datax may call for agentic work.
- `AgentAdapter` means the Datax-owned boundary between the Datax app-server
  and any downstream agent runtime.
- `codex-runtime` means the implementation boundary that can translate
  `AgentAdapter` calls into downstream Codex app-server calls.

The target runtime shape is:

- Datax TUI/CLI or other clients call the Datax app-server.
- The Datax app-server owns Datax protocol, validation, product state, and
  data-engineering projections.
- The Datax app-server calls `AgentAdapter` when agentic work is required.
- `AgentAdapter` delegates to `codex-runtime` when the selected runtime is the
  downstream Codex app-server.
- Codex `Thread`, `Turn`, and `Item` concepts remain downstream runtime
  details and must not leak into normal Datax APIs.

## Phase 2 Structure

Treat Phase 2 as these implementation phases:

1. Phase 2.1: Product architecture baseline.
2. Phase 2.2: Downstream Codex boundary inventory.
3. Phase 2.3: Agent adapter contract.
4. Phase 2.4: Codex runtime adapter skeleton.
5. Phase 2.5: Datax app-server mediation.
6. Phase 2.6: Datax persistence and runtime-link model.
7. Phase 2.7: First data-engineering domain skeleton.
8. Phase 2.8: End-to-end adapter smoke and freeze checkpoint.

Each phase must be independently reviewable. Do not combine multiple phases into
one pull request unless a documented dependency proves that separating them
would leave the repository in an unbuildable or misleading state.

## Per-Phase Requirements

For each Phase 2 implementation phase:

- Create a PLANS.md-compliant ExecPlan under `docs/plans/Phase-2/`.
- Create a dedicated git branch using a `datax/phase2-*` branch name.
- Create a GitHub issue for that phase.
- Create a draft pull request early.
- Identify all files that belong to the phase before editing implementation
  files.
- Identify which of those files require modification or update only after
  dependency order is clear.
- Include a file inventory table in the ExecPlan with these exact columns:
  `Filename`, `Modified`, and `Remarks Notes`.
- Use `Modified` values of `Pending`, `In-Progress`, `Completed`, `Failed`, or
  `Not Required`.
- Use `Not Required` for inspected files that belong to the phase but do not
  require modification.
- Document exact post-implementation commands in the ExecPlan. The commands
  must be runnable command lines, not summaries.
- Defer long-running build, test, format, fix, and schema-generation commands
  to the user unless the user explicitly asks the agent to run a specific
  command.
- Update the ExecPlan as a living document whenever implementation scope,
  dependency order, validation, or retained Codex boundaries change.

Example file inventory table:

| Filename | Modified | Remarks Notes |
| --- | --- | --- |
| `datax-rs/app-server/src/lib.rs` | `Pending` | Candidate Datax app-server mediation owner; inspect before editing. |
| `datax-rs/app-server-protocol/src/protocol/v2.rs` | `Not Required` | Inspected for Datax app-server public API impact; no public protocol change in this phase. |
| `datax-rs/codex-runtime/src/lib.rs` | `Pending` | New downstream Codex runtime boundary if the phase creates the adapter crate. |

## Required Tracking Sections

Each Phase 2 ExecPlan must include:

- Baseline: starting branch, current commit, expected prior phase, and known
  validation state.
- File Inventory: all phase-owned or dependency-relevant files.
- Codex Boundary Register: every retained Codex reference touched by the phase,
  classified as downstream runtime, compatibility shim, upstream provenance,
  protected sandbox exception, external dependency, or unresolved.
- Public Surface Checklist: whether the phase touches CLI, config, Datax
  app-server protocol, generated schemas, TypeScript bindings, persisted state,
  UI text, packaging, or release metadata.
- Dependency Order: safe order for source edits, generated artifacts, tests,
  and documentation updates.
- Validation Matrix: explicit commands and status for static checks, generation,
  formatting, fix/lint, build, tests, and smoke commands.
- Validation and Acceptance: runnable command lines grouped by working
  directory, with expected results.
- Rollback or Recovery Note: how to back out the phase without damaging later
  work or generated artifacts.
- Open Questions and Decision Log: all unresolved and resolved design choices.

## Per-Phase Exit Criteria

A Phase 2 phase is complete only when:

- The phase ExecPlan is current and self-contained.
- The file inventory table has no unresolved `Pending`, `In-Progress`, or
  `Failed` rows.
- Every retained Codex reference touched by the phase is classified in the
  Codex Boundary Register.
- Public Datax APIs do not expose Codex `Thread`, `Turn`, or `Item` concepts.
- The phase branch, issue, and draft pull request exist and refer to the same
  scope.
- All post-implementation commands are listed explicitly.
- User-provided validation results are recorded in the ExecPlan before merging,
  or the deferral reason is recorded.
- No data-engineering product feature is implemented before the adapter and
  persistence boundaries needed by that feature are established.

## Phase 2 Migration Exit Criteria

Phase 2 is complete only when:

- Datax has a documented `AgentAdapter` contract between Datax app-server and
  downstream runtime behavior.
- Codex runtime integration is isolated behind a dedicated boundary such as
  `codex-runtime` or an equivalent name approved in the phase ExecPlan.
- Compatibility-only code is isolated or explicitly marked as future
  `codex-compat` work.
- The Datax app-server can mediate at least one end-to-end agent interaction
  through `AgentAdapter` without exposing Codex public concepts to Datax
  clients.
- Datax-owned persistence can store Datax records and runtime-link records
  separately.
- The first data-engineering domain skeleton exists without being modeled as a
  thin wrapper around Codex history.
- The final Phase 2 freeze checkpoint records remaining risks, deferred
  product features, adapter limitations, and Phase 3 candidates.

## Suggested Branches

Use these branch names unless a phase ExecPlan records a better scoped name:

- `datax/phase2-1-architecture-baseline`
- `datax/phase2-2-codex-boundary-inventory`
- `datax/phase2-3-agent-adapter-contract`
- `datax/phase2-4-codex-runtime-skeleton`
- `datax/phase2-5-app-server-mediation`
- `datax/phase2-6-persistence-runtime-links`
- `datax/phase2-7-domain-skeleton`
- `datax/phase2-8-adapter-smoke-freeze`

## Protected Exceptions

Do not rename or modify code related to:

- `CODEX_SANDBOX_NETWORK_DISABLED_ENV_VAR`
- `CODEX_SANDBOX_ENV_VAR`
- `CODEX_SANDBOX_NETWORK_DISABLED`
- `CODEX_SANDBOX`

These identifiers are tied to sandbox behavior and are not product identity.

## Review Boundaries

Phase 2 pull requests should avoid:

- Broad folder restructuring of Datax-owned code that is unrelated to adapter
  or persistence boundaries.
- Data-engineering product workflows before the phase that establishes the
  required architecture boundary.
- Direct downstream Codex app-server calls from Datax app-server code outside
  `AgentAdapter` or `codex-runtime`.
- Public API names that expose Codex `Thread`, `Turn`, or `Item`.
- Mechanical internal renames that are not dependency-aware.
