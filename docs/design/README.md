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
| Live accent status bar | — | Done; clickable segments in #11 |
| Zone headers (icon, title, number, ×) | #8 | Done |
| One-row title bar with caption buttons | #9 | Done |
| Command palette | #10 | Done |
| Activity rail, owner-drawn menus, Mica | #12 | Backlog |

The report predates two decisions: default hotkeys moved from Ctrl+Win to **Win+Alt**
(so its "Ctrl+Win+]" and "Ctrl+Win+P" read as Win+Alt+] and Win+Alt+Space), and the
app icon is the Pane mark rather than the four-colour grid shown in its mockups.
