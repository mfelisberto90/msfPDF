export type PageFormat = 'a4' | 'letter' | 'thermal80mm';
export type ThemePreset =
  | 'navy_fintech'
  | 'emerald_editorial'
  | 'slate_minimal'
  | 'crimson_compliance';
export type FontFace = 'regular' | 'bold' | 'mono';
export type Currency = 'GBP' | 'EUR' | 'USD' | 'BRL' | 'JPY' | 'RUB' | 'AED' | 'SAR';
export type TextAlign = 'left' | 'center' | 'right';
export type BannerTone = 'accent' | 'warning' | 'danger' | 'neutral';

export type Rgb = readonly [number, number, number];

export interface SecurityLimits {
  readonly maxInputBytes: number;
  readonly maxPages: number;
  readonly maxBlocks: number;
  readonly maxTableRows: number;
}

export const DEFAULT_SECURITY_LIMITS: SecurityLimits = {
  maxInputBytes: 262_144,
  maxPages: 50,
  maxBlocks: 500,
  maxTableRows: 1_000,
};

export interface DocumentMetadata {
  issuer_name: string;
  document_code: string;
  issued_at_label: string;
  footer_notice?: string;
  page_counter_template?: string;
}

export interface ContinuationHeaderSpec {
  title: string;
  subtitle: string;
}

export interface BentoCardItem {
  primary_label: string;
  primary_value: string;
  secondary_label?: string;
  secondary_value?: string;
}

export interface TableColumnSpec {
  header: string;
  weight?: number;
  align?: TextAlign;
  bold_values?: boolean;
}

export interface FinancialLineItem {
  label: string;
  amount_minor?: number;
  currency?: Currency;
  text_value?: string;
  emphasize?: boolean;
}

export interface SignerSpec {
  role_label: string;
  party_name: string;
}

export interface KeyValueRow {
  label: string;
  value: string;
  emphasize?: boolean;
}

export type DocumentBlock =
  | {
      type: 'hero_header';
      kicker: string;
      title: string;
      subtitle: string;
      badge_label?: string;
      badge_code?: string;
      badge_meta?: string;
      logo_jpeg_base64?: string;
    }
  | {
      type: 'section_heading';
      title: string;
    }
  | {
      type: 'bento_grid';
      columns?: number;
      cards: BentoCardItem[];
    }
  | {
      type: 'data_table';
      columns: TableColumnSpec[];
      rows: string[][];
      zebra?: boolean;
    }
  | {
      type: 'financial_summary';
      currency: Currency;
      rows: FinancialLineItem[];
    }
  | {
      type: 'callout_banner';
      title: string;
      subtitle: string;
      right_value?: string;
      tone?: BannerTone;
    }
  | {
      type: 'clause_list';
      paragraphs: string[];
    }
  | {
      type: 'qr_verification';
      payload: string;
      headline: string;
      details: string;
      code_display?: string;
    }
  | {
      type: 'signature_grid';
      heading?: string;
      signers: SignerSpec[];
    }
  | {
      type: 'key_value_list';
      rows: KeyValueRow[];
      dashed_dividers?: boolean;
    }
  | {
      type: 'divider';
    }
  | {
      type: 'spacer';
      height_pt: number;
    };

export interface DocumentSpec {
  page_format?: PageFormat;
  theme?: ThemePreset;
  accent_hex_override?: string;
  metadata: DocumentMetadata;
  continuation_header?: ContinuationHeaderSpec;
  blocks: DocumentBlock[];
}

export class MsfPdfError extends Error {
  readonly code:
    | 'LIMIT_EXCEEDED'
    | 'INVALID_SPEC'
    | 'INVALID_IMAGE'
    | 'QR_CAPACITY_EXCEEDED'
    | 'VAULT_SECURITY_VIOLATION';

  constructor(
    code:
      | 'LIMIT_EXCEEDED'
      | 'INVALID_SPEC'
      | 'INVALID_IMAGE'
      | 'QR_CAPACITY_EXCEEDED'
      | 'VAULT_SECURITY_VIOLATION',
    message: string
  ) {
    super(message);
    this.name = 'MsfPdfError';
    this.code = code;
  }
}
