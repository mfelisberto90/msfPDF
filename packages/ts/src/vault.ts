import crypto from 'node:crypto';
import fs from 'node:fs/promises';
import path from 'node:path';

import { renderDocument } from './renderer.ts';
import type { DocumentSpec, SecurityLimits } from './types.ts';
import { DEFAULT_SECURITY_LIMITS, MsfPdfError } from './types.ts';

const SAFE_TENANT_RE = /^[a-zA-Z0-9][a-zA-Z0-9_-]{2,63}$/;
const SAFE_PREFIX_RE = /^[a-z0-9-]{2,24}$/;
const SAFE_TOKEN_RE = /^[a-f0-9]{40}$/;

export interface IssuedVaultDocument {
  readonly token: string;
  readonly fileName: string;
  readonly bytes: Uint8Array;
}

/**
 * Fail-closed multi-tenant PDF storage vault enforcing `0700` per-tenant directory boundaries,
 * `160-bit` CSPRNG non-enumerable tokens, and `0600` atomic `.tmp` + `fsync` + `rename` writes.
 */
export class TenantPdfVault {
  private readonly vaultRoot: string;
  private readonly docPrefix: string;
  private readonly limits: SecurityLimits;

  constructor(
    vaultRoot: string,
    docPrefix: string = 'document',
    limits: SecurityLimits = DEFAULT_SECURITY_LIMITS
  ) {
    if (!SAFE_PREFIX_RE.test(docPrefix)) {
      throw new MsfPdfError(
        'VAULT_SECURITY_VIOLATION',
        'docPrefix must match ^[a-z0-9-]{2,24}$'
      );
    }
    this.vaultRoot = path.resolve(vaultRoot);
    this.docPrefix = docPrefix;
    this.limits = limits;
  }

  private resolveSafePath(tenantId: string, token: string): { fileName: string; fullPath: string } {
    if (!tenantId || !SAFE_TENANT_RE.test(tenantId)) {
      throw new MsfPdfError('VAULT_SECURITY_VIOLATION', 'invalid tenantId format');
    }
    if (!token || !SAFE_TOKEN_RE.test(token)) {
      throw new MsfPdfError(
        'VAULT_SECURITY_VIOLATION',
        'invalid or enumerable 160-bit token rejected'
      );
    }

    const fileName = `${this.docPrefix}-${token}.pdf`;
    const tenantDir = path.resolve(this.vaultRoot, tenantId);
    if (!tenantDir.startsWith(this.vaultRoot + path.sep)) {
      throw new MsfPdfError('VAULT_SECURITY_VIOLATION', 'tenant vault boundary violation');
    }

    const fullPath = path.resolve(tenantDir, fileName);
    if (!fullPath.startsWith(tenantDir + path.sep)) {
      throw new MsfPdfError('VAULT_SECURITY_VIOLATION', 'document path traversal blocked');
    }

    return { fileName, fullPath };
  }

  async issueDocument(tenantId: string, spec: DocumentSpec): Promise<IssuedVaultDocument> {
    const token = crypto.randomBytes(20).toString('hex');
    const { fileName, fullPath } = this.resolveSafePath(tenantId, token);
    const bytes = renderDocument(spec, this.limits);

    const parentDir = path.dirname(fullPath);
    await fs.mkdir(parentDir, { recursive: true, mode: 0o700 });

    const randSuffix = crypto.randomBytes(4).toString('hex');
    const tmpPath = path.join(parentDir, `.${fileName}.${process.pid}.${randSuffix}.tmp`);

    let handle: fs.FileHandle | null = null;
    try {
      handle = await fs.open(tmpPath, 'wx', 0o600);
      await handle.writeFile(bytes);
      await handle.sync();
      await handle.close();
      handle = null;
      await fs.rename(tmpPath, fullPath);
    } catch (err) {
      if (handle) await handle.close().catch(() => undefined);
      await fs.unlink(tmpPath).catch(() => undefined);
      throw err;
    }

    return { token, fileName, bytes };
  }

  async readDocument(tenantId: string, token: string): Promise<Uint8Array> {
    const { fullPath } = this.resolveSafePath(tenantId, token);
    try {
      const buf = await fs.readFile(fullPath);
      return new Uint8Array(buf.buffer, buf.byteOffset, buf.byteLength);
    } catch {
      throw new MsfPdfError(
        'VAULT_SECURITY_VIOLATION',
        'document not found in verified tenant scope'
      );
    }
  }
}
