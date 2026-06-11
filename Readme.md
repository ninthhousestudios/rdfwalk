# rdfwalk

A terminal UI for browsing RDF data, either through a remote SPARQL endpoint or from a local RDF file.

![demo](demo/demo.gif)

## Installation

```
cargo install rdfwalk
```

For local file support, enable the `local` feature:

```
cargo install rdfwalk --features local
```

For RDF-star quoted triple support in local files, enable the `rdf-star` feature:

```
cargo install rdfwalk --features rdf-star
```

## Usage

**Remote mode** — query a SPARQL endpoint:
```
rdfwalk <endpoint> [start-uri]
```

**Local mode** — browse a local RDF file (requires `--features local`):
```
rdfwalk --local <file> [start-uri]
```

Supported file formats: NTriples (`.nt`), Turtle (`.ttl`), N3 (`.n3`), RDF/XML (`.rdf`, `.xml`).
RDF-star quoted triples are supported for NTriples and Turtle when built with `--features rdf-star`.

If no starting URI is given, the tool opens on the Types view. If a URI is given, it opens directly on that resource in the Browser view.

**Options:**

| Option | Default | Description |
|--------|---------|-------------|
| `--local <file>` | — | Browse a local RDF file instead of a remote endpoint (requires `--features local`) |
| `--limit <n>` | 1000 | Maximum rows returned per query |

## Views

### Browser

The main view. The top block shows the current resource: label (if an `rdfs:label` exists), raw URI, and `rdf:type` values right-aligned on the same line. A `★` appears when the resource is bookmarked.

Below that, the resource is broken into four sections, each a scrollable list:

- **Literal Properties**: `→ predicate = value  ^^type`
- **Outgoing Links**: `→ predicate → object`
- **Incoming Links**: `← predicate ← source`
- **As Predicate**: `subject ◆ object`

A line at the bottom of the browser shows the currently selected triple in N-Triple notation.

Navigation:
- `↑`/`↓` — move within the current section
- `Tab` / `Shift+Tab` — jump to the first item of the next/previous section
- `Enter` — follow the selected link
- `←` / `→` — go back or forward in history
- `b` — toggle bookmark on the current resource
- `c` — copy the current triple to the clipboard

### Types

Lists all distinct values of `rdf:type` found in the dataset, sorted alphabetically. Selecting a type and pressing `Enter` opens it in the Browser.

### SPARQL

A free-form SPARQL query editor. Type any SELECT query and press `Enter` to run it. Results are displayed in columns labelled with the query variable names. Pressing `Enter` on a result row navigates to the first URI found in that row.

`Tab` toggles focus between the input field and the results list.

Editor keybindings (input mode):

| Key | Action |
|-----|--------|
| `Enter` | Run query |
| `Ctrl+U` | Clear editor and results |
| `Ctrl+C` | Copy query to clipboard |
| `Ctrl+V` | Paste from clipboard at cursor |
| `←` / `→` | Move cursor |
| `Tab` | Switch to results |
| `Esc` | Back to Browser |

If a query fails, the full error message is displayed in the results panel.

### Search

A literal text search. Type a string and press `Enter` to find all triples whose object literal contains that string, case-insensitive. Results show the matching resource, property, and matched value. Pressing `Enter` on a result navigates to the resource.

`Tab` toggles between the input field and the results list.

### Bookmarks

Lists all bookmarked resources. Pressing `Enter` opens the resource in the Browser. Pressing `Delete` removes the bookmark. Bookmarks are stored in the OS-appropriate config directory (eg. `~/.config/rdfwalk/rdfwalk.toml` on Linux, `~/Library/Application Support/rdfwalk/rdfwalk.toml` on macOS) and persist across sessions.

## Resource display

URIs are displayed, in order of preference:
1. The value of `rdfs:label` if one exists (one arbitrary label is fetched per URI)
2. A prefixed form (`prefix:local`) if a known prefix matches
3. The full URI in angle brackets (`<http://...>`)

Built-in prefixes: `rdf`, `rdfs`, `owl`, `xsd`, `skos`, `dc`, `dct`, `foaf`, `schema`.

Literals are shown without quotes. The datatype or language tag is displayed in a separate column (e.g. `^^xsd:integer`, `@fr`). Long or multi-line values are collapsed to a single line and truncated to fit the available width.


## Keybindings

| Key | Action |
|-----|--------|
| `t` | Types view |
| `s` | SPARQL view |
| `f` | Search view |
| `m` | Bookmarks view |
| `b` | Toggle bookmark (Browser) |
| `c` | Copy current triple to clipboard (Browser) |
| `Esc` or `b` | Back to Browser (from SPARQL, Search, or Bookmarks) |
| `Delete` | Remove selected bookmark (Bookmarks view) |
| `q` | Quit |

## Known limitations

* Results are limited to `--limit` rows per query (default: 1000). Paging will be added in the future.
* Prefixes are currently limited to the mentioned built-ins.
* Full-text search is currently limited to case-insensitive partial match (uses plain `CONTAINS(LCASE(...))` clause)

## Dependencies

- [oxrdf](https://github.com/oxigraph/oxigraph) — RDF data structures
- [sparesults](https://github.com/oxigraph/oxigraph) — SPARQL result parsing
- [ratatui](https://github.com/ratatui-org/ratatui) — terminal UI
- [reqwest](https://github.com/seanmonstar/reqwest) — HTTP client
- [confy](https://github.com/rust-cli/confy) — config file management
- [arboard](https://github.com/1Password/arboard) — clipboard access

With `--features local`:
- [spareval](https://github.com/oxigraph/oxigraph/tree/main/lib/spareval) — in-memory SPARQL evaluation
- [spargebra](https://github.com/oxigraph/oxigraph/tree/main/lib/spargebra) — SPARQL query parser
- [oxrdfio](https://github.com/oxigraph/oxigraph/tree/main/lib/oxrdfio) — RDF file parsing

With `--features rdf-star`:
- Enables RDF-star support in `oxrdf`, `oxrdfio`, `spargebra`, `spareval`, and `sparesults`.
