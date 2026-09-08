# SotOR Accessible Edition development guide

## Project identity

- This repository begins from the exported SotOR Accessible Edition
  `1.1.8-a5` source.
- The original upstream SotOR reference is
  `https://github.com/StarfishXeno/sotor` at revision
  `e8dd39e18ae872bf24dadb28526224db7a6921b7`. Use it to confirm original
  behavior, but do not edit or copy over accessibility work wholesale.
- Commit `c60d8bf` is the byte-for-byte imported A5 baseline.
- The maintained edition is named **SotOR Accessible Edition 1.0**. Cargo uses
  the SemVer-compatible package version `1.0.0`.
- Preserve support for both KotOR I and KotOR II unless a task explicitly
  changes that scope.

## Architecture

- `core/` parses game data and BioWare formats: BIF, ERF, GFF, KEY, TLK, and
  2DA.
- `src/save/` maps saves into editable models, applies model changes back to
  the original structures, creates `backup.zip`, and writes the edited save.
- `src/ui/` contains the egui application and its accessible controls.
- `src/ui/editor/` implements General, Globals, Characters, Inventory, Quests,
  and Area.
- `src/ui/widgets.rs` contains the shared keyboard-list, tab-list, spin-control,
  and accessibility behavior.
- `vendor/egui/` is egui 0.25 with a narrow patch adding a genuine AccessKit
  tab role. Avoid broad vendor changes.
- `build.rs` embeds game data from `SOTOR_ASSETS_ZIP`, or generates it from both
  installed games beneath `STEAM_APPS`.

## Safety and compatibility boundaries

- Treat the existing save reader and writer as proven code. Accessibility work
  should normally stay in the UI layer.
- Never test writes against a user's only save. Use a copied save directory.
- A successful save must create `backup.zip`; failures must not be presented as
  success.
- Keep the Windows executable free of `TaskDialogIndirect`. Native status
  dialogs use the standard message-dialog path.

## Accessibility invariants

- Every feature must be keyboard-operable and expose a meaningful name, role,
  value, state, and action through AccessKit.
- Arrow-key lists contribute one Tab stop. Up/Down changes selection without
  moving focus; Left/Right must not escape combo-box-style lists.
- Editor pages use real tab roles. Left/Right, Home/End, Ctrl+Tab, and
  Ctrl+Shift+Tab change pages; Up/Down does not leave the selected tab.
- Numeric editors expose one spin button, not a duplicate slider.
- Avoid unnamed focusable or `Unknown` accessibility nodes.
- Descriptions for feats, powers, items, and quest stages must be available to
  accessibility APIs, not only through mouse hover text.
- Apply interaction fixes consistently to every control using the same pattern.

## Validation

The project requires Rust 1.74 or newer. A native build also requires either
`SOTOR_ASSETS_ZIP` or a `STEAM_APPS` directory containing both supported games.
Local Windows compiler installation is optional: the canonical release path is
the `.github/workflows/windows-release.yml` GitHub Actions workflow on a hosted
Windows runner.

The workflow downloads `sotor-assets.zip` from the repository's private
`build-assets-v1` release and verifies SHA-256
`3a1c42cfe994977246bd340471e89304af06efb2ef9d45675e8e35dfc7023e37` before
building. Do not commit the extracted asset bundle or replace it silently. If
game data must intentionally change, create a new private build-assets release
tag and update both the tag and checksum in the workflow in a dedicated commit.

Run the automated baseline with:

```powershell
cargo fmt --all --check
cargo test --workspace --locked
```

To exercise the supplied-save round-trip test, set `SOTOR_TEST_SAVE` to a
copied save directory before running the tests. Without it, that test reports a
skip and returns successfully.

For Windows releases, also verify:

1. NVDA announcements and keyboard focus across every editor page.
2. Save, reload, and `backup.zip` behavior on a copied save.
3. The edited save loads correctly in the corresponding game.
4. The executable does not import `TaskDialogIndirect`.

## Git workflow

- Keep `main` buildable and use small, focused commits.
- Prefer creating a complete batch of local commits and pushing them together,
  so GitHub Actions validates only the final commit in the push. If commits must
  be pushed one at a time, include `[skip actions]` in every intermediate commit
  message and omit it from the final commit so the final state is validated.
- Documentation or workflow-maintenance commits that do not change the program
  may use `[skip actions]`; validate the workflow syntax locally and let the next
  source change exercise the hosted build.

- Separate mechanical/versioning changes from behavior changes.
- Do not rewrite the imported A5 baseline commit.
- Treat successful GitHub Actions runs as the automated build record. Every
  push to `main` produces a temporary Windows artifact; a matching version tag
  (for example, Cargo `1.0.0` with Git tag `v1.0.0`) publishes a release.
- Keep the repository private unless the owner explicitly decides otherwise;
  its private build-assets release contains data derived from installed games.
- Before committing, inspect `git diff`, run `git diff --check`, and perform the
  relevant automated and manual validation above.
