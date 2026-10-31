//! GTK styles for Rail chrome, with one fixed monochrome design system.

use crate::*;

pub(crate) const CSS: &str = include_str!("style.css");
pub(crate) const THEME_CSS: &str = include_str!("theme.css");

const MONOCHROME_TOKENS: &str = r#"
@define-color rail_canvas #05080d;
@define-color rail_sidebar #101925;
@define-color rail_surface #0d1520;
@define-color rail_surface_raised #142031;
@define-color rail_hover #1b2b40;
@define-color rail_selected #22364f;
@define-color rail_line #28394e;
@define-color rail_line_strong #405672;
@define-color rail_text #f3f6fb;
@define-color rail_muted #a0adbf;
@define-color rail_subtle #718096;
@define-color rail_accent #83aeea;
@define-color rail_accent_contrast #07101d;
@define-color rail_accent_soft rgba(131, 174, 234, 0.12);
@define-color rail_selection rgba(131, 174, 234, 0.24);
@define-color rail_focus rgba(131, 174, 234, 0.62);
@define-color rail_shadow rgba(0, 0, 0, 0.82);
"#;

/// Install Rail’s graphite-and-navy surfaces with a restrained blue interaction accent.
pub(crate) fn install_styles(provider: &gtk::CssProvider) {
    provider.load_from_string(&format!("{CSS}\n{MONOCHROME_TOKENS}\n{THEME_CSS}"));
    if let Some(settings) = gtk::Settings::default() {
        settings.set_gtk_application_prefer_dark_theme(true);
    }
}
