pub const DEFAULT_THEME: &str = "default";

const SUPPORTED_THEMES: [&str; 6] = ["default", "blue", "green", "purple", "orange", "dark"];

#[derive(Debug, Clone, Copy)]
pub struct ThemePalette {
    pub window_bg: &'static str,
    pub surface: &'static str,
    pub surface_alt: &'static str,
    pub border: &'static str,
    pub text_primary: &'static str,
    pub text_secondary: &'static str,
    pub text_muted: &'static str,
    pub accent: &'static str,
    pub success: &'static str,
    pub warning: &'static str,
    pub danger: &'static str,
}

pub fn normalize_theme(value: &str) -> String {
    let trimmed = value.trim();
    if matches!(trimmed, "system" | "light" | "high-contrast") {
        return DEFAULT_THEME.to_string();
    }
    if SUPPORTED_THEMES.contains(&trimmed) {
        trimmed.to_string()
    } else {
        DEFAULT_THEME.to_string()
    }
}

pub fn palette(theme: &str) -> ThemePalette {
    match normalize_theme(theme).as_str() {
        "blue" => ThemePalette {
            window_bg: "#F6F8FB",
            surface: "#FFFFFF",
            surface_alt: "#EFF6FF",
            border: "#D7DEE8",
            text_primary: "#172033",
            text_secondary: "#42526B",
            text_muted: "#6B778C",
            accent: "#1D4ED8",
            success: "#0F8A5F",
            warning: "#B7791F",
            danger: "#C2413A",
        },
        "green" => ThemePalette {
            window_bg: "#F6F8F7",
            surface: "#FFFFFF",
            surface_alt: "#ECFDF5",
            border: "#D7E3DC",
            text_primary: "#172033",
            text_secondary: "#42526B",
            text_muted: "#6B778C",
            accent: "#047857",
            success: "#0F8A5F",
            warning: "#B7791F",
            danger: "#C2413A",
        },
        "purple" => ThemePalette {
            window_bg: "#F8F7FB",
            surface: "#FFFFFF",
            surface_alt: "#F5F3FF",
            border: "#DDD6FE",
            text_primary: "#172033",
            text_secondary: "#42526B",
            text_muted: "#6B778C",
            accent: "#7C3AED",
            success: "#0F8A5F",
            warning: "#B7791F",
            danger: "#C2413A",
        },
        "orange" => ThemePalette {
            window_bg: "#FBF8F5",
            surface: "#FFFFFF",
            surface_alt: "#FFF7ED",
            border: "#FED7AA",
            text_primary: "#172033",
            text_secondary: "#42526B",
            text_muted: "#6B778C",
            accent: "#EA580C",
            success: "#0F8A5F",
            warning: "#B7791F",
            danger: "#C2413A",
        },
        "dark" => ThemePalette {
            window_bg: "#111827",
            surface: "#182235",
            surface_alt: "#202B3F",
            border: "#334155",
            text_primary: "#F8FAFC",
            text_secondary: "#CBD5E1",
            text_muted: "#94A3B8",
            accent: "#60A5FA",
            success: "#34D399",
            warning: "#FBBF24",
            danger: "#F87171",
        },
        _ => ThemePalette {
            window_bg: "#F6F8FB",
            surface: "#FFFFFF",
            surface_alt: "#F1F5F9",
            border: "#D7DEE8",
            text_primary: "#172033",
            text_secondary: "#42526B",
            text_muted: "#6B778C",
            accent: "#2563EB",
            success: "#0F8A5F",
            warning: "#B7791F",
            danger: "#C2413A",
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn invalid_theme_falls_back_to_default() {
        assert_eq!(normalize_theme("unknown"), "default");
        assert_eq!(palette("unknown").window_bg, "#F6F8FB");
    }

    #[test]
    fn every_theme_has_hex_colors() {
        for theme in SUPPORTED_THEMES {
            let palette = palette(theme);
            for color in [
                palette.window_bg,
                palette.surface,
                palette.surface_alt,
                palette.border,
                palette.text_primary,
                palette.text_secondary,
                palette.text_muted,
                palette.accent,
                palette.success,
                palette.warning,
                palette.danger,
            ] {
                assert!(color.starts_with('#'));
                assert_eq!(color.len(), 7);
            }
        }
    }
}
