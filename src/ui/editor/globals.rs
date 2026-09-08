use crate::{
    save::{Global, GlobalValue, Save},
    ui::{
        styles::{set_checkbox_styles, set_spin_styles},
        widgets::{keyboard_list, UiExt},
        UiRef,
    },
    util::ContextExt,
};

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

        let options: Vec<_> = self
            .globals
            .iter()
            .map(|global| match &global.value {
                GlobalValue::Number(value) => format!("{} — {value}", global.name),
                GlobalValue::Boolean(value) => format!(
                    "{} — {}",
                    global.name,
                    if *value { "true" } else { "false" }
                ),
            })
            .collect();
        let mut selected = ui.ctx().get_data("eg_selected_global").unwrap_or(0);
        selected = selected.min(self.globals.len() - 1);
        ui.columns(2, |columns| {
            keyboard_list(
                &mut columns[0],
                "eg_global_list",
                "Global variable",
                &options,
                &mut selected,
            );
            columns[0].ctx().set_data("eg_selected_global", selected);

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
