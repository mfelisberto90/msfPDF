/**
 * Zero-dependency Ed25519 cryptographic signing and verification for msfPDF vector documents.
 * Produces and validates tamper-evident AuditManifests across Node.js 22+, Bun, and Deno.
 */

import crypto from 'node:crypto';
import type { AuditManifest } from './types.ts';
import { MsfPdfError } from './types.ts';

export const AUDIT_MARKER_PREFIX = '\n%MsfAuditManifest: ';
const ED25519_PKCS8_PREFIX = Buffer.from('302e020100300506032b657004220420', 'hex');
const ED25519_SPKI_PREFIX = Buffer.from('302a300506032b6570032100', 'hex');

/**
 * Computes the lowercase 64-character hexadecimal SHA-256 digest of a byte slice.
 */
export function computeSha256Hex(data: Uint8Array): string {
  return crypto.createHash('sha256').update(data).digest('hex');
}

/**
 * Converts a 32-byte raw seed into an Ed25519 KeyObject pair.
 */
function keyPairFromSeed(seed: Uint8Array): {
  privateKey: crypto.KeyObject;
  publicKey: crypto.KeyObject;
  rawPublicKey: Uint8Array;
} {
  if (seed.length !== 32) {
    throw new MsfPdfError('INVALID_SPEC', 'Ed25519 private seed must be exactly 32 bytes');
  }
  const pkcs8Der = Buffer.concat([ED25519_PKCS8_PREFIX, Buffer.from(seed)]);
  const privateKey = crypto.createPrivateKey({
    key: pkcs8Der,
    format: 'der',
    type: 'pkcs8',
  });
  const publicKey = crypto.createPublicKey(privateKey);
  const spkiDer = publicKey.export({ format: 'der', type: 'spki' });
  const rawPublicKey = new Uint8Array(spkiDer.subarray(spkiDer.length - 32));
  return { privateKey, publicKey, rawPublicKey };
}

/**
 * Converts a 32-byte raw public key into an Ed25519 KeyObject.
 */
function publicKeyFromRaw(rawPublicKey: Uint8Array): crypto.KeyObject {
  if (rawPublicKey.length !== 32) {
    throw new MsfPdfError('INVALID_SPEC', 'Ed25519 public key must be exactly 32 bytes');
  }
  const spkiDer = Buffer.concat([ED25519_SPKI_PREFIX, Buffer.from(rawPublicKey)]);
  return crypto.createPublicKey({
    key: spkiDer,
    format: 'der',
    type: 'spki',
  });
}

/**
 * Generates a fresh random Ed25519 keypair.
 */
export function generateKeyPair(): {
  seed: Uint8Array;
  publicKey: Uint8Array;
  seedHex: string;
  publicKeyHex: string;
} {
  const seed = crypto.randomBytes(32);
  const { rawPublicKey } = keyPairFromSeed(seed);
  return {
    seed: new Uint8Array(seed),
    publicKey: rawPublicKey,
    seedHex: Buffer.from(seed).toString('hex'),
    publicKeyHex: Buffer.from(rawPublicKey).toString('hex'),
  };
}

/**
 * Strips an appended `%MsfAuditManifest: ...` comment trailer to recover the exact base PDF bytes.
 */
export function stripEmbeddedAudit(pdfBytes: Uint8Array): Uint8Array {
  const marker = Buffer.from(AUDIT_MARKER_PREFIX, 'utf8');
  const buf = Buffer.from(pdfBytes);
  const pos = buf.lastIndexOf(marker);
  if (pos !== -1) {
    return new Uint8Array(pdfBytes.subarray(0, pos));
  }
  return pdfBytes;
}

/**
 * Signs a PDF buffer with an Ed25519 private key seed, returning an [`AuditManifest`].
 */
export function signPdf(
  pdfBytes: Uint8Array,
  privateKeySeed: Uint8Array,
  signerIdentity: string = 'msfPDF Signer',
  timestampIso?: string
): AuditManifest {
  const basePdf = stripEmbeddedAudit(pdfBytes);
  const docHashHex = computeSha256Hex(basePdf);

  const { privateKey, rawPublicKey } = keyPairFromSeed(privateKeySeed);
  const sig = crypto.sign(null, Buffer.from(docHashHex, 'utf8'), privateKey);

  return {
    algorithm: 'Ed25519',
    document_sha256: docHashHex,
    signer_identity: signerIdentity,
    public_key_hex: Buffer.from(rawPublicKey).toString('hex'),
    signature_hex: Buffer.from(sig).toString('hex'),
    timestamp_iso: timestampIso ?? new Date().toISOString(),
  };
}

/**
 * Verifies that a given signature over the base PDF matches the Ed25519 public key.
 */
export function verifyPdf(
  pdfBytes: Uint8Array,
  publicKeyBytes: Uint8Array,
  signatureBytes: Uint8Array
): boolean {
  try {
    const basePdf = stripEmbeddedAudit(pdfBytes);
    const docHashHex = computeSha256Hex(basePdf);
    const pubKey = publicKeyFromRaw(publicKeyBytes);
    return crypto.verify(
      null,
      Buffer.from(docHashHex, 'utf8'),
      pubKey,
      Buffer.from(signatureBytes)
    );
  } catch {
    return false;
  }
}

/**
 * Verifies an [`AuditManifest`] against a PDF byte buffer.
 */
export function verifyManifest(pdfBytes: Uint8Array, manifest: AuditManifest): boolean {
  if (manifest.algorithm !== 'Ed25519') {
    return false;
  }
  const basePdf = stripEmbeddedAudit(pdfBytes);
  const actualHashHex = computeSha256Hex(basePdf);
  if (actualHashHex !== manifest.document_sha256) {
    return false;
  }

  if (manifest.public_key_hex.length !== 64 || manifest.signature_hex.length !== 128) {
    return false;
  }

  const pubKeyBytes = Buffer.from(manifest.public_key_hex, 'hex');
  const sigBytes = Buffer.from(manifest.signature_hex, 'hex');
  return verifyPdf(basePdf, pubKeyBytes, sigBytes);
}

/**
 * Embeds an [`AuditManifest`] into the PDF buffer as a trailing comment block.
 */
export function embedAuditManifest(pdfBytes: Uint8Array, manifest: AuditManifest): Uint8Array {
  const basePdf = stripEmbeddedAudit(pdfBytes);
  const jsonStr = JSON.stringify(manifest);
  const trailerBytes = Buffer.from(AUDIT_MARKER_PREFIX + jsonStr + '\n', 'utf8');
  const out = new Uint8Array(basePdf.length + trailerBytes.length);
  out.set(basePdf, 0);
  out.set(trailerBytes, basePdf.length);
  return out;
}

/**
 * Extracts an embedded [`AuditManifest`] from a PDF buffer if present.
 */
export function extractAuditManifest(pdfBytes: Uint8Array): AuditManifest | null {
  const marker = Buffer.from(AUDIT_MARKER_PREFIX, 'utf8');
  const buf = Buffer.from(pdfBytes);
  const pos = buf.lastIndexOf(marker);
  if (pos === -1) {
    return null;
  }
  const manifestBuf = buf.subarray(pos + marker.length);
  const endPos = manifestBuf.indexOf(0x0a);
  const slice = endPos !== -1 ? manifestBuf.subarray(0, endPos) : manifestBuf;
  try {
    const parsed = JSON.parse(slice.toString('utf8'));
    if (
      parsed &&
      typeof parsed === 'object' &&
      typeof parsed.algorithm === 'string' &&
      typeof parsed.document_sha256 === 'string'
    ) {
      return parsed as AuditManifest;
    }
    return null;
  } catch {
    return null;
  }
}
