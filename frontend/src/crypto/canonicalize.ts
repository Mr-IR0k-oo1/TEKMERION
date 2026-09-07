/**
 * RFC 8785 JSON Canonicalization Scheme (JCS)
 * 
 * Provides deterministic serialization by:
 * - Sorting object keys lexicographically by UTF-16 code units
 * - Omitting insignificant whitespace
 * - Representing numbers in standard ECMAScript format
 * - Canonicalizing Unicode characters (NFC normalization)
 */

export function canonicalizeJson(obj: unknown): string {
  if (obj === null) return 'null';
  if (typeof obj === 'boolean') return obj ? 'true' : 'false';
  if (typeof obj === 'number') {
    if (!Number.isFinite(obj)) {
      throw new Error('RFC 8785 forbids NaN and Infinity in JSON serialization');
    }
    return String(obj);
  }
  if (typeof obj === 'string') {
    return JSON.stringify(obj.normalize('NFC'));
  }
  if (Array.isArray(obj)) {
    const elements = obj.map(item => canonicalizeJson(item));
    return `[${elements.join(',')}]`;
  }
  if (typeof obj === 'object') {
    const keys = Object.keys(obj as Record<string, unknown>).sort();
    const entries = keys.map(key => {
      const canonicalKey = JSON.stringify(key.normalize('NFC'));
      const canonicalValue = canonicalizeJson((obj as Record<string, unknown>)[key]);
      return `${canonicalKey}:${canonicalValue}`;
    });
    return `{${entries.join(',')}}`;
  }
  throw new Error(`Unsupported type for canonicalization: ${typeof obj}`);
}

/**
 * Normalizes strings to Unicode NFC form and trims whitespace
 */
export function normalizeUtf8(str: string): string {
  return str.trim().normalize('NFC');
}

/**
 * Canonical URL string representation
 */
export function normalizeUrl(urlStr: string): string {
  try {
    const parsed = new URL(urlStr);
    return parsed.toString();
  } catch {
    return urlStr.trim();
  }
}
