# Design

Self-contained HTML pages; open them in a browser (fonts load from Google Fonts,
and fall back to Segoe UI / Consolas offline).

| Page | What it is |
| --- | --- |
| [restyle-report.html](restyle-report.html) | The VS Code-style restyle study: five mockups (dark, light, and "today" for comparison), a Win32/GDI feasibility card for each idea, a summary table and the build order. |
| [logo-concepts.html](logo-concepts.html) | Four monotone icon concepts with one lit zone, at real icon sizes on dark and light taskbars. **C · Pane** was chosen; `scripts/make-icon.ps1` renders it into `resources/app.ico`. |

Live copies: [restyle report](https://claude.ai/artifact/Fhcrz535BDoiwR7fpcErjp),
[logo concepts](https://claude.ai/artifact/9s6pJveKPzP12WF85HFj9j) (private to the owner
unless shared).

## Status of the report's ideas

| Idea | Issue | State |
| --- | --- | --- |
| Active-zone glow and bevel, Segoe UI Variable | — | Done |
| Live accent status bar, clickable segments | #11 | Done |
| Zone headers (icon, title, number, ×) | #8 | Done |
| One-row title bar with caption buttons | #9 | Done |
| Command palette | #10 | Done |
| Activity rail, owner-drawn menus, Mica | #12 | Backlog |

The report predates two decisions: default hotkeys moved from Ctrl+Win to **Win+Alt**
(so its "Ctrl+Win+]" and "Ctrl+Win+P" read as Win+Alt+] and Win+Alt+Space), and the
app icon is the Pane mark rather than the four-colour grid shown in its mockups.

## Fidelity check against the mockups

`cargo run --release --example render -- <dir>` paints the report's hero scene off-screen
(Dark Modern and Light Modern) for side-by-side comparison, without opening a window.
`cargo run --release --example perf` times the per-move and per-repaint work.

Matches: 8 DIP gaps between zones and around the canvas (zones as rounded cards); one 36 DIP title row (icon, menus, 360x24 centre box with search glyph, 46 DIP
caption buttons); zone headers (icon, title, mono number chip, ×; active header lifted
with a 2 px accent line); dashed empty zones with key chips; accent status bar with
icon-led clickable segments and the live cycle hint; palette with thumbnails and key
chips; Modern palettes; 45% / 28% active glow; Segoe UI Variable with Cascadia Mono
for keys.

Known differences, kept on purpose or still open:
- The active zone's glow is a solid pre-blended tint, not the mockup's soft blur
  (GDI has no blur; the report puts a real blur with Direct2D, #12).
- Default accent stays #007ACC (mockup: #0078D4), indistinguishable in use.
- Activity rail, owner-drawn menus with thumbnails, Mica: "later, or never" in the
  report's build order (#12).
