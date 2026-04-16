# repo-codec

**Owning layer: `substrate`**

---

## Purpose

`RepoCodec` encodes and decodes between field-keyed JSON maps and the markdown+YAML frontmatter file format. It is driven entirely by `RepoSlot` values from the `FieldMapping` slices in `EntitySchema<RepoSlot>`. One implementation handles all entity types.

---

## Encoded Type

```rust
type Encoded = String;  // raw file content as a UTF-8 string
```

---

## Decode (read path)

Given a markdown file string and a `&[FieldMapping<RepoSlot>]`, `decode` produces a `HashMap<String, serde_json::Value>`:

| Slot | Decode behavior |
|---|---|
| `H1` | Extract text of first `# ` heading → `Value::String` |
| `FrontmatterKey(k)` | Parse YAML frontmatter, extract key `k` → JSON value |
| `FrontmatterFlattened` | Parse YAML frontmatter, extract all keys not mapped by other `FrontmatterKey` slots → JSON object (for extensions) |
| `DescriptionParagraph` | Extract first paragraph after H1 (before frontmatter or first section) → `Value::String` or `Value::Null` if absent |
| `Section(heading, _)` | Extract content of markdown section with the given heading → `Value::String` (Paragraph) or `Value::Array` (BulletList) |
| `FileContent` | Entire file content → `Value::String` |

---

## Encode (write path)

Given a `HashMap<&str, serde_json::Value>` and a `&[FieldMapping<RepoSlot>]`, `encode` produces a markdown string:

1. Build YAML frontmatter from `FrontmatterKey` and `FrontmatterFlattened` fields
2. Render `H1` heading from the name field
3. Render `DescriptionParagraph` if present
4. Render each `Section` in definition order

The output is a fully formed, self-contained markdown file. All fields in the asset are written — partial encode is not supported.

---

## Field Ordering

Fields are written in the order they appear in `FieldMapping` slices, not in JSON key order. Schema definition order controls file layout.

---

## `FrontmatterFlattened` Handling

During encode, all `x-*` keys in the extensions HashMap are written as top-level YAML frontmatter keys (after the explicitly named keys). During decode, any frontmatter keys not claimed by a `FrontmatterKey` slot are collected into the extensions field. This is how the `#[serde(flatten)]` extensions pattern maps to the markdown format.
