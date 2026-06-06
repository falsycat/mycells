# mycells Syntax

## File

Each file is called a **cell**. All cells live flat in a single directory — no subdirectories.

### Filename

```
ID-slug.md
```

There is one reserved filename: `index.md`. It is the top page of the site and does not follow the `ID-slug` naming convention.

- `ID` — any sequence of digits; the recommended convention is `YYYYMMDDNN` (date + two-digit daily sequence starting at `01`)
- `slug` — lowercase alphanumeric words separated by hyphens

Examples:

```
2026053101-atomic-notes.md
2026053102-linking-ideas.md
```

### ID

The ID of a cell is the leading numeric portion of its filename (everything before the first `-`).

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

Bare `https://…` URLs in the body are automatically turned into clickable links when rendered.

## Links

To link to another cell, wrap its ID in double brackets:

```
[[ID]]
```

To override the display text, add a pipe and the label:

```
[[ID|custom text]]
```

Without a pipe, the target cell's H1 title is used as the link label.

Examples:

```markdown
This idea builds on [[2026053101]].
See also [[2026053101|the atomic-notes principle]].
```

Links can appear anywhere in the body. All `[[ID]]` references are tracked; the linked cell sees this page in its backlinks.

## Tags

Write `#tagname` anywhere in the body to tag a cell:

```markdown
Note-taking is at the core of knowledge work. #zettelkasten #learning
```

Rules:

- A tag starts with `#` followed by a letter, then any mix of letters, digits, `_`, or `-`.
- Tags are normalized to **lowercase** and **deduplicated** per cell.
- The tag set is available in templates as `page.tags` (array of strings).
- `#` inside code spans, code blocks, or URLs is not treated as a tag.
