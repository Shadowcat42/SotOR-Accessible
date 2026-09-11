use crate::{
    save::{JournalEntry, Save},
    ui::{
        styles::set_spin_styles,
        widgets::{accessible_name, keyboard_list, Icon, IconButton, UiExt},
        UiRef,
    },
    util::{get_data_name, ContextExt as _},
};
use core::{GameDataMapped, Quest, QuestStage};
use egui::Id;
use std::{
    collections::HashSet,
    sync::{Arc, Mutex},
};

#[derive(Debug, Clone, PartialEq)]
pub struct EditorQuestsState {
    id: String,
    stage: i32,
}
pub struct Editor<'a> {
    journal: &'a mut Vec<JournalEntry>,
    data: &'a GameDataMapped,
}

impl<'a> Editor<'a> {
    pub fn new(save: &'a mut Save, data: &'a GameDataMapped) -> Self {
        Self {
            journal: &mut save.party_table.journal,
            data,
        }
    }

    pub fn show(&mut self, ui: UiRef) {
        if self.journal.is_empty() {
            ui.label("No current or completed quests are present in this save.");
        } else {
            self.current_quest(ui);
        }
        ui.separator();
        self.addition(ui);
    }

    fn current_quest(&mut self, ui: UiRef) {
        let mut quests = Vec::with_capacity(self.journal.len());
        for (idx, entry) in self.journal.iter().enumerate() {
            let quest = self.data.quests.get(&entry.id.to_lowercase());
            let stage = quest.and_then(|quest| quest.stages.get(&entry.stage));
            let name = get_data_name(&self.data.quests, &entry.id);
            let completed = stage.map_or(false, |s| s.end);

            quests.push((idx, completed, name.into_owned()));
        }
        quests.sort_unstable_by(|a, b| {
            let completed_eq = a.1.cmp(&b.1);
            if completed_eq.is_ne() {
                return completed_eq;
            }
            a.2.cmp(&b.2)
        });

        let options: Vec<_> = quests
            .iter()
            .map(|(_, completed, name)| {
                if *completed {
                    format!("Completed quest: {name}")
                } else {
                    format!("Current quest: {name}")
                }
            })
            .collect();
        let cursor_key = "eq_current_quest_cursor";
        let mut cursor = ui.ctx().get_data(cursor_key).unwrap_or(0);
        cursor = cursor.min(quests.len() - 1);
        let mut remove = None;
        ui.columns(3, |columns| {
            keyboard_list(
                &mut columns[0],
                "eq_current_quest",
                "Current or completed quest",
                &options,
                &mut cursor,
            );
            columns[0].ctx().set_data(cursor_key, cursor);

            let (source_idx, _, name) = &quests[cursor];
            let source_idx = *source_idx;
            let entry = &mut self.journal[source_idx];
            if let Some(quest) = self.data.quests.get(&entry.id.to_lowercase()) {
                let stages = &quest.stages;
                let mut stage_ids: Vec<_> = stages.keys().copied().collect();
                if !stages.contains_key(&entry.stage) {
                    stage_ids.insert(0, entry.stage);
                }
                let options: Vec<_> = stage_ids
                    .iter()
                    .map(|id| {
                        stages
                            .get(id)
                            .map_or_else(|| format!("{id} UNKNOWN"), |stage| stage_option(stage))
                    })
                    .collect();
                let mut stage_cursor = stage_ids
                    .iter()
                    .position(|id| *id == entry.stage)
                    .unwrap_or(0);
                keyboard_list(
                    &mut columns[1],
                    Id::new("eq_current_stage").with(source_idx),
                    &format!("{name} stage"),
                    &options,
                    &mut stage_cursor,
                );
                entry.stage = stage_ids[stage_cursor];
            } else {
                set_spin_styles(&mut columns[1]);
                columns[1].s_spin(
                    &mut entry.stage,
                    i32::MIN..=i32::MAX,
                    false,
                    &format!("{name} stage"),
                );
            }
            if columns[2]
                .s_button_basic(&format!("Remove quest {name}"))
                .clicked()
            {
                remove = Some(source_idx);
            }
        });
        if let Some(source_idx) = remove {
            self.journal.remove(source_idx);
        }
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
        let mut add = false;
        ui.columns(3, |columns| {
            keyboard_list(
                &mut columns[0],
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
                            stage_option(stage)
                        })
                        .collect()
                })
                .unwrap_or_default();
            let mut stage_cursor = stage_ids
                .iter()
                .position(|id| *id == state.stage)
                .unwrap_or(0);
            keyboard_list(
                &mut columns[1],
                "eq_new_stage",
                "New quest stage",
                &stage_options,
                &mut stage_cursor,
            );
            if let Some(stage) = stage_ids.get(stage_cursor) {
                state.stage = *stage;
            }
            add = columns[2]
                .add_enabled(
                    !available.is_empty(),
                    IconButton::new(Icon::Plus).hint("Add quest"),
                )
                .clicked();
        });

        if add {
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

fn stage_option(stage: &QuestStage) -> String {
    accessible_name(&stage.id.to_string(), Some(&stage.description))
}

#[cfg(test)]
mod tests {
    use super::stage_option;
    use core::QuestStage;

    #[test]
    fn quest_stage_option_announces_its_description_once() {
        let stage = QuestStage {
            id: 90,
            description: "Canderous needs time to think about Jagi.".to_owned(),
            end: false,
        };

        assert_eq!(
            stage_option(&stage),
            "90. Description: Canderous needs time to think about Jagi."
        );
    }
}
