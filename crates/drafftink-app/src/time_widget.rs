//! Time typography with fixed digit cells, independent of proportional advances.
use egui::{Color32, FontDefinitions, FontFamily, FontId, Pos2, Rect, Sense, Stroke, Ui, Vec2};

pub fn install_time_fonts(fonts: &mut FontDefinitions) {
    for (name, data) in [
        ("Qurso Time Inter", include_bytes!("../assets/Inter-Regular.ttf").as_slice()),
        ("Qurso Time Noto", include_bytes!("../../drafftink-render/assets/NotoSans-Regular.ttf").as_slice()),
        ("Qurso Time GelPen", include_bytes!("../../drafftink-render/assets/GelPen.ttf").as_slice()),
    ] {
        fonts.font_data.insert(name.into(), egui::FontData::from_static(data).into());
        let mut family = vec![name.to_string()];
        family.extend(fonts.families.get(&FontFamily::Monospace).cloned().unwrap_or_default());
        fonts.families.insert(FontFamily::Name(name.into()), family);
    }
}

fn time_font(font: u8, size: f32) -> FontId {
    let family = match font {
        1 => FontFamily::Name("Qurso Time Inter".into()),
        2 => FontFamily::Name("Qurso Time Noto".into()),
        3 => FontFamily::Name("Qurso Time GelPen".into()),
        _ => FontFamily::Monospace,
    };
    FontId::new(size, family)
}

pub fn draw_time_digits(ui: &mut Ui, text: &str, font: u8, size: f32, color: Color32) -> Vec<Rect> {
    let font_id = time_font(font, size);
    let digit_width = ('0'..='9').map(|c| ui.painter().layout_no_wrap(c.to_string(), font_id.clone(), color).size().x).fold(0.0, f32::max);
    let colon_width = ui.painter().layout_no_wrap(":".into(), font_id.clone(), color).size().x;
    let width: f32 = text.chars().map(|c| if c.is_ascii_digit() { digit_width } else { colon_width }).sum();
    let base_height=ui.painter().layout_no_wrap("0".into(),font_id.clone(),color).size().y;
    let scale = (ui.available_width() / width.max(1.0)).min(ui.available_height()/base_height.max(1.0)).min(1.0).max(0.1);
    let font_id = time_font(font, size * scale);
    let glyphs: Vec<_> = text.chars().map(|c| (c, ui.painter().layout_no_wrap(c.to_string(), font_id.clone(), color))).collect();
    let height = glyphs.iter().map(|(_, g)| g.size().y).fold(0.0, f32::max);
    let available=ui.available_rect_before_wrap();
    let rect=Rect::from_center_size(available.center(),Vec2::new(width*scale,height));
    ui.allocate_rect(available,Sense::hover());
    let mut x = rect.left();
    glyphs.into_iter().map(|(c, galley)| {
        let cell_width = if c.is_ascii_digit() { digit_width } else { colon_width } * scale;
        let cell = Rect::from_min_size(Pos2::new(x, rect.top()), Vec2::new(cell_width, height));
        let origin = Pos2::new(cell.center().x - galley.size().x * 0.5, cell.center().y - galley.size().y * 0.5);
        ui.painter().galley(origin, galley, color);
        x += cell_width;
        cell
    }).collect()
}

pub fn white_color_picker(ui: &mut Ui) {
    let accent = ui.visuals().selection.bg_fill;
    *ui.visuals_mut() = egui::Visuals::light();
    let visuals = ui.visuals_mut();
    visuals.selection.bg_fill = accent;
    visuals.override_text_color = Some(Color32::from_gray(45));
    visuals.extreme_bg_color = Color32::WHITE;
    visuals.widgets.inactive.bg_fill = Color32::WHITE;
    visuals.widgets.inactive.weak_bg_fill = Color32::WHITE;
    visuals.widgets.inactive.bg_stroke = Stroke::new(1.0, Color32::from_gray(205));
    visuals.widgets.hovered.bg_fill = Color32::from_gray(240);
    visuals.widgets.active.bg_fill = Color32::from_gray(230);
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn proportional_fonts_have_stable_digit_positions() {
        let ctx = egui::Context::default();
        let mut fonts = FontDefinitions::default();
        install_time_fonts(&mut fonts);
        ctx.set_fonts(fonts);
        for font in 0..=3 {
            let mut first = Vec::new();
            let mut second = Vec::new();
            let _ = ctx.run(egui::RawInput::default(), |ctx| {
                egui::CentralPanel::default().show(ctx, |ui| first = draw_time_digits(ui,"11:11:11",font,27.0,Color32::BLACK));
            });
            let _ = ctx.run(egui::RawInput::default(), |ctx| {
                egui::CentralPanel::default().show(ctx, |ui| second = draw_time_digits(ui,"88:88:88",font,27.0,Color32::BLACK));
            });
            assert_eq!(first, second, "Time columns must remain fixed for font {font}");
        }
    }
    #[test]
    fn picker_controls_are_white_and_timer_refresh_matches_visible_precision() {
        let ctx=egui::Context::default();
        let _=ctx.run(egui::RawInput::default(),|ctx| {
            egui::CentralPanel::default().show(ctx,|ui| {
                white_color_picker(ui);
                assert_eq!(ui.visuals().widgets.inactive.bg_fill,Color32::WHITE);
                assert_eq!(ui.visuals().extreme_bg_color,Color32::WHITE);
            });
        });
        assert!(clock_repaint_delay(10,false)>std::time::Duration::from_secs(49));
        assert!(clock_repaint_delay(10,true)<=std::time::Duration::from_secs(1));
    }
}

pub fn clock_repaint_delay(second: u8, seconds_visible: bool) -> std::time::Duration {
    #[cfg(target_arch="wasm32")]
    let subsecond=js_sys::Date::now().rem_euclid(1000.0) as u64;
    #[cfg(not(target_arch="wasm32"))]
    let subsecond=std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap_or_default().subsec_millis() as u64;
    let whole=if seconds_visible {1000} else {(60-second.min(59) as u64)*1000};
    std::time::Duration::from_millis(whole.saturating_sub(subsecond).max(10))
}
