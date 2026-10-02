# Application captures

Captured on Windows 11 on 2 October 2026 from the running **2.1.0-preview.1**
application, built from main at `d004e01`. These are real application captures,
not the older HTML design mockups or the packaged 2.0.0 release.

The `main-qa` profile isolates the demonstration's settings. Notepad contains a
disposable test document; File Explorer shows this repository. Native application
title bars remain visible. Fancy Window's optional zone headers are off.

| Asset | State |
| --- | --- |
| [Main screenshot](../screenshot.png) | Notepad and File Explorer hosted in two stacked panes, Dark Modern |
| [Edit layout](edit-layout.png) | The occupied Notepad pane split into rows; the tinted overlay identifies its target |
| [Light theme](light-theme.png) | Light Modern with the green `#7EE787` accent and Notepad focused |
| [Preferences](preferences.png) | Theme choices and accent controls above both hosted apps |
| [Command palette](command-palette.png) | Searchable commands and layout presets in the current title-bar palette |
| [Layout animation](../layout-demo.gif) | Seven real captures showing contextual selection and nested splitting |

## Animation sequence

1. Start with Notepad and File Explorer in two side-by-side panes.
2. Open **Edit layout** with Notepad selected.
3. Press **H** to split that occupied pane into rows.
4. Press **V** to split its selected child into columns.
5. Select the Explorer pane.
6. Press **H** to split it into rows.
7. Press **Enter** to finish and return focus to Explorer.

Both apps remain hosted throughout. The GIF is a sequence of still captures with
pauses for readability, not a real-time screen recording or a demonstration of
Alt+drag. It loops after 17 seconds, at 932 × 619 pixels. GitHub supports inline GIF
images. The main README keeps it in an expandable section and provides a static
alternative.

Source screenshots were saved by Computer Use, converted to PNG for static images,
and encoded with Pillow using one 256-colour GIF palette. The UI was not retouched,
redrawn, or composited with mock windows. Only the capture timing and image encoding
changed. Small panes visibly clip some native controls; enlarging the host provides
more usable space.

## Refreshing these images

Build the intended main revision, launch a separate QA profile, and repeat the
states above using disposable content. Keep the window size fixed during an
animation sequence. Capture the complete host with its owned desktop windows and
dialogs, check each image for unrelated or private content, and update the version,
commit, date and dimensions here. Review every GIF frame and all README links
before committing the media.

See the [QA record](../qa/2026-10-02.md) for results, compatibility failures and
remaining manual checks. The screenshots demonstrate features; they are not a
claim that every desktop app is compatible.
