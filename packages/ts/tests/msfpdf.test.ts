import assert from 'node:assert/strict';
import fs from 'node:fs/promises';
import os from 'node:os';
import path from 'node:path';
import test from 'node:test';
import { fileURLToPath } from 'node:url';

import {
  DEFAULT_SECURITY_LIMITS,
  MsfPdfError,
  embedAuditManifest,
  encodeQr,
  extractAuditManifest,
  formatMoneyMinor,
  generateKeyPair,
  pdfEscapeWinAnsi,
  renderDocument,
  renderDocumentJson,
  shapeAndReorderLine,
  shapeArabicLogical,
  signPdf,
  verifyManifest,
  verifyPdf,
} from '../src/index.ts';
import { TenantPdfVault } from '../src/vault.ts';

const ROOT_DIR = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '../../..');

function assertValidXrefAnd7BitAscii(pdf: Uint8Array): string {
  for (let i = 0; i < pdf.byteLength; i++) {
    const b = pdf[i];
    assert.ok(
      b === 0x0a || (b >= 0x20 && b <= 0x7e),
      `non-ASCII byte 0x${b.toString(16)} at offset ${i} risks drifting xref table`
    );
  }
  const text = new TextDecoder('ascii').decode(pdf);
  assert.ok(text.startsWith('%PDF-1.4\n'), 'PDF must start with %PDF-1.4 header');
  assert.ok(text.endsWith('%%EOF\n'), 'PDF must end with %%EOF trailer');

  const xrefPos = text.lastIndexOf('\nxref\n') + 1;
  assert.ok(xrefPos > 0, 'xref table must exist');
  const startxrefMarker = text.lastIndexOf('startxref\n');
  const declaredXref = Number(
    text
      .slice(startxrefMarker + 'startxref\n'.length)
      .split('\n')[0]
      .trim()
  );
  assert.equal(xrefPos, declaredXref, 'startxref must match exact byte offset of xref');

  const xrefLines = text.slice(xrefPos, startxrefMarker).split('\n').slice(3);
  let objId = 1;
  for (const line of xrefLines) {
    if (!line || line.startsWith('trailer')) break;
    const offset = Number(line.slice(0, 10));
    const expected = `${objId} 0 obj\n`;
    assert.ok(
      text.slice(offset).startsWith(expected),
      `xref object ${objId} at byte ${offset} did not match '${expected.trim()}'`
    );
    objId++;
  }
  return text;
}

test('encodes CP1252 currencies, European accents, and blocks PDF stream operator injection', () => {
  const hostile = 'Vendor ) Tj ET BT /F2 36 Tf (INJECTED — £5,130.00 • €99.50 • São Paulo m² nº 1ª';
  const escaped = pdfEscapeWinAnsi(hostile);
  assert.ok(escaped.includes('Vendor \\) Tj ET BT /F2 36 Tf \\(INJECTED'));
  assert.ok(escaped.includes('\\2435,130.00'));
  assert.ok(escaped.includes('\\20099.50'));
  assert.ok(escaped.includes('S\\343o Paulo'));
  assert.ok(escaped.includes('m\\262'));
});

test('enforces exact integer minor units in formatMoneyMinor and rejects binary floats', () => {
  assert.equal(formatMoneyMinor(513_000, 'GBP'), '£5,130.00');
  assert.equal(formatMoneyMinor(-22_500, 'GBP'), '-£225.00');
  assert.equal(formatMoneyMinor(1_650_000, 'EUR'), '€16,500.00');
  assert.equal(formatMoneyMinor(20_000_000, 'BRL'), 'R$ 200.000,00');
  assert.equal(formatMoneyMinor(12_800, 'JPY'), '¥12,800');
  assert.equal(formatMoneyMinor(45_000_050, 'RUB'), '450 000,50 ₽');
  assert.equal(formatMoneyMinor(1_942_500, 'AED'), 'AED 19,425.00');
  assert.equal(formatMoneyMinor(750_000, 'SAR'), 'SAR 7,500.00');

  assert.throws(
    () => formatMoneyMinor(19.99, 'GBP'),
    (err: unknown) => err instanceof MsfPdfError && err.code === 'INVALID_SPEC'
  );
});

test('shapes Arabic RTL ligatures and renders Russian Cyrillic & Arabic in pure 7-bit ASCII Type 3', () => {
  const logical = 'فاتورة للأنظمة رقم 8841';
  const shaped = shapeArabicLogical(logical);
  assert.ok(shaped.includes('\uFEF8'), 'must form final Lam-Alef-Hamza ligature');
  const visual = shapeAndReorderLine(logical);
  assert.ok(visual.startsWith('8841 '), 'numbers in RTL line must stay LTR on left visual end');
});

test('compiles all canonical JSON example specs with valid xref tables and vector QR matrices', async () => {
  const examples = [
    'uk-b2b-invoice.json',
    'global-multilingual-invoice.json',
    'commercial-contract-multipage.json',
    'pos-thermal-receipt.json',
  ];

  for (const name of examples) {
    const raw = await fs.readFile(path.join(ROOT_DIR, 'examples', name), 'utf8');
    const pdf = renderDocumentJson(raw);
    const ascii = assertValidXrefAnd7BitAscii(pdf);
    if (name === 'uk-b2b-invoice.json') {
      assert.ok(!ascii.includes('/Subtype /Type3'), 'pure Latin invoice must emit zero Type 3 fonts');
    }
    if (name === 'global-multilingual-invoice.json') {
      assert.ok(ascii.includes('/Subtype /Type3'), 'multilingual invoice must emit Type 3 subsets');
      assert.ok(ascii.includes('/ToUnicode'), 'multilingual invoice must emit /ToUnicode CMap');
    }
    if (name === 'commercial-contract-multipage.json') {
      assert.ok(
        ascii.includes('/Count 2') || ascii.includes('/Count 3'),
        'multi-clause contract must paginate across multiple pages'
      );
    }
  }

  const qr = encodeQr('https://verify.caledoniacloud.example/inv/INV-2026-8841');
  assert.ok(qr.size >= 21 && qr.size <= 41);
});

test('enforces pagination DoS ceilings and isolates multi-tenant vault files atomically', async () => {
  const hugeParagraphs = Array.from(
    { length: 140 },
    (_, i) => `Clause ${i + 1}: Extended obligation text designed to exceed a 2-page ceiling.`
  );

  assert.throws(
    () =>
      renderDocument(
        {
          metadata: {
            issuer_name: 'Test Issuer',
            document_code: 'DOC-01',
            issued_at_label: '2026-11-02',
          },
          blocks: [{ type: 'clause_list', paragraphs: hugeParagraphs }],
        },
        { ...DEFAULT_SECURITY_LIMITS, maxPages: 2 }
      ),
    (err: unknown) => err instanceof MsfPdfError && err.code === 'LIMIT_EXCEEDED'
  );

  const tmpRoot = await fs.mkdtemp(path.join(os.tmpdir(), 'msfpdf-ts-vault-'));
  try {
    const vault = new TenantPdfVault(tmpRoot, 'invoice');
    const issued = await vault.issueDocument('tenant_scotland_01', {
      metadata: {
        issuer_name: 'Caledonia Cloud Systems Ltd',
        document_code: 'INV-1',
        issued_at_label: '02 Nov 2026',
      },
      blocks: [
        {
          type: 'callout_banner',
          title: 'SETTLED',
          subtitle: 'Faster Payments',
          right_value: '£100.00',
        },
      ],
    });

    assert.equal(issued.token.length, 40);
    assertValidXrefAnd7BitAscii(issued.bytes);

    const readBack = await vault.readDocument('tenant_scotland_01', issued.token);
    assert.deepEqual(readBack, issued.bytes);

    await assert.rejects(() => vault.readDocument('tenant_other_02', issued.token));
    await assert.rejects(() => vault.readDocument('../etc', issued.token));
    await assert.rejects(() => vault.readDocument('tenant_scotland_01', '../passwd'));
    await assert.rejects(() => vault.readDocument('tenant_scotland_01', '000001'));
  } finally {
    await fs.rm(tmpRoot, { recursive: true, force: true });
  }
});

test('signs and verifies PDF with Ed25519 and audit manifest', () => {
  const { seed, publicKey, seedHex, publicKeyHex } = generateKeyPair();
  assert.equal(seed.length, 32);
  assert.equal(publicKey.length, 32);
  assert.equal(seedHex.length, 64);
  assert.equal(publicKeyHex.length, 64);

  const spec = {
    metadata: {
      issuer_name: 'Meridian Clearing Ltd',
      document_code: 'INV-ED25519-TS-01',
      issued_at_label: '03 Nov 2026',
    },
    blocks: [
      {
        type: 'hero_header' as const,
        kicker: 'FINTECH AUDIT SPECIMEN',
        title: 'Settlement Invoice',
        subtitle: 'London · Edinburgh · Dubai',
        badge_label: 'AUDITED',
        badge_code: '#9042',
      },
      {
        type: 'audit_manifest' as const,
        signer_identity: 'Meridian Clearing House Ltd (UK-EDINBURGH-01)',
        public_key_hex: publicKeyHex,
        algorithm: 'Ed25519',
        document_sha256: '553ef20b2245c7470c591cc93c724bef793b9894e29167cfe4546304f60ea006',
        signature_hex: '3a8f'.repeat(32),
        timestamp_iso: '2026-11-03T12:00:00Z',
      },
    ],
  };

  const pdf = renderDocument(spec);
  assertValidXrefAnd7BitAscii(pdf);

  // Sign PDF with real Ed25519 keypair
  const manifest = signPdf(
    pdf,
    seed,
    'Meridian Clearing House Ltd',
    '2026-11-03T12:00:00Z'
  );

  assert.equal(manifest.algorithm, 'Ed25519');
  assert.equal(manifest.document_sha256.length, 64);
  assert.equal(manifest.public_key_hex.length, 64);
  assert.equal(manifest.signature_hex.length, 128);

  // Verify valid signature
  const isValid = verifyManifest(pdf, manifest);
  assert.ok(isValid, 'signature must verify over authentic PDF bytes');
  const isDirectValid = verifyPdf(
    pdf,
    publicKey,
    Buffer.from(manifest.signature_hex, 'hex')
  );
  assert.ok(isDirectValid, 'direct verifyPdf must pass');

  // Embed manifest in trailer and extract
  const embeddedPdf = embedAuditManifest(pdf, manifest);
  assert.ok(embeddedPdf.length > pdf.length);
  const extracted = extractAuditManifest(embeddedPdf);
  assert.deepEqual(extracted, manifest);
  const isEmbeddedValid = verifyManifest(embeddedPdf, extracted!);
  assert.ok(isEmbeddedValid, 'embedded manifest must verify base PDF');

  // Tamper test: flip a single byte in PDF stream, must fail verification
  const tamperedPdf = new Uint8Array(embeddedPdf);
  tamperedPdf[120] ^= 0x01;
  const isTamperedValid = verifyManifest(tamperedPdf, manifest);
  assert.ok(!isTamperedValid, 'tampered PDF must fail signature check');
});
