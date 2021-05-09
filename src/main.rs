use bevy::{
    math::{
        curves::{Curve, CurveCursor, CurveVariable, TangentControl},
        interpolation::{utils::lerp_unclamped, Interpolation},
    },
    prelude::*,
};
use bevy_egui::{egui, EguiContext, EguiPlugin};

#[derive(Debug, PartialEq, Eq)]
enum TangentEdit {
    No,
    In,
    Out,
}

struct CurveEditor {
    dragging: bool,
    selected_keyframe: usize,
    display_offset: Vec2,
    display_range: Vec2,
    curve: CurveVariable<f32>,
    tangent_popup_position: egui::Pos2,
    tangent_drag: TangentEdit,
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
            tangent_popup_position: (0.0, 0.0).into(),
            tangent_drag: TangentEdit::No,
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
fn to_tangent(v: egui::Vec2) -> f32 {
    let v = v.normalized();
    v.y / v.x.abs().max(1e-12).copysign(v.x)
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
    let rect = egui::Rect {
        min: position - offset,
        max: position + offset,
    };

    if rect.contains(pointer_position) {
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

            let curve = &mut curve_editor.curve;

            // Painter and style
            let color = egui::Color32::RED;
            let stroke = egui::Stroke::new(1.0, color);

            // Curve display range
            let min = curve_editor.display_offset;
            let max = min + curve_editor.display_range;
            let duration = curve_editor.display_range.x.max(0.0);

            // Context menu to change tangents
            {
                let popup_id = id.with("popup");

                if let Some(pos) = response.hover_pos() {
                    if response.secondary_clicked() && !ui.memory().is_popup_open(popup_id) {
                        // // Relative to curve
                        // pos.x = remap(rect.min.x, rect.max.x, pos.x, min.x, max.x);
                        // pos.y = remap(rect.max.y, rect.min.y, pos.y, min.y, max.y);
                        curve_editor.tangent_popup_position = pos;
                        ui.memory().open_popup(popup_id);
                    }
                }

                let temp = response.rect;
                response.rect = egui::Rect::from_min_size(
                    curve_editor.tangent_popup_position,
                    (150.0, 1.0).into(),
                );

                let index = curve_editor
                    .selected_keyframe
                    .min(CurveCursor::MAX as usize) as CurveCursor;

                egui::popup::popup_below_widget(ui, popup_id, &response, |ui| {
                    let selected = (index as usize) < curve.len();
                    ui.set_enabled(selected);
                    let (lerp_mode, tangent_mode) = if selected {
                        (
                            curve.get_interpolation(index),
                            curve.get_tangent_control(index),
                        )
                    } else {
                        (Interpolation::Hermite, Default::default())
                    };

                    if ui
                        .selectable_label(lerp_mode == Interpolation::Step, "Step")
                        .clicked()
                    {
                        curve.set_interpolation(index, Interpolation::Step);
                    }

                    if ui
                        .selectable_label(lerp_mode == Interpolation::Linear, "Linear")
                        .clicked()
                    {
                        curve.set_interpolation(index, Interpolation::Linear);
                    }

                    ui.separator();

                    let hermite = lerp_mode == Interpolation::Hermite;
                    if ui
                        .selectable_label(hermite && tangent_mode == TangentControl::Auto, "Auto")
                        .clicked()
                    {
                        curve.set_interpolation(index, Interpolation::Hermite);
                        curve.set_tangent_control(index, TangentControl::Auto);
                    }
                    if ui
                        .selectable_label(hermite && tangent_mode == TangentControl::Free, "Free")
                        .clicked()
                    {
                        curve.set_interpolation(index, Interpolation::Hermite);
                        curve.set_tangent_control(index, TangentControl::Free);
                    }
                    if ui
                        .selectable_label(hermite && tangent_mode == TangentControl::Flat, "Flat")
                        .clicked()
                    {
                        curve.set_interpolation(index, Interpolation::Hermite);
                        curve.set_tangent_control(index, TangentControl::Flat);
                    }
                    if ui
                        .selectable_label(
                            hermite && tangent_mode == TangentControl::Broken,
                            "Broken",
                        )
                        .clicked()
                    {
                        curve.set_interpolation(index, Interpolation::Hermite);
                        curve.set_tangent_control(index, TangentControl::Broken);
                    }
                });
                response.rect = temp;
            }

            // Curve rendering
            let mut t0 = min.x;
            let (mut cursor, mut v0) = curve.sample_with_cursor(0, t0);
            for i in 1..256 {
                let t1 = (duration * i as f32 / 255.0) + min.x;
                let (next_cursor, v1) = curve.sample_with_cursor(cursor, t1);

                let x0 = remap(min.x, max.x, t0, rect.min.x, rect.max.x);
                let x1 = remap(min.x, max.x, t1, rect.min.x, rect.max.x);

                let y0 = remap(min.y, max.y, v0, rect.max.y, rect.min.y);
                let y1 = remap(min.y, max.y, v1, rect.max.y, rect.min.y);

                ui.painter()
                    .line_segment([egui::Pos2::new(x0, y0), egui::Pos2::new(x1, y1)], stroke);

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

            // Insert keyframe
            {
                let t = remap(rect.min.x, rect.max.x, pointer_position.x, min.x, max.x);
                let v = curve.sample(t);

                let position = egui::Pos2 {
                    x: pointer_position.x,
                    y: remap(min.y, max.y, v, rect.max.y, rect.min.y),
                };

                ui.painter()
                    .circle_filled(position, 2.0, egui::Color32::GRAY);

                if ui.input().key_pressed(egui::Key::I) {
                    curve_editor.selected_keyframe = curve
                        .insert()
                        .set_time(t)
                        .set_value(v)
                        .set_mode(Interpolation::Hermite)
                        .done()
                        .map_or(usize::MAX, |i| i as usize);

                    println!("{:?}", &curve);
                }
            }

            // Delete selected keyframe
            {
                if curve_editor.selected_keyframe != usize::MAX
                    && ui.input().key_pressed(egui::Key::D)
                {
                    curve.remove(curve_editor.selected_keyframe as CurveCursor);
                    curve_editor.selected_keyframe = usize::MAX;
                }
            }

            // Render keyframes
            for i in 0..curve.len() {
                let t = curve.get_time(i as CurveCursor);
                let v = *curve.get_value(i as CurveCursor);

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
                        curve.set_value(i as CurveCursor, v);
                    }

                    if (delta.x).abs() > 0.5 {
                        let t = remap(rect.min.x, rect.max.x, pointer_position.x, min.x, max.x);
                        let k = curve.set_time(i as CurveCursor, t);

                        if let Some(k) = k {
                            // Keyframe ordering changed
                            curve_editor.selected_keyframe = k as usize;
                            continue;
                        }
                    }
                }

                if selected {
                    // Display tangents when selected
                    let index = i as CurveCursor;
                    let lerp_mode = curve.get_interpolation(index);

                    if lerp_mode == Interpolation::Hermite {
                        let (a, b) = curve.get_in_out_tangent(index);

                        // In tangent
                        let a = egui::Vec2::new(t, v) - to_dir(a);
                        let a = egui::Pos2::new(
                            remap(min.x, max.x, a.x, rect.min.x, rect.max.x),
                            remap(min.y, max.y, a.y, rect.max.y, rect.min.y),
                        );
                        let a = position + (a - position).normalized() * 50.0;
                        ui.painter().line_segment([position, a], tangent_stroke);

                        // Out tangent
                        let b = egui::Vec2::new(t, v) + to_dir(b);
                        let b = egui::Pos2::new(
                            remap(min.x, max.x, b.x, rect.min.x, rect.max.x),
                            remap(min.y, max.y, b.y, rect.max.y, rect.min.y),
                        );
                        let b = position + (b - position).normalized() * 50.0;
                        ui.painter().line_segment([position, b], tangent_stroke);

                        let tangent_mode = curve.get_tangent_control(index);
                        if tangent_mode != TangentControl::Auto
                            && tangent_mode != TangentControl::Flat
                        {
                            let selected = curve_editor.tangent_drag == TangentEdit::In;
                            let (select, _) = dot(
                                ui.painter(),
                                pointer_position,
                                pointer_down,
                                selected,
                                a,
                                1.5,
                                6.0,
                                egui::Color32::GRAY,
                            );
                            if select && pointer_down {
                                curve_editor.tangent_drag = TangentEdit::In;

                                let p = egui::Pos2::new(
                                    remap(rect.min.x, rect.max.x, pointer_position.x, min.x, max.x),
                                    remap(rect.max.y, rect.min.y, pointer_position.y, min.y, max.y),
                                );
                                let tangent =
                                    to_tangent(p - egui::Pos2::new(t, v)).clamp(-1e5, 1e5);

                                if tangent_mode == TangentControl::Broken {
                                    curve.set_in_tangent(index, tangent);
                                } else {
                                    curve.set_in_out_tangent(index, tangent);
                                }
                            } else if selected {
                                curve_editor.tangent_drag = TangentEdit::No;
                            }

                            let selected = curve_editor.tangent_drag == TangentEdit::Out;
                            let (select, _) = dot(
                                ui.painter(),
                                pointer_position,
                                pointer_down,
                                selected,
                                b,
                                1.5,
                                6.0,
                                egui::Color32::GRAY,
                            );
                            if select && pointer_down {
                                curve_editor.tangent_drag = TangentEdit::Out;

                                let p = egui::Pos2::new(
                                    remap(rect.min.x, rect.max.x, pointer_position.x, min.x, max.x),
                                    remap(rect.max.y, rect.min.y, pointer_position.y, min.y, max.y),
                                );
                                let tangent =
                                    to_tangent(p - egui::Pos2::new(t, v)).clamp(-1e5, 1e5);

                                if tangent_mode == TangentControl::Broken {
                                    curve.set_out_tangent(index, tangent);
                                } else {
                                    curve.set_in_out_tangent(index, tangent);
                                }
                            } else if selected {
                                curve_editor.tangent_drag = TangentEdit::No;
                            }
                        }

                        // TODO: Edit tangents ...
                    }
                }

                // Keyframe dot
                let (select, press) = dot(
                    ui.painter(),
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
