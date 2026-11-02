use serde::{Deserialize, Serialize};

use crate::geometry::{PageFormat, ThemePreset};
use crate::money::Currency;

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct SecurityLimits {
    pub max_input_bytes: usize,
    pub max_pages: usize,
    pub max_blocks: usize,
    pub max_table_rows: usize,
}

impl Default for SecurityLimits {
    fn default() -> Self {
        Self {
            max_input_bytes: 262_144, // 256 KiB JSON ceiling
            max_pages: 50,
            max_blocks: 500,
            max_table_rows: 1_000,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DocumentSpec {
    #[serde(default)]
    pub page_format: PageFormat,
    #[serde(default)]
    pub theme: ThemePreset,
    #[serde(default)]
    pub accent_hex_override: Option<String>,
    pub metadata: DocumentMetadata,
    #[serde(default)]
    pub continuation_header: Option<ContinuationHeaderSpec>,
    pub blocks: Vec<DocumentBlock>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DocumentMetadata {
    pub issuer_name: String,
    pub document_code: String,
    pub issued_at_label: String,
    #[serde(default)]
    pub footer_notice: Option<String>,
    /// Supports `{current}` and `{total}` placeholders (default: `"Page {current} of {total}"`).
    #[serde(default = "default_page_counter_template")]
    pub page_counter_template: String,
}

fn default_page_counter_template() -> String {
    "Page {current} of {total}".to_string()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContinuationHeaderSpec {
    pub title: String,
    pub subtitle: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum TextAlign {
    #[default]
    Left,
    Center,
    Right,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum BannerTone {
    #[default]
    Accent,
    Warning,
    Danger,
    Neutral,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BentoCardItem {
    pub primary_label: String,
    pub primary_value: String,
    #[serde(default)]
    pub secondary_label: Option<String>,
    #[serde(default)]
    pub secondary_value: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TableColumnSpec {
    pub header: String,
    #[serde(default = "default_col_weight")]
    pub weight: f64,
    #[serde(default)]
    pub align: TextAlign,
    #[serde(default)]
    pub bold_values: bool,
}

fn default_col_weight() -> f64 {
    1.0
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FinancialLineItem {
    pub label: String,
    #[serde(default)]
    pub amount_minor: Option<i64>,
    #[serde(default)]
    pub currency: Option<Currency>,
    #[serde(default)]
    pub text_value: Option<String>,
    #[serde(default)]
    pub emphasize: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SignerSpec {
    pub role_label: String,
    pub party_name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KeyValueRow {
    pub label: String,
    pub value: String,
    #[serde(default)]
    pub emphasize: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum DocumentBlock {
    HeroHeader {
        kicker: String,
        title: String,
        subtitle: String,
        #[serde(default)]
        badge_label: Option<String>,
        #[serde(default)]
        badge_code: Option<String>,
        #[serde(default)]
        badge_meta: Option<String>,
        #[serde(default)]
        logo_jpeg_base64: Option<String>,
    },
    SectionHeading {
        title: String,
    },
    BentoGrid {
        #[serde(default = "default_bento_cols")]
        columns: u8,
        cards: Vec<BentoCardItem>,
    },
    DataTable {
        columns: Vec<TableColumnSpec>,
        rows: Vec<Vec<String>>,
        #[serde(default = "default_true")]
        zebra: bool,
    },
    FinancialSummary {
        currency: Currency,
        rows: Vec<FinancialLineItem>,
    },
    CalloutBanner {
        title: String,
        subtitle: String,
        #[serde(default)]
        right_value: Option<String>,
        #[serde(default)]
        tone: BannerTone,
    },
    ClauseList {
        paragraphs: Vec<String>,
    },
    QrVerification {
        payload: String,
        headline: String,
        details: String,
        #[serde(default)]
        code_display: Option<String>,
    },
    SignatureGrid {
        #[serde(default)]
        heading: Option<String>,
        signers: Vec<SignerSpec>,
    },
    KeyValueList {
        rows: Vec<KeyValueRow>,
        #[serde(default)]
        dashed_dividers: bool,
    },
    Divider,
    Spacer {
        height_pt: f64,
    },
}

fn default_bento_cols() -> u8 {
    2
}

fn default_true() -> bool {
    true
}
