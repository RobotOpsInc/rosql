## Canopy — Knowledge Graph

The org-wide knowledge graph is at `RobotOpsInc/canopy` (`vault/`). This repo is
documented at `vault/projects/rosql/`.

The vault slug is the repo name lowercased with underscores replaced by hyphens
(e.g. `robot_agent` → `robot-agent`, `web_app` → `web-app`).

### Read Canopy before…

* Making an architectural decision — check `vault/decisions/` and
  `vault/projects/rosql/decisions/`
* Touching a shared interface (protos, RMW API, config schema) — read the relevant
  project pages to understand what downstream repos depend on
* Investigating a regression that might be a known incident — check `vault/incidents/`

### Leave a raw note when…

When something notable happens — a decision is made, a public interface changes, a
non-obvious bug is fixed, a constraint is discovered — create a file at:

`vault/_raw/rosql-YYYY-MM-DD-<short-slug>.md`

in the `RobotOpsInc/canopy` repo and open a PR against `main`. Keep it factual: what
changed, why, any cross-repo implications. Especially for anything architectural or a
new feature, describe in detail. You can use illustrations, links, text — the ingestion
pipeline is very flexible. The canopy ingest workflow handles everything from there.
Do not write vault pages directly.
