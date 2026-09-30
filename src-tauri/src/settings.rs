use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum InteractionMode {
    Edit,
    Locked,
    Ghost,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct Theme {
    pub name: String,
    pub font_family: String,
    pub font_weight: u16,
    pub font_size: f64,
    pub line_height: f64,
    pub letter_spacing: f64,
    pub align: String,
    pub active_color: String,
    pub inactive_color: String,
    pub translation_color: String,
    pub gradient: bool,
    pub gradient_color: String,
    pub artwork_accent: bool,
    pub opacity: f64,
    pub shadow: f64,
    pub glow: f64,
    pub stroke: f64,
    pub stroke_color: String,
    pub background_opacity: f64,
    pub blur: f64,
    pub radius: f64,
    pub padding: f64,
    pub spacing: f64,
    pub layout: u8,
    pub show_translation: bool,
}
impl Default for Theme {
    fn default() -> Self {
        Self {
            name: "Glass Mint".into(),
            font_family: "Segoe UI".into(),
            font_weight: 600,
            font_size: 34.0,
            line_height: 1.4,
            letter_spacing: 0.4,
            align: "center".into(),
            active_color: "#c8f5e4".into(),
            inactive_color: "#879b97".into(),
            translation_color: "#a9bcb5".into(),
            gradient: false,
            gradient_color: "#92c9ed".into(),
            artwork_accent: false,
            opacity: 1.0,
            shadow: 8.0,
            glow: 0.0,
            stroke: 0.0,
            stroke_color: "#0b1714".into(),
            background_opacity: 0.12,
            blur: 0.0,
            radius: 22.0,
            padding: 24.0,
            spacing: 12.0,
            layout: 2,
            show_translation: true,
        }
    }
}
impl Theme {
    pub fn validate(&self) -> Result<(), String> {
        for (name, n, min, max) in [
            ("fontSize", self.font_size, 12., 120.),
            ("lineHeight", self.line_height, 1., 2.5),
            ("letterSpacing", self.letter_spacing, -2., 12.),
            ("opacity", self.opacity, 0.05, 1.),
            ("shadow", self.shadow, 0., 40.),
            ("glow", self.glow, 0., 40.),
            ("stroke", self.stroke, 0., 4.),
            ("backgroundOpacity", self.background_opacity, 0., 1.),
            ("blur", self.blur, 0., 60.),
            ("radius", self.radius, 0., 80.),
            ("padding", self.padding, 0., 100.),
            ("spacing", self.spacing, 0., 60.),
        ] {
            if !n.is_finite() || n < min || n > max {
                return Err(format!("Invalid {name}: expected {min}–{max}"));
            }
        }
        for color in [
            &self.active_color,
            &self.inactive_color,
            &self.translation_color,
            &self.gradient_color,
            &self.stroke_color,
        ] {
            if color.len() != 7
                || !color.starts_with('#')
                || !color[1..].chars().all(|c| c.is_ascii_hexdigit())
            {
                return Err("颜色必须是 #RRGGBB".into());
            }
        }
        if !["left", "center", "right"].contains(&self.align.as_str())
            || !(1..=3).contains(&self.layout)
            || !(100..=900).contains(&self.font_weight)
        {
            return Err("Invalid typography".into());
        }
        if self.font_family.is_empty() || self.font_family.len() > 150 || self.name.len() > 80 {
            return Err("Invalid font or theme name".into());
        }
        Ok(())
    }
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct Geometry {
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
}
impl Default for Geometry {
    fn default() -> Self {
        Self {
            x: 240,
            y: 650,
            width: 860,
            height: 230,
        }
    }
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(default, rename_all = "camelCase")]
pub struct Shortcuts {
    pub toggle: String,
    pub earlier: String,
    pub later: String,
}
impl Default for Shortcuts {
    fn default() -> Self {
        Self {
            toggle: "Ctrl+Alt+Shift+L".into(),
            earlier: "Ctrl+Alt+Shift+ArrowLeft".into(),
            later: "Ctrl+Alt+Shift+ArrowRight".into(),
        }
    }
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct Settings {
    pub version: u32,
    pub theme: Theme,
    pub mode: InteractionMode,
    pub overlay_visible: bool,
    pub preferred_player: String,
    pub global_offset_ms: i64,
    pub shortcuts: Shortcuts,
    pub geometry: Geometry,
}
impl Default for Settings {
    fn default() -> Self {
        Self {
            version: 1,
            theme: Theme::default(),
            mode: InteractionMode::Edit,
            overlay_visible: true,
            preferred_player: String::new(),
            global_offset_ms: 0,
            shortcuts: Shortcuts::default(),
            geometry: Geometry::default(),
        }
    }
}
impl Settings {
    pub fn validate(&self) -> Result<(), String> {
        if self.version != 1 {
            return Err("Unsupported settings version".into());
        }
        self.theme.validate()?;
        if !(-600000..=600000).contains(&self.global_offset_ms) {
            return Err("Offset outside ±10 minutes".into());
        }
        if self.geometry.width < 300
            || self.geometry.width > 16000
            || self.geometry.height < 100
            || self.geometry.height > 16000
        {
            return Err("Invalid overlay dimensions".into());
        }
        Ok(())
    }
}

/// Write then replace; preserve the previous file as a recovery copy on Windows.
pub fn write_json(path: &Path, value: &impl Serialize) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    let tmp = path.with_extension("tmp");
    let backup = path.with_extension("bak");
    let bytes = serde_json::to_vec_pretty(value).map_err(|e| e.to_string())?;
    {
        use std::io::Write;
        let mut f = std::fs::File::create(&tmp).map_err(|e| e.to_string())?;
        f.write_all(&bytes).map_err(|e| e.to_string())?;
        f.sync_all().map_err(|e| e.to_string())?;
    }
    if path.exists() {
        std::fs::copy(path, &backup).map_err(|e| e.to_string())?;
    }
    std::fs::rename(&tmp, path).map_err(|e| e.to_string())
}
pub fn load(path: &Path) -> Settings {
    for candidate in [path.to_path_buf(), path.with_extension("bak")] {
        if let Ok(bytes) = std::fs::read(&candidate) {
            if let Ok(mut settings) = serde_json::from_slice::<Settings>(&bytes) {
                if settings.validate().is_ok() {
                    // Migrate only the prototype's exact default triplet, which
                    // conflicts with NetEase. Leave custom bindings untouched.
                    if settings.shortcuts
                        == (Shortcuts {
                            toggle: "Ctrl+Alt+L".into(),
                            earlier: "Ctrl+Alt+ArrowLeft".into(),
                            later: "Ctrl+Alt+ArrowRight".into(),
                        })
                    {
                        settings.shortcuts = Shortcuts::default();
                    }
                    return settings;
                }
            }
        }
    }
    Settings::default()
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn settings_roundtrip_and_recovery() {
        let dir = tempfile::tempdir().unwrap();
        let p = dir.path().join("settings.json");
        let s = Settings {
            global_offset_ms: -150,
            ..Settings::default()
        };
        write_json(&p, &s).unwrap();
        write_json(&p, &s).unwrap();
        assert_eq!(load(&p).global_offset_ms, -150);
        std::fs::write(&p, b"broken").unwrap();
        assert_eq!(load(&p).global_offset_ms, -150);
    }
    #[test]
    fn rejects_invalid_theme_and_accepts_missing_defaults() {
        let t: Theme = serde_json::from_str("{\"name\":\"custom\"}").unwrap();
        assert!(t.validate().is_ok());
        assert!(Theme { opacity: 9., ..t }.validate().is_err());
    }
    #[test]
    fn migrate_prototype_hotkeys_without_overwriting_custom_bindings() {
        let d = tempfile::tempdir().unwrap();
        let p = d.path().join("settings.json");
        let mut s = Settings {
            shortcuts: Shortcuts {
                toggle: "Ctrl+Alt+L".into(),
                earlier: "Ctrl+Alt+ArrowLeft".into(),
                later: "Ctrl+Alt+ArrowRight".into(),
            },
            ..Settings::default()
        };
        write_json(&p, &s).unwrap();
        assert_eq!(load(&p).shortcuts, Shortcuts::default());
        s.shortcuts.toggle = "Ctrl+Shift+F10".into();
        write_json(&p, &s).unwrap();
        assert_eq!(load(&p).shortcuts, s.shortcuts);
    }
}
