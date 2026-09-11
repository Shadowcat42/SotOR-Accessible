use crate::{
    save::{Item, Save},
    ui::{
        styles::{set_checkbox_styles, set_spin_styles, set_striped_styles, GREEN, WHITE},
        widgets::{accessible_name, color_text, keyboard_list, UiExt},
        UiRef,
    },
    util::ContextExt,
};
use core::{Data as _, DataDescr as _, GameDataMapped};
use egui::{Button, Grid, Id, Label};


pub struct Editor<'a> {
    items: &'a mut Vec<Item>,
    data: &'a GameDataMapped,
    selected: usize,
}

impl<'a> Editor<'a> {
    pub fn new(save: &'a mut Save, data: &'a GameDataMapped) -> Self {
        Self {
            items: &mut save.inventory,
            data,
            selected: 0,
        }
    }

    pub fn show(&mut self, ui: UiRef) {
        self.list(ui);
        self.item(ui);
        ui.separator();
        self.addition(ui);
    }

    fn list(&mut self, ui: UiRef) {
        let mut sorted: Vec<_> = self
            .items
            .iter()
            .enumerate()
            .map(|(idx, item)| (idx, item.get_name().to_owned()))
            .collect();
        sorted.sort_unstable_by(|a, b| a.1.cmp(&b.1));
        let options: Vec<_> = sorted.iter().map(|(_, name)| name.clone()).collect();
        let key = Id::new("ei_current_item_cursor");
        let mut cursor = ui.ctx().get_data(key).unwrap_or(0);
        keyboard_list(ui, key, "Current inventory items", &options, &mut cursor);
        ui.ctx().set_data(key, cursor);
        self.selected = sorted.get(cursor).map(|(idx, _)| *idx).unwrap_or(0);

        if ui
            .add_enabled(!sorted.is_empty(), Button::new("Remove selected item"))
            .clicked()
        {
            self.items.remove(self.selected);
            cursor = cursor.min(self.items.len().saturating_sub(1));
            ui.ctx().set_data(key, cursor);
            let mut remaining: Vec<_> = self
                .items
                .iter()
                .enumerate()
                .map(|(idx, item)| (idx, item.get_name()))
                .collect();
            remaining.sort_unstable_by(|a, b| a.1.cmp(b.1));
            self.selected = remaining.get(cursor).map(|(idx, _)| *idx).unwrap_or(0);
        }
    }

    fn item(&mut self, ui: UiRef) {
        if self.items.is_empty() {
            return;
        }
        let item = &mut self.items[self.selected];
        let data_item = self.data.items.get(&item.tag);
        set_striped_styles(ui);
        Grid::new(ui.next_auto_id())
            .striped(true)
            .min_col_width(80.)
            .num_columns(2)
            .spacing([10., 6.])
            .show(ui, |ui| {
                if let Some(name) = &item.name {
                    ui.label("Name:");
                    ui.s_text(name);
                    ui.end_row();
                } else if let Some(name) = data_item.and_then(|i| i.name.as_ref()) {
                    ui.add(Label::new("Template name:").wrap(true));
                    ui.s_text(name);
                    ui.end_row();
                }

                ui.label("Tag:");
                ui.s_text(&item.tag);
                ui.end_row();

                ui.label("Stack size:");
                set_spin_styles(ui);
                ui.s_spin(&mut item.stack_size, 0..=99, false, "Stack size");
                ui.end_row();

                if item.max_charges > 0 {
                    ui.label(color_text("Charges:", GREEN));
                    ui.s_spin(&mut item.charges, 1..=item.max_charges, false, "Charges");
                    ui.end_row();
                }

                if let Some(description) = &item.description {
                    ui.label(color_text("Description:", GREEN));
                    ui.add(Label::new(color_text(description, WHITE)).wrap(true));
                    ui.end_row();
                } else if let Some(description) = data_item.and_then(|i| i.description.as_ref()) {
                    ui.add(Label::new(color_text("Template description:", GREEN)).wrap(true));
                    ui.add(Label::new(color_text(description, WHITE)).wrap(true));
                    ui.end_row();
                }
            });

    }

    fn addition(&mut self, ui: UiRef) {
        let show_all = ui.ctx().get_data("ei_add_all").unwrap_or(false);
        let mut checked = show_all;
        set_checkbox_styles(ui);
        ui.s_checkbox(&mut checked, "Show all item templates");
        if checked != show_all {
            ui.ctx().set_data("ei_add_all", checked);
        }

        let available: Vec<_> = self
            .data
            .inner
            .items
            .iter()
            .filter(|item| checked || item.name.is_some())
            .collect();
        let options: Vec<_> = available
            .iter()
            .map(|item| accessible_name(item.get_name(), item.get_description()))
            .collect();
        let key = Id::new("ei_available_item_cursor");
        let mut cursor = ui.ctx().get_data(key).unwrap_or(0);
        keyboard_list(ui, key, "Items available to add", &options, &mut cursor);
        ui.ctx().set_data(key, cursor);

        if ui
            .add_enabled(!available.is_empty(), Button::new("Add selected item"))
            .clicked()
        {
            let new_source_idx = self.items.len();
            self.items.push(available[cursor].into());
            let mut sorted: Vec<_> = self
                .items
                .iter()
                .enumerate()
                .map(|(idx, item)| (idx, item.get_name()))
                .collect();
            sorted.sort_unstable_by(|a, b| a.1.cmp(b.1));
            let current_cursor = sorted
                .iter()
                .position(|(idx, _)| *idx == new_source_idx)
                .unwrap_or(0);
            ui.ctx().set_data("ei_current_item_cursor", current_cursor);
        }
    }
}
