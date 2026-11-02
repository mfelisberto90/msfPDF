import type { Currency, FontFace } from './types.ts';
import { MsfPdfError } from './types.ts';

const CP1252_SPECIAL_OCTAL: Record<string, string> = {
  '€': '\\200',
  '‚': '\\202',
  'ƒ': '\\203',
  '„': '\\204',
  '…': '\\205',
  '†': '\\206',
  '‡': '\\207',
  'ˆ': '\\210',
  '‰': '\\211',
  'Š': '\\212',
  '‹': '\\213',
  'Œ': '\\214',
  'Ž': '\\216',
  '‘': '\\221',
  '’': '\\222',
  '“': '\\223',
  '”': '\\224',
  '•': '\\225',
  '–': '\\226',
  '—': '\\227',
  '˜': '\\230',
  '™': '\\231',
  'š': '\\232',
  '›': '\\233',
  'œ': '\\234',
  'ž': '\\236',
  'Ÿ': '\\237',
};

/**
 * Encodes UTF-8 strings into 100% 7-bit ASCII PDF literals using `/WinAnsiEncoding` octal escapes.
 * Neutralizes PDF stream operator injection (`\`, `(`, `)`) and control characters.
 */
export function pdfEscapeWinAnsi(input: string): string {
  let out = '';
  for (const ch of input) {
    if (ch === '\\') {
      out += '\\\\';
    } else if (ch === '(') {
      out += '\\(';
    } else if (ch === ')') {
      out += '\\)';
    } else if (ch === '\r' || ch === '\n' || ch === '\t') {
      out += ' ';
    } else if (CP1252_SPECIAL_OCTAL[ch] !== undefined) {
      out += CP1252_SPECIAL_OCTAL[ch];
    } else {
      const code = ch.codePointAt(0) ?? 32;
      if (code < 0x20 || code === 0x7f) {
        out += ' ';
      } else if (code >= 0x20 && code <= 0x7e) {
        out += ch;
      } else if (code >= 0xa0 && code <= 0xff) {
        out += '\\' + code.toString(8).padStart(3, '0');
      } else if (ch === '✓' || ch === '✔') {
        out += '[x]';
      } else if (ch === '→') {
        out += '->';
      } else {
        const folded = ch
          .normalize('NFD')
          .replace(/[\u0300-\u036f]/g, '')
          .replace(/[^\x20-\x7E]/g, ' ');
        out += folded || ' ';
      }
    }
  }
  return out;
}

export function estimateFontWidthPt(text: string, font: FontFace, sizePt: number): number {
  if (font === 'mono') {
    return Array.from(text).length * 0.6 * sizePt;
  }

  let units = 0;
  for (const ch of text) {
    const code = ch.codePointAt(0) ?? 32;
    if (' .,:;!|\'‘’ilIíìîï'.includes(ch)) units += 0.278;
    else if (ch === 'j') units += 0.222;
    else if ('frt()[]-–'.includes(ch)) units += 0.333;
    else if ('"“”'.includes(ch)) units += 0.4;
    else if ('szcçJ'.includes(ch)) units += 0.5;
    else if ('L1'.includes(ch)) units += 0.556;
    else if ('mw'.includes(ch)) units += 0.833;
    else if ('MW%‰'.includes(ch)) units += 0.889;
    else if ('@—Œœ'.includes(ch)) units += 0.98;
    else if ('GOQÓÔÕÒ'.includes(ch)) units += 0.778;
    else if ('CDHNRUÇÑÚ'.includes(ch)) units += 0.722;
    else if ((code >= 65 && code <= 90) || (code >= 0xc0 && code <= 0xdd)) units += 0.667;
    else units += 0.556;
  }

  return units * sizePt * (font === 'bold' ? 1.045 : 1.0);
}

export function truncateToWidthPt(
  text: string,
  font: FontFace,
  sizePt: number,
  maxWidthPt: number
): string {
  const clean = (text || '').trim();
  if (estimateFontWidthPt(clean, font, sizePt) <= maxWidthPt) {
    return clean;
  }
  const ellipsisW = estimateFontWidthPt('…', font, sizePt);
  const target = Math.max(sizePt, maxWidthPt - ellipsisW);
  let acc = '';
  for (const ch of clean) {
    const candidate = acc + ch;
    if (estimateFontWidthPt(candidate, font, sizePt) > target) {
      break;
    }
    acc = candidate;
  }
  return acc.trimEnd() + '…';
}

export function wrapTextToWidthPt(
  text: string,
  font: FontFace,
  sizePt: number,
  maxWidthPt: number
): string[] {
  const bound = Math.max(sizePt * 3, maxWidthPt);
  const spaceW = estimateFontWidthPt(' ', font, sizePt);
  const lines: string[] = [];

  for (const rawPara of (text || '').split('\n')) {
    const trimmed = rawPara.trim();
    if (!trimmed) continue;

    let current = '';
    let currentW = 0;

    for (const word of trimmed.split(/\s+/)) {
      const wordW = estimateFontWidthPt(word, font, sizePt);
      if (wordW > bound) {
        if (current) {
          lines.push(current);
          current = '';
        }
        let chunk = '';
        let chunkW = 0;
        for (const ch of word) {
          const chW = estimateFontWidthPt(ch, font, sizePt);
          if (chunk && chunkW + chW > bound) {
            lines.push(chunk);
            chunk = '';
            chunkW = 0;
          }
          chunk += ch;
          chunkW += chW;
        }
        current = chunk;
        currentW = chunkW;
        continue;
      }

      if (!current) {
        current = word;
        currentW = wordW;
      } else if (currentW + spaceW + wordW <= bound) {
        current += ' ' + word;
        currentW += spaceW + wordW;
      } else {
        lines.push(current);
        current = word;
        currentW = wordW;
      }
    }

    if (current) lines.push(current);
  }

  return lines;
}

function groupDigits(digits: string, sep: string): string {
  let out = '';
  for (let i = 0; i < digits.length; i++) {
    if (i > 0 && (digits.length - i) % 3 === 0) {
      out += sep;
    }
    out += digits[i];
  }
  return out;
}

/**
 * Formats exact integer minor units (`pence`, `cents`, `centavos`, `yen`) without floating-point division.
 */
export function formatMoneyMinor(minorUnits: number | bigint, currency: Currency): string {
  if (typeof minorUnits === 'number' && !Number.isSafeInteger(minorUnits)) {
    throw new MsfPdfError(
      'INVALID_SPEC',
      'monetary minor units must be an exact safe integer (no binary floats)'
    );
  }
  const big = BigInt(minorUnits);
  const negative = big < 0n;
  const abs = negative ? -big : big;
  const sign = negative ? '-' : '';

  if (currency === 'JPY') {
    return `${sign}¥${groupDigits(abs.toString(10), ',')}`;
  }

  const major = (abs / 100n).toString(10);
  const minor = (abs % 100n).toString(10).padStart(2, '0');

  switch (currency) {
    case 'GBP':
      return `${sign}£${groupDigits(major, ',')}.${minor}`;
    case 'USD':
      return `${sign}$${groupDigits(major, ',')}.${minor}`;
    case 'EUR':
      return `${sign}€${groupDigits(major, ',')}.${minor}`;
    case 'BRL':
      return `${sign}R$ ${groupDigits(major, '.')},${minor}`;
  }
}
