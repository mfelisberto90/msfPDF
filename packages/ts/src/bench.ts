import { fileURLToPath } from 'node:url';
import { renderDocument } from './renderer.ts';
import type { DocumentSpec } from './types.ts';

export function runTypeScriptBenchmark(iterations: number = 3000): {
  engine: string;
  iterations: number;
  p50_us: number;
  avg_us: number;
  p99_us: number;
  pdf_bytes: number;
  docs_per_sec_single_core: number;
} {
  const spec: DocumentSpec = {
    page_format: 'a4',
    theme: 'navy_fintech',
    metadata: {
      issuer_name: 'Caledonia Cloud Systems Ltd',
      document_code: '#INV-2026-8841',
      issued_at_label: '02 Nov 2026',
    },
    blocks: [
      {
        type: 'hero_header',
        kicker: 'ENTERPRISE SUBSCRIPTION INVOICE • UK VAT REG GB40928192',
        title: 'Caledonia Cloud Systems Ltd',
        subtitle: 'Dedicated Single-Tenant Infrastructure & Managed Compliance',
        badge_label: 'CRYPTOGRAPHIC REF',
        badge_code: '#INV-2026-8841',
        badge_meta: 'Issued: 02/11/2026 • Net 30',
      },
      {
        type: 'bento_grid',
        columns: 2,
        cards: [
          {
            primary_label: 'BILLED CUSTOMER',
            primary_value: 'Northbridge Logistics PLC',
            secondary_label: 'BILLING CONTACT',
            secondary_value: 'Eleanor Vance (Accounts Payable)',
          },
          {
            primary_label: 'SUPPLIER ENTITY',
            primary_value: 'Caledonia Cloud Systems Ltd',
            secondary_label: 'JURISDICTION',
            secondary_value: 'Edinburgh, Scotland, United Kingdom',
          },
        ],
      },
      {
        type: 'section_heading',
        title: '1. USAGE & SUBSCRIPTION SCHEDULE',
      },
      {
        type: 'data_table',
        columns: [
          { header: 'SERVICE DESCRIPTION', weight: 3.0, align: 'left', bold_values: true },
          { header: 'QTY', weight: 0.8, align: 'center' },
          { header: 'NET AMOUNT', weight: 1.2, align: 'right', bold_values: true },
        ],
        rows: [
          ['Dedicated Cluster Nodes (eu-west-2)', '4', '£3,200.00'],
          ['Automated Offsite DR Vault (10 TB)', '1', '£450.00'],
          ['24/7 Incident Response Retainer', '1', '£850.00'],
        ],
      },
      {
        type: 'financial_summary',
        currency: 'GBP',
        rows: [
          { label: 'Subtotal (Excl. UK VAT)', amount_minor: 450_000 },
          { label: 'UK Standard VAT (20.0%)', amount_minor: 90_000 },
        ],
      },
      {
        type: 'callout_banner',
        title: 'TOTAL AMOUNT DUE (GBP)',
        subtitle: 'Payable via Faster Payments / CHAPS within 30 calendar days',
        right_value: '£5,400.00',
        tone: 'accent',
      },
      {
        type: 'qr_verification',
        payload: 'https://verify.example.co.uk/inv/2026-8841-9f8a7b6c',
        headline: 'INSTANT CRYPTOGRAPHIC AUDIT VERIFICATION',
        details:
          'Scan this vector QR code to verify SHA-256 document integrity and settlement status.',
        code_display: 'SHA256: 9F8A-7B6C-4410-EE29',
      },
      {
        type: 'clause_list',
        paragraphs: [
          '• Title to deliverables and SLA credits remain governed by the Master Services Agreement under Scots law.',
          '• Late commercial settlements accrue statutory interest pursuant to the Late Payment of Commercial Debts (Interest) Act 1998.',
        ],
      },
      {
        type: 'signature_grid',
        heading: '2. AUTHORISED APPROVALS',
        signers: [
          { role_label: 'FOR SUPPLIER', party_name: 'Caledonia Cloud Systems Ltd' },
          { role_label: 'FOR CUSTOMER', party_name: 'Northbridge Logistics PLC' },
        ],
      },
    ],
  };

  for (let i = 0; i < 250; i++) renderDocument(spec);

  const samplesUs: number[] = [];
  let byteLen = 0;
  for (let i = 0; i < iterations; i++) {
    const t0 = performance.now();
    const out = renderDocument(spec);
    samplesUs.push((performance.now() - t0) * 1000);
    byteLen = out.byteLength;
  }

  samplesUs.sort((a, b) => a - b);
  const avgUs = samplesUs.reduce((acc, v) => acc + v, 0) / iterations;
  return {
    engine: 'msfpdf-typescript',
    iterations,
    p50_us: Number(samplesUs[Math.floor(iterations * 0.5)].toFixed(2)),
    avg_us: Number(avgUs.toFixed(2)),
    p99_us: Number(samplesUs[Math.floor(iterations * 0.99)].toFixed(2)),
    pdf_bytes: byteLen,
    docs_per_sec_single_core: Math.round(1_000_000 / avgUs),
  };
}

if (process.argv[1] && fileURLToPath(import.meta.url) === process.argv[1]) {
  console.log(JSON.stringify(runTypeScriptBenchmark()));
}
