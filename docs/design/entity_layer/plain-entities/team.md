# team-plain

**Entity Layer → `entity_layer/plain-entities/`**

---

## Purpose

`Team` is a top-level entity representing a group of people with assigned roles. Membership can be declared directly or composed from other teams via `import` and `include`.

---

## Definition

```rust
pub struct Team {
    pub entity_ref: EntityRef<Team>,
    pub name: String,
    pub description: Option<String>,
    pub members: Option<Vec<TeamMember>>,
    pub include: Option<HashMap<EntityRef<Team>, EntityRef<Role>>>,
    pub import: Option<Vec<EntityRef<Team>>>,
    pub extensions: Extensions,
}

pub struct TeamMember {
    pub handle: String,
    pub role: EntityRef<Role>,
}
```

---

## Fields

- `entity_ref` — carries the team's id and kind; top-level entity, defaults to `NoParent`
- `name` — human-readable display name
- `description` — optional short summary
- `members` — directly declared members; each has a handle and an assigned role
- `include` — maps a team ref to a role ref; brings in all members of the referenced team under the given role
- `import` — list of team refs whose members are brought in preserving their original roles
- `extensions` — open-ended metadata; only `x-` prefixed keys are permitted (see [13 · extensions](../value-types/extensions.md))

---

## Membership Composition

When the same handle appears via multiple sources, precedence is:

1. `members` (direct declaration wins)
2. `import` (later entry in the list wins when the same handle appears in multiple imports)
3. `include` (lowest precedence)

---

## `TeamMember`

- `handle` — unique identifier for the person within this team; must be unique across all members in the resolved team
- `role` — the role assigned to this member
