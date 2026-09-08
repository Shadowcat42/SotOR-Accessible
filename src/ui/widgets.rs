use super::{
    styles::{BLACK, BLUE, GREEN, GREEN_DARK, GREY, GREY_DARK, WHITE},
    UiRef,
};
use core::util::shorten_string;
use egui::{
    epaint::TextShape, Area, Button, Color32, CursorIcon, DragValue, EventFilter, FontSelection,
    Frame, Id, Key, Label, Modifiers, Order, Response, RichText, Rounding, Sense, Stroke,
    TextBuffer, TextEdit, TextStyle, Ui, Widget, WidgetInfo, WidgetText, WidgetType,
};
use emath::{pos2, vec2, Align, Numeric};
use std::ops::RangeInclusive;

pub fn color_text(text: &str, color: Color32) -> RichText {
    RichText::new(text).color(color)
}

pub fn accessible_name(name: &str, description: Option<&str>) -> String {
    let Some(description) = description else {
        return name.to_owned();
    };
    let description = shorten_string(&description.replace(['\r', '\n'], " "), 300);
    format!("{name}. Description: {description}")
}

/// A single-tab-stop list selector for screen-reader and keyboard users.
///
/// egui's popup combo boxes do not provide dependable Windows keyboard
/// behavior. This control exposes one stable selector to the Tab order.
/// Up/Down/Home/End change the selection without moving focus.
pub fn keyboard_list(
    ui: &mut Ui,
    id: impl std::hash::Hash,
    label: &str,
    options: &[String],
    selected: &mut usize,
) -> Response {
    if options.is_empty() {
        *selected = 0;
        let response = ui.add_enabled(false, Button::new(format!("{label}: empty")));
        response.widget_info(|| WidgetInfo {
            current_text_value: Some("Empty".to_owned()),
            ..WidgetInfo::labeled(WidgetType::ComboBox, label)
        });
        return response;
    }

    *selected = (*selected).min(options.len() - 1);
    let control_id = Id::new(id).with("keyboard_list_control");
    let mut response = ui.interact(
        ui.available_rect_before_wrap()
            .shrink2(vec2(0., 0.))
            .with_max_y(ui.cursor().top() + ui.spacing().interact_size.y),
        control_id,
        Sense::click(),
    );
    if response.clicked() {
        response.request_focus();
    }

    let previous = *selected;
    if response.has_focus() {
        // Focus navigation is calculated before widgets consume input. Lock
        // vertical arrows while this selector is focused, exactly as egui's
        // native vertical sliders do, so Up/Down cannot escape the control.
        ui.memory_mut(|memory| {
            memory.set_focus_lock_filter(
                response.id,
                EventFilter {
                    horizontal_arrows: true,
                    vertical_arrows: true,
                    ..Default::default()
                },
            );
        });
        ui.input_mut(|input| {
            if input.consume_key(Modifiers::NONE, Key::ArrowDown) {
                *selected = (*selected + 1).min(options.len() - 1);
            }
            if input.consume_key(Modifiers::NONE, Key::ArrowUp) {
                *selected = selected.saturating_sub(1);
            }
            if input.consume_key(Modifiers::NONE, Key::Home) {
                *selected = 0;
            }
            if input.consume_key(Modifiers::NONE, Key::End) {
                *selected = options.len() - 1;
            }
            // A combo box owns all arrow keys. Horizontal arrows do not
            // change this vertical list, but must never escape it either.
            input.consume_key(Modifiers::NONE, Key::ArrowLeft);
            input.consume_key(Modifiers::NONE, Key::ArrowRight);
        });
    }

    let selected_name = options[*selected].clone();
    let control_rect = response.rect;
    ui.painter().rect(
        control_rect,
        Rounding::same(2.),
        WHITE,
        (
            2.,
            if response.has_focus() {
                GREEN
            } else {
                GREEN_DARK
            },
        ),
    );
    ui.painter().text(
        control_rect.left_center() + vec2(6., 0.),
        egui::Align2::LEFT_CENTER,
        &selected_name,
        TextStyle::Button.resolve(ui.style()),
        BLACK,
    );
    ui.advance_cursor_after_rect(control_rect);

    response.widget_info(|| {
        let current_value = format!("{selected_name} {} of {}", *selected + 1, options.len());
        let previous_value = (previous != *selected).then(|| {
            format!(
                "{} {} of {}",
                options[previous],
                previous + 1,
                options.len()
            )
        });
        WidgetInfo {
            current_text_value: Some(current_value),
            prev_text_value: previous_value,
            ..WidgetInfo::labeled(WidgetType::ComboBox, label)
        }
    });

    if previous != *selected {
        response.mark_changed();
        ui.ctx().request_repaint();
    }
    response
}

/// A proper accessible tab list with roving keyboard focus.
///
/// Each page is exposed as a UIA tab. Only the selected tab is in the Tab
/// order; Left/Right wrap, Home/End jump, and Up/Down are contained.
pub fn keyboard_tab_list(
    ui: &mut Ui,
    id: impl std::hash::Hash,
    options: &[String],
    selected: &mut usize,
) -> Response {
    if options.is_empty() {
        return ui.add_enabled(false, Button::new("No pages"));
    }

    *selected = (*selected).min(options.len() - 1);
    let base_id = Id::new(id).with("keyboard_tab_list");
    let previous = *selected;
    let focused =
        (0..options.len()).find(|idx| ui.memory(|memory| memory.has_focus(base_id.with(*idx))));
    if let Some(focused) = focused {
        ui.memory_mut(|memory| {
            memory.set_focus_lock_filter(
                base_id.with(focused),
                EventFilter {
                    horizontal_arrows: true,
                    vertical_arrows: true,
                    ..Default::default()
                },
            );
        });
        ui.input_mut(|input| {
            if input.consume_key(Modifiers::NONE, Key::ArrowRight) {
                *selected = (focused + 1) % options.len();
            }
            if input.consume_key(Modifiers::NONE, Key::ArrowLeft) {
                *selected = (focused + options.len() - 1) % options.len();
            }
            if input.consume_key(Modifiers::NONE, Key::Home) {
                *selected = 0;
            }
            if input.consume_key(Modifiers::NONE, Key::End) {
                *selected = options.len() - 1;
            }
            input.consume_key(Modifiers::NONE, Key::ArrowUp);
            input.consume_key(Modifiers::NONE, Key::ArrowDown);
        });
    }

    let keyboard_changed = previous != *selected;
    let mut selected_response = None;
    ui.horizontal(|ui| {
        for (idx, name) in options.iter().enumerate() {
            let is_selected = idx == *selected;
            let text = format!("{name} {} of {}", idx + 1, options.len());
            let galley = WidgetText::from(name.clone()).into_galley(
                ui,
                Some(true),
                f32::INFINITY,
                TextStyle::Button,
            );
            let size = galley.size() + vec2(14., 6.);
            let sense = Sense {
                click: true,
                drag: false,
                focusable: is_selected,
            };
            let (_, rect) = ui.allocate_space(size);
            let response = ui.interact(rect, base_id.with(idx), sense);
            response
                .widget_info(|| WidgetInfo::selected(WidgetType::Tab, is_selected, text.clone()));

            if response.clicked() {
                *selected = idx;
                response.request_focus();
                selected_response = Some(response.clone());
            }
            if is_selected && selected_response.is_none() {
                selected_response = Some(response.clone());
            }

            let fill = if is_selected { WHITE } else { GREY_DARK };
            let stroke = if response.has_focus() {
                Stroke::new(2., GREEN)
            } else {
                Stroke::new(1., GREEN_DARK)
            };
            ui.painter().rect(rect, Rounding::same(2.), fill, stroke);
            ui.painter().galley(
                rect.center() - galley.size() / 2.,
                galley,
                if is_selected { BLACK } else { WHITE },
            );
        }
    });

    let mut response = selected_response.expect("selected tab is always rendered");
    if keyboard_changed {
        response.request_focus();
    }

    if previous != *selected {
        response.mark_changed();
        ui.ctx().request_repaint();
    }
    response
}

/// A visual label which is deliberately omitted from the Tab order. The
/// associated interactive control carries the accessible name.
pub fn visual_label(ui: &mut Ui, text: impl Into<WidgetText>) -> Response {
    ui.add(Label::new(text).sense(Sense::hover()))
}

pub trait UiExt {
    fn s_text_edit(&mut self, text: &mut dyn TextBuffer, width: f32, label: &str) -> Response;
    fn s_spin<T: Numeric>(
        &mut self,
        value: &mut T,
        range: RangeInclusive<T>,
        logarithmic: bool,
        label: &str,
    ) -> Response;
    fn s_button(&mut self, text: &str, selected: bool, disabled: bool) -> Response;
    fn s_button_basic(&mut self, text: &str) -> Response;
    fn s_checkbox(&mut self, value: &mut bool, label: &str) -> Response;
    fn s_text(&mut self, text: &str) -> Response;
    fn s_offset(&mut self, x: f32, y: f32);
    fn s_empty(&mut self);
    fn s_icon_button(&mut self, icon: Icon, hint: &str) -> Response;
    fn s_list_item(&mut self, selected: bool, text: impl Into<WidgetText>) -> Response;
}

impl UiExt for Ui {
    fn s_text_edit(&mut self, text: &mut dyn TextBuffer, width: f32, label: &str) -> Response {
        let response = TextEdit::singleline(text)
            .vertical_align(egui::Align::Center)
            .min_size([width, 0.].into())
            .margin([4., 1.].into())
            .desired_width(width)
            .text_color(BLACK)
            .ui(self);
        response.widget_info(|| WidgetInfo {
            label: Some(label.to_owned()),
            ..WidgetInfo::text_edit("", text.as_str())
        });
        response
    }

    fn s_spin<T: Numeric>(
        &mut self,
        value: &mut T,
        range: RangeInclusive<T>,
        _logarithmic: bool,
        label: &str,
    ) -> Response {
        let response = self.add(DragValue::new(value).speed(1.).clamp_range(range));
        self.memory_mut(|memory| {
            memory.set_focus_lock_filter(
                response.id,
                EventFilter {
                    vertical_arrows: true,
                    ..Default::default()
                },
            );
        });
        response.widget_info(|| WidgetInfo {
            label: Some(label.to_owned()),
            ..WidgetInfo::drag_value(value.to_f64())
        });
        response
    }

    fn s_button(&mut self, text: &str, selected: bool, disabled: bool) -> Response {
        let mut text = RichText::new(text);
        if selected {
            text = text.color(WHITE);
        }
        let mut btn = Button::new(text).selected(selected).rounding(2.);
        if selected {
            btn = btn.stroke((2., WHITE));
        }
        self.add_enabled(!disabled, btn)
            .on_hover_cursor(if disabled {
                CursorIcon::NotAllowed
            } else {
                CursorIcon::PointingHand
            })
    }

    fn s_button_basic(&mut self, text: &str) -> Response {
        self.s_button(text, false, false)
    }

    fn s_checkbox(&mut self, value: &mut bool, label: &str) -> Response {
        self.checkbox(value, label)
            .on_hover_cursor(CursorIcon::PointingHand)
    }

    fn s_text(&mut self, text: &str) -> Response {
        self.label(color_text(text, WHITE))
    }

    fn s_offset(&mut self, x: f32, y: f32) {
        self.allocate_exact_size([x, y].into(), Sense::hover());
    }

    fn s_empty(&mut self) {
        self.s_offset(0., 0.);
    }

    fn s_icon_button(&mut self, icon: Icon, hint: &str) -> Response {
        self.add(IconButton::new(icon).hint(hint))
    }

    fn s_list_item(&mut self, selected: bool, text: impl Into<WidgetText>) -> Response {
        ListItem::new(selected, text).ui(self)
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Icon {
    Check,
    Close,
    #[cfg(not(target_arch = "wasm32"))]
    Gear,
    Leave,
    Plus,
    Refresh,
    #[cfg(not(target_arch = "wasm32"))]
    Reload,
    Remove,
    Save,
    #[allow(dead_code)]
    Triangle,
}

impl Icon {
    pub fn symbol(self) -> &'static str {
        match self {
            Self::Check => "\u{f00c}",
            Self::Close => "\u{f00d}",
            #[cfg(not(target_arch = "wasm32"))]
            Self::Gear => "\u{f013}",
            Self::Leave => "\u{f2f5}",
            Self::Plus => "\u{002b}",
            Self::Refresh => "\u{f021}",
            #[cfg(not(target_arch = "wasm32"))]
            Self::Reload => "\u{f079}",
            Self::Remove => "\u{f2ed}",
            Self::Save => "\u{f0c7}",
            Self::Triangle => "\u{f0d7}",
        }
    }
}

pub struct IconButton<'a> {
    icon: Icon,
    hint: Option<&'a str>,
    color: Color32,
    color_hovered: Color32,
    size: f32,
}

impl<'a> Default for IconButton<'a> {
    fn default() -> Self {
        Self {
            icon: Icon::Close,
            hint: None,
            color: GREEN,
            color_hovered: WHITE,
            size: 18.,
        }
    }
}

impl<'a> IconButton<'a> {
    pub fn new(icon: Icon) -> Self {
        Self {
            icon,
            ..Default::default()
        }
    }

    pub fn hint(mut self, hint: &'a str) -> Self {
        self.hint = Some(hint);
        self
    }

    #[cfg(not(target_arch = "wasm32"))]
    pub fn size(mut self, size: f32) -> Self {
        self.size = size;
        self
    }
}

impl<'a> Widget for IconButton<'a> {
    fn ui(self, ui: &mut Ui) -> Response {
        let valign = ui.layout().vertical_align();
        let text: WidgetText = RichText::new(self.icon.symbol())
            .text_style(TextStyle::Name("icon".into()))
            .size(self.size)
            .into();
        let mut layout_job = text.into_layout_job(ui.style(), FontSelection::Default, valign);

        layout_job.wrap.max_width = f32::INFINITY;
        layout_job.halign = ui.layout().horizontal_placement();
        layout_job.justify = ui.layout().horizontal_justify();

        let text_galley = ui.fonts(|f| f.layout_job(layout_job));

        let (rect, mut response) = ui.allocate_exact_size(text_galley.size(), Sense::click());
        let pos = match text_galley.job.halign {
            Align::LEFT => rect.left_top(),
            Align::Center => rect.center_top(),
            Align::RIGHT => rect.right_top(),
        };

        response.widget_info(|| {
            WidgetInfo::labeled(
                WidgetType::Button,
                self.hint.unwrap_or_else(|| text_galley.text()),
            )
        });

        if ui.is_rect_visible(response.rect) {
            let override_text_color = if !ui.is_enabled() {
                Some(GREY)
            } else if response.hovered() {
                Some(self.color_hovered)
            } else {
                Some(self.color)
            };

            ui.painter().add(TextShape {
                fallback_color: GREEN,
                pos,
                galley: text_galley,
                override_text_color,
                underline: Stroke::NONE,
                angle: 0.,
            });
        }

        if let Some(hint) = self.hint {
            response = response.on_hover_text(hint);
        }
        response = response.on_hover_cursor(CursorIcon::PointingHand);

        response
    }
}

pub struct ListItem {
    selected: bool,
    text: WidgetText,
    focusable: bool,
}

impl ListItem {
    pub fn new(selected: bool, text: impl Into<WidgetText>) -> Self {
        Self {
            selected,
            text: text.into(),
            focusable: true,
        }
    }
}

impl Widget for ListItem {
    fn ui(self, ui: &mut Ui) -> Response {
        let Self {
            selected,
            text,
            focusable,
        } = self;

        let width = ui.available_width();
        let padding = vec2(8., 2.);

        let text_galley =
            text.into_galley(ui, Some(true), width - padding.x * 2., TextStyle::Button);

        let desired_size = [width, text_galley.size().y + padding.y].into();
        let sense = Sense {
            click: true,
            drag: false,
            focusable,
        };
        let (rect, mut response) = ui.allocate_at_least(desired_size, sense);

        response.widget_info(|| {
            WidgetInfo::selected(WidgetType::SelectableLabel, selected, text_galley.text())
        });
        response = response.on_hover_cursor(CursorIcon::PointingHand);

        if ui.is_rect_visible(response.rect) {
            let pos = rect.shrink2(padding).left_top();

            if selected || response.hovered() || response.highlighted() || response.has_focus() {
                ui.painter()
                    .rect(rect, Rounding::ZERO, GREY_DARK, Stroke::NONE);
            }

            let override_text_color = if selected { Some(BLUE) } else { None };

            ui.painter().add(TextShape {
                fallback_color: GREEN,
                pos,
                galley: text_galley,
                override_text_color,
                underline: Stroke::NONE,
                angle: 0.,
            });
        }

        response
    }
}

// on_hover_text that doesn't cover things above or below, or the item itself, useful for comboboxes
pub fn on_hover_text_side(ui: UiRef, r: &Response, text: &str) {
    if !r.hovered() || text.is_empty() {
        return;
    }
    let description = shorten_string(text, 1000);
    let ctx = ui.ctx();
    let style = ui.style();
    let screen_rect = ctx.screen_rect();
    let right_space = ctx.screen_rect().right() - r.rect.right();
    let right = right_space > r.rect.left();
    let mut layout_job = WidgetText::RichText(color_text(&description, WHITE)).into_layout_job(
        ui.style(),
        FontSelection::Default,
        Align::Min,
    );
    layout_job.wrap.max_width = if right { right_space } else { r.rect.left() } - 30.;

    let galley = ui.fonts(|f| f.layout_job(layout_job));
    let size = galley.size() + style.spacing.menu_margin.sum();
    let y_offset = 'b: {
        let below = r.rect.top();
        if below + size.y < screen_rect.max.y {
            break 'b below;
        }
        let above = r.rect.bottom() - size.y;
        if above > 0. {
            break 'b above;
        }
        screen_rect.max.y / 2. - size.y / 2.
    };
    let x_offset = if right {
        r.rect.right() + 10.
    } else {
        r.rect.left() - 10. - size.x
    };

    let pos = screen_rect.left_top() + vec2(x_offset, y_offset);

    Area::new(ui.next_auto_id())
        .order(Order::Tooltip)
        .fixed_pos(pos)
        .interactable(false)
        .show(ctx, |ui| {
            Frame::popup(style)
                .stroke((2.0, GREEN_DARK))
                .show(ui, |ui| {
                    ui.set_max_width(ui.max_rect().width());
                    let pos = pos2(ui.max_rect().left(), ui.cursor().top());
                    for row in &galley.rows {
                        let rect = row.rect.translate(vec2(pos.x, pos.y));
                        ui.allocate_rect(rect, Sense::hover());
                    }
                    ui.painter().add(TextShape::new(pos, galley, GREEN));
                });
        });
}

#[cfg(test)]
mod tests {
    use super::*;
    use egui::accesskit::{Action, Role};

    #[test]
    fn keyboard_list_has_one_named_tab_stop_and_no_unknown_focus_nodes() {
        let ctx = egui::Context::default();
        ctx.options_mut(|options| options.screen_reader = true);
        ctx.enable_accesskit();
        let output = ctx.run(Default::default(), |ctx| {
            egui::CentralPanel::default().show(ctx, |ui| {
                let mut selected = 0;
                keyboard_list(
                    ui,
                    "test_list",
                    "Test list",
                    &["One".to_owned(), "Two".to_owned(), "Three".to_owned()],
                    &mut selected,
                );
            });
        });
        let update = output.platform_output.accesskit_update.unwrap();
        let mut combo_boxes = 0;
        let mut unknown_focus_nodes = 0;
        for (_, node) in update.nodes {
            if node.role() == Role::ComboBox {
                combo_boxes += 1;
                assert!(node.supports_action(Action::Focus));
                assert_eq!(node.name(), Some("Test list"));
                assert_eq!(node.value(), Some("One 1 of 3"));
            }
            if node.role() == Role::Unknown && node.supports_action(Action::Focus) {
                unknown_focus_nodes += 1;
            }
        }
        assert_eq!(combo_boxes, 1);
        assert_eq!(unknown_focus_nodes, 0);
    }

    fn key_event(key: Key) -> egui::Event {
        egui::Event::Key {
            key,
            physical_key: None,
            pressed: true,
            repeat: false,
            modifiers: Modifiers::NONE,
        }
    }

    #[test]
    fn keyboard_list_arrows_change_value_without_moving_focus() {
        let ctx = egui::Context::default();
        let control_id = Id::new("test_list").with("keyboard_list_control");
        ctx.memory_mut(|memory| memory.request_focus(control_id));
        let mut selected = 0;
        let options = ["One".to_owned(), "Two".to_owned(), "Three".to_owned()];

        for _ in 0..2 {
            let _ = ctx.run(Default::default(), |ctx| {
                egui::CentralPanel::default().show(ctx, |ui| {
                    keyboard_list(ui, "test_list", "Test list", &options, &mut selected);
                });
            });
        }

        let mut input = egui::RawInput::default();
        input.events.push(key_event(Key::ArrowDown));
        let _ = ctx.run(input, |ctx| {
            egui::CentralPanel::default().show(ctx, |ui| {
                keyboard_list(ui, "test_list", "Test list", &options, &mut selected);
                let _ = ui.button("Following control");
            });
        });

        assert_eq!(selected, 1);
        assert_eq!(ctx.memory(|memory| memory.focus()), Some(control_id));

        for key in [Key::ArrowLeft, Key::ArrowRight] {
            let mut input = egui::RawInput::default();
            input.events.push(key_event(key));
            let _ = ctx.run(input, |ctx| {
                egui::CentralPanel::default().show(ctx, |ui| {
                    keyboard_list(ui, "test_list", "Test list", &options, &mut selected);
                    let _ = ui.button("Following control");
                });
            });
            assert_eq!(selected, 1);
            assert_eq!(ctx.memory(|memory| memory.focus()), Some(control_id));
        }
    }

    #[test]
    fn keyboard_tab_list_wraps_and_keeps_focus() {
        let ctx = egui::Context::default();
        let base_id = Id::new("test_tabs").with("keyboard_tab_list");
        let control_id = base_id.with(0usize);
        ctx.memory_mut(|memory| memory.request_focus(control_id));
        let mut selected = 0;
        let options = [
            "General".to_owned(),
            "Characters".to_owned(),
            "Area".to_owned(),
        ];

        for _ in 0..2 {
            let _ = ctx.run(Default::default(), |ctx| {
                egui::CentralPanel::default().show(ctx, |ui| {
                    keyboard_tab_list(ui, "test_tabs", &options, &mut selected);
                });
            });
        }

        let mut input = egui::RawInput::default();
        input.events.push(key_event(Key::ArrowLeft));
        let _ = ctx.run(input, |ctx| {
            egui::CentralPanel::default().show(ctx, |ui| {
                keyboard_tab_list(ui, "test_tabs", &options, &mut selected);
                let _ = ui.button("Following control");
            });
        });

        assert_eq!(selected, 2);
        assert_eq!(
            ctx.memory(|memory| memory.focus()),
            Some(base_id.with(2usize))
        );
    }

    #[test]
    fn keyboard_tabs_have_tab_roles_and_down_does_not_escape() {
        let ctx = egui::Context::default();
        ctx.options_mut(|options| options.screen_reader = true);
        ctx.enable_accesskit();
        let base_id = Id::new("semantic_tabs").with("keyboard_tab_list");
        ctx.memory_mut(|memory| memory.request_focus(base_id.with(0usize)));
        let options = [
            "General".to_owned(),
            "Characters".to_owned(),
            "Area".to_owned(),
        ];
        let mut selected = 0;

        // Render once after focus arrives so egui installs this control's
        // directional focus lock before the key event, as it does on screen.
        let _ = ctx.run(Default::default(), |ctx| {
            egui::CentralPanel::default().show(ctx, |ui| {
                keyboard_tab_list(ui, "semantic_tabs", &options, &mut selected);
                let _ = ui.button("Following control");
            });
        });

        let mut input = egui::RawInput::default();
        input.events.push(key_event(Key::ArrowDown));
        let output = ctx.run(input, |ctx| {
            egui::CentralPanel::default().show(ctx, |ui| {
                keyboard_tab_list(ui, "semantic_tabs", &options, &mut selected);
                let _ = ui.button("Following control");
            });
        });

        let update = output.platform_output.accesskit_update.unwrap();
        let tabs: Vec<_> = update
            .nodes
            .iter()
            .filter(|(_, node)| node.role() == Role::Tab)
            .collect();
        assert_eq!(tabs.len(), 3);
        assert!(tabs
            .iter()
            .any(|(_, node)| node.name() == Some("General 1 of 3")));
        assert!(tabs.iter().any(|(_, node)| {
            node.name() == Some("General 1 of 3") && node.is_selected() == Some(true)
        }));
        assert_eq!(
            tabs.iter()
                .filter(|(_, node)| node.is_selected() == Some(true))
                .count(),
            1
        );
        assert_eq!(selected, 0);
        assert_eq!(
            ctx.memory(|memory| memory.focus()),
            Some(base_id.with(0usize))
        );
    }

    #[test]
    fn spin_control_exposes_no_slider() {
        let ctx = egui::Context::default();
        ctx.options_mut(|options| options.screen_reader = true);
        ctx.enable_accesskit();
        let output = ctx.run(Default::default(), |ctx| {
            egui::CentralPanel::default().show(ctx, |ui| {
                let mut value = 42i32;
                ui.s_spin(&mut value, 0..=100, false, "Experience");
            });
        });
        let update = output.platform_output.accesskit_update.unwrap();
        assert_eq!(
            update
                .nodes
                .iter()
                .filter(|(_, node)| node.role() == Role::SpinButton)
                .count(),
            1
        );
        assert_eq!(
            update
                .nodes
                .iter()
                .filter(|(_, node)| node.role() == Role::Slider)
                .count(),
            0
        );
    }

    #[test]
    fn spin_control_arrows_change_value_and_keep_focus() {
        let ctx = egui::Context::default();
        let mut value = 42i32;
        let mut control_id = None;
        let _ = ctx.run(Default::default(), |ctx| {
            egui::CentralPanel::default().show(ctx, |ui| {
                control_id = Some(ui.s_spin(&mut value, 0..=100, false, "Experience").id);
            });
        });
        let control_id = control_id.unwrap();
        ctx.memory_mut(|memory| memory.request_focus(control_id));

        let mut input = egui::RawInput::default();
        input.events.push(key_event(Key::ArrowUp));
        let _ = ctx.run(input, |ctx| {
            egui::CentralPanel::default().show(ctx, |ui| {
                ui.s_spin(&mut value, 0..=100, false, "Experience");
                let _ = ui.button("Following control");
            });
        });
        assert_eq!(value, 43);
        assert_eq!(ctx.memory(|memory| memory.focus()), Some(control_id));

        let mut input = egui::RawInput::default();
        input.events.push(key_event(Key::ArrowDown));
        let _ = ctx.run(input, |ctx| {
            egui::CentralPanel::default().show(ctx, |ui| {
                ui.s_spin(&mut value, 0..=100, false, "Experience");
            });
        });
        assert_eq!(value, 42);
        assert_eq!(ctx.memory(|memory| memory.focus()), Some(control_id));
    }
}
