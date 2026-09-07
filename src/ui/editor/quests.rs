use crate::{
    save::{JournalEntry, Save},
    ui::{
        styles::{set_drag_value_styles, set_striped_styles, GREY, WHITE},
        widgets::{accessible_name, color_text, keyboard_list, Icon, IconButton, UiExt},
        UiRef,
    },
    util::{get_data_name, ColumnCounter, ContextExt as _},
};
use core::{GameDataMapped, Quest, QuestStage};
use egui::{DragValue, Grid, Id, RichText, ScrollArea, WidgetInfo};
use std::{
    collections::{BTreeMap, HashSet},
    sync::{Arc, Mutex},
};

#[derive(Debug, Clone, PartialEq)]
pub struct EditorQuestsState {
    id: String,
    stage: i32,
}
pub struct Editor<'a> {
    journal: &'a mut Vec<JournalEntry>,
    width: f32,
    data: &'a GameDataMapped,
}

impl<'a> Editor<'a> {
    pub fn new(save: &'a mut Save, data: &'a GameDataMapped) -> Self {
        Self {
            journal: &mut save.party_table.journal,
            data,
            width: 0.,
        }
    }

    pub fn show(&mut self, ui: UiRef) {
        self.width = ui.available_width();

        ScrollArea::vertical()
            .scroll_bar_visibility(egui::containers::scroll_area::ScrollBarVisibility::AlwaysHidden)
            .drag_to_scroll(false)
            .id_source("eq_scroll")
            .stick_to_bottom(true)
            .max_height(ui.available_height() - 30.)
            .show(ui, |ui| {
                ui.set_width(self.width);
                ui.set_height(ui.available_height());
                set_striped_styles(ui);

                Grid::new("eq_grid")
                    .spacing([0., 5.])
                    .striped(true)
                    .num_columns(3)
                    .max_col_width(230.)
                    .show(ui, |ui| {
                        self.table(ui);
                    });
            });
        ui.separator();
        self.addition(ui);
    }

    fn table(&mut self, ui: UiRef) {
        let columns = ((self.width / 715.) as usize).clamp(1, self.journal.len().max(1));
        let counter = &mut ColumnCounter::new(columns);

        for _ in 0..columns {
            ui.s_empty();
            ui.label(RichText::new("Name").underline());
            ui.add_space(10.);
            ui.label(RichText::new("Stage").underline());
            ui.add_space(10.);
            counter.next(ui);
        }
        let mut quests = Vec::with_capacity(self.journal.len());
        for (idx, entry) in self.journal.iter_mut().enumerate() {
            let quest = self.data.quests.get(&entry.id.to_lowercase());
            let stages = quest.map(|q| &q.stages);
            let stage = stages.and_then(|s| s.get(&entry.stage));
            let name = get_data_name(&self.data.quests, &entry.id);
            let completed = stage.map_or(false, |s| s.end);

            quests.push((completed, name, entry, stages, idx));
        }
        // sort them by completeness -> name
        quests.sort_unstable_by(|a, b| {
            let completed_eq = a.0.cmp(&b.0);
            if completed_eq.is_ne() {
                return completed_eq;
            }
            a.1.cmp(&b.1)
        });

        let mut removed = None;
        for (completed, name, entry, stages, idx) in quests {
            let remove = || removed = Some(idx);
            Self::quest(ui, completed, &name, entry, stages, idx, remove);
            counter.next(ui);
        }

        if let Some(idx) = removed {
            self.journal.remove(idx);
        }
    }

    fn quest(
        ui: UiRef,
        completed: bool,
        name: &str,
        entry: &mut JournalEntry,
        stages: Option<&BTreeMap<i32, QuestStage>>,
        source_idx: usize,
        remove: impl FnOnce(),
    ) {
        if ui
            .s_icon_button(Icon::Remove, &format!("Remove quest {name}"))
            .clicked()
        {
            remove();
        }

        let name_color = if completed { GREY } else { WHITE };
        let quest_accessible_name = if completed {
            format!("Completed quest: {name}")
        } else {
            format!("Quest: {name}")
        };
        let label_r = ui.label(color_text(&quest_accessible_name, name_color));
        ui.add_space(10.);
        // it's not already in the name
        if stages.is_some() {
            label_r.on_hover_text(color_text(&entry.id, WHITE));
        }

        if let Some(stages) = stages {
            let mut stage_ids: Vec<_> = stages.keys().copied().collect();
            if !stages.contains_key(&entry.stage) {
                stage_ids.insert(0, entry.stage);
            }
            let options: Vec<_> = stage_ids
                .iter()
                .map(|id| {
                    stages.get(id).map_or_else(
                        || format!("{id} UNKNOWN"),
                        |stage| accessible_name(&stage.get_name(60), Some(&stage.description)),
                    )
                })
                .collect();
            let mut cursor = stage_ids
                .iter()
                .position(|id| *id == entry.stage)
                .unwrap_or(0);
            keyboard_list(
                ui,
                Id::new("eq_current_stage").with(source_idx),
                &format!("{name} stage"),
                &options,
                &mut cursor,
            );
            entry.stage = stage_ids[cursor];
        } else {
            set_drag_value_styles(ui);
            let response = ui.add(DragValue::new(&mut entry.stage));
            response.widget_info(|| WidgetInfo {
                label: Some(format!("{name} stage")),
                ..WidgetInfo::drag_value(entry.stage.into())
            });
        }
        ui.add_space(10.);
    }

    fn get_present_ids(&self) -> HashSet<&String> {
        self.journal.iter().map(|e| &e.id).collect()
    }

    fn addition(&mut self, ui: UiRef) {
        const QUESTS_STATE_ID: &str = "eq_state";
        let state = ui.ctx().get_data(QUESTS_STATE_ID).unwrap_or_else(|| {
            let present = self.get_present_ids();
            let mut quest = None;

            for q in &self.data.inner.quests {
                if !present.contains(&q.id) {
                    quest = Some(q);
                    break;
                }
            }
            let state = Arc::new(Mutex::new(EditorQuestsState {
                id: quest.map(|q| q.id.clone()).unwrap_or_default(),
                stage: quest.map(Quest::get_first_stage_id).unwrap_or_default(),
            }));
            ui.ctx().set_data(QUESTS_STATE_ID, state.clone());

            state
        });
        let mut state = state.lock().unwrap();
        let present = self.get_present_ids();
        let available: Vec<_> = self
            .data
            .inner
            .quests
            .iter()
            .filter(|quest| !present.contains(&quest.id))
            .collect();
        if !available.is_empty() && !available.iter().any(|quest| quest.id == state.id) {
            state.id = available[0].id.clone();
            state.stage = available[0].get_first_stage_id();
        }
        let quest_options: Vec<_> = available
            .iter()
            .map(|quest| format!("{} internal ID {}", quest.name, quest.id))
            .collect();
        let mut quest_cursor = available
            .iter()
            .position(|quest| quest.id == state.id)
            .unwrap_or(0);
        let previous_quest = quest_cursor;
        keyboard_list(
            ui,
            "eq_new_id",
            "Quest to add",
            &quest_options,
            &mut quest_cursor,
        );
        if !available.is_empty() && quest_cursor != previous_quest {
            state.id = available[quest_cursor].id.clone();
            state.stage = available[quest_cursor].get_first_stage_id();
        }

        let current_quest = available.get(quest_cursor).copied();
        let stage_ids: Vec<_> = current_quest
            .map(|quest| quest.stages.keys().copied().collect())
            .unwrap_or_default();
        let stage_options: Vec<_> = current_quest
            .map(|quest| {
                stage_ids
                    .iter()
                    .map(|id| {
                        let stage = &quest.stages[id];
                        accessible_name(&stage.get_name(40), Some(&stage.description))
                    })
                    .collect()
            })
            .unwrap_or_default();
        let mut stage_cursor = stage_ids
            .iter()
            .position(|id| *id == state.stage)
            .unwrap_or(0);
        keyboard_list(
            ui,
            "eq_new_stage",
            "New quest stage",
            &stage_options,
            &mut stage_cursor,
        );
        if let Some(stage) = stage_ids.get(stage_cursor) {
            state.stage = *stage;
        }
        let btn = ui.add_enabled(
            !available.is_empty(),
            IconButton::new(Icon::Plus).hint("Add quest"),
        );

        if btn.clicked() {
            let last = self.journal.last();
            self.journal.push(JournalEntry {
                id: state.id.trim().to_owned(),
                stage: state.stage,
                date: last.map_or(0, |e| e.date),
                time: last.map_or(0, |e| e.time + 1),
            });
            state.id = String::new();
            state.stage = 0;
        }
    }
}
