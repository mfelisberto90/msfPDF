use serde::{Deserialize, Serialize};

use crate::error::PdfError;

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum PageFormat {
    #[default]
    A4,
    Letter,
    Thermal80mm,
}

impl PageFormat {
    pub fn dimensions_pt(self) -> (f64, f64) {
        match self {
            Self::A4 => (595.28, 841.89),
            Self::Letter => (612.00, 792.00),
            Self::Thermal80mm => (226.77, 510.00),
        }
    }

    pub fn margin_x_pt(self) -> f64 {
        match self {
            Self::A4 | Self::Letter => 40.0,
            Self::Thermal80mm => 14.0,
        }
    }

    pub fn footer_top_pt(self) -> f64 {
        let (_, h) = self.dimensions_pt();
        match self {
            Self::A4 | Self::Letter => h - 44.0,
            Self::Thermal80mm => h - 32.0,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum FontFace {
    #[default]
    Regular,
    Bold,
    Mono,
}

impl FontFace {
    pub fn pdf_alias(self) -> &'static str {
        match self {
            Self::Regular => "F1",
            Self::Bold => "F2",
            Self::Mono => "F3",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Rgb(pub f64, pub f64, pub f64);

impl Rgb {
    pub const fn new(r: f64, g: f64, b: f64) -> Self {
        Self(r, g, b)
    }

    pub fn from_hex(hex: &str) -> Result<Self, PdfError> {
        let clean = hex.trim().strip_prefix('#').unwrap_or(hex.trim());
        if clean.len() != 6 || !clean.bytes().all(|b| b.is_ascii_hexdigit()) {
            return Err(PdfError::InvalidSpec(
                "colour override must be a 6-digit #RRGGBB hex string".to_string(),
            ));
        }
        let r = u8::from_str_radix(&clean[0..2], 16).unwrap_or(0) as f64 / 255.0;
        let g = u8::from_str_radix(&clean[2..4], 16).unwrap_or(0) as f64 / 255.0;
        let b = u8::from_str_radix(&clean[4..6], 16).unwrap_or(0) as f64 / 255.0;
        Ok(Self(r, g, b))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum ThemePreset {
    #[default]
    NavyFintech,
    EmeraldEditorial,
    SlateMinimal,
    CrimsonCompliance,
}

#[derive(Debug, Clone, Copy)]
pub struct ThemePalette {
    pub header_bg: Rgb,
    pub badge_bg: Rgb,
    pub badge_border: Rgb,
    pub accent: Rgb,
    pub accent_bright: Rgb,
    pub header_mist: Rgb,
    pub white: Rgb,
    pub card_bg: Rgb,
    pub table_head_bg: Rgb,
    pub callout_bg: Rgb,
    pub warning_bg: Rgb,
    pub warning_accent: Rgb,
    pub danger_bg: Rgb,
    pub danger_accent: Rgb,
    pub border: Rgb,
    pub signature_line: Rgb,
    pub ink_strong: Rgb,
    pub ink_body: Rgb,
    pub ink_muted: Rgb,
}

impl ThemePalette {
    pub fn from_preset(preset: ThemePreset) -> Self {
        let common_white = Rgb::new(1.0, 1.0, 1.0);
        let warning_bg = Rgb::new(0.996, 0.976, 0.914);
        let warning_accent = Rgb::new(0.769, 0.459, 0.047);
        let danger_bg = Rgb::new(0.992, 0.933, 0.933);
        let danger_accent = Rgb::new(0.745, 0.141, 0.141);

        match preset {
            ThemePreset::NavyFintech => Self {
                header_bg: Rgb::new(0.055, 0.114, 0.212),
                badge_bg: Rgb::new(0.086, 0.169, 0.302),
                badge_border: Rgb::new(0.180, 0.310, 0.510),
                accent: Rgb::new(0.114, 0.388, 0.784),
                accent_bright: Rgb::new(0.576, 0.773, 0.992),
                header_mist: Rgb::new(0.824, 0.890, 0.973),
                white: common_white,
                card_bg: Rgb::new(0.973, 0.980, 0.992),
                table_head_bg: Rgb::new(0.933, 0.953, 0.980),
                callout_bg: Rgb::new(0.929, 0.961, 0.996),
                warning_bg,
                warning_accent,
                danger_bg,
                danger_accent,
                border: Rgb::new(0.824, 0.859, 0.910),
                signature_line: Rgb::new(0.580, 0.635, 0.718),
                ink_strong: Rgb::new(0.059, 0.090, 0.165),
                ink_body: Rgb::new(0.157, 0.212, 0.306),
                ink_muted: Rgb::new(0.369, 0.435, 0.541),
            },
            ThemePreset::EmeraldEditorial => Self {
                header_bg: Rgb::new(0.059, 0.141, 0.122),
                badge_bg: Rgb::new(0.086, 0.200, 0.173),
                badge_border: Rgb::new(0.165, 0.361, 0.302),
                accent: Rgb::new(0.122, 0.478, 0.329),
                accent_bright: Rgb::new(0.525, 0.937, 0.675),
                header_mist: Rgb::new(0.820, 0.925, 0.886),
                white: common_white,
                card_bg: Rgb::new(0.973, 0.980, 0.976),
                table_head_bg: Rgb::new(0.933, 0.953, 0.941),
                callout_bg: Rgb::new(0.925, 0.965, 0.945),
                warning_bg,
                warning_accent,
                danger_bg,
                danger_accent,
                border: Rgb::new(0.820, 0.863, 0.843),
                signature_line: Rgb::new(0.580, 0.643, 0.616),
                ink_strong: Rgb::new(0.059, 0.122, 0.110),
                ink_body: Rgb::new(0.149, 0.220, 0.200),
                ink_muted: Rgb::new(0.353, 0.443, 0.412),
            },
            ThemePreset::SlateMinimal => Self {
                header_bg: Rgb::new(0.094, 0.094, 0.106),
                badge_bg: Rgb::new(0.153, 0.153, 0.173),
                badge_border: Rgb::new(0.247, 0.247, 0.275),
                accent: Rgb::new(0.220, 0.235, 0.278),
                accent_bright: Rgb::new(0.831, 0.831, 0.847),
                header_mist: Rgb::new(0.863, 0.863, 0.878),
                white: common_white,
                card_bg: Rgb::new(0.976, 0.976, 0.980),
                table_head_bg: Rgb::new(0.945, 0.945, 0.953),
                callout_bg: Rgb::new(0.949, 0.953, 0.961),
                warning_bg,
                warning_accent,
                danger_bg,
                danger_accent,
                border: Rgb::new(0.843, 0.847, 0.863),
                signature_line: Rgb::new(0.620, 0.627, 0.659),
                ink_strong: Rgb::new(0.071, 0.075, 0.090),
                ink_body: Rgb::new(0.196, 0.204, 0.231),
                ink_muted: Rgb::new(0.427, 0.435, 0.475),
            },
            ThemePreset::CrimsonCompliance => Self {
                header_bg: Rgb::new(0.220, 0.063, 0.094),
                badge_bg: Rgb::new(0.310, 0.094, 0.137),
                badge_border: Rgb::new(0.478, 0.169, 0.227),
                accent: Rgb::new(0.698, 0.133, 0.204),
                accent_bright: Rgb::new(0.988, 0.690, 0.725),
                header_mist: Rgb::new(0.965, 0.855, 0.871),
                white: common_white,
                card_bg: Rgb::new(0.984, 0.973, 0.976),
                table_head_bg: Rgb::new(0.961, 0.933, 0.937),
                callout_bg: Rgb::new(0.984, 0.937, 0.945),
                warning_bg,
                warning_accent,
                danger_bg,
                danger_accent,
                border: Rgb::new(0.882, 0.827, 0.835),
                signature_line: Rgb::new(0.667, 0.580, 0.596),
                ink_strong: Rgb::new(0.141, 0.055, 0.075),
                ink_body: Rgb::new(0.259, 0.157, 0.180),
                ink_muted: Rgb::new(0.478, 0.369, 0.392),
            },
        }
    }
}
