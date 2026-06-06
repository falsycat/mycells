# Template Reference

`mcs` uses [Tera](https://keats.github.io/tera/) as its template engine.

## Using a custom template

Pass `--template <dir>` to `build` or `serve`. The directory must contain at least `page.html`, which is used for every page (including `index.md`). You can also include any number of partial/base templates and reference them with Tera's `{% extends %}` / `{% include %}` directives.

```
my-theme/
  base.html      ← optional base template
  page.html      ← required; rendered for every cell
```

## Template variables

### `page` — the current cell

| Variable | Type | Description |
|---|---|---|
| `page.id` | string | Cell ID (e.g. `2026053101`), or `"index"` for the index page |
| `page.slug` | string | URL slug (`atomic-notes`), empty string for the index page |
| `page.date` | string | Creation date `YYYY-MM-DD` from git history, empty if not tracked |
| `page.date_formatted` | string | Same as `page.date` |
| `page.last_modified` | string | Date of most recent git commit (`YYYY-MM-DD`), empty if not tracked |
| `page.title` | string | Plain-text H1 title |
| `page.url` | string | Absolute URL path (`/atomic-notes/` or `/`) |
| `page.body` | string (HTML) | Full rendered HTML including the H1 heading |
| `page.body_without_title` | string (HTML) | Rendered HTML with the first H1 removed |
| `page.backlinks` | array of **PageRef** | Pages that contain a `[[ID]]` link to this page |
| `page.history` | array of **GitCommit** | Git commit history for this page's source file |
| `page.tags` | array of strings | Tags extracted from `#tagname` markers in the body |

`page.body` and `page.body_without_title` contain raw HTML — use the `| safe` filter to render them:

```html
{{ page.body_without_title | safe }}
```

### `recent_pages` — recently created cells

An array of **PageRef** objects sorted by ID descending (newest first). The index page is not included.

### `site_graph` — full site graph

Contains all pages and links between them. Useful for embedding graph/search data.

| Field | Type | Description |
|---|---|---|
| `site_graph.nodes` | array of **SiteNode** | All pages in the site |
| `site_graph.edges` | array of **SiteEdge** | All `[[ID]]` links between pages |

**SiteNode** fields: `id`, `slug`, `title`, `url`, `text` (plain text for search), `tags`.

**SiteEdge** fields: `source` (page ID), `target` (page ID).

Use `{{ site_graph | json_encode | safe }}` to embed as JSON.

### `vars` — user-defined variables

A map of key/value strings passed via `--var KEY=VALUE` on the command line.

```
mcs build --cells ./cells --var site_title="My Notes" --var author="Alice"
```

Access in templates:

```
{{ vars.site_title | default(value="mycells") }}
{{ vars.author }}
```

---

## PageRef object

Used in `page.backlinks` and `recent_pages`.

| Field | Type | Description |
|---|---|---|
| `id` | string | Cell ID |
| `slug` | string | URL slug |
| `date` | string | Creation date `YYYY-MM-DD` from git history |
| `title` | string | Plain-text H1 title |
| `url` | string | Absolute URL path |
| `tags` | array of strings | Tags extracted from `#tagname` markers |

---

## GitCommit object

Used in `page.history`.

| Field | Type | Description |
|---|---|---|
| `hash` | string | Full commit hash |
| `author` | string | Author name |
| `author_date` | string | Author date in ISO format (`2026-05-31 12:00:00 +0900`) |
| `message` | string | Commit subject line |
| `diff` | string | Unified diff for this file (truncated at 8 KB) |

---

## Generated artifacts

In addition to per-page HTML, `mcs build` also writes:

| File | Description |
|---|---|
| `search.json` | Array of all pages with plain-text content for client-side search |
| `graph.json` | `{nodes, edges}` — all pages and `[[ID]]` links between them |

In serve mode, these are available at `/search.json` and `/graph.json`.

---

## Minimal template example

```html
<!DOCTYPE html>
<html>
<head>
  <meta charset="UTF-8">
  <title>{{ page.title }} — {{ vars.site_title | default(value="mycells") }}</title>
</head>
<body>
  <nav><a href="/">Home</a></nav>

  <h1>{{ page.title }}</h1>
  {{ page.body_without_title | safe }}

  {% if page.tags %}
  <p>
    {% for tag in page.tags %}
    <span>#{{ tag }}</span>
    {% endfor %}
  </p>
  {% endif %}

  {% if page.backlinks %}
  <section>
    <h2>Backlinks</h2>
    <ul>
      {% for link in page.backlinks %}
      <li><a href="{{ link.url }}">{{ link.title }}</a></li>
      {% endfor %}
    </ul>
  </section>
  {% endif %}

  <aside>
    <h3>Recent</h3>
    <ul>
      {% for p in recent_pages %}
      <li><a href="{{ p.url }}">{{ p.title }}</a></li>
      {% endfor %}
    </ul>
  </aside>
</body>
</html>
```

---

## CLI reference

### Build mode

```
mcs build [OPTIONS]

Options:
  --cells <DIR>       Path to the cells directory  [default: .]
  --output <DIR>      Output directory             [default: dist]
  --template <DIR>    Custom template directory
  --var KEY=VALUE     User variable (repeatable)
```

Output layout:

| Cell file | Output path |
|---|---|
| `index.md` | `<output>/index.html` |
| `2026053101-atomic-notes.md` | `<output>/atomic-notes/index.html` |

### Serve mode (live preview)

```
mcs serve [OPTIONS]

Options:
  --cells <DIR>       Path to the cells directory  [default: .]
  --port <PORT>       HTTP port                    [default: 3000]
  --template <DIR>    Custom template directory
  --var KEY=VALUE     User variable (repeatable)
```

The site is re-read from disk on every request, so edits to cells are reflected immediately without restarting.

URL routing:

| URL | Cell |
|---|---|
| `/` | `index.md` |
| `/<slug>/` | `ID-<slug>.md` |
| `/<slug>` | redirects to `/<slug>/` |
