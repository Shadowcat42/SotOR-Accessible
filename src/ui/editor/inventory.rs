use crate::{
    save::{Item, Save},
    ui::{
        styles::{set_checkbox_styles, set_spin_styles, set_striped_styles, GREEN, WHITE},
        widgets::{accessible_name, color_text, keyboard_list, visual_label, UiExt},
        UiRef,
    },
    util::{ContextExt, Message},
};
use core::{Data as _, DataDescr as _, GameDataMapped};
use egui::{Grid, Id, Label};

const CURRENT_FILTER_ID: &str = "ei_current_item_filter";
const CURRENT_CURSOR_ID: &str = "ei_current_item_cursor";
const AVAILABLE_FILTER_ID: &str = "ei_available_item_filter";
const AVAILABLE_CURSOR_ID: &str = "ei_available_item_cursor";

pub struct Editor<'a> {
    items: &'a mut Vec<Item>,
    data: &'a GameDataMapped,
    selected: Option<usize>,
    clipboard_available: bool,
}

impl<'a> Editor<'a> {
    pub fn new(save: &'a mut Save, data: &'a GameDataMapped, clipboard_available: bool) -> Self {
        Self {
            items: &mut save.inventory,
            data,
            selected: None,
            clipboard_available,
        }
    }

    pub fn show(&mut self, ui: UiRef) {
        self.clipboard_actions(ui);
        ui.separator();
        self.list(ui);
        self.item(ui);
        ui.separator();
        self.addition(ui);
    }

    fn clipboard_actions(&self, ui: UiRef) {
        ui.horizontal(|ui| {
            if ui.s_button_basic("Copy inventory").clicked() {
                ui.ctx().send_message(Message::CopyInventory);
            }
            if ui
                .s_button("Paste inventory", false, !self.clipboard_available)
                .clicked()
            {
                ui.ctx().send_message(Message::PasteInventory);
            }
        });
    }

    fn list(&mut self, ui: UiRef) {
        visual_label(ui, "Filter current inventory:");
        let mut filter: String = ui.ctx().get_data(CURRENT_FILTER_ID).unwrap_or_default();
        if ui
            .s_text_edit(&mut filter, 300., "Filter current inventory")
            .changed()
        {
            ui.ctx().set_data(CURRENT_FILTER_ID, filter.clone());
            ui.ctx().set_data(CURRENT_CURSOR_ID, 0usize);
        }

        let sorted = sorted_inventory(self.items, &filter);
        let options: Vec<_> = sorted.iter().map(|(_, name)| name.clone()).collect();
        let key = Id::new(CURRENT_CURSOR_ID);
        let mut cursor = ui.ctx().get_data(key).unwrap_or(0);
        keyboard_list(ui, key, "Current inventory items", &options, &mut cursor);
        ui.ctx().set_data(key, cursor);
        self.selected = sorted.get(cursor).map(|(idx, _)| *idx);

        if ui
            .s_button("Remove selected item", false, sorted.is_empty())
            .clicked()
        {
            let selected = self
                .selected
                .expect("enabled only when an item is selected");
            self.items.remove(selected);
            let remaining = sorted_inventory(self.items, &filter);
            cursor = cursor.min(remaining.len().saturating_sub(1));
            ui.ctx().set_data(key, cursor);
            self.selected = remaining.get(cursor).map(|(idx, _)| *idx);
        }
    }

    fn item(&mut self, ui: UiRef) {
        let Some(selected) = self.selected else {
            return;
        };
        let item = &mut self.items[selected];
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
            ui.ctx().set_data(AVAILABLE_CURSOR_ID, 0usize);
        }

        visual_label(ui, "Filter items available to add:");
        let mut filter: String = ui.ctx().get_data(AVAILABLE_FILTER_ID).unwrap_or_default();
        if ui
            .s_text_edit(&mut filter, 300., "Filter items available to add")
            .changed()
        {
            ui.ctx().set_data(AVAILABLE_FILTER_ID, filter.clone());
            ui.ctx().set_data(AVAILABLE_CURSOR_ID, 0usize);
        }

        let available: Vec<_> = self
            .data
            .inner
            .items
            .iter()
            .filter(|item| checked || item.name.is_some())
            .filter(|item| matches_filter(item.get_name(), &item.tag, &filter))
            .collect();
        let options: Vec<_> = available
            .iter()
            .map(|item| accessible_name(item.get_name(), item.get_description()))
            .collect();
        let key = Id::new(AVAILABLE_CURSOR_ID);
        let mut cursor = ui.ctx().get_data(key).unwrap_or(0);
        keyboard_list(ui, key, "Items available to add", &options, &mut cursor);
        ui.ctx().set_data(key, cursor);

        if ui
            .s_button("Add selected item", false, available.is_empty())
            .clicked()
        {
            let new_source_idx = self.items.len();
            self.items.push(available[cursor].into());
            let current_filter: String = ui.ctx().get_data(CURRENT_FILTER_ID).unwrap_or_default();
            let sorted = sorted_inventory(self.items, &current_filter);
            if let Some(current_cursor) = sorted.iter().position(|(idx, _)| *idx == new_source_idx)
            {
                ui.ctx().set_data(CURRENT_CURSOR_ID, current_cursor);
            }
        }
    }
}

pub(super) fn reset_selection(ctx: &egui::Context) {
    ctx.set_data(CURRENT_CURSOR_ID, 0usize);
}

fn sorted_inventory(items: &[Item], filter: &str) -> Vec<(usize, String)> {
    let mut sorted: Vec<_> = items
        .iter()
        .enumerate()
        .filter(|(_, item)| matches_filter(item.get_name(), &item.tag, filter))
        .map(|(idx, item)| (idx, item.get_name().to_owned()))
        .collect();
    sorted.sort_unstable_by(|a, b| a.1.cmp(&b.1));
    sorted
}

fn matches_filter(name: &str, tag: &str, filter: &str) -> bool {
    let filter = filter.trim();
    if filter.is_empty() {
        return true;
    }

    let filter = filter.to_lowercase();
    name.to_lowercase().contains(&filter) || tag.to_lowercase().contains(&filter)
}
