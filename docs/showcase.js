import {
  encodeQr,
  formatMoneyMinor,
  renderDocument,
} from './msfpdf-browser.min.js';

const THEME_HEX = {
  navy_fintech: { banner: '#0d1f3c', badge: '#2563eb' },
  emerald_editorial: { banner: '#0a3829', badge: '#109461' },
  slate_minimal: { banner: '#1a202c', badge: '#384b66' },
  crimson_compliance: { banner: '#420d19', badge: '#ba2438' },
};

const PRESETS = {
  trilingual: {
    theme: 'navy_fintech',
    currency: 'USD',
    badge: 'PAID',
    title: 'Invoice / Счёт / فاتورة',
    counterparty: 'ООО Северный Мост · شركة الجسر الشمالي للتقنية',
    note: 'فاتورة ضريبية معتمدة · لا توجد رسوم إضافية على التسوية رقم 8841',
    qr: 'https://github.com/mfelisberto90/msfPDF#trilingual-8841',
    subtotalMinor: 1850000,
    taxMinor: 92500,
    totalMinor: 1942500,
  },
  uk_vat: {
    theme: 'emerald_editorial',
    currency: 'GBP',
    badge: 'SETTLED',
    title: 'Tax Invoice INV-2026-0419',
    counterparty: 'Caledonian Clearing House Ltd · Edinburgh UK',
    note: 'Pure 7-bit WinAnsi stream with zero embedded font dictionary overhead.',
    qr: 'https://github.com/mfelisberto90/msfPDF#gbp-0419',
    subtotalMinor: 425000,
    taxMinor: 85000,
    totalMinor: 510000,
  },
  ru_invoice: {
    theme: 'slate_minimal',
    currency: 'RUB',
    badge: 'ОПЛАЧЕНО',
    title: 'Счёт-фактура № 904-РФ',
    counterparty: 'АО Балтийские Вычислительные Системы · Санкт-Петербург',
    note: 'Векторная генерация документа без внешних шрифтов и сетевых вызовов.',
    qr: 'https://github.com/mfelisberto90/msfPDF#rub-904',
    subtotalMinor: 24000000,
    taxMinor: 4800000,
    totalMinor: 28800000,
  },
  ar_zatca: {
    theme: 'crimson_compliance',
    currency: 'AED',
    badge: 'معتمدة',
    title: 'فاتورة ضريبية رقم 7720',
    counterparty: 'شركة المدار للأنظمة المالية · دبي، الإمارات',
    note: 'لا حاجة لملفات خطوط خارجية · ربط الحروف العربية تلقائياً من اليمين إلى اليسار',
    qr: 'https://github.com/mfelisberto90/msfPDF#aed-7720',
    subtotalMinor: 640000,
    taxMinor: 32000,
    totalMinor: 672000,
  },
};

function buildSpecFromState(state) {
  const firstSplit = Math.round(state.subtotalMinor * 0.6);
  const secondSplit = state.subtotalMinor - firstSplit;
  return {
    page_format: 'a4',
    theme: state.theme,
    metadata: {
      issuer_name: 'msfPDF Live Browser Compiler',
      document_code: '#SPEC-2026-8841',
      issued_at_label: '03 Nov 2026',
      footer_notice: 'Deterministic 7-bit ASCII %PDF-1.4 Specimen',
    },
    blocks: [
      {
        type: 'hero_header',
        kicker: 'DETERMINISTIC %PDF-1.4 SPECIMEN',
        title: state.title,
        subtitle: state.counterparty,
        badge_label: 'STATUS',
        badge_code: state.badge,
      },
      {
        type: 'bento_grid',
        columns: 2,
        cards: [
          {
            primary_label: 'COUNTERPARTY / КЛИЕНТ / العميل',
            primary_value: state.counterparty,
            secondary_label: 'ENCODING',
            secondary_value: 'WinAnsi + Dynamic Type 3 + /ToUnicode CMap',
          },
          {
            primary_label: 'SETTLEMENT TOTAL / ИТОГО / الإجمالي',
            primary_value: formatMoneyMinor(state.totalMinor, state.currency),
            secondary_label: 'ARITHMETIC',
            secondary_value: `${state.currency} exact integer minor units`,
          },
        ],
      },
      {
        type: 'data_table',
        zebra: true,
        columns: [
          { header: 'Service / Услуга / الخدمة', weight: 3, align: 'left' },
          { header: 'Script Engine', weight: 1.3, align: 'left' },
          { header: 'Amount', weight: 1.2, align: 'right', bold_values: true },
        ],
        rows: [
          [
            'Core %PDF-1.4 Vector Settlement Pipeline',
            'WinAnsi + AFM',
            formatMoneyMinor(firstSplit, state.currency),
          ],
          [
            'Кириллический и арабский модуль · خدمة الفوترة',
            'Type 3 Subset',
            formatMoneyMinor(secondSplit, state.currency),
          ],
        ],
      },
      {
        type: 'financial_summary',
        currency: state.currency,
        rows: [
          { label: 'Subtotal / Подытог / المجموع الفرعي', amount_minor: state.subtotalMinor },
          { label: 'Tax / НДС / ضريبة القيمة المضافة', amount_minor: state.taxMinor },
          {
            label: 'Total Due / Итого / الإجمالي',
            amount_minor: state.totalMinor,
            emphasize: true,
          },
        ],
      },
      {
        type: 'callout_banner',
        tone: 'accent',
        title: 'MULTILINGUAL SHAPING & ZERO-DEPENDENCY AUDIT',
        subtitle: state.note,
        right_value: formatMoneyMinor(state.totalMinor, state.currency),
      },
      {
        type: 'qr_verification',
        payload: state.qr,
        headline: 'CRYPTOGRAPHIC AUDIT ANCHOR · التحقق الرقمي',
        details: 'Compiled directly in your browser with 0 external packages and 100% 7-bit ASCII byte offsets.',
        code_display: 'SHA-256 PARITY: RUST == TYPESCRIPT',
      },
      {
        type: 'clause_list',
        paragraphs: [state.note],
      },
    ],
  };
}

function renderSvgQr(container, payload) {
  while (container.firstChild) {
    container.removeChild(container.firstChild);
  }
  let matrix;
  try {
    matrix = encodeQr(payload || 'https://github.com/mfelisberto90/msfPDF');
  } catch {
    matrix = encodeQr('https://github.com/mfelisberto90/msfPDF');
  }

  const ns = 'http://www.w3.org/2000/svg';
  const quiet = 2;
  const dim = matrix.size + quiet * 2;
  const svg = document.createElementNS(ns, 'svg');
  svg.setAttribute('viewBox', `0 0 ${dim} ${dim}`);
  svg.setAttribute('role', 'img');
  svg.setAttribute('aria-label', 'Vector QR code generated live');

  const bg = document.createElementNS(ns, 'rect');
  bg.setAttribute('width', String(dim));
  bg.setAttribute('height', String(dim));
  bg.setAttribute('fill', '#ffffff');
  svg.appendChild(bg);

  const parts = [];
  for (let r = 0; r < matrix.size; r += 1) {
    for (let c = 0; c < matrix.size; c += 1) {
      if (matrix.modules[r * matrix.size + c] === 1) {
        parts.push(`M${c + quiet},${r + quiet}h1v1h-1z`);
      }
    }
  }
  const path = document.createElementNS(ns, 'path');
  path.setAttribute('d', parts.join(''));
  path.setAttribute('fill', '#0f172a');
  svg.appendChild(path);
  container.appendChild(svg);
}

function inspectAsciiAndSubset(bytes) {
  let asciiPure = true;
  for (let i = 0; i < bytes.length; i += 1) {
    const b = bytes[i];
    if (b !== 0x0a && (b < 0x20 || b > 0x7e)) {
      asciiPure = false;
      break;
    }
  }
  const text = new TextDecoder('ascii').decode(bytes);
  const glyphMatches = text.match(/\/g[0-9a-f]{2}\s+\d+\s+0\s+R/gi);
  const glyphCount = glyphMatches ? glyphMatches.length : 0;
  const xrefPos = text.indexOf('\nxref\n');
  const tail = xrefPos >= 0 ? text.slice(xrefPos + 1).trim() : text.slice(-340).trim();
  return { asciiPure, glyphCount, tail };
}

function setupMotion() {
  const shutter = document.querySelector('.shutter');
  if (shutter) {
    requestAnimationFrame(() => {
      shutter.classList.add('is-in');
    });
  }

  const targets = document.querySelectorAll('[data-reveal], [data-bar]');
  if (!('IntersectionObserver' in window)) {
    targets.forEach((el) => el.classList.add('is-in'));
    return;
  }

  const io = new IntersectionObserver(
    (entries) => {
      for (const entry of entries) {
        if (entry.isIntersecting) {
          entry.target.classList.add('is-in');
          io.unobserve(entry.target);
        }
      }
    },
    { rootMargin: '0px 0px -8% 0px', threshold: 0.08 }
  );

  targets.forEach((el) => io.observe(el));
}

function setupDepartureBoard() {
  const rows = Array.from(document.querySelectorAll('.board-row'));
  rows.forEach((row) => {
    row.addEventListener('click', () => {
      rows.forEach((other) => {
        const active = other === row;
        other.classList.toggle('is-open', active);
        other.setAttribute('aria-expanded', active ? 'true' : 'false');
      });
    });
  });
}

function setupStudio() {
  const titleInput = document.getElementById('studio-title');
  const partyInput = document.getElementById('studio-party');
  const noteInput = document.getElementById('studio-note');
  const qrInput = document.getElementById('studio-qr');
  const themeSelect = document.getElementById('studio-theme');
  const currencySelect = document.getElementById('studio-currency');

  const valLatency = document.getElementById('tel-latency');
  const valBytes = document.getElementById('tel-bytes');
  const valGlyphs = document.getElementById('tel-glyphs');
  const valAscii = document.getElementById('tel-ascii');

  const paperBanner = document.getElementById('paper-banner');
  const paperTitle = document.getElementById('paper-title');
  const paperSub = document.getElementById('paper-sub');
  const paperBadge = document.getElementById('paper-badge');
  const paperNote = document.getElementById('paper-note');
  const paperTotal = document.getElementById('paper-total');
  const qrBox = document.getElementById('paper-qr');
  const xrefCode = document.getElementById('xref-tail');

  const btnDownload = document.getElementById('btn-download-pdf');
  const btnOpen = document.getElementById('btn-open-pdf');
  const presetBtns = Array.from(document.querySelectorAll('[data-preset]'));

  if (!titleInput || !btnDownload) return;

  let currentPreset = PRESETS.trilingual;
  let latestBytes = null;
  let activeBlobUrl = null;

  function currentState() {
    return {
      theme: themeSelect.value,
      currency: currencySelect.value,
      badge: currentPreset.badge,
      title: titleInput.value.trim() || currentPreset.title,
      counterparty: partyInput.value.trim() || currentPreset.counterparty,
      note: noteInput.value.trim() || currentPreset.note,
      qr: qrInput.value.trim() || currentPreset.qr,
      subtotalMinor: currentPreset.subtotalMinor,
      taxMinor: currentPreset.taxMinor,
      totalMinor: currentPreset.totalMinor,
    };
  }

  function compileNow() {
    const state = currentState();
    const spec = buildSpecFromState(state);

    const t0 = performance.now();
    const bytes = renderDocument(spec);
    const elapsedMs = performance.now() - t0;
    latestBytes = bytes;

    const info = inspectAsciiAndSubset(bytes);
    valLatency.textContent = `${elapsedMs.toFixed(2)} ms`;
    valBytes.textContent = `${bytes.length.toLocaleString('en-US')} B`;
    valGlyphs.textContent = info.glyphCount > 0 ? `${info.glyphCount} Type 3` : 'WinAnsi only';
    valAscii.textContent = info.asciiPure ? '7-bit PASS' : 'Check bytes';

    const colors = THEME_HEX[state.theme] || THEME_HEX.navy_fintech;
    paperBanner.style.backgroundColor = colors.banner;
    paperBadge.style.backgroundColor = colors.badge;
    paperTitle.textContent = state.title;
    paperSub.textContent = state.counterparty;
    paperBadge.textContent = state.badge;
    paperNote.textContent = state.note;
    paperTotal.textContent = formatMoneyMinor(state.totalMinor, state.currency);

    renderSvgQr(qrBox, state.qr);
    xrefCode.textContent = info.tail;
  }

  function applyPreset(key) {
    const p = PRESETS[key] || PRESETS.trilingual;
    currentPreset = p;
    themeSelect.value = p.theme;
    currencySelect.value = p.currency;
    titleInput.value = p.title;
    partyInput.value = p.counterparty;
    noteInput.value = p.note;
    qrInput.value = p.qr;

    presetBtns.forEach((b) => {
      b.setAttribute('aria-pressed', b.dataset.preset === key ? 'true' : 'false');
    });
    compileNow();
  }

  presetBtns.forEach((btn) => {
    btn.addEventListener('click', () => applyPreset(btn.dataset.preset));
  });

  [titleInput, partyInput, noteInput, qrInput].forEach((el) => {
    el.addEventListener('input', compileNow);
  });
  [themeSelect, currencySelect].forEach((el) => {
    el.addEventListener('change', compileNow);
  });

  function makeBlobUrl() {
    if (!latestBytes) compileNow();
    if (activeBlobUrl) URL.revokeObjectURL(activeBlobUrl);
    const blob = new Blob([latestBytes], { type: 'application/pdf' });
    activeBlobUrl = URL.createObjectURL(blob);
    return activeBlobUrl;
  }

  btnDownload.addEventListener('click', () => {
    const url = makeBlobUrl();
    const link = document.createElement('a');
    link.href = url;
    link.download = 'msfpdf-live-specimen.pdf';
    document.body.appendChild(link);
    link.click();
    document.body.removeChild(link);
  });

  btnOpen.addEventListener('click', () => {
    const url = makeBlobUrl();
    window.open(url, '_blank', 'noopener');
  });

  // Warm up V8 JIT briefly so initial telemetry reflects steady-state browser compilation
  applyPreset('trilingual');
  compileNow();
  compileNow();
}

setupMotion();
setupDepartureBoard();
setupStudio();
