use crate::{
    save::{Global, GlobalValue, Save},
    ui::{
        styles::{set_checkbox_styles, set_spin_styles},
        widgets::{keyboard_list, visual_label, UiExt},
        UiRef,
    },
    util::ContextExt,
};

const FILTER_ID: &str = "eg_global_filter";
const CURSOR_ID: &str = "eg_selected_global";

pub struct Editor<'a> {
    globals: &'a mut Vec<Global>,
}

impl<'a> Editor<'a> {
    pub fn new(save: &'a mut Save) -> Self {
        Self {
            globals: &mut save.globals,
        }
    }

    pub fn show(&mut self, ui: UiRef) {
        if self.globals.is_empty() {
            ui.label("No globals are present in this save.");
            return;
        }

        visual_label(ui, "Filter global variables:");
        let mut filter: String = ui.ctx().get_data(FILTER_ID).unwrap_or_default();
        if ui
            .s_text_edit(&mut filter, 300., "Filter global variables")
            .changed()
        {
            ui.ctx().set_data(FILTER_ID, filter.clone());
            ui.ctx().set_data(CURSOR_ID, 0usize);
        }

        let filter = filter.trim().to_lowercase();
        let filtered: Vec<_> = self
            .globals
            .iter()
            .enumerate()
            .filter(|(_, global)| {
                filter.is_empty() || global.name.to_lowercase().contains(&filter)
            })
            .map(|(idx, global)| {
                let label = match &global.value {
                    GlobalValue::Number(value) => format!("{} — {value}", global.name),
                    GlobalValue::Boolean(value) => format!(
                        "{} — {}",
                        global.name,
                        if *value { "true" } else { "false" }
                    ),
                };
                (idx, label)
            })
            .collect();
        let options: Vec<_> = filtered.iter().map(|(_, label)| label.clone()).collect();
        let mut cursor = ui.ctx().get_data(CURSOR_ID).unwrap_or(0);
        ui.columns(2, |columns| {
            keyboard_list(
                &mut columns[0],
                "eg_global_list",
                "Global variable",
                &options,
                &mut cursor,
            );
            columns[0].ctx().set_data(CURSOR_ID, cursor);

            let Some(selected) = filtered.get(cursor).map(|(idx, _)| *idx) else {
                columns[1].label("No global variables match the filter.");
                return;
            };

            let global = &mut self.globals[selected];
            match &mut global.value {
                GlobalValue::Number(value) => {
                    set_spin_styles(&mut columns[1]);
                    columns[1].s_spin(value, u8::MIN..=u8::MAX, false, &global.name);
                }
                GlobalValue::Boolean(value) => {
                    set_checkbox_styles(&mut columns[1]);
                    columns[1].s_checkbox(value, &global.name);
                }
            }
        });
    }
}
