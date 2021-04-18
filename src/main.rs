use bevy::{
    math::{
        curves::{Curve, CurveVariable},
        interpolation::utils::lerp_unclamped,
    },
    prelude::*,
};
use bevy_egui::{egui, EguiContext, EguiPlugin};

struct CurveEditor {
    display_offset: Vec2,
    display_range: Vec2,
    curve: CurveVariable<f32>,
}

fn main() {
    App::build()
        .insert_resource(CurveEditor {
            display_offset: Vec2::new(0.0, -0.5),
            display_range: Vec2::new(2.0, 3.5),
            curve: CurveVariable::with_auto_tangents(
                vec![0.0, 1.0, 1.3, 1.6, 1.7, 1.8, 1.9, 2.0],
                vec![3.0, 0.0, 1.0, 0.0, 0.5, 0.0, 0.25, 0.0],
            )
            .unwrap(),
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

fn ui_example(mut curve_editor: ResMut<CurveEditor>, egui_context: Res<EguiContext>) {
    let curve_editor = &mut *curve_editor;
    egui::Window::new("Hello")
        .default_size([700.0, 300.0])
        .show(egui_context.ctx(), |ui| {
            let (id, rect) = ui.allocate_space(ui.available_size());

            // Input handling
            let response = ui.interact(rect, id, egui::Sense::click_and_drag());
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

            // Painter and style
            let painter = ui.painter();
            let stroke = egui::Stroke::new(1.0, egui::Color32::RED);

            // Curve display range
            let min = curve_editor.display_offset;
            let max = min + curve_editor.display_range;
            let duration = curve_editor.display_range.x.max(0.0);

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
        });
}
