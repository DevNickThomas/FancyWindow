# Terminal and named-window stories

Manual acceptance checks for the current main preview. These are procedures,
not a claim that the checks have passed. Record the commit, Windows and terminal
versions, display scaling, result and evidence for each run.

## Set up

1. Build main and use a separate `--profile terminal-demo` in a writable portable
   folder. Leave unrelated profiles and running agent work alone.
2. Choose Layout > 2×2 grid. Keep optional zone headers off. Rename the window to
   `Coding agents` using File > Rename window.
3. Open four separate Windows Terminal windows, with one CLI per window: Codex 1,
   Codex 2, Claude 1 and Claude 2. Use disposable or public project content. Finish
   authentication and folder-trust prompts yourself.
4. Alt-drag the two Codex windows into the top panes and the two Claude windows
   into the bottom panes. Release the mouse while Alt is still held. Confirm the
   status bar says four zones and four hosted windows, with all four CLIs readable.

## Acceptance stories

| ID | Action | Expected result and evidence |
| --- | --- | --- |
| T01 | Use each terminal, then press Win+Alt+] through a full cycle and Win+Alt+[ back through it | Each terminal receives focus once in reading order; the background tint follows it. Check with a short **unsent** input marker, then clear it. Capture two distinct focus states. |
| T02 | Focus a terminal and press Ctrl+Alt+E | The editor identifies that terminal's pane without needing exposed background. Native terminal input is covered by the editor. |
| T03 | Press H, then V in the selected child | The occupied pane becomes nested rows/columns; all four terminal processes remain hosted. The selected terminal stays in the intended child. |
| T04 | Use Tab, Shift+Tab and arrow keys to choose another occupied pane; press H | The labelled target and tint agree; only that pane is split. |
| T05 | Press J and join an occupied pane with an empty neighbour; finish with Enter | The intended pane expands and focus returns to its terminal. Repeat finishing separately with Esc and Ctrl+Alt+E; changes remain applied. |
| T06 | Drag an occupied splitter in both directions, then resize/maximise/restore the host | Hosted terminals follow the geometry, render at their new size and remain usable. Record clipping, stale strips or blank content as failures. |
| T07 | Alt-drag one Codex and one Claude window out, then back in; repeat twice | Each release reduces the hosted count by one; each reattachment restores it. The same terminal session survives and its native frame remains usable. |
| T08 | Drag a hosted title bar without Alt | The window stays attached and returns to its pane; the hosted count is unchanged. |
| T09 | Change the accent blue → green → custom colour in Preferences, then change Dark Modern → Light Modern → Dark Modern | The focused pane's background and status bar update. The native terminal theme stays independent. No added focus outline crosses its rounded corners. Restore the intended screenshot theme. |
| T10 | Save the layout, change it, then rename the saved layout and reload it | Renaming preserves the saved geometry and shortcut; loading restores that geometry. The window's project name remains visible. |
| T11 | File > New window; name it for another project. Open a saved layout through Workspaces > Open in new window | Independent host windows and settings are created. Editing one layout does not alter the other. Saved layouts do not launch or duplicate agents. |
| T12 | Keep host canvases separate, attach a disposable terminal to the second host, and cycle across the edge of the first | Next/previous focus crosses between the intended named windows and returns. Check the input marker in the destination terminal. Do not infer this result from empty-window tests. |
| T13 | Rename a window, close and reopen it using File > Open window; select it again while running | Name, layout and appearance survive. Reopening a running profile raises the existing window without creating a duplicate. |
| T14 | Close a host containing the disposable terminals | All terminals remain open and usable. Reopening the saved host restores layout/settings; it does not restore application sessions automatically. |

Use the app's keyboard menus (Alt+F / Alt+L / Alt+W / Alt+H when the host has focus)
and the command palette for a second pass through window management. Check that
cancelling a dialog leaves the name, layout and process count unchanged.

## README capture

Return to four panes with two real Codex and two real Claude CLI sessions. Keep
the app large enough for readable prompts and titles. Capture a clean idle state,
then a short sequence of keyboard focus changes. Do not publish login prompts,
account details, private task content or an overlay from another app.

Show the exact tested shortcut beside the media. If alternate bindings were used,
say so and restore them afterwards. Use unaltered application captures; do not
composite fake CLI sessions into the image. Update the media provenance and QA
record before replacing the README headline image.

## Tool coverage

The installed Computer Use tool cannot hold Alt during a drag or send Windows-key
chords, and its guidance excludes terminal UI and security-permission automation.
Those steps need manual input. Host menu/overlay checks can be driven separately.
Do not mark manual gestures or actual terminal typing as passed from model tests,
process counts, screenshots of empty panes or a different shortcut alone.
