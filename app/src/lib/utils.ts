import { openUrl } from '@tauri-apps/plugin-opener';

export function safeOpenUrl(url: string): void {
  try {
    const parsed = new URL(url);
    if (parsed.protocol === 'http:' || parsed.protocol === 'https:') {
      openUrl(url);
    }
  } catch {
    // invalid URL, ignore
  }
}

export function sanitizeAmount(raw: string): string {
  let v = raw.replace(/,/g, '.');
  v = v.replace(/[^0-9.]/g, '');
  v = v.replace(/^0+(?=\d)/, '');
  const dotIdx = v.indexOf('.');
  if (dotIdx >= 0) {
    // Remove extra dots and limit to 2 decimal places
    const decimals = v.substring(dotIdx + 1).replace(/\./g, '').substring(0, 2);
    v = v.substring(0, dotIdx + 1) + decimals;
  }
  return v;
}

/** Convert a sanitized ruble string directly to kopecks (integer), avoiding float precision loss. */
export function rublesStringToKopecks(display: string): number {
  if (!display) return 0;
  const parts = display.split('.');
  const rubles = parseInt(parts[0] || '0', 10) || 0;
  let kopecks = 0;
  if (parts[1] !== undefined) {
    const dec = parts[1].padEnd(2, '0').substring(0, 2);
    kopecks = parseInt(dec, 10) || 0;
  }
  return rubles * 100 + kopecks;
}

export function formatMoney(amount: number): string {
  // Amount is stored in kopecks — divide by 100 for display
  const rubles = amount / 100;
  return new Intl.NumberFormat('ru-RU', { minimumFractionDigits: 2, maximumFractionDigits: 2 }).format(rubles) + ' ₽';
}

const monthNames = [
  'янв', 'фев', 'мар', 'апр', 'май', 'июн',
  'июл', 'авг', 'сен', 'окт', 'ноя', 'дек'
];

export function formatDate(dateStr: string): string {
  if (!dateStr) return '';
  const d = new Date(dateStr + 'T00:00:00'); // Parse as local time
  return `${d.getDate()} ${monthNames[d.getMonth()]} ${d.getFullYear()}`;
}

export function formatRelativeDate(dateStr: string): string {
  if (!dateStr) return '';
  const d = new Date(dateStr + 'T00:00:00');
  const today = new Date();
  today.setHours(0, 0, 0, 0);
  const yesterday = new Date(today);
  yesterday.setDate(yesterday.getDate() - 1);
  const target = new Date(d);
  target.setHours(0, 0, 0, 0);

  if (target.getTime() === today.getTime()) return 'Сегодня';
  if (target.getTime() === yesterday.getTime()) return 'Вчера';
  return formatDate(dateStr);
}
