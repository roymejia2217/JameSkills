# Capability Map: JameSkills

JameSkills is a cross-platform desktop application for authoring, validating,
versioning, syncing, and installing structured skill suites for coding agents.
Each skill combines portable instructions with explicit standards and
machine-checkable policies; the application should report and enforce those
policies through normal tools and workflows rather than opaque repair scripts.

| Module id | Responsibility | Depends on |
|---|---|---|
| `skill-format` | Define the portable on-disk skill bundle, metadata, policy declarations, validation rules, and compatibility/versioning contract. | — |
| `policy-engine` | Evaluate declared requirements and produce clear, actionable checks for repository setup, secrets, commits, README, branches/PRs, CI, and releases. | `skill-format` |
| `agent-adapters` | Detect supported coding-agent CLIs and translate/install the common skill bundle into each agent's documented local format. | `skill-format` |
| `skill-library` | Create, edit, import, browse, version, and locally store skills and their assets. | `skill-format`, `policy-engine` |
| `cloud-sync` | Back up and restore the local library through Google Drive, with explicit account linking, conflict handling, and protected backup data. | `skill-format`, `skill-library` |
| `desktop-app` | Provide the native Windows/Linux shell, onboarding, library and skill editor, validation results, agent detection/installation, and sync settings. | `skill-library`, `agent-adapters`, `cloud-sync` |

Build order: `skill-format` → (`policy-engine`, `agent-adapters`) →
`skill-library` → `cloud-sync` → `desktop-app`.

## Assumptions to validate

1. The current deliverable is a complete specification and implementation
   plan for v1, requested explicitly by the user, executable by GPT6 Luna.
   All application implementation tasks remain pending.
2. Skills are portable directories with a common manifest and policy files;
   agent-specific files are generated or installed by adapters and do not
   become the canonical source. Codex, OpenCode, Pi, Antigravity CLI and
   Grok Build contracts are documented in docs/SPEC-agent-adapters.md.
3. “Enforce” means checks that can block or warn at clear workflow points
   (validation, local Git hooks, or agent-native rules), with each check
   explainable and inspectable. No opaque automatic rewriting.
4. Windows and Linux are the required initial desktop targets. macOS can be
   considered later if it follows the same architecture.
5. Google Drive is the initial backup provider. Restore and conflict behavior
   must be designed before sync is considered complete.
6. GPUI Kit 0.7.0 and the public formats of all five named CLI agents have
   official source evidence in docs/SOURCES.md; local version compatibility
   and platform runtime still require implementation-time verification.

## Main risks

- GPUI's supported targets and maturity may differ between Windows and Linux;
  GPUI Kit 0.7.0 platform coverage is documented and selected; native build,
  accessibility and runtime evidence remain implementation gates.
- Agent instruction formats and installation paths can change independently;
  adapters need versioned capability checks and must fail visibly when unsure.
- GitHub branch protection and PR policy generally require repository-host
  permissions and cannot be guaranteed by a local skill alone. JameSkills can
  validate settings and guide setup, while remote enforcement depends on the
  host and the user's authorization.
- Cloud backup security depends on OAuth token storage, encryption boundaries,
  and restore/conflict behavior; these need an explicit design before enabling
  cloud writes.

## Specifications and execution index

- docs/REQUIREMENTS.md: complete v1 acceptance and scope.
- docs/ARCHITECTURE.md + docs/CONTRACTS.md: files, boundaries, functions and wiring.
- docs/SPEC-<module-id>.md: one contract per module above.
- docs/GUI.md, docs/SECURITY.md, docs/TESTING.md, docs/OPERATIONS.md.
- tasks/plan.md + tasks/todo.md + tasks/HANDOFF-LUNA.md: full execution plan.

The user requested the entire documentary plan without intermediate approval
rounds on 2026-10-02. This does not mark application work, GitHub settings,
Google authorization or native runtime checks complete.
