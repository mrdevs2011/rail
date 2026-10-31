//! GTK styles for Rail chrome, with one fixed monochrome design system.

use crate::*;

pub(crate) const CSS: &str = include_str!("style.css");
pub(crate) const THEME_CSS: &str = include_str!("theme.css");

const MONOCHROME_TOKENS: &str = r#"
@define-color rail_canvas #000000;
@define-color rail_sidebar #000000;
@define-color rail_surface #080808;
@define-color rail_surface_raised #101010;
@define-color rail_hover #181818;
@define-color rail_selected #202020;
@define-color rail_line #242424;
@define-color rail_line_strong #3a3a3a;
@define-color rail_text #f5f5f5;
@define-color rail_muted #a1a1a1;
@define-color rail_subtle #717171;
@define-color rail_accent #f5f5f5;
@define-color rail_accent_contrast #080808;
@define-color rail_accent_soft rgba(245, 245, 245, 0.10);
@define-color rail_selection rgba(245, 245, 245, 0.22);
@define-color rail_focus rgba(245, 245, 245, 0.58);
@define-color rail_shadow rgba(0, 0, 0, 0.78);
"#;

/// Install the fixed black / white visual language across all native Rail surfaces.
pub(crate) fn install_styles(provider: &gtk::CssProvider) {
    provider.load_from_string(&format!("{CSS}\n{MONOCHROME_TOKENS}\n{THEME_CSS}"));
    if let Some(settings) = gtk::Settings::default() {
        settings.set_gtk_application_prefer_dark_theme(true);
    }
}
