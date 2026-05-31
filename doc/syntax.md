# mycells Syntax

## File

Each file is called a **cell**. All cells live flat in a single directory — no subdirectories.

### Filename

```
YYYYMMDDXX-slug.md
```

There is one reserved filename: `index.md`. It is the top page of the site and does not follow the `YYYYMMDDXX-slug` naming convention.

- `YYYYMMDD` — date the cell was created
- `XX` — two-digit sequence number starting at `01`, scoped to that date
- `slug` — lowercase alphanumeric words separated by hyphens

Examples:

```
2026053101-atomic-notes.md
2026053102-linking-ideas.md
```

### ID

The ID of a cell is the `YYYYMMDDXX` portion of its filename (10 characters).

## Cell Format

Cells are plain Markdown with the following conventions.

### Title

The first line must be an H1 heading. This is the cell's display name.

```markdown
# The Title of This Cell
```

### Frontmatter

Not used. No YAML/TOML frontmatter blocks.

### Body

Free-form Markdown. Standard formatting (bold, italic, lists, code blocks, etc.) is allowed.

## Links

To link to another cell, use its ID wrapped in double brackets:

```
[[YYYYMMDDXX]]
```

Example:

```markdown
This idea builds on the principle described in [[2026053101]].
```

Links can appear anywhere in the body. A link does not carry display text — the target cell's H1 title is used as the label when rendered.
