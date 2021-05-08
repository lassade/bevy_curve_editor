use bevy::{
    math::{
        curves::{Curve, CurveCursor, CurveVariable, TangentControl},
        interpolation::{utils::lerp_unclamped, Interpolation},
    },
    prelude::*,
};
use bevy_egui::{egui, EguiContext, EguiPlugin};

struct CurveEditor {
    dragging: bool,
    selected_keyframe: usize,
    display_offset: Vec2,
    display_range: Vec2,
    curve: CurveVariable<f32>,
    popup: egui::Pos2,
}

fn main() {
    App::build()
        .insert_resource(CurveEditor {
            dragging: false,
            selected_keyframe: usize::MAX,
            display_offset: Vec2::new(0.0, -0.5),
            display_range: Vec2::new(2.0, 3.5),
            curve: CurveVariable::with_auto_tangents(
                vec![0.0, 1.0, 1.3, 1.6, 1.7, 1.8, 1.9, 2.0],
                vec![3.0, 0.0, 1.0, 0.0, 0.5, 0.0, 0.25, 0.0],
            )
            .unwrap(),
            popup: (0.0, 0.0).into(),
        })
        .add_plugins(DefaultPlugins)
        .add_plugin(EguiPlugin)
        .add_system(ui_example.system())
        .run();
}

#[inline]
fn remap(min: f32, max: f32, t: f32, out_min: f32, out_max: f32) -> f32 {
    let n = (t - min) / (max - min);
    lerp_unclamped(out_min, out_max, n)
}

#[inline]
fn to_dir(a: f32) -> egui::Vec2 {
    // TODO: There's something wrong with this tangent generation
    let (y, x) = a.atan().sin_cos();
    (x, y).into()
}

#[inline]
fn to_tangent(y: f32, x: f32) -> f32 {
    (-y).atan2(x)
}

#[inline]
fn dot(
    painter: &egui::Painter,
    pointer_position: egui::Pos2,
    pointer_down: bool,
    selected: bool,
    position: egui::Pos2,
    radius: f32,
    select_radius: f32,
    color: egui::Color32,
) -> (bool, bool) {
    let offset = (select_radius, select_radius).into();
    let keyframe_region = egui::Rect {
        min: position - offset,
        max: position + offset,
    };

    if keyframe_region.contains(pointer_position) {
        // Hovered
        painter.circle_filled(position, radius * 1.2, egui::Color32::WHITE);
        // Select on mouse down
        (selected || pointer_down, pointer_down)
    } else if selected {
        // Selected
        painter.circle_filled(position, radius, egui::Color32::YELLOW);
        (true, false)
    } else {
        // Default
        painter.circle_filled(position, radius, color);
        (false, false)
    }
}

fn ui_example(mut curve_editor: ResMut<CurveEditor>, egui_context: Res<EguiContext>) {
    let curve_editor = &mut *curve_editor;
    egui::Window::new("Curve Editor")
        .default_size([700.0, 300.0])
        .show(egui_context.ctx(), |ui| {
            let (id, rect) = ui.allocate_space(ui.available_size());

            // Input handling
            let mut response = ui.interact(rect, id, egui::Sense::click_and_drag());
            if response.dragged_by(egui::PointerButton::Middle) {
                // Pan
                let range = curve_editor.display_range;
                let delta = response.drag_delta();
                let size = rect.size();
                let dx = remap(0.0, -size.x, delta.x, 0.0, range.x);
                let dy = remap(0.0, size.y, delta.y, 0.0, range.y);

                curve_editor.display_offset.x += dx;
                curve_editor.display_offset.y += dy;
            } else if let Some(mut pos) = response.hover_pos() {
                // Zoom
                let delta = response.ctx.input().scroll_delta.y;
                let size = rect.size();

                // Window relative
                pos.x -= response.rect.left();
                pos.y = response.rect.size().y - (pos.y - response.rect.top());

                if response.ctx.input().modifiers.command {
                    // Zoom Y (when holding ctrl)
                    let n = 1.0 / -size.y;
                    let dy = delta * n * curve_editor.display_range.y;
                    let y = pos.y * n;
                    let y0 = y * curve_editor.display_range.y;

                    curve_editor.display_range.y += dy;

                    let y1 = y * curve_editor.display_range.y;
                    curve_editor.display_offset.y += y1 - y0;
                } else {
                    // Zoom X
                    let n = 1.0 / -size.x;
                    let dx = delta * n * curve_editor.display_range.x;
                    let x = pos.x * n;
                    let x0 = x * curve_editor.display_range.x;

                    curve_editor.display_range.x += dx;

                    let x1 = x * curve_editor.display_range.x;
                    curve_editor.display_offset.x += x1 - x0;
                }
            }

            let popup_id = id.with("popup");

            if let Some(pos) = response.hover_pos() {
                if response.secondary_clicked() && !ui.memory().is_popup_open(popup_id) {
                    curve_editor.popup = pos;
                    ui.memory().open_popup(popup_id);
                }
            }

            let temp = response.rect;
            response.rect = egui::Rect::from_min_size(curve_editor.popup, (150.0, 1.0).into());
            egui::popup::popup_below_widget(ui, popup_id, &response, |ui| {
                let selected = curve_editor.selected_keyframe < curve_editor.curve.len();
                ui.set_enabled(selected);

                let index = curve_editor
                    .selected_keyframe
                    .min(CurveCursor::MAX as usize) as CurveCursor;

                let (lerp_mode, tangent_mode) = if selected {
                    (
                        curve_editor.curve.get_interpolation(index),
                        curve_editor.curve.get_tangent_control(index),
                    )
                } else {
                    (Interpolation::Hermite, Default::default())
                };

                if ui
                    .selectable_label(lerp_mode == Interpolation::Step, "Step")
                    .clicked()
                {
                    curve_editor
                        .curve
                        .set_interpolation(index, Interpolation::Step);
                }

                if ui
                    .selectable_label(lerp_mode == Interpolation::Linear, "Linear")
                    .clicked()
                {
                    curve_editor
                        .curve
                        .set_interpolation(index, Interpolation::Linear);
                }

                ui.separator();

                let hermite = lerp_mode == Interpolation::Hermite;
                if ui
                    .selectable_label(hermite && tangent_mode == TangentControl::Auto, "Auto")
                    .clicked()
                {
                    curve_editor
                        .curve
                        .set_interpolation(index, Interpolation::Hermite);
                    curve_editor
                        .curve
                        .set_tangent_control(index, TangentControl::Auto);
                }
                if ui
                    .selectable_label(hermite && tangent_mode == TangentControl::Free, "Free")
                    .clicked()
                {
                    curve_editor
                        .curve
                        .set_interpolation(index, Interpolation::Hermite);
                    curve_editor
                        .curve
                        .set_tangent_control(index, TangentControl::Free);
                }
                if ui
                    .selectable_label(hermite && tangent_mode == TangentControl::Flat, "Flat")
                    .clicked()
                {
                    curve_editor
                        .curve
                        .set_interpolation(index, Interpolation::Hermite);
                    curve_editor
                        .curve
                        .set_tangent_control(index, TangentControl::Flat);
                }
                if ui
                    .selectable_label(hermite && tangent_mode == TangentControl::Broken, "Broken")
                    .clicked()
                {
                    curve_editor
                        .curve
                        .set_interpolation(index, Interpolation::Hermite);
                    curve_editor
                        .curve
                        .set_tangent_control(index, TangentControl::Broken);
                }
            });
            response.rect = temp;

            // Painter and style
            let painter = ui.painter();
            let color = egui::Color32::RED;
            let stroke = egui::Stroke::new(1.0, color);

            // Curve display range
            let min = curve_editor.display_offset;
            let max = min + curve_editor.display_range;
            let duration = curve_editor.display_range.x.max(0.0);

            // Curve rendering
            let mut t0 = min.x;
            let (mut cursor, mut v0) = curve_editor.curve.sample_with_cursor(0, t0);
            for i in 1..256 {
                let t1 = (duration * i as f32 / 255.0) + min.x;
                let (next_cursor, v1) = curve_editor.curve.sample_with_cursor(cursor, t1);

                let x0 = remap(min.x, max.x, t0, rect.min.x, rect.max.x);
                let x1 = remap(min.x, max.x, t1, rect.min.x, rect.max.x);

                let y0 = remap(min.y, max.y, v0, rect.max.y, rect.min.y);
                let y1 = remap(min.y, max.y, v1, rect.max.y, rect.min.y);

                painter.line_segment([egui::Pos2::new(x0, y0), egui::Pos2::new(x1, y1)], stroke);

                v0 = v1;
                t0 = t1;
                cursor = next_cursor;
            }

            // Curve keyframes
            // Appearance
            let tangent_stroke = egui::Stroke::new(1.0, egui::Color32::GRAY);

            // Pointer state
            let pointer_position = response.hover_pos().unwrap_or((-1.0, -1.0).into());
            let pointer_down = response
                .ctx
                .input()
                .pointer
                .button_down(egui::PointerButton::Primary);

            if !pointer_down {
                curve_editor.dragging = false;
            }

            // Render keyframes
            for i in 0..curve_editor.curve.len() {
                let t = curve_editor.curve.get_time(i as CurveCursor);
                let v = *curve_editor.curve.get_value(i as CurveCursor);

                let position = egui::Pos2::new(
                    remap(min.x, max.x, t, rect.min.x, rect.max.x),
                    remap(min.y, max.y, v, rect.max.y, rect.min.y),
                );

                if !rect.contains(position) {
                    continue;
                }

                let selected = i == curve_editor.selected_keyframe;

                if selected && curve_editor.dragging {
                    let delta = position - pointer_position;
                    if (delta.y).abs() > 0.5 {
                        let v = remap(rect.max.y, rect.min.y, pointer_position.y, min.y, max.y);
                        curve_editor.curve.set_value(i as CurveCursor, v);
                    }

                    if (delta.x).abs() > 0.5 {
                        let t = remap(rect.min.x, rect.max.x, pointer_position.x, min.x, max.x);
                        let k = curve_editor.curve.set_time(i as CurveCursor, t);

                        if let Some(k) = k {
                            // Keyframe ordering changed
                            curve_editor.selected_keyframe = k as usize;
                            continue;
                        }
                    }
                }

                if selected {
                    // Display tangents when selected
                    let (a, b) = curve_editor.curve.get_in_out_tangent(i as CurveCursor);

                    // In tangent
                    let a = egui::Vec2::new(t, v) - to_dir(a);
                    let a = egui::Pos2::new(
                        remap(min.x, max.x, a.x, rect.min.x, rect.max.x),
                        remap(min.y, max.y, a.y, rect.max.y, rect.min.y),
                    );
                    let a = position + (a - position).normalized() * 50.0;
                    painter.line_segment([position, a], tangent_stroke);
                    dot(
                        painter,
                        pointer_position,
                        pointer_down,
                        false,
                        a,
                        1.5,
                        6.0,
                        egui::Color32::GRAY,
                    );

                    // Out tangent
                    let b = egui::Vec2::new(t, v) + to_dir(b);
                    let b = egui::Pos2::new(
                        remap(min.x, max.x, b.x, rect.min.x, rect.max.x),
                        remap(min.y, max.y, b.y, rect.max.y, rect.min.y),
                    );
                    let b = position + (b - position).normalized() * 50.0;
                    painter.line_segment([position, b], tangent_stroke);
                    dot(
                        painter,
                        pointer_position,
                        pointer_down,
                        false,
                        b,
                        1.5,
                        6.0,
                        egui::Color32::GRAY,
                    );

                    // TODO: Edit tangents ...
                }

                // Keyframe dot
                let (select, press) = dot(
                    painter,
                    pointer_position,
                    pointer_down,
                    selected,
                    position,
                    2.5,
                    6.0,
                    color,
                );
                if select {
                    curve_editor.selected_keyframe = i;
                    curve_editor.dragging |= press;
                } else if selected {
                    // Deselect
                    curve_editor.selected_keyframe = usize::MAX;
                }
            }
        });
}
