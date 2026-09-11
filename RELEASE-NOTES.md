# SotOR Accessible Edition 1.3

This release completes another accessibility-focused UI pass, improving
keyboard containment, list navigation, and screen-reader descriptions.

## Keyboard and focus behavior

- Editor text buttons and icon buttons now retain focus when any arrow key is
  pressed, matching the containment already provided by edit boxes, simulated
  combo boxes, tabs, and spin buttons.
- Simulated combo boxes now support timed prefix navigation. Typing characters
  in quick succession searches the complete prefix, so `i` followed by `o`
  selects an entry beginning with `io` instead of starting a new `o` search.
- The prefix resets after one second, when arrow navigation is used, or when
  focus leaves the list. Repeated single letters still cycle through entries
  beginning with that letter.

## Screen-reader descriptions

- Item, feat, power, and quest-stage descriptions are no longer shortened to
  300 characters in accessibility announcements.
- Current Inventory entries now announce descriptions just like items available
  to add. A custom description stored in the save takes priority, with the game
  template description used as a fallback.
- Quest-stage selectors now announce the stage number followed by the complete
  description once. The redundant truncated copy has been removed from both
  existing-quest and add-quest controls.

## Release delivery

- Official builds now attach one versioned executable directly to the release,
  such as `SOTOR-1.3.exe`, instead of wrapping the program and documentation in
  another ZIP file.
- GitHub's automatic source ZIP and tarball remain available.
- Automatic Windows builds now run only for version tags. A manual validation
  run remains available when a build is needed before tagging.

## Compatibility and safety

- KotOR I and KotOR II remain supported.
- The original SotOR save-reading, save-writing, and `backup.zip` behavior are
  unchanged.
- The executable is unsigned. Continue testing edits on copied saves and keep
  the automatic `backup.zip` until the edited save has loaded successfully in
  the corresponding game.
