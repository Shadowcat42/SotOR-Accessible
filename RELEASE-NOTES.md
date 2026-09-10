# SotOR Accessible Edition 1.1

This release continues the keyboard and screen-reader polish of the redesigned
editor.

- Numeric spin buttons now keep keyboard focus when Left or Right is pressed,
  while Up and Down continue to change the value by exactly one.
- Combo-box-style lists support first-letter navigation. Repeating a letter
  cycles through matching entries and wraps around the list.
- Inventory and equipped items can now expose and edit their installed upgrade
  data in both KotOR games.
- KotOR I upgrades are represented by the installed `upgrade.2da` rows; KotOR
  II exposes its six upgrade slots. The bundled data does not include upgrade
  names, so this advanced editor deliberately uses exact row numbers and warns
  users to choose only upgrades compatible with the item.
- The original SotOR save-reading and save-writing engine remains unchanged.

This executable is unsigned. Continue testing on a copied save until the NVDA
pass and in-game load test are complete.
