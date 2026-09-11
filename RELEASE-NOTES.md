# SotOR Accessible Edition 1.2

This release adds faster navigation for large save-data collections and safe
KSE-style inventory transfer between saves.

- The Inventory page now provides separate filters for the current inventory
  and the full list of item templates available to add.
- The Globals page now filters both current globals and globals available to
  add.
- A loaded save's complete unequipped inventory can be copied and used to
  replace another save's unequipped inventory. The confirmation identifies the
  source and destination saves, and cross-game pasting is rejected.
- Inventory copying preserves complete item structures, including custom raw
  data, but never changes equipped character items or other save data.
- Pasted inventory remains an in-memory edit until Save is chosen, preserving
  the normal unsaved-change warning and `backup.zip` behavior.
- The experimental item-upgrade editor has been removed. Upgrade components can
  still be added to inventory and installed through the game itself.
- The original SotOR save-reading and save-writing engine remains unchanged.

This executable is unsigned. Continue testing on a copied save until the NVDA
pass and in-game load test are complete.
