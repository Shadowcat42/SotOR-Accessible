use crate::{
    save::Item,
    ui::{
        styles::set_spin_styles,
        widgets::{keyboard_list, UiExt},
        UiRef,
    },
    util::{ContextExt, Game},
};
use egui::Id;

pub fn show(ui: UiRef, item: &mut Item, game: Game, id: Id) {
    ui.separator();
    ui.heading("Item upgrades");
    ui.label(
        "Advanced: upgrade values are upgrade.2da row numbers. Only use rows compatible with this item.",
    );

    match game {
        Game::One => kotor_one(ui, item, id),
        Game::Two => kotor_two(ui, item, id),
    }
}

fn kotor_one(ui: UiRef, item: &mut Item, id: Id) {
    let installed = kotor_one_rows(item.upgrades);
    let options: Vec<_> = installed
        .iter()
        .map(|row| format!("upgrade.2da row {row}"))
        .collect();
    let current_key = id.with("k1_current_upgrade");
    let mut current = ui.ctx().get_data(current_key).unwrap_or(0);
    keyboard_list(
        ui,
        current_key,
        "Installed item upgrades",
        &options,
        &mut current,
    );
    ui.ctx().set_data(current_key, current);

    if ui
        .s_button(
            "Remove selected upgrade",
            false,
            installed.is_empty(),
        )
        .clicked()
    {
        item.upgrades &= !(1u32 << installed[current]);
        current = current.min(kotor_one_rows(item.upgrades).len().saturating_sub(1));
        ui.ctx().set_data(current_key, current);
    }

    let add_key = id.with("k1_add_upgrade");
    let mut row = ui.ctx().get_data(add_key).unwrap_or(0u8);
    set_spin_styles(ui);
    ui.s_spin(&mut row, 0..=31, false, "Upgrade row to add");
    ui.ctx().set_data(add_key, row);
    if ui.s_button_basic("Add upgrade row").clicked() {
        item.upgrades |= 1u32 << row;
    }
}

fn kotor_two(ui: UiRef, item: &mut Item, id: Id) {
    let Some(slots) = item.upgrade_slots.as_mut() else {
        ui.label("This item has no KotOR II upgrade slots.");
        return;
    };

    let options: Vec<_> = slots
        .iter()
        .enumerate()
        .map(|(index, row)| {
            if *row < 0 {
                format!("Slot {}: empty", index + 1)
            } else {
                format!("Slot {}: upgrade.2da row {row}", index + 1)
            }
        })
        .collect();
    let slot_key = id.with("k2_upgrade_slot");
    let mut selected = ui.ctx().get_data(slot_key).unwrap_or(0);
    keyboard_list(
        ui,
        slot_key,
        "Item upgrade slots",
        &options,
        &mut selected,
    );
    ui.ctx().set_data(slot_key, selected);

    set_spin_styles(ui);
    ui.s_spin(
        &mut slots[selected],
        i32::MIN..=i32::MAX,
        false,
        "Selected upgrade slot row; minus 1 means empty",
    );
    if ui
        .s_button(
            "Clear selected upgrade slot",
            false,
            slots[selected] < 0,
        )
        .clicked()
    {
        slots[selected] = -1;
    }
}

fn kotor_one_rows(upgrades: u32) -> Vec<u8> {
    (0..32)
        .filter(|row| upgrades & (1u32 << row) != 0)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::kotor_one_rows;

    #[test]
    fn kotor_one_upgrade_rows_decode_the_bitmask() {
        assert_eq!(kotor_one_rows(0), Vec::<u8>::new());
        assert_eq!(
            kotor_one_rows(1u32 | (1u32 << 7) | (1u32 << 31)),
            vec![0, 7, 31]
        );
    }
}
