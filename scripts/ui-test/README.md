# UI tests

End-to-end user stories (S01-S20) run against a real `fancy-window.exe`. The script
starts its own instance with `--profile uitest` (its own `settings-uitest.json`) and
four throwaway `FW Test N` windows from `..\test-windows.ps1`, then drives them with
the mouse, keyboard and hotkeys on the **primary monitor**.

```
cargo build --release
.\scripts\ui-test\ui-test.ps1 -Exe .\target\release\fancy-window.exe -Out $env:TEMP\fw-ui
# Optional: -Only S02,S05   -HotkeysFrom <a settings.json whose hotkeys to use>
```

Results print as PASS/FAIL per check; `-Out` gets `log.txt`, `results.json` and a
screenshot per story. A run takes about two minutes: keep hands off the mouse and
keyboard meanwhile.

`UI.cs` is not part of the app. PowerShell can't call Win32 directly, so the script
compiles this small helper (`Add-Type`) for SendInput, window lookup and screenshots.

## Safety

- Every click, drag and key press first checks that the window under the cursor (or
  in front) belongs to the test instance or a test window, and refuses otherwise.
  Other windows on the desktop are never touched.
- Hotkeys reach whichever Fancy Window owns the foreground window, so the script
  only presses one when that is the test instance.
- Every instance hit-tests every Alt+drop against its own canvas, so the script
  refuses to start if another Fancy Window's client area overlaps the primary
  monitor. Move it to another monitor (or minimise it) first.
- Smart App Control can block freshly built, unsigned exes; see the issue tracker.

## Known gaps

- S12 (merge) and S18 (workspace menu) are screenshot-only checks.
- The cycle story never presses "next" on the last window while another instance
  runs, because that hands focus to the other instance.
