import {
  estimateFontWidthPt,
  formatMoneyMinor,
  pdfEscapeWinAnsi,
  truncateToWidthPt,
  wrapTextToWidthPt,
} from './encoding.ts';
import {
  lookupVectorGlyph,
  VECTOR_FONT_BBOX,
  VECTOR_FONT_MATRIX,
  VECTOR_UPM,
} from './glyphsData.ts';
import { encodeQr } from './qr.ts';
import type { QrMatrix } from './qr.ts';
import { isRtlDominant, shapeAndReorderLine } from './shaping.ts';
import type {
  BannerTone,
  DocumentSpec,
  FontFace,
  PageFormat,
  Rgb,
  SecurityLimits,
  TextAlign,
  ThemePreset,
} from './types.ts';
import { DEFAULT_SECURITY_LIMITS, MsfPdfError } from './types.ts';

interface ThemePalette {
  headerBg: Rgb;
  badgeBg: Rgb;
  badgeBorder: Rgb;
  accent: Rgb;
  accentBright: Rgb;
  headerMist: Rgb;
  white: Rgb;
  cardBg: Rgb;
  tableHeadBg: Rgb;
  calloutBg: Rgb;
  warningBg: Rgb;
  warningAccent: Rgb;
  dangerBg: Rgb;
  dangerAccent: Rgb;
  border: Rgb;
  signatureLine: Rgb;
  inkStrong: Rgb;
  inkBody: Rgb;
  inkMuted: Rgb;
}

function pageDimensionsPt(format: PageFormat): readonly [number, number] {
  switch (format) {
    case 'letter':
      return [612.0, 792.0];
    case 'thermal80mm':
      return [226.77, 510.0];
    default:
      return [595.28, 841.89];
  }
}

function marginXPt(format: PageFormat): number {
  return format === 'thermal80mm' ? 14.0 : 40.0;
}

function footerTopPt(format: PageFormat): number {
  const [, h] = pageDimensionsPt(format);
  return format === 'thermal80mm' ? h - 32.0 : h - 44.0;
}

function fontAlias(font: FontFace): 'F1' | 'F2' | 'F3' {
  if (font === 'bold') return 'F2';
  if (font === 'mono') return 'F3';
  return 'F1';
}

function parseHexRgb(hex: string): Rgb {
  const clean = hex.trim().replace(/^#/, '');
  if (!/^[0-9a-fA-F]{6}$/.test(clean)) {
    throw new MsfPdfError(
      'INVALID_SPEC',
      'accent_hex_override must be a 6-digit #RRGGBB hex string'
    );
  }
  return [
    parseInt(clean.slice(0, 2), 16) / 255,
    parseInt(clean.slice(2, 4), 16) / 255,
    parseInt(clean.slice(4, 6), 16) / 255,
  ];
}

function paletteFromPreset(preset: ThemePreset): ThemePalette {
  const white: Rgb = [1, 1, 1];
  const warningBg: Rgb = [0.996, 0.976, 0.914];
  const warningAccent: Rgb = [0.769, 0.459, 0.047];
  const dangerBg: Rgb = [0.992, 0.933, 0.933];
  const dangerAccent: Rgb = [0.745, 0.141, 0.141];

  switch (preset) {
    case 'emerald_editorial':
      return {
        headerBg: [0.059, 0.141, 0.122],
        badgeBg: [0.086, 0.2, 0.173],
        badgeBorder: [0.165, 0.361, 0.302],
        accent: [0.122, 0.478, 0.329],
        accentBright: [0.525, 0.937, 0.675],
        headerMist: [0.82, 0.925, 0.886],
        white,
        cardBg: [0.973, 0.98, 0.976],
        tableHeadBg: [0.933, 0.953, 0.941],
        calloutBg: [0.925, 0.965, 0.945],
        warningBg,
        warningAccent,
        dangerBg,
        dangerAccent,
        border: [0.82, 0.863, 0.843],
        signatureLine: [0.58, 0.643, 0.616],
        inkStrong: [0.059, 0.122, 0.11],
        inkBody: [0.149, 0.22, 0.2],
        inkMuted: [0.353, 0.443, 0.412],
      };
    case 'slate_minimal':
      return {
        headerBg: [0.094, 0.094, 0.106],
        badgeBg: [0.153, 0.153, 0.173],
        badgeBorder: [0.247, 0.247, 0.275],
        accent: [0.22, 0.235, 0.278],
        accentBright: [0.831, 0.831, 0.847],
        headerMist: [0.863, 0.863, 0.878],
        white,
        cardBg: [0.976, 0.976, 0.98],
        tableHeadBg: [0.945, 0.945, 0.953],
        calloutBg: [0.949, 0.953, 0.961],
        warningBg,
        warningAccent,
        dangerBg,
        dangerAccent,
        border: [0.843, 0.847, 0.863],
        signatureLine: [0.62, 0.627, 0.659],
        inkStrong: [0.071, 0.075, 0.09],
        inkBody: [0.196, 0.204, 0.231],
        inkMuted: [0.427, 0.435, 0.475],
      };
    case 'crimson_compliance':
      return {
        headerBg: [0.22, 0.063, 0.094],
        badgeBg: [0.31, 0.094, 0.137],
        badgeBorder: [0.478, 0.169, 0.227],
        accent: [0.698, 0.133, 0.204],
        accentBright: [0.988, 0.69, 0.725],
        headerMist: [0.965, 0.855, 0.871],
        white,
        cardBg: [0.984, 0.973, 0.976],
        tableHeadBg: [0.961, 0.933, 0.937],
        calloutBg: [0.984, 0.937, 0.945],
        warningBg,
        warningAccent,
        dangerBg,
        dangerAccent,
        border: [0.882, 0.827, 0.835],
        signatureLine: [0.667, 0.58, 0.596],
        inkStrong: [0.141, 0.055, 0.075],
        inkBody: [0.259, 0.157, 0.18],
        inkMuted: [0.478, 0.369, 0.392],
      };
    default:
      return {
        headerBg: [0.055, 0.114, 0.212],
        badgeBg: [0.086, 0.169, 0.302],
        badgeBorder: [0.18, 0.31, 0.51],
        accent: [0.114, 0.388, 0.784],
        accentBright: [0.576, 0.773, 0.992],
        headerMist: [0.824, 0.89, 0.973],
        white,
        cardBg: [0.973, 0.98, 0.992],
        tableHeadBg: [0.933, 0.953, 0.98],
        calloutBg: [0.929, 0.961, 0.996],
        warningBg,
        warningAccent,
        dangerBg,
        dangerAccent,
        border: [0.824, 0.859, 0.91],
        signatureLine: [0.58, 0.635, 0.718],
        inkStrong: [0.059, 0.09, 0.165],
        inkBody: [0.157, 0.212, 0.306],
        inkMuted: [0.369, 0.435, 0.541],
      };
  }
}

interface ValidatedJpeg {
  widthPx: number;
  heightPx: number;
  colorSpace: '/DeviceGray' | '/DeviceRGB';
  asciiHexStream: string;
}

const MAX_JPEG_BYTES = 262_144;

function decodeBase64Bytes(input: string): Uint8Array {
  const clean = input
    .trim()
    .replace(/^data:image\/jpe?g;base64,/, '')
    .replace(/\s+/g, '');

  const out = new Uint8Array(Math.floor((clean.length * 3) / 4));
  let ptr = 0;
  let buf = 0;
  let bits = 0;

  for (let i = 0; i < clean.length; i++) {
    const c = clean.charCodeAt(i);
    if (c === 61) break; // '='
    let val = -1;
    if (c >= 65 && c <= 90) val = c - 65;
    else if (c >= 97 && c <= 122) val = c - 71;
    else if (c >= 48 && c <= 57) val = c + 4;
    else if (c === 43 || c === 45) val = 62;
    else if (c === 47 || c === 95) val = 63;
    if (val < 0) {
      throw new MsfPdfError('INVALID_IMAGE', 'invalid base64 byte in JPEG asset');
    }
    buf = (buf << 6) | val;
    bits += 6;
    if (bits >= 8) {
      bits -= 8;
      if (ptr >= MAX_JPEG_BYTES) {
        throw new MsfPdfError('LIMIT_EXCEEDED', 'embedded JPEG exceeds 256 KiB byte ceiling');
      }
      out[ptr++] = (buf >> bits) & 0xff;
    }
  }
  return out.subarray(0, ptr);
}

function parseAndValidateJpegBase64(b64: string): ValidatedJpeg {
  const bytes = decodeBase64Bytes(b64);
  if (bytes.length < 24 || bytes[0] !== 0xff || bytes[1] !== 0xd8) {
    throw new MsfPdfError('INVALID_IMAGE', 'missing JPEG SOI magic bytes (FFD8)');
  }

  let pos = 2;
  while (pos + 4 <= bytes.length) {
    if (bytes[pos] !== 0xff) {
      pos++;
      continue;
    }
    while (pos < bytes.length && bytes[pos] === 0xff) pos++;
    if (pos >= bytes.length) break;
    const marker = bytes[pos++];
    if (marker === 0xd9 || marker === 0xda) break;
    if (marker === 0x00 || (marker >= 0xd0 && marker <= 0xd7) || marker === 0x01) continue;
    if (pos + 2 > bytes.length) {
      throw new MsfPdfError('INVALID_IMAGE', 'truncated JPEG segment header');
    }
    const segLen = (bytes[pos] << 8) | bytes[pos + 1];
    if (segLen < 2 || pos + segLen > bytes.length) {
      throw new MsfPdfError('INVALID_IMAGE', 'malformed JPEG segment length');
    }

    if (marker >= 0xc0 && marker <= 0xc2) {
      if (segLen < 8 || bytes[pos + 2] !== 8) {
        throw new MsfPdfError('INVALID_IMAGE', 'unsupported JPEG SOF frame');
      }
      const heightPx = (bytes[pos + 3] << 8) | bytes[pos + 4];
      const widthPx = (bytes[pos + 5] << 8) | bytes[pos + 6];
      const components = bytes[pos + 7];
      if (widthPx === 0 || heightPx === 0 || widthPx > 4096 || heightPx > 4096) {
        throw new MsfPdfError('INVALID_IMAGE', 'JPEG dimensions out of safe bounds');
      }
      if (components !== 1 && components !== 3) {
        throw new MsfPdfError('INVALID_IMAGE', 'only Grayscale and RGB JPEGs are permitted');
      }
      const hexChars = '0123456789ABCDEF';
      let hex = '';
      for (let i = 0; i < bytes.length; i++) {
        hex += hexChars[bytes[i] >> 4] + hexChars[bytes[i] & 0x0f];
      }
      hex += '>';
      return {
        widthPx,
        heightPx,
        colorSpace: components === 1 ? '/DeviceGray' : '/DeviceRGB',
        asciiHexStream: hex,
      };
    }
    pos += segLen;
  }

  throw new MsfPdfError('INVALID_IMAGE', 'no valid SOF0/SOF2 frame header found in JPEG');
}

type PageOp =
  | { kind: 'raw'; line: string }
  | {
      kind: 'vector';
      bold: boolean;
      size: number;
      color: Rgb;
      x: number;
      y: number;
      cps: number[];
    };

type MixedRun = { kind: 'win'; text: string } | { kind: 'vec'; cps: number[] };

function splitMixedRuns(visual: string): MixedRun[] {
  const runs: MixedRun[] = [];
  for (const ch of visual) {
    const cp = ch.codePointAt(0) ?? 32;
    if (lookupVectorGlyph(cp) !== undefined) {
      const last = runs[runs.length - 1];
      if (last && last.kind === 'vec') {
        last.cps.push(cp);
      } else {
        runs.push({ kind: 'vec', cps: [cp] });
      }
    } else {
      const last = runs[runs.length - 1];
      if (last && last.kind === 'win') {
        last.text += ch;
      } else {
        runs.push({ kind: 'win', text: ch });
      }
    }
  }
  return runs;
}

class PdfPageOps {
  readonly pageW: number;
  readonly pageH: number;
  readonly marginX: number;
  readonly ops: PageOp[] = [];

  constructor(pageW: number, pageH: number, marginX: number) {
    this.pageW = pageW;
    this.pageH = pageH;
    this.marginX = marginX;
  }

  private pushRaw(line: string): void {
    this.ops.push({ kind: 'raw', line });
  }

  fillRect(x: number, yTop: number, w: number, h: number, color: Rgb): void {
    const y = this.pageH - yTop - h;
    this.pushRaw(`${color[0].toFixed(3)} ${color[1].toFixed(3)} ${color[2].toFixed(3)} rg`);
    this.pushRaw(`${x.toFixed(2)} ${y.toFixed(2)} ${w.toFixed(2)} ${h.toFixed(2)} re f`);
  }

  strokeRect(x: number, yTop: number, w: number, h: number, lineW: number, color: Rgb): void {
    const y = this.pageH - yTop - h;
    this.pushRaw(
      `${lineW.toFixed(2)} w ${color[0].toFixed(3)} ${color[1].toFixed(3)} ${color[2].toFixed(3)} RG`
    );
    this.pushRaw(`${x.toFixed(2)} ${y.toFixed(2)} ${w.toFixed(2)} ${h.toFixed(2)} re S`);
  }

  fillAndStrokeRect(
    x: number,
    yTop: number,
    w: number,
    h: number,
    fill: Rgb,
    lineW: number,
    stroke: Rgb
  ): void {
    this.fillRect(x, yTop, w, h, fill);
    this.strokeRect(x, yTop, w, h, lineW, stroke);
  }

  line(x1: number, yTop1: number, x2: number, yTop2: number, lineW: number, color: Rgb): void {
    const y1 = this.pageH - yTop1;
    const y2 = this.pageH - yTop2;
    this.pushRaw(
      `${lineW.toFixed(2)} w ${color[0].toFixed(3)} ${color[1].toFixed(3)} ${color[2].toFixed(3)} RG`
    );
    this.pushRaw(`${x1.toFixed(2)} ${y1.toFixed(2)} m ${x2.toFixed(2)} ${y2.toFixed(2)} l S`);
  }

  dashedLine(
    x1: number,
    yTop1: number,
    x2: number,
    yTop2: number,
    lineW: number,
    color: Rgb
  ): void {
    const y1 = this.pageH - yTop1;
    const y2 = this.pageH - yTop2;
    this.pushRaw('q [2.5 2.5] 0 d');
    this.pushRaw(
      `${lineW.toFixed(2)} w ${color[0].toFixed(3)} ${color[1].toFixed(3)} ${color[2].toFixed(3)} RG`
    );
    this.pushRaw(`${x1.toFixed(2)} ${y1.toFixed(2)} m ${x2.toFixed(2)} ${y2.toFixed(2)} l S Q`);
  }

  text(content: string, x: number, yTop: number, font: FontFace, size: number, color: Rgb): void {
    const y = this.pageH - yTop;
    const visual = shapeAndReorderLine(content);
    let hasVector = false;
    for (const ch of visual) {
      if (lookupVectorGlyph(ch.codePointAt(0) ?? 32) !== undefined) {
        hasVector = true;
        break;
      }
    }

    if (!hasVector) {
      this.pushRaw('BT');
      this.pushRaw(`/${fontAlias(font)} ${size.toFixed(2)} Tf`);
      this.pushRaw(`${color[0].toFixed(3)} ${color[1].toFixed(3)} ${color[2].toFixed(3)} rg`);
      this.pushRaw(`1 0 0 1 ${x.toFixed(2)} ${y.toFixed(2)} Tm`);
      this.pushRaw(`(${pdfEscapeWinAnsi(visual)}) Tj`);
      this.pushRaw('ET');
      return;
    }

    const isBold = font === 'bold';
    let curX = x;
    for (const run of splitMixedRuns(visual)) {
      if (run.kind === 'win') {
        const segW = estimateFontWidthPt(run.text, font, size);
        this.pushRaw('BT');
        this.pushRaw(`/${fontAlias(font)} ${size.toFixed(2)} Tf`);
        this.pushRaw(`${color[0].toFixed(3)} ${color[1].toFixed(3)} ${color[2].toFixed(3)} rg`);
        this.pushRaw(`1 0 0 1 ${curX.toFixed(2)} ${y.toFixed(2)} Tm`);
        this.pushRaw(`(${pdfEscapeWinAnsi(run.text)}) Tj`);
        this.pushRaw('ET');
        curX += segW;
      } else {
        let funits = 0;
        for (const cp of run.cps) {
          const rec = lookupVectorGlyph(cp);
          if (rec !== undefined) {
            funits += isBold ? rec.boldAdv : rec.regularAdv;
          }
        }
        const segW = (funits / VECTOR_UPM) * size;
        this.ops.push({
          kind: 'vector',
          bold: isBold,
          size,
          color,
          x: curX,
          y,
          cps: run.cps,
        });
        curX += segW;
      }
    }
  }

  textRight(
    content: string,
    xRight: number,
    yTop: number,
    font: FontFace,
    size: number,
    color: Rgb
  ): void {
    const est = estimateFontWidthPt(content, font, size);
    this.text(content, Math.max(this.marginX, xRight - est), yTop, font, size, color);
  }

  textCenter(
    content: string,
    xCenter: number,
    yTop: number,
    font: FontFace,
    size: number,
    color: Rgb
  ): void {
    const est = estimateFontWidthPt(content, font, size);
    this.text(content, Math.max(this.marginX, xCenter - est / 2), yTop, font, size, color);
  }

  drawQrMatrix(
    qr: QrMatrix,
    xLeft: number,
    yTop: number,
    boxSizePt: number,
    fg: Rgb,
    bg: Rgb
  ): void {
    this.fillRect(xLeft, yTop, boxSizePt, boxSizePt, bg);
    const quiet = 2;
    const modPt = boxSizePt / (qr.size + quiet * 2);
    this.pushRaw(`${fg[0].toFixed(3)} ${fg[1].toFixed(3)} ${fg[2].toFixed(3)} rg`);

    for (let r = 0; r < qr.size; r++) {
      let c = 0;
      while (c < qr.size) {
        if (qr.modules[r * qr.size + c] === 0) {
          c++;
          continue;
        }
        const startC = c;
        while (c < qr.size && qr.modules[r * qr.size + c] !== 0) c++;
        const rx = xLeft + (startC + quiet) * modPt;
        const ryTop = yTop + (r + quiet) * modPt;
        const ryPdf = this.pageH - ryTop - modPt;
        const rw = (c - startC) * modPt;
        this.pushRaw(
          `${rx.toFixed(3)} ${ryPdf.toFixed(3)} ${rw.toFixed(3)} ${modPt.toFixed(3)} re`
        );
      }
    }
    this.pushRaw('f');
  }

  drawJpeg(alias: string, x: number, yTop: number, w: number, h: number): void {
    const y = this.pageH - yTop - h;
    this.pushRaw(
      `q ${w.toFixed(2)} 0 0 ${h.toFixed(2)} ${x.toFixed(2)} ${y.toFixed(2)} cm /${alias} Do Q`
    );
  }
}

interface VectorFontBank {
  alias: string;
  pdfName: string;
  bold: boolean;
  cps: number[];
}

class VectorSubsetPool {
  readonly banks: VectorFontBank[] = [];

  resolveSlot(bold: boolean, cp: number): readonly [string, number] {
    for (const bank of this.banks) {
      if (bank.bold === bold) {
        const pos = bank.cps.indexOf(cp);
        if (pos !== -1) {
          return [bank.alias, pos + 1];
        }
      }
    }
    for (const bank of this.banks) {
      if (bank.bold === bold && bank.cps.length < 254) {
        bank.cps.push(cp);
        return [bank.alias, bank.cps.length];
      }
    }
    const sameWeightIdx = this.banks.filter((b) => b.bold === bold).length;
    const baseAlias = bold ? 'F5' : 'F4';
    const alias = sameWeightIdx === 0 ? baseAlias : `${baseAlias}_${sameWeightIdx}`;
    const pdfName = bold
      ? `MsfVectorBold${sameWeightIdx}`
      : `MsfVectorRegular${sameWeightIdx}`;
    this.banks.push({ alias, pdfName, bold, cps: [cp] });
    return [alias, 1];
  }
}

function assemblePdf14(pages: PdfPageOps[], jpegs: ValidatedJpeg[]): Uint8Array {
  const pool = new VectorSubsetPool();
  const resolvedPageStreams: string[] = [];

  for (const p of pages) {
    const lines: string[] = [];
    for (const op of p.ops) {
      if (op.kind === 'raw') {
        lines.push(op.line);
      } else {
        let curX = op.x;
        let idx = 0;
        while (idx < op.cps.length) {
          const [alias, firstCode] = pool.resolveSlot(op.bold, op.cps[idx]);
          let hex = firstCode.toString(16).toUpperCase().padStart(2, '0');
          const firstRec = lookupVectorGlyph(op.cps[idx]);
          let runFunits = firstRec ? (op.bold ? firstRec.boldAdv : firstRec.regularAdv) : 0;
          idx += 1;

          while (idx < op.cps.length) {
            const [nextAlias, nextCode] = pool.resolveSlot(op.bold, op.cps[idx]);
            if (nextAlias !== alias) break;
            hex += nextCode.toString(16).toUpperCase().padStart(2, '0');
            const rec = lookupVectorGlyph(op.cps[idx]);
            runFunits += rec ? (op.bold ? rec.boldAdv : rec.regularAdv) : 0;
            idx += 1;
          }

          lines.push('BT');
          lines.push(`/${alias} ${op.size.toFixed(2)} Tf`);
          lines.push(
            `${op.color[0].toFixed(3)} ${op.color[1].toFixed(3)} ${op.color[2].toFixed(3)} rg`
          );
          lines.push(`1 0 0 1 ${curX.toFixed(2)} ${op.y.toFixed(2)} Tm`);
          lines.push(`<${hex}> Tj`);
          lines.push('ET');
          curX += (runFunits / VECTOR_UPM) * op.size;
        }
      }
    }
    resolvedPageStreams.push(lines.join('\n') + '\n');
  }

  const pageCount = Math.max(1, pages.length);
  const imgCount = jpegs.length;
  const vectorObjCount = pool.banks.reduce((acc, b) => acc + 2 + b.cps.length, 0);
  const firstPageId = 6 + imgCount + vectorObjCount;
  const objects: string[] = [];

  const kidsRefs = Array.from({ length: pageCount }, (_, i) => `${firstPageId + i * 2} 0 R`).join(
    ' '
  );
  objects.push(`1 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\n`);
  objects.push(`2 0 obj\n<< /Type /Pages /Kids [${kidsRefs}] /Count ${pageCount} >>\nendobj\n`);
  objects.push(
    `3 0 obj\n<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica /Encoding /WinAnsiEncoding >>\nendobj\n`
  );
  objects.push(
    `4 0 obj\n<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica-Bold /Encoding /WinAnsiEncoding >>\nendobj\n`
  );
  objects.push(
    `5 0 obj\n<< /Type /Font /Subtype /Type1 /BaseFont /Courier /Encoding /WinAnsiEncoding >>\nendobj\n`
  );

  let xobjDict = '';
  if (imgCount > 0) {
    xobjDict += ' /XObject <<';
    for (let i = 0; i < imgCount; i++) {
      const objId = 6 + i;
      const img = jpegs[i];
      xobjDict += ` /Im${i + 1} ${objId} 0 R`;
      const streamBody = img.asciiHexStream + '\n';
      objects.push(
        `${objId} 0 obj\n<< /Type /XObject /Subtype /Image /Width ${img.widthPx} /Height ${img.heightPx} /ColorSpace ${img.colorSpace} /BitsPerComponent 8 /Filter [/ASCIIHexDecode /DCTDecode] /Length ${streamBody.length} >>\nstream\n${streamBody}endstream\nendobj\n`
      );
    }
    xobjDict += ' >>';
  }

  let extraFontRefs = '';
  let nextObjId = 6 + imgCount;
  for (const bank of pool.banks) {
    const fontObjId = nextObjId;
    const cmapObjId = nextObjId + 1;
    const firstProcId = nextObjId + 2;
    nextObjId += 2 + bank.cps.length;

    extraFontRefs += ` /${bank.alias} ${fontObjId} 0 R`;

    const charProcs: string[] = [];
    const diffs: string[] = ['1'];
    const widths: string[] = [];
    const bfchars: string[] = [];

    bank.cps.forEach((cp, i) => {
      const code = i + 1;
      const procId = firstProcId + i;
      const rec = lookupVectorGlyph(cp)!;
      const adv = bank.bold ? rec.boldAdv : rec.regularAdv;
      charProcs.push(`/g${code} ${procId} 0 R`);
      diffs.push(`/g${code}`);
      widths.push(String(adv));
      bfchars.push(`<${code.toString(16).toUpperCase().padStart(2, '0')}> <${rec.toUnicodeHex}>`);
    });

    objects.push(
      `${fontObjId} 0 obj\n<< /Type /Font /Subtype /Type3 /Name /${bank.pdfName} /FontBBox ${VECTOR_FONT_BBOX} /FontMatrix ${VECTOR_FONT_MATRIX} /CharProcs << ${charProcs.join(' ')} >> /Encoding << /Type /Encoding /Differences [${diffs.join(' ')}] >> /FirstChar 1 /LastChar ${bank.cps.length} /Widths [${widths.join(' ')}] /ToUnicode ${cmapObjId} 0 R >>\nendobj\n`
    );

    const cmapStream =
      `/CIDInit /ProcSet findresource begin\n12 dict begin\nbegincmap\n/CIDSystemInfo << /Registry (Adobe) /Ordering (UCS) /Supplement 0 >> def\n/CMapName /Adobe-Identity-UCS def\n/CMapType 2 def\n1 begincodespacerange\n<01> <FE>\nendcodespacerange\n${bfchars.length} beginbfchar\n` +
      bfchars.join('\n') +
      `\nendbfchar\nendcmap\nCMapName currentdict /CMap defineresource pop\nend\nend\n`;
    objects.push(
      `${cmapObjId} 0 obj\n<< /Length ${cmapStream.length} >>\nstream\n${cmapStream}endstream\nendobj\n`
    );

    bank.cps.forEach((cp, i) => {
      const procId = firstProcId + i;
      const rec = lookupVectorGlyph(cp)!;
      const procBody = (bank.bold ? rec.boldProc : rec.regularProc) + '\n';
      objects.push(
        `${procId} 0 obj\n<< /Length ${procBody.length} >>\nstream\n${procBody}endstream\nendobj\n`
      );
    });
  }

  for (let i = 0; i < pageCount; i++) {
    const p = pages[i];
    const pageObjId = firstPageId + i * 2;
    const contentObjId = pageObjId + 1;
    const streamBody = resolvedPageStreams[i];
    objects.push(
      `${pageObjId} 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [0 0 ${p.pageW.toFixed(2)} ${p.pageH.toFixed(2)}] /Resources << /Font << /F1 3 0 R /F2 4 0 R /F3 5 0 R${extraFontRefs} >>${xobjDict} >> /Contents ${contentObjId} 0 R >>\nendobj\n`
    );
    objects.push(
      `${contentObjId} 0 obj\n<< /Length ${streamBody.length} >>\nstream\n${streamBody}endstream\nendobj\n`
    );
  }

  let pdf = '%PDF-1.4\n';
  const offsets: number[] = [0];
  for (const obj of objects) {
    offsets.push(pdf.length);
    pdf += obj;
  }

  const xrefOffset = pdf.length;
  pdf += `xref\n0 ${objects.length + 1}\n0000000000 65535 f \n`;
  for (let i = 1; i < offsets.length; i++) {
    pdf += `${String(offsets[i]).padStart(10, '0')} 00000 n \n`;
  }
  pdf += `trailer\n<< /Size ${objects.length + 1} /Root 1 0 R >>\nstartxref\n${xrefOffset}\n%%EOF\n`;

  return new TextEncoder().encode(pdf);
}

export function renderDocument(
  spec: DocumentSpec,
  limits: SecurityLimits = DEFAULT_SECURITY_LIMITS
): Uint8Array {
  if (!spec || !Array.isArray(spec.blocks)) {
    throw new MsfPdfError('INVALID_SPEC', 'DocumentSpec.blocks must be an array');
  }
  if (spec.blocks.length > limits.maxBlocks) {
    throw new MsfPdfError('LIMIT_EXCEEDED', 'document block count exceeds maxBlocks ceiling');
  }

  const format: PageFormat = spec.page_format ?? 'a4';
  const isThermal = format === 'thermal80mm';
  const palette = paletteFromPreset(spec.theme ?? 'navy_fintech');
  if (spec.accent_hex_override) {
    palette.accent = parseHexRgb(spec.accent_hex_override);
  }

  const [pageW, pageH] = pageDimensionsPt(format);
  const marginX = marginXPt(format);
  const contentW = pageW - marginX * 2;
  const footerTop = footerTopPt(format);
  const safeBottomY = footerTop - 10;

  const pages: PdfPageOps[] = [new PdfPageOps(pageW, pageH, marginX)];
  const jpegs: ValidatedJpeg[] = [];
  let cursorY =
    spec.blocks[0]?.type === 'hero_header' ? 0 : isThermal ? 16 : 36;

  const currentPage = (): PdfPageOps => pages[pages.length - 1];

  const pushContinuationPage = (): void => {
    if (pages.length >= limits.maxPages) {
      throw new MsfPdfError(
        'LIMIT_EXCEEDED',
        'document pagination exceeded maxPages ceiling'
      );
    }
    const p = new PdfPageOps(pageW, pageH, marginX);
    if (isThermal) {
      p.fillRect(0, 0, pageW, 28, palette.headerBg);
      p.fillRect(0, 26, pageW, 2, palette.accent);
      p.text(
        truncateToWidthPt(spec.metadata.issuer_name, 'bold', 8.2, contentW),
        marginX,
        17,
        'bold',
        8.2,
        palette.white
      );
      pages.push(p);
      cursorY = 38;
      return;
    }

    const contTitle = spec.continuation_header?.title ?? spec.metadata.issuer_name;
    const contSub = spec.continuation_header?.subtitle ?? 'CONTINUATION SHEET';
    p.fillRect(0, 0, pageW, 50, palette.headerBg);
    p.fillRect(0, 47.5, pageW, 2.5, palette.accent);
    p.text(
      truncateToWidthPt(contTitle, 'bold', 11.0, contentW * 0.68),
      marginX,
      23.5,
      'bold',
      11.0,
      palette.white
    );
    p.text(
      truncateToWidthPt(contSub, 'regular', 7.6, contentW * 0.68),
      marginX,
      37.5,
      'regular',
      7.6,
      palette.accentBright
    );
    p.textRight(
      truncateToWidthPt(spec.metadata.document_code, 'mono', 8.6, contentW * 0.3),
      pageW - marginX,
      30.5,
      'mono',
      8.6,
      palette.white
    );
    pages.push(p);
    cursorY = 68;
  };

  const ensureSpace = (neededH: number): void => {
    if (cursorY + neededH > safeBottomY) {
      pushContinuationPage();
    }
  };

  const drawSectionHeading = (title: string, keepWithNextPt: number): void => {
    ensureSpace(18 + keepWithNextPt);
    const y = cursorY + 9;
    const p = currentPage();
    p.fillRect(marginX, y - 8.2, 3.2, 10.2, palette.accent);
    p.text(
      truncateToWidthPt(title, 'bold', 8.8, contentW - 10),
      marginX + 8.5,
      y,
      'bold',
      8.8,
      palette.inkStrong
    );
    cursorY += 17.5;
  };

  for (const block of spec.blocks) {
    switch (block.type) {
      case 'hero_header': {
        let logoAlias: { alias: string; w: number; h: number } | null = null;
        if (block.logo_jpeg_base64) {
          if (jpegs.length >= 8) {
            throw new MsfPdfError('LIMIT_EXCEEDED', 'maximum of 8 embedded JPEGs per document');
          }
          const img = parseAndValidateJpegBase64(block.logo_jpeg_base64);
          const targetH = isThermal ? 22 : 28;
          const targetW = Math.min(90, Math.max(16, targetH * (img.widthPx / img.heightPx)));
          jpegs.push(img);
          logoAlias = { alias: `Im${jpegs.length}`, w: targetW, h: targetH };
        }

        const p = currentPage();
        if (isThermal) {
          const bannerH = block.badge_code ? 76 : 60;
          p.fillRect(0, 0, pageW, bannerH, palette.headerBg);
          p.fillRect(0, bannerH - 2.5, pageW, 2.5, palette.accent);
          p.textCenter(
            truncateToWidthPt(block.kicker, 'bold', 6.4, contentW),
            pageW / 2,
            15,
            'bold',
            6.4,
            palette.accentBright
          );
          p.textCenter(
            truncateToWidthPt(block.title, 'bold', 11.2, contentW),
            pageW / 2,
            31,
            'bold',
            11.2,
            palette.white
          );
          p.textCenter(
            truncateToWidthPt(block.subtitle, 'regular', 7.2, contentW),
            pageW / 2,
            44,
            'regular',
            7.2,
            palette.headerMist
          );
          if (block.badge_code) {
            p.textCenter(
              truncateToWidthPt(block.badge_code, 'mono', 7.4, contentW),
              pageW / 2,
              61,
              'mono',
              7.4,
              palette.white
            );
          }
          cursorY = bannerH + 12;
        } else {
          const bannerH = 88;
          const hasBadge = Boolean(block.badge_label || block.badge_code);
          const badgeW = hasBadge ? 172 : 0;
          const leftMaxW = contentW - badgeW - 16;

          p.fillRect(0, 0, pageW, bannerH, palette.headerBg);
          p.fillRect(0, bannerH - 3, pageW, 3, palette.accent);

          let textX = marginX;
          if (logoAlias) {
            p.drawJpeg(logoAlias.alias, marginX, 18, logoAlias.w, logoAlias.h);
            textX += logoAlias.w + 10;
          }
          const adjW = Math.max(120, leftMaxW - (textX - marginX));

          p.text(
            truncateToWidthPt(block.kicker, 'bold', 7.2, adjW),
            textX,
            23,
            'bold',
            7.2,
            palette.accentBright
          );
          p.text(
            truncateToWidthPt(block.title, 'bold', 15.2, adjW),
            textX,
            45,
            'bold',
            15.2,
            palette.white
          );
          p.text(
            truncateToWidthPt(block.subtitle, 'regular', 8.6, adjW),
            textX,
            63,
            'regular',
            8.6,
            palette.headerMist
          );

          if (hasBadge) {
            const bx = pageW - marginX - badgeW;
            p.fillAndStrokeRect(bx, 15, badgeW, 55, palette.badgeBg, 0.7, palette.badgeBorder);
            if (block.badge_label) {
              p.text(
                truncateToWidthPt(block.badge_label, 'bold', 6.8, badgeW - 18),
                bx + 10,
                28,
                'bold',
                6.8,
                palette.accentBright
              );
            }
            if (block.badge_code) {
              p.text(
                truncateToWidthPt(block.badge_code, 'mono', 8.5, badgeW - 18),
                bx + 10,
                44,
                'mono',
                8.5,
                palette.white
              );
            }
            if (block.badge_meta) {
              p.text(
                truncateToWidthPt(block.badge_meta, 'regular', 7.4, badgeW - 18),
                bx + 10,
                58.5,
                'regular',
                7.4,
                palette.headerMist
              );
            }
          }
          cursorY = bannerH + 16;
        }
        break;
      }

      case 'section_heading':
        drawSectionHeading(block.title, 36);
        break;

      case 'bento_grid': {
        if (block.cards.length === 0) break;
        const cols = isThermal ? 1 : Math.min(3, Math.max(1, block.columns ?? 2));
        const gap = 12;
        const cardW = (contentW - gap * (cols - 1)) / cols;

        for (let i = 0; i < block.cards.length; i += cols) {
          const rowCards = block.cards.slice(i, i + cols);
          const hasSec = rowCards.some((c) => c.secondary_label || c.secondary_value);
          const cardH = hasSec ? 70 : 44;
          ensureSpace(cardH + 10);

          const p = currentPage();
          rowCards.forEach((card, cIdx) => {
            const cx = marginX + cIdx * (cardW + gap);
            const innerW = cardW - 20;
            p.fillAndStrokeRect(cx, cursorY, cardW, cardH, palette.cardBg, 0.6, palette.border);
            p.fillRect(cx, cursorY, cardW, 2.2, palette.accent);
            p.text(
              truncateToWidthPt(card.primary_label, 'bold', 6.8, innerW),
              cx + 10,
              cursorY + 15,
              'bold',
              6.8,
              palette.inkMuted
            );
            p.text(
              truncateToWidthPt(card.primary_value, 'bold', 10.0, innerW),
              cx + 10,
              cursorY + 30.5,
              'bold',
              10.0,
              palette.inkStrong
            );
            if (card.secondary_label) {
              p.text(
                truncateToWidthPt(card.secondary_label, 'bold', 6.6, innerW),
                cx + 10,
                cursorY + 47,
                'bold',
                6.6,
                palette.inkMuted
              );
            }
            if (card.secondary_value) {
              p.text(
                truncateToWidthPt(card.secondary_value, 'regular', 8.4, innerW),
                cx + 10,
                cursorY + 60,
                'regular',
                8.4,
                palette.inkBody
              );
            }
          });
          cursorY += cardH + 12;
        }
        break;
      }

      case 'data_table': {
        if (block.columns.length === 0) {
          throw new MsfPdfError('INVALID_SPEC', 'data_table requires at least one column');
        }
        if (block.rows.length > limits.maxTableRows) {
          throw new MsfPdfError('LIMIT_EXCEEDED', 'table row count exceeds maxTableRows ceiling');
        }

        const totalWeight = block.columns.reduce(
          (acc, c) => acc + Math.max(0.2, c.weight ?? 1),
          0
        );
        const colWidths: number[] = [];
        const colStarts: number[] = [];
        let accX = marginX;
        for (const col of block.columns) {
          colStarts.push(accX);
          const w = (Math.max(0.2, col.weight ?? 1) / totalWeight) * contentW;
          colWidths.push(w);
          accX += w;
        }

        const headH = 19;
        const rowH = 20.5;
        const zebra = block.zebra ?? true;
        ensureSpace(headH + rowH + 8);

        let rowIdx = 0;
        while (rowIdx < block.rows.length || (block.rows.length === 0 && rowIdx === 0)) {
          const avail = Math.max(rowH, safeBottomY - cursorY - headH);
          const maxRowsHere = Math.max(1, Math.floor(avail / rowH));
          const takeN =
            block.rows.length === 0 ? 0 : Math.min(block.rows.length - rowIdx, maxRowsHere);
          const segH = headH + takeN * rowH;

          const p = currentPage();
          p.fillRect(marginX, cursorY, contentW, headH, palette.tableHeadBg);
          block.columns.forEach((col, ci) => {
            const cx = colStarts[ci];
            const w = colWidths[ci];
            const hdr = truncateToWidthPt(col.header, 'bold', 6.9, Math.max(12, w - 14));
            const align: TextAlign = col.align ?? 'left';
            if (align === 'center')
              p.textCenter(hdr, cx + w / 2, cursorY + 12.6, 'bold', 6.9, palette.inkMuted);
            else if (align === 'right')
              p.textRight(hdr, cx + w - 7, cursorY + 12.6, 'bold', 6.9, palette.inkMuted);
            else p.text(hdr, cx + 7, cursorY + 12.6, 'bold', 6.9, palette.inkMuted);
          });

          for (let localI = 0; localI < takeN; localI++) {
            const rData = block.rows[rowIdx + localI];
            const ry = cursorY + headH + localI * rowH;
            if (zebra && localI % 2 === 1) {
              p.fillRect(marginX + 0.4, ry + 0.4, contentW - 0.8, rowH - 0.8, palette.cardBg);
            }
            p.line(marginX, ry, marginX + contentW, ry, 0.5, palette.border);

            block.columns.forEach((col, ci) => {
              const rawCell = rData[ci] ?? '';
              const cx = colStarts[ci];
              const w = colWidths[ci];
              const face: FontFace = col.bold_values ? 'bold' : 'regular';
              const color = col.bold_values ? palette.inkStrong : palette.inkBody;
              const val = truncateToWidthPt(rawCell, face, 8.5, Math.max(12, w - 14));
              const align: TextAlign = col.align ?? 'left';
              if (align === 'center')
                p.textCenter(val, cx + w / 2, ry + 13.6, face, 8.5, color);
              else if (align === 'right')
                p.textRight(val, cx + w - 7, ry + 13.6, face, 8.5, color);
              else p.text(val, cx + 7, ry + 13.6, face, 8.5, color);
            });
          }

          for (let ci = 1; ci < colStarts.length; ci++) {
            p.line(colStarts[ci], cursorY, colStarts[ci], cursorY + segH, 0.45, palette.border);
          }
          p.strokeRect(marginX, cursorY, contentW, segH, 0.65, palette.border);

          cursorY += segH + 12;
          if (block.rows.length === 0) break;
          rowIdx += takeN;
          if (rowIdx < block.rows.length) pushContinuationPage();
        }
        break;
      }

      case 'financial_summary': {
        if (block.rows.length === 0) break;
        if (block.rows.length > limits.maxTableRows) {
          throw new MsfPdfError(
            'LIMIT_EXCEEDED',
            'financial_summary rows exceed maxTableRows ceiling'
          );
        }

        const rowH = 21.5;
        ensureSpace(Math.min(120, rowH * block.rows.length));

        let idx = 0;
        while (idx < block.rows.length) {
          const avail = Math.max(rowH, safeBottomY - cursorY);
          const takeN = Math.min(block.rows.length - idx, Math.max(1, Math.floor(avail / rowH)));
          const segH = takeN * rowH;

          const p = currentPage();
          for (let localI = 0; localI < takeN; localI++) {
            const item = block.rows[idx + localI];
            const ry = cursorY + localI * rowH;
            if (localI % 2 === 0) {
              p.fillRect(marginX + 0.4, ry + 0.4, contentW - 0.8, rowH - 0.8, palette.cardBg);
            }
            if (localI > 0) p.line(marginX, ry, marginX + contentW, ry, 0.5, palette.border);

            const formattedVal =
              item.amount_minor !== undefined
                ? formatMoneyMinor(item.amount_minor, item.currency ?? block.currency)
                : (item.text_value ?? '-');

            const valFace: FontFace = item.emphasize ? 'bold' : 'regular';
            const valSize = item.emphasize ? 9.6 : 8.8;

            p.text(
              truncateToWidthPt(item.label, 'regular', 8.8, contentW * 0.64),
              marginX + 10,
              ry + 14.2,
              'regular',
              8.8,
              palette.inkBody
            );
            p.textRight(
              truncateToWidthPt(formattedVal, valFace, valSize, contentW * 0.32),
              marginX + contentW - 10,
              ry + 14.2,
              valFace,
              valSize,
              palette.inkStrong
            );
          }
          p.strokeRect(marginX, cursorY, contentW, segH, 0.65, palette.border);

          cursorY += segH + 10;
          idx += takeN;
          if (idx < block.rows.length) pushContinuationPage();
        }
        break;
      }

      case 'callout_banner': {
        const boxH = isThermal && block.right_value ? 54 : 44;
        ensureSpace(boxH + 10);

        const tone: BannerTone = block.tone ?? 'accent';
        const [bg, bar]: [Rgb, Rgb] =
          tone === 'warning'
            ? [palette.warningBg, palette.warningAccent]
            : tone === 'danger'
              ? [palette.dangerBg, palette.dangerAccent]
              : tone === 'neutral'
                ? [palette.cardBg, palette.inkMuted]
                : [palette.calloutBg, palette.accent];

        const p = currentPage();
        p.fillAndStrokeRect(marginX, cursorY, contentW, boxH, bg, 0.85, bar);
        p.fillRect(marginX, cursorY, 4, boxH, bar);

        if (isThermal) {
          p.text(
            truncateToWidthPt(block.title, 'bold', 7.6, contentW - 18),
            marginX + 10,
            cursorY + 14,
            'bold',
            7.6,
            bar
          );
          p.text(
            truncateToWidthPt(block.subtitle, 'regular', 7.2, contentW - 18),
            marginX + 10,
            cursorY + 26.5,
            'regular',
            7.2,
            palette.inkBody
          );
          if (block.right_value) {
            p.text(
              truncateToWidthPt(block.right_value, 'bold', 10.5, contentW - 18),
              marginX + 10,
              cursorY + 43,
              'bold',
              10.5,
              palette.inkStrong
            );
          }
        } else {
          const leftLimit = block.right_value ? contentW * 0.6 : contentW - 24;
          p.text(
            truncateToWidthPt(block.title, 'bold', 8.2, leftLimit),
            marginX + 13,
            cursorY + 16.5,
            'bold',
            8.2,
            bar
          );
          p.text(
            truncateToWidthPt(block.subtitle, 'regular', 7.8, leftLimit),
            marginX + 13,
            cursorY + 31,
            'regular',
            7.8,
            palette.inkMuted
          );
          if (block.right_value) {
            p.textRight(
              truncateToWidthPt(block.right_value, 'bold', 12.4, contentW * 0.36),
              marginX + contentW - 12,
              cursorY + 27,
              'bold',
              12.4,
              palette.inkStrong
            );
          }
        }
        cursorY += boxH + 12;
        break;
      }

      case 'clause_list': {
        const fontSz = isThermal ? 7.4 : 8.4;
        const lineStep = isThermal ? 11.0 : 13.0;
        const wrapped: Array<{ text: string; rtl: boolean }> = [];
        for (const para of block.paragraphs) {
          const rtl = isRtlDominant(para);
          for (const ln of wrapTextToWidthPt(para, 'regular', fontSz, contentW - 20)) {
            wrapped.push({ text: ln, rtl });
          }
        }
        if (wrapped.length === 0) break;

        ensureSpace(lineStep * 2 + 18);
        let idx = 0;
        while (idx < wrapped.length) {
          const avail = Math.max(lineStep, safeBottomY - cursorY - 16);
          const takeN = Math.min(wrapped.length - idx, Math.max(1, Math.floor(avail / lineStep)));
          const boxH = takeN * lineStep + 14;

          const p = currentPage();
          p.fillAndStrokeRect(marginX, cursorY, contentW, boxH, palette.cardBg, 0.6, palette.border);
          for (let offset = 0; offset < takeN; offset++) {
            const item = wrapped[idx + offset];
            const ly = cursorY + 12.5 + offset * lineStep;
            if (item.rtl) {
              p.textRight(item.text, marginX + contentW - 10, ly, 'regular', fontSz, palette.inkBody);
            } else {
              p.text(item.text, marginX + 10, ly, 'regular', fontSz, palette.inkBody);
            }
          }
          cursorY += boxH + 12;
          idx += takeN;
          if (idx < wrapped.length) pushContinuationPage();
        }
        break;
      }

      case 'qr_verification': {
        const qr = encodeQr(block.payload);
        if (isThermal) {
          const qrBox = 86;
          const detailLines = wrapTextToWidthPt(block.details, 'regular', 7.0, contentW - 16);
          const totalH = 28 + qrBox + detailLines.length * 10 + 14;
          ensureSpace(totalH);

          const p = currentPage();
          p.fillAndStrokeRect(marginX, cursorY, contentW, totalH, palette.cardBg, 0.6, palette.border);
          p.textCenter(
            truncateToWidthPt(block.headline, 'bold', 7.6, contentW - 12),
            pageW / 2,
            cursorY + 14,
            'bold',
            7.6,
            palette.inkStrong
          );
          p.drawQrMatrix(
            qr,
            (pageW - qrBox) / 2,
            cursorY + 20,
            qrBox,
            palette.inkStrong,
            palette.white
          );
          detailLines.forEach((ln, li) => {
            p.textCenter(
              ln,
              pageW / 2,
              cursorY + 32 + qrBox + li * 10,
              'regular',
              7.0,
              palette.inkMuted
            );
          });
          cursorY += totalH + 10;
        } else {
          const cardH = 74;
          const qrSize = 58;
          ensureSpace(cardH + 10);

          const textX = marginX + qrSize + 18;
          const textW = Math.max(80, contentW - qrSize - 28);
          const detailLines = wrapTextToWidthPt(block.details, 'regular', 7.8, textW);

          const p = currentPage();
          p.fillAndStrokeRect(marginX, cursorY, contentW, cardH, palette.cardBg, 0.6, palette.border);
          p.strokeRect(marginX + 8, cursorY + 8, qrSize, qrSize, 0.5, palette.border);
          p.drawQrMatrix(
            qr,
            marginX + 8.5,
            cursorY + 8.5,
            qrSize - 1,
            palette.inkStrong,
            palette.white
          );
          p.text(
            truncateToWidthPt(block.headline, 'bold', 8.6, textW),
            textX,
            cursorY + 20,
            'bold',
            8.6,
            palette.inkStrong
          );
          detailLines.slice(0, 2).forEach((ln, li) => {
            p.text(ln, textX, cursorY + 34 + li * 11.2, 'regular', 7.8, palette.inkBody);
          });
          if (block.code_display) {
            p.text(
              truncateToWidthPt(block.code_display, 'mono', 7.8, textW),
              textX,
              cursorY + 61,
              'mono',
              7.8,
              palette.accent
            );
          }
          cursorY += cardH + 12;
        }
        break;
      }

      case 'signature_grid': {
        if (block.signers.length === 0) break;
        ensureSpace(block.heading ? 76 : 56);
        if (block.heading) drawSectionHeading(block.heading, 46);

        const cols = isThermal ? 1 : Math.min(4, Math.max(1, block.signers.length));
        for (let i = 0; i < block.signers.length; i += cols) {
          const rowSigners = block.signers.slice(i, i + cols);
          ensureSpace(52);
          const gap = 16;
          const colW = (contentW - gap * (rowSigners.length - 1)) / rowSigners.length;
          const lineY = cursorY + 26;
          const p = currentPage();

          rowSigners.forEach((signer, si) => {
            const cx = marginX + si * (colW + gap);
            p.line(cx, lineY, cx + colW, lineY, 0.7, palette.signatureLine);
            p.text(
              truncateToWidthPt(signer.role_label, 'bold', 7.0, colW - 4),
              cx,
              lineY + 11.5,
              'bold',
              7.0,
              palette.inkMuted
            );
            p.text(
              truncateToWidthPt(signer.party_name, 'regular', 8.2, colW - 4),
              cx,
              lineY + 23,
              'regular',
              8.2,
              palette.inkStrong
            );
          });
          cursorY += 54;
        }
        break;
      }

      case 'key_value_list': {
        if (block.rows.length === 0) break;
        if (block.rows.length > limits.maxTableRows) {
          throw new MsfPdfError('LIMIT_EXCEEDED', 'key_value_list rows exceed maxTableRows ceiling');
        }
        const rowH = 16.5;
        block.rows.forEach((item, ri) => {
          ensureSpace(rowH + 4);
          const p = currentPage();
          if (ri > 0 && block.dashed_dividers) {
            p.dashedLine(marginX, cursorY, marginX + contentW, cursorY, 0.45, palette.border);
          }
          const face: FontFace = item.emphasize ? 'bold' : 'regular';
          const valTrunc = truncateToWidthPt(item.value, face, 8.2, contentW * 0.38);
          const valW = estimateFontWidthPt(valTrunc, face, 8.2);
          const lblMaxW = Math.max(contentW * 0.52, contentW - valW - 10);
          p.text(
            truncateToWidthPt(item.label, face, 8.0, lblMaxW),
            marginX,
            cursorY + 11.5,
            face,
            8.0,
            palette.inkBody
          );
          p.textRight(valTrunc, marginX + contentW, cursorY + 11.5, face, 8.2, palette.inkStrong);
          cursorY += rowH;
        });
        cursorY += 8;
        break;
      }

      case 'divider': {
        ensureSpace(12);
        currentPage().line(marginX, cursorY + 4, marginX + contentW, cursorY + 4, 0.6, palette.border);
        cursorY += 10;
        break;
      }

      case 'spacer': {
        const h = Math.min(200, Math.max(2, block.height_pt));
        ensureSpace(h);
        cursorY += h;
        break;
      }

      case 'audit_manifest': {
        const isThermal = spec.page_format === 'thermal80mm';
        const algorithm = block.algorithm ?? 'Ed25519';

        if (isThermal) {
          const boxH = 74.0;
          ensureSpace(boxH);
          const p = currentPage();
          p.fillAndStrokeRect(marginX, cursorY, contentW, boxH, palette.cardBg, 0.6, palette.border);
          p.text('CRYPTOGRAPHIC AUDIT MANIFEST', marginX + 6.0, cursorY + 11.0, 'bold', 6.8, palette.inkStrong);
          p.line(marginX + 6.0, cursorY + 15.0, marginX + contentW - 6.0, cursorY + 15.0, 0.4, palette.border);
          p.text(
            truncateToWidthPt(`Signer: ${block.signer_identity}`, 'regular', 6.2, contentW - 12.0),
            marginX + 6.0,
            cursorY + 25.0,
            'regular',
            6.2,
            palette.inkBody
          );
          p.text(
            truncateToWidthPt(`SHA-256: ${block.document_sha256}`, 'mono', 5.8, contentW - 12.0),
            marginX + 6.0,
            cursorY + 36.0,
            'mono',
            5.8,
            palette.inkMuted
          );
          p.text(
            truncateToWidthPt(`Key: ${block.public_key_hex}`, 'mono', 5.8, contentW - 12.0),
            marginX + 6.0,
            cursorY + 47.0,
            'mono',
            5.8,
            palette.inkMuted
          );
          const sigShort =
            block.signature_hex.length > 32
              ? `Sig: ${block.signature_hex.slice(0, 32)}...`
              : `Sig: ${block.signature_hex}`;
          p.text(truncateToWidthPt(sigShort, 'mono', 5.8, contentW - 12.0), marginX + 6.0, cursorY + 58.0, 'mono', 5.8, palette.inkMuted);
          p.text(
            truncateToWidthPt(`${algorithm} · ${block.timestamp_iso}`, 'regular', 5.8, contentW - 12.0),
            marginX + 6.0,
            cursorY + 68.0,
            'regular',
            5.8,
            palette.inkMuted
          );
          cursorY += boxH + 8.0;
        } else {
          const boxH = 58.0;
          ensureSpace(boxH);
          const p = currentPage();
          p.fillAndStrokeRect(marginX, cursorY, contentW, boxH, palette.cardBg, 0.6, palette.border);

          // Header row
          p.text('CRYPTOGRAPHIC AUDIT MANIFEST', marginX + 10.0, cursorY + 13.0, 'bold', 7.4, palette.inkStrong);
          p.textRight(`ALGORITHM: ${algorithm}`, marginX + contentW - 10.0, cursorY + 13.0, 'bold', 7.0, palette.accent);
          p.line(marginX + 10.0, cursorY + 18.0, marginX + contentW - 10.0, cursorY + 18.0, 0.4, palette.border);

          // Line 1: Signer & Timestamp
          p.text(
            truncateToWidthPt(`Signer: ${block.signer_identity}`, 'bold', 6.8, contentW * 0.65),
            marginX + 10.0,
            cursorY + 28.5,
            'bold',
            6.8,
            palette.inkBody
          );
          p.textRight(
            truncateToWidthPt(block.timestamp_iso, 'regular', 6.8, contentW * 0.3),
            marginX + contentW - 10.0,
            cursorY + 28.5,
            'regular',
            6.8,
            palette.inkMuted
          );

          // Line 2: Document SHA-256
          p.text(
            truncateToWidthPt(`Doc SHA-256: ${block.document_sha256}`, 'mono', 6.5, contentW - 20.0),
            marginX + 10.0,
            cursorY + 38.5,
            'mono',
            6.5,
            palette.inkStrong
          );

          // Line 3: Public Key & Signature
          const sigPreview =
            block.signature_hex.length > 36
              ? `${block.signature_hex.slice(0, 36)}...`
              : block.signature_hex;
          p.text(
            truncateToWidthPt(`Ed25519 PubKey: ${block.public_key_hex}`, 'mono', 6.3, contentW * 0.55),
            marginX + 10.0,
            cursorY + 48.5,
            'mono',
            6.3,
            palette.inkMuted
          );
          p.textRight(
            truncateToWidthPt(`Sig: ${sigPreview}`, 'mono', 6.3, contentW * 0.42),
            marginX + contentW - 10.0,
            cursorY + 48.5,
            'mono',
            6.3,
            palette.inkMuted
          );

          cursorY += boxH + 8.0;
        }
        break;
      }
    }
  }

  const total = pages.length;
  const tpl = spec.metadata.page_counter_template ?? 'Page {current} of {total}';
  const defaultNotice = `${spec.metadata.issuer_name}  •  Ref ${spec.metadata.document_code}  •  ${spec.metadata.issued_at_label}`;
  const notice = spec.metadata.footer_notice ?? defaultNotice;

  pages.forEach((p, idx) => {
    p.line(marginX, footerTop, pageW - marginX, footerTop, 0.55, palette.border);
    const pageStr = tpl
      .replace('{current}', String(idx + 1))
      .replace('{total}', String(total));

    if (isThermal) {
      p.textCenter(
        truncateToWidthPt(`${spec.metadata.document_code} • ${pageStr}`, 'regular', 6.8, contentW),
        pageW / 2,
        footerTop + 13,
        'regular',
        6.8,
        palette.inkMuted
      );
    } else {
      p.text(
        truncateToWidthPt(notice, 'regular', 7.3, contentW * 0.76),
        marginX,
        footerTop + 14,
        'regular',
        7.3,
        palette.inkMuted
      );
      p.textRight(
        truncateToWidthPt(pageStr, 'bold', 7.5, contentW * 0.22),
        pageW - marginX,
        footerTop + 14,
        'bold',
        7.5,
        palette.inkMuted
      );
    }
  });

  return assemblePdf14(pages, jpegs);
}

export function renderDocumentJson(
  jsonTextOrBytes: string | Uint8Array,
  limits: SecurityLimits = DEFAULT_SECURITY_LIMITS
): Uint8Array {
  const bytes =
    typeof jsonTextOrBytes === 'string'
      ? new TextEncoder().encode(jsonTextOrBytes)
      : jsonTextOrBytes;

  if (bytes.byteLength > limits.maxInputBytes) {
    throw new MsfPdfError('LIMIT_EXCEEDED', 'JSON payload exceeds maxInputBytes ceiling');
  }

  let parsed: DocumentSpec;
  try {
    const str =
      typeof jsonTextOrBytes === 'string'
        ? jsonTextOrBytes
        : new TextDecoder().decode(bytes);
    parsed = JSON.parse(str) as DocumentSpec;
  } catch (err) {
    throw new MsfPdfError(
      'INVALID_SPEC',
      `malformed JSON specification: ${err instanceof Error ? err.message : String(err)}`
    );
  }

  return renderDocument(parsed, limits);
}
