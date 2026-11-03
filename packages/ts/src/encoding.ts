import { lookupVectorGlyph, VECTOR_UPM } from './glyphsData.ts';
import { hasArabicOrRtl, shapeArabicLogical } from './shaping.ts';
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
      } else if (ch === '✗' || ch === '✘') {
        out += '[ ]';
      } else if (ch === '→') {
        out += '->';
      } else if (ch === '←') {
        out += '<-';
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

function afmWinAnsiUnit(ch: string, code: number): number {
  if (' .,:;!|/\'\‘’ilIíìîï'.includes(ch)) return 0.278;
  if (ch === 'j') return 0.222;
  if ('frt()[]-–'.includes(ch)) return 0.333;
  if (ch === '•') return 0.35;
  if ('"“”'.includes(ch)) return 0.4;
  if ('szcçJ'.includes(ch)) return 0.5;
  if ('L1'.includes(ch)) return 0.556;
  if ('mw'.includes(ch)) return 0.833;
  if ('MW%‰'.includes(ch)) return 0.889;
  if ('@—Œœ'.includes(ch)) return 0.98;
  if ('GOQÓÔÕÒ'.includes(ch)) return 0.778;
  if ('CDHNRUÇÑÚ'.includes(ch)) return 0.722;
  if ((code >= 65 && code <= 90) || (code >= 0xc0 && code <= 0xdd)) return 0.667;
  return 0.556;
}

export function estimateFontWidthPt(text: string, font: FontFace, sizePt: number): number {
  const effective = hasArabicOrRtl(text) ? shapeArabicLogical(text) : text;
  let winAnsiUnits = 0;
  let winAnsiCount = 0;
  let vectorFunits = 0;

  for (const ch of effective) {
    const code = ch.codePointAt(0) ?? 32;
    const rec = lookupVectorGlyph(code);
    if (rec !== undefined) {
      vectorFunits += font === 'bold' ? rec.boldAdv : rec.regularAdv;
    } else {
      winAnsiCount += 1;
      if (font !== 'mono') {
        winAnsiUnits += afmWinAnsiUnit(ch, code);
      }
    }
  }

  const winPt =
    font === 'mono'
      ? winAnsiCount * 0.6 * sizePt
      : winAnsiUnits * sizePt * (font === 'bold' ? 1.045 : 1.0);

  return vectorFunits === 0 ? winPt : winPt + (vectorFunits / VECTOR_UPM) * sizePt;
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
 * Formats exact integer minor units (`pence`, `cents`, `centavos`, `kopecks`, `fils`, `halalas`, `yen`)
 * without floating-point division.
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
    case 'RUB':
      return `${sign}${groupDigits(major, ' ')},${minor} ₽`;
    case 'AED':
      return `${sign}AED ${groupDigits(major, ',')}.${minor}`;
    case 'SAR':
      return `${sign}SAR ${groupDigits(major, ',')}.${minor}`;
  }
}
