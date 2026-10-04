# site/wl

Static, dependency-free viewer for Weisfeiler-Lehman (WL) traces: for a pair of functions it shows per-level label matches and the similarity score, so you can see why a pair does or doesn't cluster.

## Run

```sh
scripts/wl_site.sh --serve     # regenerate data, serve on http://localhost:8765
scripts/wl_site.sh             # regenerate only
cd site/wl && python3 -m http.server 8765   # serve existing data
```

Any static server works; `index.html` loads `data/index.json` via fetch, so `file://` won't.

## Layout

| Path | Role |
| --- | --- |
| `index.html`, `app.js`, `style.css` | UI. |
| `lib/trace.js` | Pure functions over a trace JSON. |
| `scorers/index.js` | Scorer registry (similarity variants shown in the UI). |
| `data/` | Generated: `index.json` (pair list) + one `<id>.json` per pair, from the `wl_trace` example in `rstyle-core`. |
| `tests/` | `cd site/wl && npm test` (`node --test`, no install needed). |

## Add a pair

Add a row to the heredoc in `scripts/wl_site.sh`:

```
id|project|fn a|fn b|expect (high|low)|title|note
```

`project` is a crate under `rust_refactoring_examples/`; functions are `module::name`. Rerun the script.
