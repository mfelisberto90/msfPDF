import { MsfPdfError } from './types.ts';

export interface QrMatrix {
  readonly size: number;
  readonly modules: Uint8Array;
}

interface VersionSpec {
  readonly version: number;
  readonly dataCodewords: number;
  readonly ecCodewordsPerBlock: number;
  readonly numBlocks: number;
  readonly alignCenters: readonly number[];
}

const VERSIONS_ECC_L: readonly VersionSpec[] = [
  { version: 1, dataCodewords: 19, ecCodewordsPerBlock: 7, numBlocks: 1, alignCenters: [] },
  { version: 2, dataCodewords: 34, ecCodewordsPerBlock: 10, numBlocks: 1, alignCenters: [6, 18] },
  { version: 3, dataCodewords: 55, ecCodewordsPerBlock: 15, numBlocks: 1, alignCenters: [6, 22] },
  { version: 4, dataCodewords: 80, ecCodewordsPerBlock: 20, numBlocks: 1, alignCenters: [6, 26] },
  { version: 5, dataCodewords: 108, ecCodewordsPerBlock: 26, numBlocks: 1, alignCenters: [6, 30] },
  { version: 6, dataCodewords: 136, ecCodewordsPerBlock: 18, numBlocks: 2, alignCenters: [6, 34] },
];

function gfMul(x: number, y: number): number {
  let a = x & 0xff;
  let b = y & 0xff;
  let z = 0;
  for (let i = 0; i < 8; i++) {
    if ((b & 1) !== 0) z ^= a;
    const hi = (a & 0x80) !== 0;
    a = (a << 1) & 0xff;
    if (hi) a ^= 0x1d;
    b >>= 1;
  }
  return z;
}

function rsDivisor(degree: number): Uint8Array {
  const result = new Uint8Array(degree);
  result[degree - 1] = 1;
  let root = 1;
  for (let i = 0; i < degree; i++) {
    for (let j = 0; j < degree; j++) {
      result[j] = gfMul(result[j], root);
      if (j + 1 < degree) result[j] ^= result[j + 1];
    }
    root = gfMul(root, 0x02);
  }
  return result;
}

function rsRemainder(data: Uint8Array, divisor: Uint8Array): Uint8Array {
  const result = new Uint8Array(divisor.length);
  for (let i = 0; i < data.length; i++) {
    const factor = data[i] ^ result[0];
    result.copyWithin(0, 1);
    result[result.length - 1] = 0;
    for (let j = 0; j < divisor.length; j++) {
      result[j] ^= gfMul(divisor[j], factor);
    }
  }
  return result;
}

function maskBit(mask: number, r: number, c: number): boolean {
  switch (mask) {
    case 0:
      return (r + c) % 2 === 0;
    case 1:
      return r % 2 === 0;
    case 2:
      return c % 3 === 0;
    case 3:
      return (r + c) % 3 === 0;
    case 4:
      return (Math.floor(r / 2) + Math.floor(c / 3)) % 2 === 0;
    case 5:
      return ((r * c) % 2) + ((r * c) % 3) === 0;
    case 6:
      return (((r * c) % 2) + ((r * c) % 3)) % 2 === 0;
    default:
      return (((r + c) % 2) + ((r * c) % 3)) % 2 === 0;
  }
}

function formatBitsEccL(mask: number): number {
  const data = (0b01 << 3) | (mask & 0x07);
  let rem = data << 10;
  for (let i = 4; i >= 0; i--) {
    if (((rem >> (i + 10)) & 1) !== 0) {
      rem ^= 0x537 << i;
    }
  }
  return ((data << 10) | rem) ^ 0x5412;
}

function drawFormatBits(
  modules: Uint8Array,
  isFunc: Uint8Array,
  size: number,
  mask: number
): void {
  const bits = formatBitsEccL(mask);
  const getB = (idx: number): number => (bits >> idx) & 1;

  const tlCoords: ReadonlyArray<readonly [number, number, number]> = [
    [8, 0, 14],
    [8, 1, 13],
    [8, 2, 12],
    [8, 3, 11],
    [8, 4, 10],
    [8, 5, 9],
    [8, 7, 8],
    [8, 8, 7],
    [7, 8, 6],
    [5, 8, 5],
    [4, 8, 4],
    [3, 8, 3],
    [2, 8, 2],
    [1, 8, 1],
    [0, 8, 0],
  ];

  for (const [r, c, bIdx] of tlCoords) {
    modules[r * size + c] = getB(bIdx);
    isFunc[r * size + c] = 1;
  }

  for (let i = 0; i < 7; i++) {
    const r = size - 1 - i;
    modules[r * size + 8] = getB(14 - i);
    isFunc[r * size + 8] = 1;
  }
  modules[(size - 8) * size + 8] = 1;
  isFunc[(size - 8) * size + 8] = 1;

  for (let i = 0; i < 8; i++) {
    const c = size - 8 + i;
    modules[8 * size + c] = getB(7 - i);
    isFunc[8 * size + c] = 1;
  }
}

function penaltyScore(modules: Uint8Array, size: number): number {
  let score = 0;
  for (let r = 0; r < size; r++) {
    let runVal = modules[r * size];
    let runLen = 1;
    for (let c = 1; c < size; c++) {
      const v = modules[r * size + c];
      if (v === runVal) runLen++;
      else {
        if (runLen >= 5) score += 3 + (runLen - 5);
        runVal = v;
        runLen = 1;
      }
    }
    if (runLen >= 5) score += 3 + (runLen - 5);
  }

  for (let c = 0; c < size; c++) {
    let runVal = modules[c];
    let runLen = 1;
    for (let r = 1; r < size; r++) {
      const v = modules[r * size + c];
      if (v === runVal) runLen++;
      else {
        if (runLen >= 5) score += 3 + (runLen - 5);
        runVal = v;
        runLen = 1;
      }
    }
    if (runLen >= 5) score += 3 + (runLen - 5);
  }

  for (let r = 0; r < size - 1; r++) {
    for (let c = 0; c < size - 1; c++) {
      const v = modules[r * size + c];
      if (
        modules[r * size + c + 1] === v &&
        modules[(r + 1) * size + c] === v &&
        modules[(r + 1) * size + c + 1] === v
      ) {
        score += 3;
      }
    }
  }
  return score;
}

export function encodeQr(payload: string): QrMatrix {
  const raw = new TextEncoder().encode(payload);
  if (raw.length === 0) {
    throw new MsfPdfError('INVALID_SPEC', 'QR payload cannot be empty');
  }

  const spec = VERSIONS_ECC_L.find((s) => 12 + raw.length * 8 <= s.dataCodewords * 8);
  if (!spec) {
    throw new MsfPdfError(
      'QR_CAPACITY_EXCEEDED',
      `QR payload (${raw.length} bytes) exceeds version 6 capacity`
    );
  }

  const bits: number[] = [];
  const pushBits = (val: number, len: number): void => {
    for (let i = len - 1; i >= 0; i--) {
      bits.push((val >> i) & 1);
    }
  };

  pushBits(0b0100, 4);
  pushBits(raw.length, 8);
  for (const b of raw) pushBits(b, 8);

  const capacityBits = spec.dataCodewords * 8;
  const term = Math.min(4, capacityBits - bits.length);
  pushBits(0, term);
  while (bits.length % 8 !== 0) bits.push(0);

  const dataBytes = new Uint8Array(spec.dataCodewords);
  const byteLen = bits.length / 8;
  for (let i = 0; i < byteLen; i++) {
    let val = 0;
    for (let b = 0; b < 8; b++) val = (val << 1) | bits[i * 8 + b];
    dataBytes[i] = val;
  }

  const pads = [0xec, 0x11];
  for (let i = byteLen; i < spec.dataCodewords; i++) {
    dataBytes[i] = pads[(i - byteLen) % 2];
  }

  const divisor = rsDivisor(spec.ecCodewordsPerBlock);
  const blockDataLen = spec.dataCodewords / spec.numBlocks;
  const dataBlocks: Uint8Array[] = [];
  const ecBlocks: Uint8Array[] = [];

  for (let b = 0; b < spec.numBlocks; b++) {
    const slice = dataBytes.subarray(b * blockDataLen, (b + 1) * blockDataLen);
    dataBlocks.push(slice);
    ecBlocks.push(rsRemainder(slice, divisor));
  }

  const allCodewords = new Uint8Array(
    spec.dataCodewords + spec.ecCodewordsPerBlock * spec.numBlocks
  );
  let ptr = 0;
  for (let i = 0; i < blockDataLen; i++) {
    for (const blk of dataBlocks) allCodewords[ptr++] = blk[i];
  }
  for (let i = 0; i < spec.ecCodewordsPerBlock; i++) {
    for (const ec of ecBlocks) allCodewords[ptr++] = ec[i];
  }

  const size = spec.version * 4 + 17;
  const baseModules = new Uint8Array(size * size);
  const isFunc = new Uint8Array(size * size);
  const setFunc = (r: number, c: number, dark: boolean): void => {
    baseModules[r * size + c] = dark ? 1 : 0;
    isFunc[r * size + c] = 1;
  };

  const drawFinder = (topR: number, leftC: number): void => {
    for (let dr = -1; dr <= 7; dr++) {
      for (let dc = -1; dc <= 7; dc++) {
        const rr = topR + dr;
        const cc = leftC + dc;
        if (rr >= 0 && rr < size && cc >= 0 && cc < size) {
          const inOuter = dr >= 0 && dr <= 6 && dc >= 0 && dc <= 6;
          const onRing = dr === 0 || dr === 6 || dc === 0 || dc === 6;
          const inCore = dr >= 2 && dr <= 4 && dc >= 2 && dc <= 4;
          setFunc(rr, cc, inOuter && (onRing || inCore));
        }
      }
    }
  };

  drawFinder(0, 0);
  drawFinder(0, size - 7);
  drawFinder(size - 7, 0);

  for (let i = 8; i < size - 8; i++) {
    setFunc(6, i, i % 2 === 0);
    setFunc(i, 6, i % 2 === 0);
  }

  const nAlign = spec.alignCenters.length;
  for (let ri = 0; ri < nAlign; ri++) {
    for (let ci = 0; ci < nAlign; ci++) {
      if (
        (ri === 0 && ci === 0) ||
        (ri === 0 && ci === nAlign - 1) ||
        (ri === nAlign - 1 && ci === 0)
      ) {
        continue;
      }
      const cr = spec.alignCenters[ri];
      const cc = spec.alignCenters[ci];
      for (let dr = -2; dr <= 2; dr++) {
        for (let dc = -2; dc <= 2; dc++) {
          setFunc(cr + dr, cc + dc, Math.max(Math.abs(dr), Math.abs(dc)) !== 1);
        }
      }
    }
  }

  drawFormatBits(baseModules, isFunc, size, 0);

  let bitIdx = 0;
  const totalBits = allCodewords.length * 8;
  let rightCol = size - 1;
  let upward = true;

  while (rightCol >= 1) {
    if (rightCol === 6) rightCol -= 1;
    for (let step = 0; step < size; step++) {
      const r = upward ? size - 1 - step : step;
      for (let dc = 0; dc < 2; dc++) {
        const c = rightCol - dc;
        if (isFunc[r * size + c] === 0) {
          let dark = 0;
          if (bitIdx < totalBits) {
            const byte = allCodewords[bitIdx >> 3];
            dark = (byte >> (7 - (bitIdx & 7))) & 1;
          }
          baseModules[r * size + c] = dark;
          bitIdx++;
        }
      }
    }
    upward = !upward;
    rightCol -= 2;
  }

  let bestModules = baseModules;
  let bestScore = Number.MAX_SAFE_INTEGER;

  for (let mask = 0; mask < 8; mask++) {
    const candidate = new Uint8Array(baseModules);
    for (let r = 0; r < size; r++) {
      for (let c = 0; c < size; c++) {
        const idx = r * size + c;
        if (isFunc[idx] === 0 && maskBit(mask, r, c)) {
          candidate[idx] ^= 1;
        }
      }
    }
    const dummyFunc = new Uint8Array(isFunc);
    drawFormatBits(candidate, dummyFunc, size, mask);
    const score = penaltyScore(candidate, size);
    if (score < bestScore) {
      bestScore = score;
      bestModules = candidate;
    }
  }

  return { size, modules: bestModules };
}
