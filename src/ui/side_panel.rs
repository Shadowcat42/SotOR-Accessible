use crate::{
    ui::{
        styles::{BLUE, RED},
        widgets::{color_text, keyboard_list, Icon, UiExt},
        SaveDirectories, UiRef,
    },
    util::{open_file_manager, ContextExt, Game, Message},
};
use core::GameDataMapped;
use egui::{Frame, Layout, Margin};

pub struct SidePanel<'a> {
    current_save: &'a Option<String>,
    game_data: &'a [Option<GameDataMapped>; Game::COUNT],
    save_list: &'a [Vec<SaveDirectories>; Game::COUNT],
}

impl<'a> SidePanel<'a> {
    pub fn new(
        current_save: &'a Option<String>,
        game_data: &'a [Option<GameDataMapped>; Game::COUNT],
        save_list: &'a [Vec<SaveDirectories>; Game::COUNT],
    ) -> Self {
        Self {
            current_save,
            game_data,
            save_list,
        }
    }

    pub fn show(&self, ui: UiRef) {
        Self::padding_frame(ui, |ui| {
            ui.horizontal(|ui| self.header(ui));
            ui.separator();
        });

        self.lists(ui);
    }

    fn header_game_label(&self, ui: UiRef, game: Game) {
        let (color, tooltip) = if self.game_data[game.idx()].is_some() {
            (BLUE, "Game data loaded")
        } else {
            (
                RED,
                "Game data not loaded, select a valid game path in the settings",
            )
        };
        ui.vertical(|ui| {
            ui.s_offset(0., -2.);
            ui.label(color_text(
                &format!(
                    "K{game}: {}",
                    if self.game_data[game.idx()].is_some() {
                        "game data loaded"
                    } else {
                        "game data not loaded"
                    }
                ),
                color,
            ))
            .on_hover_text(tooltip);
        });
    }

    fn header(&self, ui: UiRef) {
        let _ = Game::LIST.map(|game| self.header_game_label(ui, game));

        ui.with_layout(Layout::right_to_left(emath::Align::Center), |ui| {
            let settings_btn = ui.s_icon_button(Icon::Gear, "Settings");
            if settings_btn.clicked() {
                ui.ctx().send_message(Message::ToggleSettingsOpen);
            }

            let reload_game_btn = ui.s_icon_button(Icon::Reload, "Reload game data");
            if reload_game_btn.clicked() {
                ui.ctx().send_message(Message::ReloadGameData);
            }

            let reload_saves_btn = ui.s_icon_button(Icon::Refresh, "Refresh save list");
            if reload_saves_btn.clicked() {
                ui.ctx().send_message(Message::ReloadSaveList);
            }
        });
    }

    fn lists(&self, ui: UiRef) {
        let mut saves = Vec::new();
        for game in Game::LIST {
            for group in &self.save_list[game.idx()] {
                for save in &group.dirs {
                    let location = if group.cloud { "cloud" } else { "local" };
                    saves.push((
                        format!("KotOR {game} {} {location}", save.name),
                        save.path.as_str(),
                    ));
                }
            }
        }

        let options: Vec<_> = saves.iter().map(|(name, _)| name.clone()).collect();
        let mut selected = ui
            .ctx()
            .get_data_raw("sp_save_cursor")
            .or_else(|| {
                self.current_save
                    .as_ref()
                    .and_then(|path| saves.iter().position(|(_, candidate)| candidate == path))
            })
            .unwrap_or(0);
        keyboard_list(ui, "sp_save_list", "Saves", &options, &mut selected);
        ui.ctx().set_data_raw("sp_save_cursor", selected);

        let Some((_, path)) = saves.get(selected) else {
            return;
        };
        if ui.s_button_basic("Load selected save").clicked() {
            ui.ctx()
                .send_message(Message::LoadSaveFromDir((*path).to_owned()));
        }
        if ui.s_button_basic("Open selected save folder").clicked() {
            open_file_manager(path);
        }
    }

    fn padding_frame(ui: UiRef, add_contents: impl FnOnce(UiRef)) {
        Frame::default()
            .inner_margin(Margin {
                top: 8.,
                right: 10.,
                bottom: 2.,
                left: 8.,
            })
            .show(ui, add_contents);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::util::Directory;
    use egui::accesskit::{Action, Role};

    #[test]
    fn save_panel_has_one_combo_and_no_focusable_unknown_nodes() {
        let ctx = egui::Context::default();
        ctx.options_mut(|options| options.screen_reader = true);
        ctx.enable_accesskit();
        let current_save = None;
        let game_data: [Option<GameDataMapped>; Game::COUNT] = [None, None];
        let save_list = [
            vec![SaveDirectories {
                cloud: false,
                dirs: vec![Directory {
                    path: "C:\\KOTOR\\saves\\000001 - Game1".to_owned(),
                    name: "Game1".to_owned(),
                    date: 1,
                }],
            }],
            vec![],
        ];

        let output = ctx.run(Default::default(), |ctx| {
            egui::SidePanel::left("test_save_panel")
                .resizable(false)
                .show(ctx, |ui| {
                    SidePanel::new(&current_save, &game_data, &save_list).lists(ui);
                });
        });
        let update = output.platform_output.accesskit_update.unwrap();
        let combos: Vec<_> = update
            .nodes
            .iter()
            .filter(|(_, node)| node.role() == Role::ComboBox)
            .collect();
        assert_eq!(combos.len(), 1);
        assert_eq!(combos[0].1.name(), Some("Saves"));
        assert_eq!(combos[0].1.value(), Some("KotOR 1 Game1 local 1 of 1"));
        assert_eq!(
            update
                .nodes
                .iter()
                .filter(|(_, node)| {
                    node.role() == Role::Unknown && node.supports_action(Action::Focus)
                })
                .count(),
            0
        );
    }
}
