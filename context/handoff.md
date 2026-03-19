# Definition Layer — Entity Schema Decisions

Session summary from explore mode. Use this when authoring the proposal, spec, and tasks for the entity schema change.

---

## Entities

Five entities in the definition layer: **Team**, **Role**, **Workflow**, **Task**, **Relay**, **Hook**.

Embedded types (not standalone entities): **RACI**, **Step** (WorkStep, ReviewStep), **Artifact**, **state_map**.

---

## Team

```yaml
id: platform-team          # kebab-case, unique in repo, immutable
name: Platform Team
description: string
includes:                  # all members of listed teams assigned a uniform role here
  backend-team: engineering-lead
import:                    # all members carry their roles from source team
  - qa-team
  - android-designers
members:
  - handle: "@alice"
    role: engineering-lead
```

**Key decisions:**
- Multiple teams per repo — not a singleton
- `handle` format: `@[a-z0-9_-]+` (Slack-style), unique within team
- `role`: exactly one per member per team — multiple roles dilute accountability
- `includes` → assigns a uniform role to all members of the referenced team
- `import` → carries roles from source team as-is
- Precedence on conflict: direct `members` > `import` > `includes`. Last `import` entry wins on conflict between imports.
- No circular `includes`/`import` chains
- Member can belong to multiple teams with different roles

---

## Role

```yaml
id: EngineeringLead        # CamelCase, unique in repo, immutable
name: Engineering Lead
purpose: string
traits:                    # optional
  - string
```

**Key decisions:**
- Roles are repo-level, not team-scoped — referenced by Team members and Workflow/Task RACI
- Not typed as human or agent — neutral by design
- `traits` optional — add incrementally

---

## Workflow

```yaml
id: Initiative             # CamelCase, unique within parent scope, immutable
name: Initiative
description: string
purpose: string            # delivered at check-in — the "why"
accountability:            # required on Workflow, always
  responsible: EngineeringLead
  accountable: ProductManager
  consulted: [Designer]
  informed: [SRELead]
steps:
  - Proposal:
      depends_on: [Shape]
  - review: LegalApproval
    approver: LegalCounsel
    on_reject: Shape
  - LegalSignoff:          # a Relay step
hooks:
  before: hook_id
  after: [hook_id, hook_id]
  on_state_change: hook_id
  on_reject: hook_id
  on_delegation: hook_id
states:
  - Active: "Work underway"
    semantic: active
  - UnderReview: "Awaiting gate decision"
    semantic: reviewing
  - Done: "Completed"
    semantic: complete
  - Blocked: "Cannot proceed"
    semantic: blocked
  - Failed: "Terminal failure"
    semantic: failed
guidance: string           # optional
```

**Key decisions:**
- `when_to_start` dropped — redundant with `depends_on` for inline/shared workflows; for top-level workflows, fold into `description`
- `needs` dropped — intra-workflow sequencing handled by `depends_on`; no strong use case for external gating at definition time
- `produces` dropped — derivable from child Task artifacts
- Workflow `accountability` always required — Task and Relay can inherit from it
- `guidance` optional

**Step types:**

*WorkStep* — references a Task, Relay, or inline Workflow:
```yaml
- Proposal:
    depends_on: [Shape]    # optional — absent means parallel-eligible
```

*ReviewStep* — inline gate:
```yaml
- review: LegalApproval   # slug is the name
  approver: LegalCounsel
  on_reject: Shape
```

**ReviewStep validations:**
- `name` (the slug value): unique within the workflow
- `on_reject` must reference an earlier step in the same steps list

**States semantics (Workflow):**
- `reviewing`: required if steps contain a ReviewStep (at least one)
- `complete`: required (at least one)
- `blocked`, `failed`: optional
- Multiple states can share the same semantic
- Unmapped states ignored by Pari

**Hooks lifecycle points (Workflow):** `before`, `after`, `on_state_change`, `on_review`, `on_reject`, `on_delegation`
- Hook value: single hook_id or list of hook_ids

---

## Task

```yaml
id: Proposal               # CamelCase, unique within parent scope, immutable
name: Proposal
description: string
purpose: string            # delivered at check-in
instructions:              # required — ordered, any participant can follow
  - string
criteria:                  # required — reviewer's checklist at the following gate
  - string
accountability:            # optional — inherits from parent Workflow if absent (full override when present)
  responsible: ProductManager
  accountable: ProductManager
  consulted: [EngineeringLead]
  informed: [SRELead]
artifact:
  name: proposal.md
  template: string         # optional
hooks:
  before: hook_id
  after: hook_id
  on_state_change: hook_id
states:
  - Draft: "Being written"
  - Done: "Completed"
    semantic: complete
  - Blocked: "Cannot proceed"
    semantic: blocked
  - Failed: "Terminal failure"
    semantic: failed
guidance: string           # optional
```

**Key decisions:**
- Task is never standalone — always within a parent Workflow
- `artifact` always required on Task
- Delegation concerns removed — that's Relay's job
- `accountability` optional — inherits parent Workflow RACI if absent, full override if present
- `instructions` always required

**States semantics (Task):**
- `complete`: required (at least one)
- `blocked`, `failed`: optional
- No `reviewing` semantic required — teams can define it freely, Pari doesn't act on it

**Hooks lifecycle points (Task):** `before`, `after`, `on_state_change`

---

## Relay

A distinct entity for handing off to a shared workflow and resuming when it completes. Not a Task.

```yaml
id: LegalSignoff           # CamelCase, unique within parent scope, immutable
name: Legal Signoff
description: string
purpose: string
accountability:            # optional — inherits from parent Workflow if absent
  responsible: LegalCounsel
  accountable: LegalCounsel
  consulted: [ProductManager]
  informed: [EngineeringLead]
delegates_to: SharedLegalReview   # references a workflow in shared/
briefing: string           # optional — pre-handoff work. If absent, delegates immediately on start
debriefing: string         # optional — post-completion work after shared workflow returns
state_map:
  In Progress:
    maps_to: Active
  Done:
    maps_to: Complete
    semantic: complete
  Failed:
    maps_to: Failed
    semantic: failed
hooks:
  before: hook_id
  after: hook_id
  on_state_change: hook_id
guidance: string           # optional
```

**Key decisions:**
- Relay is never standalone — always within a parent Workflow
- Relay is distinct from Task — different schema, different semantics
- `briefing` / `debriefing` naming chosen over preparation/resumption
- `briefing` absent → delegates immediately on start
- `states` field dropped — state vocabulary inferred from `state_map` values
- Semantic markers declared inline in `state_map`
- `state_map` keys must match shared workflow state names exactly
- Unmapped shared workflow states silently ignored
- At least one mapping with `semantic: complete` required
- Relay's `accountability` = who owns the handoff itself (distinct from shared workflow's accountability)
- Shared workflows referenced by Relay live in `shared/` — never appear directly in steps

**State_map validations:**
- Keys: must match state names in the referenced shared workflow's `states`
- Values (`maps_to`): the Relay's own state vocabulary
- `semantic` values from closed set: `complete`, `blocked`, `failed`
- At least one `semantic: complete` required

**Hooks lifecycle points (Relay):** `before`, `after`, `on_state_change`

---

## Hook

A first-class reusable entity. Bundled with scripts and templates in its own folder.

```yaml
id: UpdateJiraStatus       # CamelCase, unique in repo, immutable
name: Update Jira Status
description: string
instructions:              # optional — bundled files may be self-explanatory
  - string
inputs:                    # optional — caller-provided args at invocation
  - name: status
    description: string
    required: true
```

**Folder structure:**
```
shared/hooks/UpdateJiraStatus/
  hook.yaml
  update-jira.sh           # bundled script
  signal.md                # bundled template
```

**Invocation site** (on Workflow/Task/Relay `hooks` field):
```yaml
hooks:
  after:
    hook: UpdateJiraStatus
    with:
      status: Done
```

**Key decisions:**
- Hook is a first-class definition-layer entity — not a string instruction
- Reusable across Workflows, Tasks, and Relays
- Bundled files (scripts, templates) live alongside the hook definition
- `inputs` declares caller-provided args; Pari also injects runtime context automatically
- Required inputs must be present in `with` at invocation site; no unknown keys allowed
- Hook value on a Workflow/Task/Relay can be a single id or a list

---

## RACI (embedded)

| Field | Type | Required |
|---|---|---|
| `responsible` | role_id | yes — exactly one |
| `accountable` | role_id | yes — exactly one |
| `consulted` | role_id[] | yes — can be empty list |
| `informed` | role_id[] | yes — can be empty list |

All referenced role_ids must exist in repo.

---

## Entity Relationships

| Entity | Relationship | Arity |
|---|---|---|
| Role | repo-level | standalone |
| Hook | repo-level, in `shared/hooks/` | standalone |
| Workflow (top-level) | no parent | standalone |
| Workflow (inline) | belongs to parent Workflow | exactly one parent |
| Workflow (shared) | no parent, in `shared/` | standalone; referenced by 0..many Relays |
| Task | belongs to a Workflow | exactly one parent Workflow |
| Relay | belongs to a Workflow | exactly one parent Workflow |
| Relay → shared Workflow | delegates to | exactly one |
| Team → Role | references via members/includes/import | many-to-many |
| Workflow/Task/Relay → Hook | references | many-to-many |
| Workflow/Task/Relay → Role (RACI) | references | many-to-many |

**Key constraint:** Task and Relay are never standalone. Workflow `accountability` is always required (Tasks and Relays may inherit from it).

---

## Deferred / Parked

1. **Accountability model** — larger entity/model that hooks feed into. Cross-cutting signals and metrics. Needs its own design session.
2. **Runtime layer** — Run, WorkItem, Gate, Participant. Not started.
3. **Design principles doc** — make implicit principles explicit (`not prescriptive`, `no two sources of truth`, `plain language over slugs`, etc.).
4. **`shared/` folder convention** — discovery, validation, and versioning of shared workflows and hooks within the repo.
5. **Pari-injected hook context** — what runtime context Pari automatically passes to hooks (task id, current state, event type, etc.).
