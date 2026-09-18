use crate::{
    save::{Character, Class, Gender, Item, Save},
    ui::{
        styles::{set_checkbox_styles, set_drag_value_styles, set_spin_styles},
        widgets::{accessible_name, keyboard_list, visual_label, UiExt},
        UiRef,
    },
    util::{get_data_name, ContextExt},
};
use core::{Data, DataDescr, GameDataMapped, ItemSlot, UsableBy, WeaponType};
use egui::Id;
use std::{collections::HashSet, mem};

const SELECTED_ID: &str = "ec_selected";
const FIELD_ID: &str = "ec_accessible_field";
const SKILLS: [&str; 8] = [
    "Computer Use",
    "Demolitions",
    "Stealth",
    "Awareness",
    "Persuade",
    "Repair",
    "Security",
    "Treat Injury",
];
const ATTRIBUTES: [&str; 6] = [
    "Strength",
    "Dexterity",
    "Constitution",
    "Intelligence",
    "Wisdom",
    "Charisma",
];
const HUMAN_SLOTS: [&str; 12] = [
    "Implant",
    "Head",
    "Gloves",
    "Arm Left",
    "Armor",
    "Arm Right",
    "Belt",
    "Mainhand",
    "Offhand",
    "Mainhand 2",
    "Offhand 2",
    "Hidden slot",
];
const DROID_SLOTS: [&str; 12] = [
    "Utility Left",
    "Sensor",
    "Utility Right",
    "Arm Left",
    "Plating",
    "Arm Right",
    "Shield",
    "Mainhand",
    "Offhand",
    "Mainhand 2",
    "Offhand 2",
    "Hidden slot",
];

#[derive(Clone, Copy)]
enum Field {
    Name,
    Tag,
    MaxHp,
    Hp,
    MinOneHp,
    Invulnerable,
    MaxFp,
    Fp,
    Alignment,
    Experience,
    Influence,
    Skill(usize),
    Attribute(usize),
    Gender,
    Portrait,
    Appearance,
    Soundset,
    EquipmentType,
    ShowAllEquipment,
    Equipment(usize),
    Feats,
    Classes,
    ClassLevel(usize),
    Powers(usize),
}

#[derive(Clone, Copy)]
enum AppearanceKind {
    Portrait,
    Appearance,
    Soundset,
}

pub struct Editor<'a> {
    selected: usize,
    save: &'a mut Save,
    data: &'a GameDataMapped,
}

impl<'a> Editor<'a> {
    pub fn new(save: &'a mut Save, data: &'a GameDataMapped) -> Self {
        Self {
            selected: 0,
            save,
            data,
        }
    }

    pub fn show(&mut self, ui: UiRef) {
        if self.save.characters.is_empty() {
            visual_label(ui, "No characters are present in this save.");
            return;
        }
        self.selected = ui
            .ctx()
            .get_data(SELECTED_ID)
            .unwrap_or(0)
            .min(self.save.characters.len() - 1);
        self.character_selector(ui);
        ui.separator();

        let fields = self.fields(ui);
        let labels: Vec<_> = fields.iter().map(|(_, label)| label.clone()).collect();
        let mut selected_field = ui.ctx().get_data(FIELD_ID).unwrap_or(0);
        keyboard_list(
            ui,
            Id::new("ec_field_list").with(self.selected),
            "Character fields",
            &labels,
            &mut selected_field,
        );
        selected_field = selected_field.min(fields.len() - 1);
        ui.ctx().set_data(FIELD_ID, selected_field);
        ui.separator();
        self.field_editor(ui, fields[selected_field].0);
    }

    fn character_selector(&mut self, ui: UiRef) {
        let options: Vec<_> = self
            .save
            .characters
            .iter()
            .map(|character| character.get_name().to_owned())
            .collect();
        let old = self.selected;
        keyboard_list(
            ui,
            "ec_character_list",
            "Character",
            &options,
            &mut self.selected,
        );
        if old != self.selected {
            ui.ctx().set_data(SELECTED_ID, self.selected);
            ui.ctx().set_data(FIELD_ID, 0usize);
        }
    }

    fn equipment_type(&self, ui: UiRef) -> UsableBy {
        ui.ctx()
            .get_data(Id::new("ec_eq_usable_accessible").with(self.selected))
            .unwrap_or_else(|| droid_or_human(&self.save.characters[self.selected].tag))
    }

    fn show_all_equipment(&self, ui: UiRef) -> bool {
        ui.ctx()
            .get_data(Id::new("ec_eq_all_accessible").with(self.selected))
            .unwrap_or(false)
    }

    fn fields(&self, ui: UiRef) -> Vec<(Field, String)> {
        let character = &self.save.characters[self.selected];
        let mut fields = vec![
            (Field::Name, format!("Name — {}", character.get_name())),
            (
                Field::Tag,
                format!(
                    "Tag — {}",
                    if character.tag.is_empty() {
                        "None"
                    } else {
                        &character.tag
                    }
                ),
            ),
            (Field::MaxHp, format!("Maximum HP — {}", character.hp_max)),
            (Field::Hp, format!("Current HP — {}", character.hp)),
            (
                Field::MinOneHp,
                format!("Minimum 1 HP — {}", character.min_1_hp),
            ),
            (
                Field::Invulnerable,
                format!("Invulnerable — {}", character.invulnerable),
            ),
            (
                Field::MaxFp,
                format!("Maximum Force points — {}", character.fp_max),
            ),
            (
                Field::Fp,
                format!("Current Force points — {}", character.fp),
            ),
            (
                Field::Alignment,
                format!("Alignment — {}", character.good_evil),
            ),
            (
                Field::Experience,
                format!("Experience — {}", character.experience),
            ),
        ];
        if let Some(value) = self
            .save
            .party_table
            .influence
            .as_ref()
            .and_then(|values| values.get(character.idx))
        {
            fields.push((Field::Influence, format!("Influence — {value}")));
        }
        fields.extend(SKILLS.iter().enumerate().map(|(idx, name)| {
            (
                Field::Skill(idx),
                format!("Skill: {name} — {}", character.skills[idx]),
            )
        }));
        fields.extend(ATTRIBUTES.iter().enumerate().map(|(idx, name)| {
            (
                Field::Attribute(idx),
                format!("Attribute: {name} — {}", character.attributes[idx]),
            )
        }));
        fields.extend([
            (
                Field::Gender,
                format!("Gender — {}", character.gender.to_str()),
            ),
            (
                Field::Portrait,
                format!(
                    "Portrait — {}",
                    get_data_name(&self.data.portraits, &character.portrait)
                ),
            ),
            (
                Field::Appearance,
                format!(
                    "Appearance — {}",
                    get_data_name(&self.data.appearances, &character.appearance)
                ),
            ),
            (
                Field::Soundset,
                format!(
                    "Soundset — {}",
                    get_data_name(&self.data.soundsets, &character.soundset)
                ),
            ),
            (
                Field::EquipmentType,
                format!(
                    "Equipment type — {}",
                    if self.equipment_type(ui) == UsableBy::Droids {
                        "Droid"
                    } else {
                        "Human"
                    }
                ),
            ),
            (
                Field::ShowAllEquipment,
                format!(
                    "Show all equipment templates — {}",
                    self.show_all_equipment(ui)
                ),
            ),
        ]);
        let slot_names = if self.equipment_type(ui) == UsableBy::Droids {
            DROID_SLOTS
        } else {
            HUMAN_SLOTS
        };
        fields.extend(slot_names.iter().enumerate().map(|(idx, name)| {
            let value = character.equipment[idx]
                .as_ref()
                .map(Item::get_name)
                .unwrap_or("None");
            (
                Field::Equipment(idx),
                format!("Equipment: {name} — {value}"),
            )
        }));
        fields.push((
            Field::Feats,
            format!("Feats — {} current", character.feats.len()),
        ));
        fields.push((
            Field::Classes,
            format!("Classes — {} current", character.classes.len()),
        ));
        for (idx, class) in character.classes.iter().enumerate() {
            let name = get_data_name(&self.data.classes, &class.id);
            fields.push((
                Field::ClassLevel(idx),
                format!("Class level: {name} — {}", class.level),
            ));
            if let Some(powers) = &class.powers {
                fields.push((
                    Field::Powers(idx),
                    format!("Powers: {name} — {} current", powers.len()),
                ));
            }
        }
        fields
    }

    fn field_editor(&mut self, ui: UiRef, field: Field) {
        match field {
            Field::Name => {
                set_drag_value_styles(ui);
                ui.s_text_edit(
                    &mut self.save.characters[self.selected].name,
                    320.,
                    "Character name",
                );
            }
            Field::Tag => self.read_only(ui, "Character tag", |c| {
                if c.tag.is_empty() {
                    "None".to_owned()
                } else {
                    c.tag.clone()
                }
            }),
            Field::MaxHp => self.read_only(ui, "Maximum HP", |c| c.hp_max.to_string()),
            Field::Hp => {
                let character = &mut self.save.characters[self.selected];
                set_spin_styles(ui);
                ui.s_spin(
                    &mut character.hp,
                    0..=character.hp_max,
                    false,
                    "Current HP; displayed maximum may exclude gear and feat bonuses",
                );
            }
            Field::MinOneHp => {
                set_checkbox_styles(ui);
                ui.s_checkbox(
                    &mut self.save.characters[self.selected].min_1_hp,
                    "Minimum 1 HP",
                );
            }
            Field::Invulnerable => {
                set_checkbox_styles(ui);
                ui.s_checkbox(
                    &mut self.save.characters[self.selected].invulnerable,
                    "Invulnerable; character takes no damage",
                );
            }
            Field::MaxFp => self.read_only(ui, "Maximum Force points", |c| c.fp_max.to_string()),
            Field::Fp => {
                let character = &mut self.save.characters[self.selected];
                set_spin_styles(ui);
                ui.s_spin(
                    &mut character.fp,
                    0..=character.fp_max,
                    false,
                    "Current Force points; displayed maximum may exclude gear and feat bonuses",
                );
            }
            Field::Alignment => {
                set_spin_styles(ui);
                ui.s_spin(
                    &mut self.save.characters[self.selected].good_evil,
                    0..=100,
                    false,
                    "Alignment",
                );
            }
            Field::Experience => {
                set_spin_styles(ui);
                ui.s_spin(
                    &mut self.save.characters[self.selected].experience,
                    0..=9_999_999,
                    true,
                    "Experience",
                );
            }
            Field::Influence => {
                let idx = self.save.characters[self.selected].idx;
                if let Some(value) = self
                    .save
                    .party_table
                    .influence
                    .as_mut()
                    .and_then(|values| values.get_mut(idx))
                {
                    set_spin_styles(ui);
                    ui.s_spin(value, 0..=100, false, "Influence");
                }
            }
            Field::Skill(idx) => self.drag_value(ui, true, idx),
            Field::Attribute(idx) => self.drag_value(ui, false, idx),
            Field::Gender => self.gender_editor(ui),
            Field::Portrait => self.appearance_editor(ui, AppearanceKind::Portrait),
            Field::Appearance => self.appearance_editor(ui, AppearanceKind::Appearance),
            Field::Soundset => self.appearance_editor(ui, AppearanceKind::Soundset),
            Field::EquipmentType => self.equipment_type_editor(ui),
            Field::ShowAllEquipment => self.show_all_equipment_editor(ui),
            Field::Equipment(idx) => self.equipment_editor(ui, idx),
            Field::Feats => self.feats_editor(ui),
            Field::Classes => self.classes_editor(ui),
            Field::ClassLevel(idx) => self.class_level_editor(ui, idx),
            Field::Powers(idx) => self.powers_editor(ui, idx),
        }
    }

    fn read_only(&mut self, ui: UiRef, label: &str, value: impl FnOnce(&Character) -> String) {
        ui.s_button(
            &format!(
                "{label}: {}. Read only",
                value(&self.save.characters[self.selected])
            ),
            false,
            true,
        );
    }

    fn drag_value(&mut self, ui: UiRef, skill: bool, idx: usize) {
        set_spin_styles(ui);
        let (value, label) = if skill {
            (
                &mut self.save.characters[self.selected].skills[idx],
                SKILLS[idx],
            )
        } else {
            (
                &mut self.save.characters[self.selected].attributes[idx],
                ATTRIBUTES[idx],
            )
        };
        ui.s_spin(value, u8::MIN..=u8::MAX, false, label);
    }

    fn gender_editor(&mut self, ui: UiRef) {
        let options: Vec<_> = Gender::LIST
            .iter()
            .map(|value| value.to_str().to_owned())
            .collect();
        let character = &mut self.save.characters[self.selected];
        let mut selected = Gender::LIST
            .iter()
            .position(|value| *value == character.gender)
            .unwrap_or(0);
        keyboard_list(ui, "ec_gender_list", "Gender", &options, &mut selected);
        character.gender = Gender::LIST[selected];
    }

    fn appearance_editor(&mut self, ui: UiRef, kind: AppearanceKind) {
        let (label, key, data) = match kind {
            AppearanceKind::Portrait => {
                ("Portrait", "ec_portrait_list", &self.data.inner.portraits)
            }
            AppearanceKind::Appearance => (
                "Appearance",
                "ec_appearance_list",
                &self.data.inner.appearances,
            ),
            AppearanceKind::Soundset => {
                ("Soundset", "ec_soundset_list", &self.data.inner.soundsets)
            }
        };
        let options: Vec<_> = data.iter().map(|item| item.name.clone()).collect();
        let character = &mut self.save.characters[self.selected];
        let current = match kind {
            AppearanceKind::Portrait => &mut character.portrait,
            AppearanceKind::Appearance => &mut character.appearance,
            AppearanceKind::Soundset => &mut character.soundset,
        };
        let mut selected = data
            .iter()
            .position(|item| item.id == *current)
            .unwrap_or(0);
        keyboard_list(ui, key, label, &options, &mut selected);
        if let Some(item) = data.get(selected) {
            *current = item.id;
        }
    }

    fn equipment_type_editor(&mut self, ui: UiRef) {
        let key = Id::new("ec_eq_usable_accessible").with(self.selected);
        let mut selected = if self.equipment_type(ui) == UsableBy::Droids {
            1
        } else {
            0
        };
        keyboard_list(
            ui,
            "ec_equipment_type_list",
            "Equipment type",
            &["Human gear".to_owned(), "Droid gear".to_owned()],
            &mut selected,
        );
        ui.ctx().set_data(
            key,
            if selected == 1 {
                UsableBy::Droids
            } else {
                UsableBy::Humans
            },
        );
    }

    fn show_all_equipment_editor(&mut self, ui: UiRef) {
        let key = Id::new("ec_eq_all_accessible").with(self.selected);
        let mut value = self.show_all_equipment(ui);
        set_checkbox_styles(ui);
        ui.s_checkbox(&mut value, "Show all equipment templates");
        ui.ctx().set_data(key, value);
    }

    fn equipment_editor(&mut self, ui: UiRef, slot_idx: usize) {
        let usable_by = self.equipment_type(ui);
        let show_all = self.show_all_equipment(ui);
        let allowed_slots = self.allowed_slots(slot_idx);
        let slot_name = if usable_by == UsableBy::Droids {
            DROID_SLOTS[slot_idx]
        } else {
            HUMAN_SLOTS[slot_idx]
        };
        let mut candidates: Vec<Option<usize>> = vec![None];
        for (idx, item) in self.data.inner.items.iter().enumerate() {
            let Some(base) = self.data.inner.base_items.get(&item.base_item) else {
                continue;
            };
            if !allowed_slots.iter().any(|slot| *slot == base.slot) {
                continue;
            }
            if base.usable_by != UsableBy::All && base.usable_by != usable_by {
                continue;
            }
            if !show_all && item.name.is_none() {
                continue;
            }
            candidates.push(Some(idx));
        }
        let options: Vec<_> = candidates
            .iter()
            .map(|candidate| match candidate {
                None => "None".to_owned(),
                Some(idx) => {
                    let item = &self.data.inner.items[*idx];
                    accessible_name(item.get_name(), item.get_description())
                }
            })
            .collect();
        let cursor_key = Id::new("ec_equipment_cursor")
            .with(self.selected)
            .with(slot_idx);
        let current_tag = self.save.characters[self.selected].equipment[slot_idx]
            .as_ref()
            .map(|item| item.tag.as_str());
        let actual = candidates
            .iter()
            .position(|candidate| {
                candidate
                    .and_then(|idx| self.data.inner.items.get(idx))
                    .map(|item| item.tag.as_str())
                    == current_tag
            })
            .unwrap_or(0);
        let mut cursor = ui.ctx().get_data(cursor_key).unwrap_or(actual);
        keyboard_list(
            ui,
            cursor_key,
            &format!("Available equipment for {slot_name}"),
            &options,
            &mut cursor,
        );
        ui.ctx().set_data(cursor_key, cursor);
        if ui
            .s_button_basic(&format!("Equip selected item in {slot_name}"))
            .clicked()
        {
            let current = &mut self.save.characters[self.selected].equipment[slot_idx];
            match candidates[cursor] {
                None => *current = None,
                Some(template_idx) => {
                    let mut new_item: Item = (&self.data.inner.items[template_idx]).into();
                    if let Some(previous) = current.as_mut() {
                        mem::swap(&mut new_item, previous);
                        self.save.inventory.push(new_item);
                    } else {
                        *current = Some(new_item);
                    }
                }
            }
        }
    }

    fn allowed_slots(&self, idx: usize) -> Vec<ItemSlot> {
        match idx {
            0 => vec![ItemSlot::Implant],
            1 => vec![ItemSlot::Head],
            2 => vec![ItemSlot::Gloves],
            3 | 5 => vec![ItemSlot::Arms],
            4 => vec![ItemSlot::Armor],
            6 => vec![ItemSlot::Belt],
            7 | 9 => WeaponType::LIST.map(ItemSlot::Weapon).to_vec(),
            8 | 10 => {
                let mainhand = if idx == 8 { 7 } else { 9 };
                self.save.characters[self.selected].equipment[mainhand]
                    .as_ref()
                    .and_then(|item| self.data.inner.base_items.get(&item.base_item))
                    .and_then(|base| match base.slot {
                        ItemSlot::Weapon(kind) => Some(kind.offhand_option().to_vec()),
                        _ => None,
                    })
                    .unwrap_or_default()
            }
            _ => vec![],
        }
    }

    fn feats_editor(&mut self, ui: UiRef) {
        let current_ids = self.save.characters[self.selected].feats.clone();
        let mut current: Vec<_> = current_ids
            .iter()
            .enumerate()
            .map(|(source_idx, id)| {
                let name = self
                    .data
                    .feats
                    .get(id)
                    .map(|feat| accessible_name(feat.get_name(), feat.get_description()))
                    .unwrap_or_else(|| format!("Unknown feat {id}"));
                (source_idx, *id, name)
            })
            .collect();
        current.sort_unstable_by_key(|(source_idx, id, _)| {
            (
                self.data
                    .inner
                    .feats
                    .iter()
                    .position(|feat| feat.id == *id)
                    .unwrap_or(usize::MAX),
                *source_idx,
            )
        });
        let current_options: Vec<_> = current.iter().map(|(_, _, name)| name.clone()).collect();
        let current_key = Id::new("ec_current_feat_cursor").with(self.selected);
        let mut current_cursor = ui.ctx().get_data(current_key).unwrap_or(0);
        keyboard_list(
            ui,
            current_key,
            "Current feats",
            &current_options,
            &mut current_cursor,
        );
        ui.ctx().set_data(current_key, current_cursor);
        if ui
            .s_button("Remove selected feat", false, current.is_empty())
            .clicked()
        {
            self.save.characters[self.selected]
                .feats
                .remove(current[current_cursor].0);
        }

        ui.separator();
        let present: HashSet<_> = current_ids.iter().copied().collect();
        let available: Vec<_> = self
            .data
            .inner
            .feats
            .iter()
            .filter(|feat| !present.contains(&feat.id))
            .collect();
        let options: Vec<_> = available
            .iter()
            .map(|feat| accessible_name(feat.get_name(), feat.get_description()))
            .collect();
        let available_key = Id::new("ec_available_feat_cursor").with(self.selected);
        let mut cursor = ui.ctx().get_data(available_key).unwrap_or(0);
        keyboard_list(
            ui,
            available_key,
            "Feats available to add",
            &options,
            &mut cursor,
        );
        ui.ctx().set_data(available_key, cursor);
        if ui
            .s_button("Add selected feat", false, available.is_empty())
            .clicked()
        {
            self.save.characters[self.selected]
                .feats
                .push(available[cursor].id);
        }
    }

    fn classes_editor(&mut self, ui: UiRef) {
        let current_ids: Vec<_> = self.save.characters[self.selected]
            .classes
            .iter()
            .map(|class| class.id)
            .collect();
        let options: Vec<_> = current_ids
            .iter()
            .map(|id| get_data_name(&self.data.classes, id).into_owned())
            .collect();
        let current_key = Id::new("ec_current_class_cursor").with(self.selected);
        let mut cursor = ui.ctx().get_data(current_key).unwrap_or(0);
        keyboard_list(ui, current_key, "Current classes", &options, &mut cursor);
        ui.ctx().set_data(current_key, cursor);
        if ui
            .s_button("Remove selected class", false, current_ids.is_empty())
            .clicked()
        {
            self.save.characters[self.selected].classes.remove(cursor);
        }

        ui.separator();
        let present: HashSet<_> = current_ids.iter().copied().collect();
        let available: Vec<_> = self
            .data
            .inner
            .classes
            .iter()
            .filter(|class| !present.contains(&class.id))
            .collect();
        let available_options: Vec<_> = available.iter().map(|class| class.name.clone()).collect();
        let available_key = Id::new("ec_available_class_cursor").with(self.selected);
        let mut available_cursor = ui.ctx().get_data(available_key).unwrap_or(0);
        keyboard_list(
            ui,
            available_key,
            "Classes available to add",
            &available_options,
            &mut available_cursor,
        );
        ui.ctx().set_data(available_key, available_cursor);
        if ui
            .s_button("Add selected class", false, available.is_empty())
            .clicked()
        {
            let class = available[available_cursor];
            self.save.characters[self.selected].classes.push(Class {
                id: class.id,
                level: 1,
                powers: class.force_user.then(Vec::new),
            });
        }
    }

    fn class_level_editor(&mut self, ui: UiRef, class_idx: usize) {
        let class = &mut self.save.characters[self.selected].classes[class_idx];
        let name = get_data_name(&self.data.classes, &class.id);
        set_spin_styles(ui);
        ui.s_spin(&mut class.level, 0..=40, false, &format!("{name} level"));
    }

    fn powers_editor(&mut self, ui: UiRef, class_idx: usize) {
        let Some(current_ids) = self.save.characters[self.selected].classes[class_idx]
            .powers
            .as_ref()
            .cloned()
        else {
            return;
        };
        let mut current: Vec<_> = current_ids
            .iter()
            .enumerate()
            .map(|(source_idx, id)| {
                let name = self
                    .data
                    .powers
                    .get(id)
                    .map(|power| accessible_name(power.get_name(), power.get_description()))
                    .unwrap_or_else(|| format!("Unknown power {id}"));
                (source_idx, name)
            })
            .collect();
        current.sort_unstable_by(|a, b| a.1.cmp(&b.1));
        let options: Vec<_> = current.iter().map(|(_, name)| name.clone()).collect();
        let current_key = Id::new("ec_current_power_cursor")
            .with(self.selected)
            .with(class_idx);
        let mut cursor = ui.ctx().get_data(current_key).unwrap_or(0);
        keyboard_list(ui, current_key, "Current powers", &options, &mut cursor);
        ui.ctx().set_data(current_key, cursor);
        if ui
            .s_button("Remove selected power", false, current.is_empty())
            .clicked()
        {
            self.save.characters[self.selected].classes[class_idx]
                .powers
                .as_mut()
                .unwrap()
                .remove(current[cursor].0);
        }

        ui.separator();
        let present: HashSet<_> = current_ids.iter().copied().collect();
        let show_all_key = Id::new("ec_powers_all_accessible")
            .with(self.selected)
            .with(class_idx);
        let mut show_all = ui.ctx().get_data(show_all_key).unwrap_or(false);
        set_checkbox_styles(ui);
        ui.s_checkbox(&mut show_all, "Show all powers, including internal entries");
        ui.ctx().set_data(show_all_key, show_all);
        let available: Vec<_> = self
            .data
            .inner
            .powers
            .iter()
            .filter(|power| !present.contains(&power.id) && (show_all || !power.extra))
            .collect();
        let available_options: Vec<_> = available
            .iter()
            .map(|power| accessible_name(power.get_name(), power.get_description()))
            .collect();
        let available_key = Id::new("ec_available_power_cursor")
            .with(self.selected)
            .with(class_idx);
        let mut available_cursor = ui.ctx().get_data(available_key).unwrap_or(0);
        keyboard_list(
            ui,
            available_key,
            "Powers available to add",
            &available_options,
            &mut available_cursor,
        );
        ui.ctx().set_data(available_key, available_cursor);
        if ui
            .s_button("Add selected power", false, available.is_empty())
            .clicked()
        {
            self.save.characters[self.selected].classes[class_idx]
                .powers
                .as_mut()
                .unwrap()
                .push(available[available_cursor].id);
        }
    }
}

fn droid_or_human(tag: &str) -> UsableBy {
    match tag {
        "t3m4" | "hk47" | "g0t0" | "remote" | "3cfd" | "b4d4" => UsableBy::Droids,
        _ => UsableBy::Humans,
    }
}
