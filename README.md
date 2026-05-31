# mycells

A static site generator for interconnected Markdown notes, inspired by the Zettelkasten method.

Each note is a **cell** — a single Markdown file with a unique ID. Cells link to each other via `[[ID]]` references, forming a browsable knowledge graph. `mcs` compiles a directory of cells into a static HTML site, or serves them locally with live preview.

## Features

- Flat file structure — all notes live in one directory, no nested folders
- Wiki-style `[[ID]]` links with automatic backlink tracking
- Parallel HTML rendering
- Live preview server (edits reflected immediately, no restart needed)
- Git history per cell exposed to templates
- Customizable [Tera](https://keats.github.io/tera/) HTML templates
- Generates `search.json` and `graph.json` for client-side search and graph visualization

## Installation

Requires [Rust](https://rustup.rs/).

```sh
git clone https://github.com/falsycat/mycells
cd mycells
cargo install --path .
```

## Quick Start

1. Create a directory for your cells:

```sh
mkdir my-notes
cd my-notes
```

2. Create `index.md` (the home page):

```markdown
# My Notes

Welcome to my notes.
```

3. Create your first cell. The filename format is `YYYYMMDDXX-slug.md`:

```markdown
# The Principle of Atomic Notes

One note, one idea.
```

Save it as `2026053101-atomic-notes.md`.

4. Preview in the browser:

```sh
mcs serve --cells .
# → open http://localhost:3000
```

5. Build the static site:

```sh
mcs build --cells . --output dist
```

## Cell Format

See [`doc/syntax.md`](doc/syntax.md) for the full specification. The essentials:

### Filename

```
YYYYMMDDXX-slug.md
```

- `YYYYMMDD` — creation date
- `XX` — two-digit sequence number per day, starting at `01`
- `slug` — lowercase words separated by hyphens

**Example:** `2026053101-atomic-notes.md`

The special file `index.md` is the site's home page and does not follow this naming convention.

### Content

- The first line must be an H1 heading — this becomes the cell's title.
- No YAML/TOML frontmatter.
- Standard Markdown everywhere else.

### Linking

Link to another cell using its 10-character ID:

```markdown
This idea builds on [[2026053101]].
```

The target cell's H1 title is used as the link label. Backlinks are tracked automatically and exposed in templates.

## CLI Reference

### `mcs build`

Build a static site into an output directory.

```
mcs build [OPTIONS]

Options:
  --cells <DIR>       Path to the cells directory  [default: .]
  --output, -o <DIR>  Output directory             [default: dist]
  --template <DIR>    Custom Tera template directory (must contain page.html)
  --var KEY=VALUE     Pass a variable to templates (repeatable)
```

In addition to per-page HTML, the following files are always written:

| File | Description |
|---|---|
| `search.json` | All pages with plain-text content for client-side search |
| `graph.json` | `{nodes, edges}` — all pages and `[[ID]]` links between them |

### `mcs serve`

Start a live preview HTTP server. The site is re-read on every request, so edits are reflected immediately.

```
mcs serve [OPTIONS]

Options:
  --cells <DIR>       Path to the cells directory  [default: .]
  --port, -p <PORT>   HTTP port                    [default: 3000]
  --template <DIR>    Custom Tera template directory
  --var KEY=VALUE     Pass a variable to templates (repeatable)
```

## Custom Templates

Pass `--template <dir>` to use your own HTML templates. The directory must contain `page.html`, which is rendered for every cell including the index.

See [`doc/template.md`](doc/template.md) for the full list of template variables and a minimal example.

## Example Cells

The [`test/example-cells/`](test/example-cells/) directory contains a sample vault you can use as a reference or to try out the tool:

```sh
mcs serve --cells test/example-cells
```

## License

MIT
