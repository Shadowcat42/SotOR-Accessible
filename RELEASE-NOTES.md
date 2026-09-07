# SotOR Accessible Edition 1.1.8-a5

This build makes the keyboard-first controls consistent across the editor.

- Editor pages now expose real accessibility tab roles instead of pretending to
  be a combo box. Left/Right, Home/End, Ctrl+Tab, and Ctrl+Shift+Tab switch
  pages; Up/Down remain on the selected tab.
- Combo-box lists use concise names and values such as `Character`,
  `Aria 1 of 8`, with no embedded instructions or punctuation.
- All four arrow keys are contained by combo-box lists. Up/Down browse; Left
  and Right do nothing and cannot move focus elsewhere.
- The save browser is one Saves combo box with explicit Load and Open Folder
  buttons. Its collapsing groups, scroll container, and resizable panel were
  removed to eliminate stray focusable accessibility nodes.
- Inventory now mirrors the Feats workflow: separate Current inventory items
  and Items available to add combo boxes with explicit Remove and Add buttons.
- Numeric values now expose one spin button each; the duplicate slider node is
  gone.
- Quest and quest-stage popup selectors now use the same keyboard combo-box
  control as characters, feats, equipment, inventory, and other choices.
- The original SotOR save-reading and save-writing engine remains unchanged.

This executable is unsigned. Continue testing on a copied save until the NVDA
pass and in-game load test are complete.
