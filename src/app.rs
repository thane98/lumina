use std::sync::Arc;

use egui::{FontData, FontDefinitions, FontFamily};

use crate::MainState;

pub struct App {
    state: MainState,
}

impl App {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        let mut font_definitions = FontDefinitions::default();
        font_definitions.font_data.insert(
            "inter".to_owned(),
            Arc::new(FontData::from_static(include_bytes!(
                "../assets/Inter-Medium.ttf"
            ))),
        );
        font_definitions.font_data.insert(
            "monoid".to_owned(),
            Arc::new(FontData::from_static(include_bytes!(
                "../assets/Monoid-Regular.ttf"
            ))),
        );
        font_definitions
            .families
            .get_mut(&FontFamily::Proportional)
            .unwrap()
            .insert(0, "inter".into());
        font_definitions
            .families
            .get_mut(&FontFamily::Monospace)
            .unwrap()
            .insert(0, "monoid".into());
        cc.egui_ctx.set_fonts(font_definitions);

        let mut style = (*cc.egui_ctx.style()).clone();
        style.text_styles.insert(
            egui::TextStyle::Body,
            egui::FontId::new(14.0, egui::FontFamily::Proportional),
        );
        style.text_styles.insert(
            egui::TextStyle::Monospace,
            egui::FontId::new(12.0, egui::FontFamily::Monospace),
        );
        style.text_styles.insert(
            egui::TextStyle::Button,
            egui::FontId::new(18.0, egui::FontFamily::Proportional),
        );
        cc.egui_ctx.set_style(style);

        catppuccin_egui::set_theme(&cc.egui_ctx, catppuccin_egui::MOCHA);

        Self {
            state: MainState::default(),
        }
    }
}

impl eframe::App for App {
    fn update(&mut self, ctx: &egui::Context, _: &mut eframe::Frame) {
        self.state.show(ctx);
    }
}
