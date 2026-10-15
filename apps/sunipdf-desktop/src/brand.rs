//! Identité visuelle : symbole (page et ruban) et logo complet, clair ou sombre, chargés une fois
//! depuis les PNG embarqués (`assets/logo`, `assets/icon`).

use crate::app::SuniApp;

pub struct Brand {
    pub symbol: egui::TextureHandle,
    pub wordmark_light: egui::TextureHandle,
    pub wordmark_dark: egui::TextureHandle,
}

fn texture(ctx: &egui::Context, name: &str, png: &[u8]) -> Option<egui::TextureHandle> {
    let img = image::load_from_memory(png).ok()?;
    let rgba = img.to_rgba8();
    let size = [rgba.width() as usize, rgba.height() as usize];
    Some(ctx.load_texture(
        name,
        egui::ColorImage::from_rgba_unmultiplied(size, rgba.as_raw()),
        egui::TextureOptions::LINEAR,
    ))
}

impl Brand {
    fn load(ctx: &egui::Context) -> Option<Self> {
        Some(Self {
            symbol: texture(
                ctx,
                "brand-symbol",
                include_bytes!("../../../assets/icon/sunipdf-128.png"),
            )?,
            wordmark_light: texture(
                ctx,
                "brand-wordmark-light",
                include_bytes!("../../../assets/logo/sunipdf-wordmark-light.png"),
            )?,
            wordmark_dark: texture(
                ctx,
                "brand-wordmark-dark",
                include_bytes!("../../../assets/logo/sunipdf-wordmark-dark.png"),
            )?,
        })
    }

    pub fn wordmark(&self, dark: bool) -> &egui::TextureHandle {
        if dark {
            &self.wordmark_dark
        } else {
            &self.wordmark_light
        }
    }
}

impl SuniApp {
    /// Textures de marque, chargées au premier usage.
    pub fn brand(&mut self, ctx: &egui::Context) -> Option<&Brand> {
        if self.brand.is_none() {
            self.brand = Brand::load(ctx);
        }
        self.brand.as_ref()
    }

    /// Symbole carré (barre d'onglets, en-têtes).
    pub fn brand_symbol(&mut self, ctx: &egui::Context, size: f32) -> Option<egui::Image<'static>> {
        let tex = self.brand(ctx)?.symbol.clone();
        Some(egui::Image::new(&tex).fit_to_exact_size(egui::vec2(size, size)))
    }

    /// Logo complet adapté au thème (accueil, À propos), de hauteur `height`.
    pub fn brand_wordmark(
        &mut self,
        ctx: &egui::Context,
        height: f32,
    ) -> Option<egui::Image<'static>> {
        let dark = self.settings.dark_theme;
        let tex = self.brand(ctx)?.wordmark(dark).clone();
        let [w, h] = tex.size();
        let width = height * w as f32 / h.max(1) as f32;
        Some(egui::Image::new(&tex).fit_to_exact_size(egui::vec2(width, height)))
    }
}
