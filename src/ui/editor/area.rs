use crate::{
    save::{Door, Save},
    ui::{
        styles::{set_checkbox_styles, set_spin_styles},
        widgets::{keyboard_list, UiExt},
        UiRef,
    },
    util::ContextExt,
};

pub struct Editor<'a> {
    doors: &'a mut Option<Vec<Door>>,
}

impl<'a> Editor<'a> {
    pub fn new(save: &'a mut Save) -> Self {
        Self {
            doors: &mut save.doors,
        }
    }

    pub fn show(&mut self, ui: UiRef) {
        if self.doors.is_none() {
            ui.horizontal_centered(|ui| {
                ui.s_offset(ui.max_rect().width() / 2. - 150., 0.);
                ui.label("Area editing isn't available for this save");
            });
            return;
        };

        let doors = self.doors.as_mut().unwrap();
        if doors.is_empty() {
            ui.label("No doors are present in this area.");
            return;
        }

        let options: Vec<_> = doors
            .iter()
            .map(|door| {
                format!(
                    "{} — {}, open state {}",
                    door.tag,
                    if door.locked { "locked" } else { "unlocked" },
                    door.open_state
                )
            })
            .collect();
        let mut selected = ui.ctx().get_data("ea_selected_door").unwrap_or(0);
        selected = selected.min(doors.len() - 1);
        ui.columns(3, |columns| {
            keyboard_list(
                &mut columns[0],
                "ea_door_list",
                "Door",
                &options,
                &mut selected,
            );
            columns[0].ctx().set_data("ea_selected_door", selected);

            let door = &mut doors[selected];
            set_checkbox_styles(&mut columns[1]);
            columns[1].s_checkbox(&mut door.locked, &format!("{} locked", door.tag));
            set_spin_styles(&mut columns[2]);
            columns[2].s_spin(
                &mut door.open_state,
                0..=2,
                false,
                &format!("{} open state", door.tag),
            );
        });
    }
}
