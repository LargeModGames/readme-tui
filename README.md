<div align="center">

# readme-tui

My [GitHub profile](https://github.com/LargeModGames) README, running as a live terminal UI.

<img src="https://readme-tui.spotatui.workers.dev/tui.svg" width="830" alt="The live profile TUI as it renders right now" />

</div>

The image above is not a screenshot. It is a [ratatui](https://ratatui.rs) app that runs on a
Cloudflare Worker every time someone opens the page. It pulls in what I am listening to in
[spotatui](https://github.com/LargeModGames/spotatui), my Discord status, my GitHub stats and my
contribution graph, draws a frame, and returns it as an SVG. The tabs in the middle are shared:
when someone clicks one on my profile, everyone who loads the page after them sees that tab.

## How it works

A GitHub README cannot run JavaScript, so the whole thing is built from three things GitHub does
allow:

1. **Images from your own server.** GitHub proxies every README image through its image proxy,
   Camo. Camo honours `Cache-Control: no-cache`, so the Worker answers with that header and every
   page view fetches a fresh frame.
2. **SVG as a rendering target.** An SVG image can hold styled text and shapes, which is all a
   terminal frame is.
3. **Plain links.** A link can point at the Worker, change some stored state, and redirect back to
   the profile. That is how the tabs work without any script on the page.

### From data to pixels

```
 sources (cards, badges, APIs)          src/sources.rs
        │  parse each response
        ▼
 Profile struct                         src/ui.rs
        │  ratatui widgets render into a Buffer (96 × 46 cells)
        ▼
 ratatui Buffer
        │  walk every cell
        ▼
 SVG                                    src/svg.rs
```

**`svg.rs`** turns a ratatui `Buffer` into an SVG on a fixed character grid:

- Every glyph gets its own x position (`<text x="0 8.5 17 …">`), so columns line up in whatever
  monospace font the viewer has. Cells are 8.5 × 18 px, a width that is exact in floating point,
  so the output has no rounding noise.
- Block characters (`█ ▀ ▄ ▁…▇ ▏…▉`) become filled rectangles. Drawn as text, fonts leave gaps
  between them and bar charts look striped.
- Box-drawing characters (`│ ─ ╭ ╮ ╰ ╯`) become stroked paths through the cell centre, so borders
  join across rows instead of looking dashed.
- Backgrounds are merged into one rectangle per run of equal colour.

**`ui.rs`** lays out the frame with normal ratatui widgets: `Block`, `Tabs`, `Paragraph`, and a
few hand-drawn parts. The contribution graph fits two days into each cell: an upper half block
`▀` takes one day's colour as its foreground and the next day's as its background, which turns a
53-week calendar into 53 × 4 cells.

The header (intro, now playing, Discord, badges) and the GitHub strip (stats, languages,
contributions) are always visible. Only the middle pane switches between What I Do, spotatui,
steamie and Tech I Use.

### Data sources

The TUI shows the same information my README used to show as separate cards, so it reads the same
services:

| What | Source | Freshness |
| --- | --- | --- |
| Stats, top languages, repository cards | my [github-readme-stats](https://github.com/LargeModGames/github-readme-stats) deployment | cached 30 min |
| Followers, stars | shields.io `.json` badges | cached 30 min |
| Contribution graph | GitHub GraphQL `contributionCalendar` | cached 30 min |
| Profile views | komarev counter | every render, so views keep counting |
| Discord status | [Lanyard](https://github.com/Phineas/lanyard) | every render |
| Now playing | `spotatui.com/widget/<user>.json` | every render |

Reading the cards, instead of reimplementing their maths, keeps the numbers identical to what the
cards showed before.

The slow sources are cached in Workers KV. A response is only cached after its parser accepts it,
so an error page never replaces good data, and when a refresh fails the last good copy is served.
Any value whose source has no data at all is drawn as `?`; a failed source never breaks the image.

### Shared tabs

The links under the image point at `/tab/<name>`. The Worker writes that name to KV and redirects
back to the profile. The next render reads it, so the tab changes for every visitor, not just for
the one who clicked.

## Project layout

| Path | What it does |
| --- | --- |
| `src/svg.rs` | ratatui `Buffer` to SVG |
| `src/ui.rs` | the layout and every panel |
| `src/sources.rs` | parsers for each data source, and assembling the `Profile` |
| `src/worker.rs` | the Worker: routes, fetching, KV cache (wasm32 only) |
| `src/bin/preview.rs` | renders every tab from saved responses, no network |
| `tests/fixtures/` | real responses from each source, used by the tests and the preview |

## Development

Everything except the Worker builds and tests natively:

```sh
cargo test                        # parsers against real fixtures, layout, SVG output
cargo run --bin preview           # writes preview/<tab>.svg from the fixtures
```

Open the files in `preview/` in a browser to see each tab.

To run the Worker itself you need the `wasm32-unknown-unknown` target and Wrangler:

```sh
rustup target add wasm32-unknown-unknown
npx wrangler dev                  # serves /tui.svg and /tab/<name> on localhost
npx wrangler deploy
npx wrangler secret put GITHUB_TOKEN
```

The token is only used for the contribution calendar. Without it, the graph shows `?` and
everything else still works.

## Limits

- A tab click shows up after the page reloads, which the redirect does for you.
- There is one shared tab for everyone, so visitors can switch it out from under each other.
- Fonts come from the viewer's system, so the exact look varies a little between platforms.
- GitHub's mobile app scales the image down to the screen width.

## License

[MIT](LICENSE), copyright (c) 2026 LargeModGames.
