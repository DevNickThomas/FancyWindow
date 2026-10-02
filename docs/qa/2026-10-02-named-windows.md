# Named-window QA — 2 October 2026

Tested the named-window changes on top of main `24b93d5`, version
**2.1.0-preview.1**, on Windows 11. Three disposable profiles were used.
Computer Use exercised the visible application menus and dialogs.

## Verified in the running application

| Scenario | Result |
| --- | --- |
| Rename the current window | File > Rename window changed the title and status to `Codex - Project Alpha` immediately |
| Create another named instance | File > New window opened `Claude - Project Beta` on the next monitor with an independent settings file |
| Keep layouts independent | Changing Beta to two stacked panes left Alpha's four-pane layout intact |
| Save and rename a layout | Saved `Two agents`, renamed it to `Review pair`; the window name remained visible before the layout name |
| Close and reopen | File > Open window restored Beta's name, two-pane layout and saved workspace |
| Reopen an already-running profile | The second launch exited; the process count stayed at two and the existing profile remained open |
| Open a saved layout in a new window | Created `Codex - Review` from `Review pair`; it opened with two stacked panes and left Beta intact |
| Keyboard menu access | Alt+F, Alt+L and Alt+W, menu letters/arrows and Enter completed the flows above without clicking |

These organisational checks used empty panes. They do not establish terminal
hosting, cross-instance focus cycling or drag-in/out compatibility. The earlier
[occupied-pane QA](2026-10-02.md) records the separate native-app tests and failures.

## Automated verification

- `cargo test --locked`: **218 tests passed**, zero failures.
- `cargo build --release --locked`: passed with the Windows GNU toolchain.
- New regression tests cover persistent names, independent settings with identical
  display names, copying saved geometry instead of current unsaved edits, retaining
  saved geometry/hotkeys while renaming, and command-palette discoverability.
- A named mutex prevents simultaneous writers to the same settings file, including
  the interval before the first instance creates its window.
- GitHub Actions returned no workflow runs at this check; these are local results.

## Remaining checks

- Two Codex CLI and two Claude Code terminals, hosted together, including focus
  cycling, occupied splits, release and reattachment.
- Exact default Windows-key shortcuts and Alt+drag require manual input with the
  current Computer Use tooling.
- Single-monitor cascading and mixed-DPI transitions need separate visual checks.
- Overlapping-host drop arbitration remains tracked in issue #3.

Named windows restore layout and preferences. They do not automatically start
terminals, resume agent sessions or restore application documents.

## Restart follow-up

After a machine restart, reopening the `terminal-demo` profile restored the
`Coding agents - FancyWindow` name, four-pane layout and Dark Modern theme. Both
CLI sign-ins persisted, and two Codex CLI and two Claude Code processes relaunched.
The host still had zero attached windows; terminal attachment and screenshots
were not completed by this restart check. The repeatable
[terminal acceptance stories](../../scripts/ui-test/terminal-workspaces.md)
describe the remaining test pass.
