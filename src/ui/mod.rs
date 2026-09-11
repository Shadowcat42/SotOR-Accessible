#[cfg(not(target_arch = "wasm32"))]
use crate::util::{get_extra_save_directories, read_dir_dirs, Directory};
use crate::{
    save::{Item, Save},
    util::{load_default_game_data, ContextExt as _, Game, Message},
};
#[cfg(target_arch = "wasm32")]
use ahash::HashMap;
use core::{util::fs::read_dir_filemap, GameData, GameDataMapped};
#[cfg(not(target_arch = "wasm32"))]
use eframe::APP_KEY;
use egui::{output::OutputEvent, Context, Ui, WidgetInfo, WidgetType};
use egui_toast::Toasts;
use log::error;
#[cfg(not(target_arch = "wasm32"))]
use std::path::PathBuf;
use std::sync::mpsc::{channel, Receiver, Sender};

use self::toasts::{init_toasts, make_toast};
#[cfg(target_arch = "wasm32")]
use self::widgets::UiExt as _;

mod editor;
#[cfg(not(target_arch = "wasm32"))]
mod settings;
#[cfg(not(target_arch = "wasm32"))]
mod side_panel;
mod styles;
mod toasts;
mod widgets;

type UiRef<'a> = &'a mut Ui;

#[cfg(not(target_arch = "wasm32"))]
struct SaveDirectories {
    cloud: bool,
    dirs: Vec<Directory>,
}

#[cfg(not(target_arch = "wasm32"))]
#[derive(Default, serde::Serialize, serde::Deserialize)]
struct PersistentState {
    steam_path: Option<String>,
    game_paths: [Option<String>; Game::COUNT],
}

#[derive(Clone)]
struct InventoryClipboard {
    items: Vec<Item>,
    game: Game,
    source: String,
}

impl InventoryClipboard {
    fn copy(items: &[Item], game: Game, source: String) -> Self {
        Self {
            items: items.to_vec(),
            game,
            source,
        }
    }

    fn paste_into(&self, game: Game, inventory: &mut Vec<Item>) -> Result<usize, String> {
        if self.game != game {
            return Err(format!(
                "Inventory copied from KotOR {} cannot be pasted into KotOR {game}",
                self.game
            ));
        }

        *inventory = self.items.clone();
        Ok(inventory.len())
    }
}

#[cfg(not(target_arch = "wasm32"))]
pub struct SotorApp {
    save: Option<Save>,
    persisted_save: Option<Save>,
    inventory_clipboard: Option<InventoryClipboard>,
    channel: (Sender<Message>, Receiver<Message>),
    default_game_data: [GameDataMapped; Game::COUNT],
    toasts: Toasts,
    save_path: Option<String>,
    settings_open: bool,
    save_list: [Vec<SaveDirectories>; Game::COUNT],
    latest_save: Option<Directory>,
    game_data: [Option<GameDataMapped>; Game::COUNT],
    prs: PersistentState,
    last_status: String,
}

#[cfg(target_arch = "wasm32")]
pub struct SotorApp {
    save: Option<Save>,
    persisted_save: Option<Save>,
    inventory_clipboard: Option<InventoryClipboard>,
    inventory_paste_confirmation_open: bool,
    channel: (Sender<Message>, Receiver<Message>),
    default_game_data: [GameDataMapped; Game::COUNT],
    toasts: Toasts,
    last_status: String,
}

impl SotorApp {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        styles::set_styles(&cc.egui_ctx);
        let (sender, receiver) = channel();
        cc.egui_ctx.set_channel(sender.clone());
        let default_game_data = load_default_game_data().map(GameData::into);
        let toasts = init_toasts();

        #[cfg(not(target_arch = "wasm32"))]
        {
            let prs = cc.storage.and_then(|s| eframe::get_value(s, APP_KEY));
            let mut app = Self {
                save: None,
                persisted_save: None,
                inventory_clipboard: None,
                save_path: None,
                channel: (sender, receiver),
                default_game_data,
                toasts,
                settings_open: prs.is_none(),
                save_list: [vec![], vec![]],
                latest_save: None,
                game_data: [None, None],
                prs: prs.unwrap_or_default(),
                last_status: String::new(),
            };

            app.reload_game_data(&cc.egui_ctx, true);
            app.reload_save_list(&cc.egui_ctx, true);

            app
        }
        #[cfg(target_arch = "wasm32")]
        {
            Self {
                save: None,
                persisted_save: None,
                inventory_clipboard: None,
                inventory_paste_confirmation_open: false,
                channel: (sender, receiver),
                default_game_data,
                toasts,
                last_status: String::new(),
            }
        }
    }

    fn close_save(&mut self) {
        self.save = None;
        self.persisted_save = None;
        #[cfg(not(target_arch = "wasm32"))]
        {
            self.save_path = None;
        }
    }

    fn has_unsaved_changes(&self) -> bool {
        matches!(
            (&self.save, &self.persisted_save),
            (Some(save), Some(persisted)) if save != persisted
        )
    }

    fn add_toast(&mut self, text: impl Into<String>, content: Option<String>, success: bool) {
        let text = text.into();
        self.last_status = if let Some(content) = &content {
            format!("{text} {content}")
        } else {
            text.clone()
        };

        #[cfg(not(target_arch = "wasm32"))]
        if !success || text.starts_with("Saved successfully") {
            let level = if success {
                rfd::MessageLevel::Info
            } else {
                rfd::MessageLevel::Error
            };
            rfd::MessageDialog::new()
                .set_title(if success { "SotOR" } else { "SotOR error" })
                .set_description(&self.last_status)
                .set_level(level)
                .set_buttons(rfd::MessageButtons::Ok)
                .show();
        }

        self.toasts.add(make_toast(text, content, success));
    }

    fn reload_save(&mut self, ctx: &Context) {
        // platform-specific
        let success = self._reload_save(ctx);
        if success {
            self.add_toast("Reloaded successfully", None, true);
        }
    }

    fn announce(&self, ctx: &Context, announcement: String) {
        ctx.output_mut(|output| {
            output
                .events
                .push(OutputEvent::ValueChanged(WidgetInfo::labeled(
                    WidgetType::Other,
                    announcement,
                )));
        });
    }

    fn copy_inventory(&mut self, ctx: &Context) {
        let Some(source) = self.loaded_save_label() else {
            return;
        };
        let clipboard = {
            let save = self.save.as_ref().expect("a loaded save has a label");
            InventoryClipboard::copy(&save.inventory, save.game, source)
        };
        let count = clipboard.items.len();
        let announcement = format!(
            "Copied {count} unequipped inventory {} from {}",
            if count == 1 { "item" } else { "items" },
            clipboard.source
        );
        self.inventory_clipboard = Some(clipboard);
        self.add_toast(announcement.clone(), None, true);
        self.announce(ctx, announcement);
    }

    fn apply_inventory_paste(&mut self, ctx: &Context) {
        let Some(clipboard) = self.inventory_clipboard.as_ref() else {
            return;
        };
        let source = clipboard.source.clone();
        let result = self
            .save
            .as_mut()
            .ok_or_else(|| "No save is loaded".to_owned())
            .and_then(|save| clipboard.paste_into(save.game, &mut save.inventory));

        match result {
            Ok(count) => {
                editor::reset_inventory_selection(ctx);
                let announcement = format!(
                    "Replaced the current unequipped inventory with {count} {} from {source}. Save to write the change to disk",
                    if count == 1 { "item" } else { "items" }
                );
                self.add_toast(announcement.clone(), None, true);
                self.announce(ctx, announcement);
            }
            Err(err) => {
                self.add_toast("Couldn't paste inventory:", Some(err.clone()), false);
                self.announce(ctx, format!("Couldn't paste inventory: {err}"));
            }
        }
    }
}

#[cfg(target_arch = "wasm32")]
impl SotorApp {
    fn set_meta_id(&self, ctx: &Context) {
        let Some(save) = &self.save else {
            return;
        };

        ctx.set_meta_id(&self.default_game_data[save.game.idx()], save);
    }

    fn loaded_save_label(&self) -> Option<String> {
        let save = self.save.as_ref()?;
        Some(format!("KotOR {} save {}", save.game, save.nfo.save_name))
    }

    fn request_paste_inventory(&mut self, ctx: &Context) {
        let Some(clipboard) = self.inventory_clipboard.as_ref() else {
            return;
        };
        let Some(save) = self.save.as_ref() else {
            return;
        };
        if clipboard.game != save.game {
            let err = format!(
                "Inventory copied from KotOR {} cannot be pasted into KotOR {}",
                clipboard.game, save.game
            );
            self.add_toast("Couldn't paste inventory:", Some(err.clone()), false);
            self.announce(ctx, format!("Couldn't paste inventory: {err}"));
            return;
        }

        self.inventory_paste_confirmation_open = true;
        ctx.request_repaint();
    }

    fn show_inventory_paste_confirmation(&mut self, ctx: &Context) {
        if !self.inventory_paste_confirmation_open {
            return;
        }

        let Some(clipboard) = self.inventory_clipboard.as_ref() else {
            self.inventory_paste_confirmation_open = false;
            return;
        };
        let source = clipboard.source.clone();
        let destination = self
            .loaded_save_label()
            .unwrap_or_else(|| "the loaded save".to_owned());
        let mut paste = false;
        let mut cancel = false;
        egui::Window::new("Confirm inventory replacement")
            .collapsible(false)
            .resizable(false)
            .show(ctx, |ui| {
                ui.label(format!(
                    "Replace the entire unequipped inventory in {destination} with the inventory copied from {source}? Equipped items and other save data will not change. The replacement remains unsaved until you choose Save."
                ));
                ui.horizontal(|ui| {
                    paste = ui.s_button_basic("Replace inventory").clicked();
                    cancel = ui.s_button_basic("Cancel").clicked();
                });
            });

        if paste {
            self.inventory_paste_confirmation_open = false;
            self.apply_inventory_paste(ctx);
        } else if cancel {
            self.inventory_paste_confirmation_open = false;
        }
    }

    fn load_save(&mut self, files: &HashMap<String, Vec<u8>>, ctx: &Context) {
        match Save::read_from_files(files, ctx) {
            Ok(save) => {
                self.persisted_save = Some(save.clone());
                self.save = Some(save);
            }
            Err(err) => {
                error!("{err}");
                self.add_toast("Couldn't load save:", Some(err), false);
            }
        }
        self.set_meta_id(ctx);
    }

    fn save(&mut self) {
        let Some(save) = &mut self.save else {
            return;
        };
        let bytes = Save::save_to_zip(save, &self.default_game_data[save.game.idx()]);
        crate::util::download_save(bytes);
    }

    fn _reload_save(&mut self, _ctx: &Context) -> bool {
        self.save = self.save.clone().map(Save::reload);
        true
    }
}

#[cfg(not(target_arch = "wasm32"))]
impl SotorApp {
    fn set_meta_id(&self, ctx: &Context) {
        let Some(save) = &self.save else {
            return;
        };
        let game_data = if let Some(data) = &self.game_data[save.game.idx()] {
            data
        } else {
            &self.default_game_data[save.game.idx()]
        };
        ctx.set_meta_id(game_data, save);
    }

    fn loaded_save_label(&self) -> Option<String> {
        let save = self.save.as_ref()?;
        let save_name = save.nfo.save_name.trim();
        let folder_label = self
            .save_path
            .as_deref()
            .map(|path| self.save_label(path))
            .unwrap_or_else(|| format!("KotOR {} save", save.game));
        if save_name.is_empty() || folder_label.contains(save_name) {
            Some(folder_label)
        } else {
            Some(format!("{folder_label}, named {save_name}"))
        }
    }

    fn request_paste_inventory(&mut self, ctx: &Context) {
        let Some(clipboard) = self.inventory_clipboard.as_ref() else {
            return;
        };
        let Some(save) = self.save.as_ref() else {
            return;
        };
        if clipboard.game != save.game {
            let err = format!(
                "Inventory copied from KotOR {} cannot be pasted into KotOR {}",
                clipboard.game, save.game
            );
            self.add_toast("Couldn't paste inventory:", Some(err.clone()), false);
            self.announce(ctx, format!("Couldn't paste inventory: {err}"));
            return;
        }

        let destination = self
            .loaded_save_label()
            .unwrap_or_else(|| "the loaded save".to_owned());
        let confirmed = rfd::MessageDialog::new()
            .set_title("Replace unequipped inventory?")
            .set_description(format!(
                "Replace the entire unequipped inventory in {destination} with the inventory copied from {}?\n\nEquipped items and other save data will not change. The replacement remains unsaved until you choose Save.",
                clipboard.source
            ))
            .set_level(rfd::MessageLevel::Warning)
            .set_buttons(rfd::MessageButtons::YesNo)
            .show();
        if confirmed == rfd::MessageDialogResult::Yes {
            self.apply_inventory_paste(ctx);
        }
    }

    fn save(&mut self) {
        let game = self.save.as_ref().unwrap().game.idx();
        let game_data = if let Some(data) = &self.game_data[game] {
            data
        } else {
            &self.default_game_data[game]
        };
        let res = Save::save_to_directory(
            self.save_path.as_ref().unwrap(),
            self.save.as_mut().unwrap(),
            game_data,
        );
        match res {
            Ok(_) => {
                self.persisted_save = self.save.clone();
                self.add_toast("Saved successfully", None, true);
            }
            Err(err) => {
                error!("{err}");
                self.add_toast("Couldn't save: ", Some(err), false);
            }
        }
    }

    fn load_save(&mut self, path: String, ctx: &Context, silent: bool) -> bool {
        let success = match Save::read_from_directory(&path, ctx) {
            Ok(save) => {
                self.persisted_save = Some(save.clone());
                self.save = Some(save);
                self.save_path = Some(path);
                true
            }
            Err(err) => {
                error!("{err}");
                if !silent {
                    self.add_toast("Couldn't load save:", Some(err), false);
                }
                false
            }
        };
        self.set_meta_id(ctx);
        success
    }

    fn save_label(&self, path: &str) -> String {
        for game in Game::LIST {
            for group in &self.save_list[game.idx()] {
                for save in &group.dirs {
                    if save.path == path {
                        let location = if group.cloud { "cloud" } else { "local" };
                        return format!("KotOR {game} {} {location}", save.name);
                    }
                }
            }
        }

        PathBuf::from(path)
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or(path)
            .to_owned()
    }

    fn request_load_save(&mut self, path: String, ctx: &Context) {
        let loading_different_save = self
            .save_path
            .as_deref()
            .is_some_and(|current| current != path.as_str());
        if loading_different_save && self.has_unsaved_changes() {
            let confirmed = rfd::MessageDialog::new()
                .set_title("Unsaved changes")
                .set_description(
                    "Loading another save will discard your unsaved changes. Continue?",
                )
                .set_level(rfd::MessageLevel::Warning)
                .set_buttons(rfd::MessageButtons::YesNo)
                .show();
            if confirmed != rfd::MessageDialogResult::Yes {
                return;
            }
        }

        let label = self.save_label(&path);
        if self.load_save(path, ctx, false) {
            let announcement = format!("Loaded save: {label}");
            self.add_toast(announcement.clone(), None, true);
            ctx.output_mut(|output| {
                output
                    .events
                    .push(OutputEvent::ValueChanged(WidgetInfo::labeled(
                        WidgetType::Other,
                        announcement,
                    )));
            });
        }
    }

    fn _reload_save(&mut self, ctx: &Context) -> bool {
        self.load_save(self.save_path.clone().unwrap(), ctx, false)
    }

    fn load_latest_save(&mut self, ctx: &Context) {
        if self.save.is_some() {
            return;
        }
        if let Some(save) = &self.latest_save {
            self.load_save(save.path.clone(), ctx, true);
        }
    }

    fn load_save_list(&mut self, game: Game) {
        let mut saves = vec![];
        let mut latest: Option<Directory> = None;

        let extra_directories = get_extra_save_directories(game);

        let game_directory = self.prs.game_paths[game.idx()]
            .as_ref()
            .map(|path| vec![PathBuf::from(path)])
            .unwrap_or_default();

        let all_paths: Vec<_> = [game_directory, extra_directories]
            .into_iter()
            .flatten()
            .flat_map(|dir| {
                let mut cloud_path = dir.clone();
                cloud_path.push("cloudsaves");

                let cloud_dirs = read_dir_dirs(&cloud_path)
                    .unwrap_or_default()
                    .into_iter()
                    .map(|d| (true, cloud_path.clone(), PathBuf::from(d.name)))
                    .collect();

                [
                    vec![(false, dir.clone(), PathBuf::from("saves"))],
                    cloud_dirs,
                ]
                .concat()
            })
            .collect();

        for (cloud, base_dir, saves_dir) in all_paths {
            let mut save_dirs = vec![];
            let Ok(map) = read_dir_filemap(&base_dir) else {
                continue;
            };
            let Some(real_saves_dir) = map.get(&saves_dir.to_string_lossy().to_string()) else {
                continue;
            };
            let final_dir = PathBuf::from_iter([&base_dir, &real_saves_dir.into()]);
            let Ok(dirs) = read_dir_dirs(&final_dir) else {
                continue;
            };

            for dir in dirs {
                if PathBuf::from_iter([&dir.path, "savenfo.res"]).exists() {
                    if latest.is_none() || latest.as_ref().unwrap().date < dir.date {
                        latest = Some(dir.clone());
                    }

                    save_dirs.push(dir);
                }
            }

            if !save_dirs.is_empty() {
                save_dirs.sort_unstable_by(|a, b| b.date.cmp(&a.date));
                saves.push(SaveDirectories {
                    cloud,
                    dirs: save_dirs,
                });
            }
        }
        saves.sort_unstable_by(|a, b| b.dirs[0].date.cmp(&a.dirs[0].date));
        self.save_list[game.idx()] = saves;
        if let Some(dir) = latest {
            if self.latest_save.is_none() || self.latest_save.as_ref().unwrap().date < dir.date {
                self.latest_save = Some(dir);
            }
        }
    }

    fn reload_save_list(&mut self, ctx: &Context, silent: bool) {
        let _ = Game::LIST.map(|game| self.load_save_list(game));
        if !silent {
            self.add_toast("Save list refreshed", None, true);
        }
        if self.save.is_none() {
            self.load_latest_save(ctx);
        }
    }

    fn load_game_data(&mut self, game: Game, ctx: &Context, silent: bool) -> bool {
        let idx = game.idx();
        let success = if let Some(game_path) = self.prs.game_paths[idx].as_ref() {
            let game_data = GameData::read(game, game_path, self.prs.steam_path.as_ref());
            match game_data {
                Ok(data) => {
                    self.game_data[idx] = Some(data.into());
                    true
                }
                Err(err) => {
                    self.game_data[idx] = None;
                    error!("KotOR {game}: {err:?}");
                    if !silent {
                        self.add_toast(
                            format!("Couldn't load game data for KotOR {game}:"),
                            Some(err),
                            false,
                        );
                    }
                    false
                }
            }
        } else {
            self.game_data[idx] = None;
            true
        };
        self.set_meta_id(ctx);
        success
    }

    fn reload_game_data(&mut self, ctx: &Context, silent: bool) {
        let res = Game::LIST.map(|game| self.load_game_data(game, ctx, silent));
        if !silent && res.iter().all(|s| *s) {
            self.add_toast("Reloaded successfully", None, true);
        }
    }

    fn set_steam_path(&mut self, path: Option<String>, ctx: &Context) {
        self.prs.steam_path = path;
        let Some(path) = self.prs.steam_path.clone() else {
            return;
        };
        // set game paths if they aren't selected yet
        for game in Game::LIST {
            let game_path = &mut self.prs.game_paths[game.idx()];
            if game_path.is_some() {
                continue;
            };
            let game_dir = game.steam_dir();
            let new_path = PathBuf::from_iter([path.as_str(), "common", game_dir]);
            if new_path.exists() {
                *game_path = Some(new_path.to_str().unwrap().to_owned());
            }
            self.load_save_list(game);
            self.load_game_data(game, ctx, false);
        }
        self.load_latest_save(ctx);
    }

    fn set_game_path(&mut self, game: Game, path: Option<String>, ctx: &Context) {
        self.prs.game_paths[game.idx()] = path;
        self.load_save_list(game);
        self.load_game_data(game, ctx, false);
        self.load_latest_save(ctx);
    }

    fn toggle_settings_open(&mut self) {
        self.settings_open = !self.settings_open;
    }
}

impl eframe::App for SotorApp {
    #[cfg(not(target_arch = "wasm32"))]
    fn save(&mut self, storage: &mut dyn eframe::Storage) {
        eframe::set_value(storage, eframe::APP_KEY, &self.prs);
    }

    fn update(&mut self, ctx: &Context, _frame: &mut eframe::Frame) {
        #[cfg(not(target_arch = "wasm32"))]
        {
            use egui::{Key, Modifiers};
            if ctx.input_mut(|i| i.consume_key(Modifiers::CTRL, Key::S)) && self.save.is_some() {
                self.channel.0.send(Message::Save).unwrap();
            }
            if ctx.input_mut(|i| i.consume_key(Modifiers::CTRL, Key::R)) && self.save.is_some() {
                self.channel.0.send(Message::ReloadSave).unwrap();
            }
            if ctx.input_mut(|i| i.consume_key(Modifiers::CTRL, Key::W)) && self.save.is_some() {
                self.channel.0.send(Message::CloseSave).unwrap();
            }
            if ctx.input_mut(|i| i.consume_key(Modifiers::NONE, Key::Escape)) && self.settings_open
            {
                self.channel.0.send(Message::ToggleSettingsOpen).unwrap();
            }
        }

        while let Ok(message) = self.channel.1.try_recv() {
            #[cfg(not(target_arch = "wasm32"))]
            match message {
                Message::Save => self.save(),
                Message::CloseSave => self.close_save(),
                Message::ReloadSave => self.reload_save(ctx),
                Message::CopyInventory => self.copy_inventory(ctx),
                Message::PasteInventory => self.request_paste_inventory(ctx),
                Message::LoadSaveFromDir(path) => {
                    self.request_load_save(path, ctx);
                }
                Message::ToggleSettingsOpen => self.toggle_settings_open(),
                Message::SetSteamPath(path) => self.set_steam_path(path, ctx),
                Message::SetGamePath(game, path) => self.set_game_path(game, path, ctx),
                Message::ReloadSaveList => self.reload_save_list(ctx, false),
                Message::ReloadGameData => self.reload_game_data(ctx, false),
            }
            #[cfg(target_arch = "wasm32")]
            match message {
                Message::Save => self.save(),
                Message::CloseSave => self.close_save(),
                Message::ReloadSave => self.reload_save(ctx),
                Message::CopyInventory => self.copy_inventory(ctx),
                Message::PasteInventory => self.request_paste_inventory(ctx),
                Message::LoadSaveFromFiles(files) => self.load_save(&files, ctx),
            }
        }

        #[cfg(not(target_arch = "wasm32"))]
        if self.settings_open {
            settings::Settings::new(
                || self.channel.0.send(Message::ToggleSettingsOpen).unwrap(),
                &self.prs.steam_path,
                &self.prs.game_paths,
            )
            .show(ctx);
        }

        #[cfg(not(target_arch = "wasm32"))]
        egui::SidePanel::new(egui::panel::Side::Left, "sp")
            .frame(egui::Frame::side_top_panel(&ctx.style()).inner_margin(egui::Margin::ZERO))
            .resizable(false)
            .default_width(160.)
            .min_width(160.)
            .max_width(ctx.screen_rect().width() - 760.)
            .show(ctx, |ui| {
                side_panel::SidePanel::new(&self.save_path, &self.game_data, &self.save_list)
                    .show(ui);
            });

        egui::CentralPanel::default().show(ctx, |ui| {
            if !self.last_status.is_empty() {
                ui.label(format!("Status: {}", self.last_status));
                ui.separator();
            }
            let inventory_clipboard_available = self.inventory_clipboard.is_some();
            if let Some(save) = &mut self.save {
                #[cfg(not(target_arch = "wasm32"))]
                let current_data = if let Some(data) = &self.game_data[save.game.idx()] {
                    data
                } else {
                    &self.default_game_data[save.game.idx()]
                };
                #[cfg(target_arch = "wasm32")]
                let current_data = &self.default_game_data[save.game.idx()];
                editor::Editor::new(save, current_data, inventory_clipboard_available).show(ui);
            } else {
                editor::editor_placeholder(ui);
            }
        });

        #[cfg(target_arch = "wasm32")]
        self.show_inventory_paste_confirmation(ctx);

        self.toasts.show(ctx);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use core::gff::{Field, Struct};

    fn item(tag: &str, stack_size: u16, raw_marker: &str) -> Item {
        Item {
            tag: tag.to_owned(),
            base_item: 1,
            name: Some(format!("Item {tag}")),
            description: Some(format!("Description {tag}")),
            stack_size,
            max_charges: 4,
            charges: 3,
            new: false,
            upgrades: 7,
            upgrade_slots: Some([1, 2, 3, 4, 5, 6]),
            raw: Struct::new(vec![(
                "CustomRawMarker",
                Field::String(raw_marker.to_owned()),
            )]),
        }
    }

    #[test]
    fn copying_inventory_is_independent_and_does_not_mutate_source() {
        let mut source = vec![item("source", 2, "source raw data")];
        let original = source.clone();

        let clipboard =
            InventoryClipboard::copy(&source, Game::One, "KotOR 1 source save".to_owned());

        assert_eq!(source, original);
        source[0].stack_size = 99;
        source[0]
            .raw
            .insert("CustomRawMarker", Field::String("changed".to_owned()));
        assert_eq!(clipboard.items, original);
    }

    #[test]
    fn paste_replaces_inventory_and_destination_does_not_alias_clipboard() {
        let source = vec![
            item("source-a", 2, "custom a"),
            item("source-b", 3, "custom b"),
        ];
        let clipboard =
            InventoryClipboard::copy(&source, Game::One, "KotOR 1 source save".to_owned());
        let mut destination = vec![item("destination", 8, "destination raw data")];

        assert_eq!(clipboard.paste_into(Game::One, &mut destination), Ok(2));
        assert_eq!(destination, source);
        assert!(!destination.iter().any(|item| item.tag == "destination"));
        assert_eq!(destination[0].raw, clipboard.items[0].raw);

        destination[0].stack_size = 42;
        destination[0]
            .raw
            .insert("CustomRawMarker", Field::String("changed".to_owned()));
        assert_eq!(clipboard.items, source);
    }

    #[test]
    fn empty_inventory_can_be_copied_and_pasted() {
        let clipboard = InventoryClipboard::copy(&[], Game::Two, "KotOR 2 empty save".to_owned());
        let mut destination = vec![item("destination", 1, "destination raw data")];

        assert_eq!(clipboard.paste_into(Game::Two, &mut destination), Ok(0));
        assert!(destination.is_empty());
    }

    #[test]
    fn cross_game_paste_is_rejected_without_changing_inventory_or_equipment() {
        let clipboard = InventoryClipboard::copy(
            &[item("source", 2, "source raw data")],
            Game::One,
            "KotOR 1 source save".to_owned(),
        );
        let mut destination = vec![item("destination", 5, "destination raw data")];
        let original_destination = destination.clone();
        let equipment = vec![Some(item("equipped", 1, "equipped raw data"))];
        let original_equipment = equipment.clone();

        let result = clipboard.paste_into(Game::Two, &mut destination);

        assert!(result.is_err());
        assert_eq!(destination, original_destination);
        assert_eq!(equipment, original_equipment);
    }

    #[test]
    fn same_game_paste_does_not_touch_equipped_items() {
        let clipboard = InventoryClipboard::copy(
            &[item("source", 2, "source raw data")],
            Game::One,
            "KotOR 1 source save".to_owned(),
        );
        let mut destination = vec![item("destination", 5, "destination raw data")];
        let equipment = vec![Some(item("equipped", 1, "equipped raw data"))];
        let original_equipment = equipment.clone();

        clipboard.paste_into(Game::One, &mut destination).unwrap();

        assert_eq!(destination, clipboard.items);
        assert_eq!(equipment, original_equipment);
    }
}
