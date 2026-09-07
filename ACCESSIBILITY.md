# SotOR Accessible Edition

This edition keeps SotOR 1.1.8's save-reading and save-writing engine and makes
the complete editor interface operable with a keyboard and Windows screen
readers through AccessKit.

Accessible edition 2 and later use the standard Windows message-box backend, so the
portable executable does not require a separate Common Controls v6 manifest.

## Safety first

Work on a copied save until you are satisfied with the results. SotOR creates
`backup.zip` in the edited save directory immediately before writing. A later
save can replace that backup, so it is not a substitute for your own untouched
copy.

The application displays a native confirmation dialog after a successful save
and a native error dialog when loading or saving fails. It never treats a failed
write as success.

## Opening a save

1. Start `SotOR-Accessible.exe`.
2. If Settings opens on the first run, press Escape to close it. Game paths are
   optional because the executable contains built-in KOTOR I Community Patch
   and KOTOR II TSLRCM data.
3. Tab to **Select** and press Enter.
4. Select the save directory itself, such as `000000 - QUICKSAVE`. Do not select
   `SAVEGAME.sav` alone.

This community build is not digitally signed. Windows SmartScreen may therefore
show an unknown-publisher warning the first time it is launched. The package
includes a SHA-256 checksum so the downloaded executable can be verified.

You can alternatively configure Steam or game paths in Settings. Doing so lets
SotOR discover saves automatically and load data from your installed mods.

## Keyboard operation

- Tab and Shift+Tab move between controls.
- Enter or Space activates buttons.
- Up and Down operate combo-box lists and spin buttons without moving focus.
- Left and Right switch editor tabs. Left and Right are contained when a combo
  box is focused, so they cannot move focus out of it.
- Ctrl+S saves and opens a confirmation dialog.
- Ctrl+R reloads the current save from disk.
- Ctrl+W closes the current save without closing the application.
- Ctrl+Tab and Ctrl+Shift+Tab move forward and backward through editor pages.
- Escape closes Settings.

General, Globals, Characters, Inventory, Quests, and Area are exposed as proper
tabs. Only the selected tab is in the Tab order. Left and Right switch pages
with wraparound; Home and End select the first and last page. Up and Down remain
on the selected tab.

The Saves control is one combo box. Browse it with Up and Down, then activate
**Load selected save**. **Open selected save folder** opens its directory.

### Character editor navigation

- The Character control is one list. Use Up and Down to choose a party member.
- Tab once to Character fields. Use Up and Down to choose the field to edit.
- Tab again to reach only that field's editor. Numeric fields expose one spin
  button; names expose an edit box; multi-choice fields expose another
  arrow-key list.
- Feats have separate Current feats and Feats available to add lists, followed
  by explicit Remove and Add buttons. Classes and Force powers use the same
  structure.
- Inventory has separate Current inventory items and Items available to add
  lists, followed by explicit Remove and Add buttons.
- Home and End move to the beginning and end of any arrow-key list.

The editor pages are General, Globals, Characters, Inventory, Quests, and Area.
Feat, Force power, item, and quest-stage choices expose their descriptions to
the accessibility tree rather than relying only on mouse hover text.

## First NVDA validation pass

Please test a copy of a save and note the exact speech for anything unexpected.
The most useful first pass is:

1. Confirm the window title is announced as **Saves of the Old Republic —
   Accessible Edition**.
2. Tab through Settings, close it with Escape, and open a copied save directory.
3. Confirm the six editor tabs, Save, Reload, and Close Save are named. On a
   selected tab, verify Left/Right and Ctrl+Tab/Ctrl+Shift+Tab switch pages.
4. In Characters, confirm the character list announces the current name and
   that Up/Down do not move focus away from the list.
5. Confirm HP, Force points, alignment, XP, attributes, and skills announce both
   a name and numeric value.
6. Focus Feats available to add and verify that Up and Down announce feat names
   and descriptions without moving focus.
7. Add a harmless feat on the copy, press Ctrl+S, and confirm the success dialog.
8. Reload the save and confirm the feat remains present.
9. Check Inventory, Quests, Globals, and Area for sensible names and values.

Reports such as “NVDA says blank after Character” or “Tab cannot reach Add feat”
are more actionable than “that screen is inaccessible.” Include the NVDA
version and the exact keys used.

## Data-integrity validation

The source includes a round-trip test that loads a real save directory, edits
representative fields through SotOR's data model, writes the save, reloads it,
and verifies the intended changes. It covers save metadata, party credits and
XP, boolean and numeric globals, character XP, attributes, skills and feats,
inventory stack data, area-door state when available, backup creation, and
preservation of major unedited structure.
