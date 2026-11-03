/**
 * Zero-dependency Arabic contextual glyph shaping (Isolated/Final/Initial/Medial + Lam-Alef ligatures)
 * and UAX #9 bidirectional (BiDi) visual line reordering (`levels 0..2`).
 */

type JoiningEntry = readonly [boolean, readonly [number, number, number, number]];

const ARABIC_JOINING: ReadonlyMap<number, JoiningEntry> = new Map<number, JoiningEntry>([
  // Right-only joiners
  [0x0622, [false, [0xfe81, 0xfe82, 0xfe81, 0xfe82]]],
  [0x0623, [false, [0xfe83, 0xfe84, 0xfe83, 0xfe84]]],
  [0x0624, [false, [0xfe85, 0xfe86, 0xfe85, 0xfe86]]],
  [0x0625, [false, [0xfe87, 0xfe88, 0xfe87, 0xfe88]]],
  [0x0627, [false, [0xfe8d, 0xfe8e, 0xfe8d, 0xfe8e]]],
  [0x0629, [false, [0xfe93, 0xfe94, 0xfe93, 0xfe94]]],
  [0x062f, [false, [0xfea9, 0xfeaa, 0xfea9, 0xfeaa]]],
  [0x0630, [false, [0xfeab, 0xfeac, 0xfeab, 0xfeac]]],
  [0x0631, [false, [0xfead, 0xfeae, 0xfead, 0xfeae]]],
  [0x0632, [false, [0xfeaf, 0xfeb0, 0xfeaf, 0xfeb0]]],
  [0x0648, [false, [0xfeed, 0xfeee, 0xfeed, 0xfeee]]],
  [0x0649, [false, [0xfeef, 0xfef0, 0xfeef, 0xfef0]]],
  [0x0698, [false, [0xfb8a, 0xfb8b, 0xfb8a, 0xfb8b]]],
  // Dual joiners
  [0x0626, [true, [0xfe89, 0xfe8a, 0xfe8b, 0xfe8c]]],
  [0x0628, [true, [0xfe8f, 0xfe90, 0xfe91, 0xfe92]]],
  [0x062a, [true, [0xfe95, 0xfe96, 0xfe97, 0xfe98]]],
  [0x062b, [true, [0xfe99, 0xfe9a, 0xfe9b, 0xfe9c]]],
  [0x062c, [true, [0xfe9d, 0xfe9e, 0xfe9f, 0xfea0]]],
  [0x062d, [true, [0xfea1, 0xfea2, 0xfea3, 0xfea4]]],
  [0x062e, [true, [0xfea5, 0xfea6, 0xfea7, 0xfea8]]],
  [0x0633, [true, [0xfeb1, 0xfeb2, 0xfeb3, 0xfeb4]]],
  [0x0634, [true, [0xfeb5, 0xfeb6, 0xfeb7, 0xfeb8]]],
  [0x0635, [true, [0xfeb9, 0xfeba, 0xfebb, 0xfebc]]],
  [0x0636, [true, [0xfebd, 0xfebe, 0xfebf, 0xfec0]]],
  [0x0637, [true, [0xfec1, 0xfec2, 0xfec3, 0xfec4]]],
  [0x0638, [true, [0xfec5, 0xfec6, 0xfec7, 0xfec8]]],
  [0x0639, [true, [0xfec9, 0xfeca, 0xfecb, 0xfecc]]],
  [0x063a, [true, [0xfecd, 0xfece, 0xfecf, 0xfed0]]],
  [0x0640, [true, [0x0640, 0x0640, 0x0640, 0x0640]]],
  [0x0641, [true, [0xfed1, 0xfed2, 0xfed3, 0xfed4]]],
  [0x0642, [true, [0xfed5, 0xfed6, 0xfed7, 0xfed8]]],
  [0x0643, [true, [0xfed9, 0xfeda, 0xfedb, 0xfedc]]],
  [0x0644, [true, [0xfedd, 0xfede, 0xfedf, 0xfee0]]],
  [0x0645, [true, [0xfee1, 0xfee2, 0xfee3, 0xfee4]]],
  [0x0646, [true, [0xfee5, 0xfee6, 0xfee7, 0xfee8]]],
  [0x0647, [true, [0xfee9, 0xfeea, 0xfeeb, 0xfeec]]],
  [0x064a, [true, [0xfef1, 0xfef2, 0xfef3, 0xfef4]]],
  [0x067e, [true, [0xfb56, 0xfb57, 0xfb58, 0xfb59]]],
  [0x0686, [true, [0xfb7a, 0xfb7b, 0xfb7c, 0xfb7d]]],
  [0x06a9, [true, [0xfb8e, 0xfb8f, 0xfb90, 0xfb91]]],
  [0x06af, [true, [0xfb92, 0xfb93, 0xfb94, 0xfb95]]],
  [0x06cc, [true, [0xfbfc, 0xfbfd, 0xfbfe, 0xfbff]]],
]);

function isIgnorableArabicMark(cp: number): boolean {
  return (
    (cp >= 0x064b && cp <= 0x065f) ||
    cp === 0x0670 ||
    (cp >= 0x06d6 && cp <= 0x06ed) ||
    cp === 0x061c ||
    cp === 0x200e ||
    cp === 0x200f ||
    (cp >= 0x202a && cp <= 0x202e) ||
    (cp >= 0x2066 && cp <= 0x2069)
  );
}

function lamAlefLigature(alefCp: number, prevJoinsLeft: boolean): number | undefined {
  switch (alefCp) {
    case 0x0622:
      return prevJoinsLeft ? 0xfef6 : 0xfef5;
    case 0x0623:
      return prevJoinsLeft ? 0xfef8 : 0xfef7;
    case 0x0625:
      return prevJoinsLeft ? 0xfefa : 0xfef9;
    case 0x0627:
      return prevJoinsLeft ? 0xfefc : 0xfefb;
    default:
      return undefined;
  }
}

export function hasArabicOrRtl(input: string): boolean {
  for (const ch of input) {
    const cp = ch.codePointAt(0) ?? 0;
    if (
      (cp >= 0x0590 && cp <= 0x08ff) ||
      (cp >= 0xfb50 && cp <= 0xfdff) ||
      (cp >= 0xfe70 && cp <= 0xfeff)
    ) {
      return true;
    }
  }
  return false;
}

export function shapeArabicLogical(input: string): string {
  if (!hasArabicOrRtl(input)) return input;

  const cps: number[] = [];
  for (const ch of input) {
    const cp = ch.codePointAt(0) ?? 0x20;
    if (cp === 0xfdfc) {
      cps.push(0x0631, 0x064a, 0x0627, 0x0644);
    } else if (!isIgnorableArabicMark(cp)) {
      cps.push(cp);
    }
  }

  let out = '';
  let prevJoinsLeft = false;
  let i = 0;
  const n = cps.length;

  while (i < n) {
    const cp = cps[i];
    if (cp === 0x200c) {
      prevJoinsLeft = false;
      i += 1;
      continue;
    }
    if (cp === 0x200d) {
      prevJoinsLeft = true;
      i += 1;
      continue;
    }
    if (cp === 0x0621) {
      out += String.fromCodePoint(0xfe80);
      prevJoinsLeft = false;
      i += 1;
      continue;
    }
    if (cp === 0x0644 && i + 1 < n) {
      const lig = lamAlefLigature(cps[i + 1], prevJoinsLeft);
      if (lig !== undefined) {
        out += String.fromCodePoint(lig);
        prevJoinsLeft = false;
        i += 2;
        continue;
      }
    }

    const entry = ARABIC_JOINING.get(cp);
    if (entry !== undefined) {
      const [curJoinsLeft, forms] = entry;
      const nextJoinsRight =
        i + 1 < n ? cps[i + 1] === 0x200d || ARABIC_JOINING.has(cps[i + 1]) : false;
      const right = prevJoinsLeft;
      const left = curJoinsLeft && nextJoinsRight;
      const idx = !right && !left ? 0 : right && !left ? 1 : !right && left ? 2 : 3;
      out += String.fromCodePoint(forms[idx]);
      prevJoinsLeft = curJoinsLeft;
    } else {
      out += String.fromCodePoint(cp);
      prevJoinsLeft = false;
    }

    i += 1;
  }

  return out;
}

type BidiClass = 'L' | 'R' | 'Num' | 'Neutral';

function classifyBidi(cp: number): BidiClass {
  if (
    (cp >= 0x0660 && cp <= 0x0669) ||
    (cp >= 0x06f0 && cp <= 0x06f9) ||
    (cp >= 0x0030 && cp <= 0x0039)
  ) {
    return 'Num';
  }
  if (
    (cp >= 0x0590 && cp <= 0x08ff) ||
    (cp >= 0xfb50 && cp <= 0xfdff) ||
    (cp >= 0xfe70 && cp <= 0xfeff)
  ) {
    return 'R';
  }
  if (
    (cp >= 0x0041 && cp <= 0x005a) ||
    (cp >= 0x0061 && cp <= 0x007a) ||
    (cp >= 0x00c0 && cp <= 0x02af) ||
    (cp >= 0x0370 && cp <= 0x052f) ||
    cp === 0x00a3 ||
    cp === 0x00a5 ||
    cp === 0x20ac ||
    cp === 0x20bd ||
    cp === 0x2116
  ) {
    return 'L';
  }
  return 'Neutral';
}

export function isRtlDominant(text: string): boolean {
  for (const ch of text) {
    const cls = classifyBidi(ch.codePointAt(0) ?? 0x20);
    if (cls === 'R') return true;
    if (cls === 'L') return false;
  }
  return false;
}

const RTL_MIRROR: Readonly<Record<string, string>> = {
  '(': ')',
  ')': '(',
  '[': ']',
  ']': '[',
  '{': '}',
  '}': '{',
  '<': '>',
  '>': '<',
  '«': '»',
  '»': '«',
};

export function shapeAndReorderLine(logicalLine: string): string {
  if (!hasArabicOrRtl(logicalLine)) return logicalLine;

  const shaped = shapeArabicLogical(logicalLine);
  const chars = Array.from(shaped);
  const n = chars.length;
  if (n === 0) return shaped;

  const baseRtl = isRtlDominant(shaped);
  const classes: BidiClass[] = chars.map((ch) => classifyBidi(ch.codePointAt(0) ?? 0x20));

  for (let i = 0; i < n; i++) {
    const ch = chars[i];
    if (
      (ch === '.' || ch === ',' || ch === '٫' || ch === '٬' || ch === ':' || ch === '/') &&
      i > 0 &&
      i + 1 < n &&
      classes[i - 1] === 'Num' &&
      classes[i + 1] === 'Num'
    ) {
      classes[i] = 'Num';
    } else if ((ch === '%' || ch === '٪') && i > 0 && classes[i - 1] === 'Num') {
      classes[i] = 'Num';
    } else if (
      (ch === '#' || ch === '+') &&
      i + 1 < n &&
      (classes[i + 1] === 'Num' || classes[i + 1] === 'L') &&
      (i === 0 || classes[i - 1] === 'Neutral')
    ) {
      classes[i] = classes[i + 1];
    } else if (
      ch === '-' &&
      i > 0 &&
      i + 1 < n &&
      (classes[i - 1] === 'Num' || classes[i - 1] === 'L') &&
      (classes[i + 1] === 'Num' || classes[i + 1] === 'L')
    ) {
      classes[i] = 'L';
    }
  }

  const baseLevel = baseRtl ? 1 : 0;
  const levels = new Uint8Array(n);
  levels.fill(baseLevel);
  let lastStrong: BidiClass = baseRtl ? 'R' : 'L';

  for (let i = 0; i < n; i++) {
    const cls = classes[i];
    if (cls === 'L') {
      levels[i] = baseRtl ? 2 : 0;
      lastStrong = 'L';
    } else if (cls === 'R') {
      levels[i] = 1;
      lastStrong = 'R';
    } else if (cls === 'Num') {
      levels[i] = baseRtl || lastStrong === 'R' ? 2 : 0;
    }
  }

  let i = 0;
  while (i < n) {
    if (classes[i] !== 'Neutral') {
      i += 1;
      continue;
    }
    const start = i;
    while (i < n && classes[i] === 'Neutral') i += 1;
    const leftLvl = start > 0 ? levels[start - 1] : baseLevel;
    const rightLvl = i < n ? levels[i] : baseLevel;
    const resolved = leftLvl === rightLvl ? leftLvl : baseLevel;
    for (let k = start; k < i; k++) levels[k] = resolved;
  }

  for (let idx = 0; idx < n; idx++) {
    if (levels[idx] % 2 === 1 && RTL_MIRROR[chars[idx]] !== undefined) {
      chars[idx] = RTL_MIRROR[chars[idx]];
    }
  }

  for (const targetLvl of [2, 1]) {
    let idx = 0;
    while (idx < n) {
      if (levels[idx] < targetLvl) {
        idx += 1;
        continue;
      }
      const start = idx;
      while (idx < n && levels[idx] >= targetLvl) idx += 1;
      let lo = start;
      let hi = idx - 1;
      while (lo < hi) {
        const tmpCh = chars[lo];
        chars[lo] = chars[hi];
        chars[hi] = tmpCh;
        const tmpLv = levels[lo];
        levels[lo] = levels[hi];
        levels[hi] = tmpLv;
        lo += 1;
        hi -= 1;
      }
    }
  }

  return chars.join('');
}
