<!--
================================================================================
SYNC IMPACT REPORT
================================================================================
Version Change: 0.0.0 → 1.0.0 (initial constitution)
Modified Principles: N/A (new document)
Added Sections:
  - Core Principles (5 principles defined)
  - Security & Privacy Requirements (new section)
  - Development Workflow (new section)
  - Governance (new section)
Removed Sections: N/A
Templates Requiring Updates:
  ✅ .specify/templates/plan-template.md - verified compatible
  ✅ .specify/templates/spec-template.md - verified compatible
  ✅ .specify/templates/tasks-template.md - verified compatible
  ⚠ .specify/templates/commands/*.md - directory does not exist, no action needed
Follow-up TODOs: None - all placeholders resolved
================================================================================
-->

# LogPilot for tmux Constitution

## Core Principles

### I. Local-First Architecture (NON-NEGOTIABLE)
All log capture and processing MUST execute locally on the user's machine. No cloud dependencies for core functionality. No data exfiltration - logs never leave the local environment unless explicitly configured by the user. Security and privacy are paramount for production incident data.

**Rationale**: Support engineers handle sensitive production logs. Local-first ensures compliance, reduces attack surface, and eliminates network latency for real-time incident response.

### II. Real-Time Performance
System MUST maintain <2 second latency from log emission to AI visibility. Handle 10k log lines/minute per pane efficiently. Implement streaming data processing with minimal buffering. Rolling window context management with token-aware summarization.

**Rationale**: Incident response requires immediate awareness. Delays in anomaly detection reduce effectiveness of AI assistance during critical outages.

### III. CLI-Native Interface
All functionality MUST be accessible via CLI commands matching the proposed interface (`logpilot watch`, `logpilot summarize`, `logpilot ask`). Support both human-readable and JSON output formats. No GUI dependencies for MVP. Text in/out protocol: stdin/args → stdout, errors → stderr.

**Rationale**: Target users are terminal-native engineers already working in tmux. CLI-first respects their workflow and enables scriptability for automation.

### IV. AI Context Bridge
Structured, token-aware output optimized for LLM consumption. MCP (Model Context Protocol) support for Claude/Codex integration. Implement deduplication to reduce noise. Cluster similar errors into incidents. Export summaries in formats compatible with AI assistant context APIs.

**Rationale**: The product's core value is bridging tmux logs to AI assistants. Direct, efficient integration with Claude Code and Codex is the primary differentiator.

### V. Test-First & Observability
TDD mandatory: Tests written → fail → implement → pass. Red-Green-Refactor cycle enforced. Integration tests REQUIRED for tmux interaction. Structured logging required for self-observability (dogfooding principle). Error rate, latency, and throughput metrics exposed.

**Rationale**: Critical tooling for production incidents must be reliable. Self-observability ensures LogPilot can monitor its own performance during customer incidents.

## Security & Privacy Requirements

- **Local execution preferred**: All processing on user's machine
- **No credential storage in plain text**: Use system keychain or environment
- **Secure IPC only**: tmux socket communication via standard mechanisms
- **Audit logging**: Log all AI interaction requests (not content) for compliance
- **Opt-in remote**: Any remote capability must be explicitly enabled

## Development Workflow

- **TDD Cycle**: Write failing test → implement minimal code → verify pass → refactor
- **Commit discipline**: Commit after each task or logical group; never broken builds
- **Integration priority**: tmux integration tests pass before feature considered complete
- **Performance gates**: Benchmarks for 10k lines/minute throughput in CI
- **Documentation**: Update quickstart.md before marking feature complete

## Governance

This constitution supersedes all other project practices. All PRs must verify compliance with principles above. Complexity must be justified against Principle V (Simplicity through YAGNI).

**Amendment Procedure**:
1. Document proposed change with rationale
2. Review impact on all principles
3. Update templates if constitution adds/removes mandatory sections
4. Version bump per semantic rules below
5. Approval requires explicit acknowledgment

**Versioning Policy**:
- MAJOR: Backward incompatible governance/principle removals or redefinitions
- MINOR: New principle/section added or materially expanded guidance
- PATCH: Clarifications, wording, typo fixes, non-semantic refinements

**Compliance Review**: All feature specifications must include "Constitution Check" gate. Violations require explicit justification in Complexity Tracking section.

---

**Version**: 1.0.0 | **Ratified**: 2026-04-10 | **Last Amended**: 2026-04-10
