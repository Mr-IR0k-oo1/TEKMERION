/**
 * Cryptographic SHA-256 implementation using Web Crypto API.
 * Supports domain-separated leaf (0x00) and internal node (0x01) prefixes
 * adhering to RFC 6962 and TEKMERION's Rust engine.
 */

export const LEAF_PREFIX = 0x00;
export const NODE_PREFIX = 0x01;

function toHex(buffer: ArrayBuffer): string {
  const bytes = new Uint8Array(buffer);
  let hex = '';
  for (let i = 0; i < bytes.length; i++) {
    hex += bytes[i].toString(16).padStart(2, '0');
  }
  return hex;
}

export async function sha256(data: string | Uint8Array): Promise<string> {
  const bytes = typeof data === 'string' ? new TextEncoder().encode(data) : data;
  const digest = await crypto.subtle.digest('SHA-256', bytes as unknown as BufferSource);
  return toHex(digest);
}

/**
 * Domain-separated leaf hashing:
 * SHA-256(0x00 || leaf_bytes)
 */
export async function hashLeaf(leaf: string): Promise<string> {
  const leafBytes = new TextEncoder().encode(leaf);
  const buffer = new Uint8Array(1 + leafBytes.length);
  buffer[0] = LEAF_PREFIX;
  buffer.set(leafBytes, 1);
  const digest = await crypto.subtle.digest('SHA-256', buffer as unknown as BufferSource);
  return toHex(digest);
}

/**
 * Domain-separated internal node hashing:
 * SHA-256(0x01 || left_bytes || right_bytes)
 */
export async function hashParent(left: string, right: string): Promise<string> {
  const leftBytes = new TextEncoder().encode(left);
  const rightBytes = new TextEncoder().encode(right);
  const buffer = new Uint8Array(1 + leftBytes.length + rightBytes.length);
  buffer[0] = NODE_PREFIX;
  buffer.set(leftBytes, 1);
  buffer.set(rightBytes, 1 + leftBytes.length);
  const digest = await crypto.subtle.digest('SHA-256', buffer as unknown as BufferSource);
  return toHex(digest);
}

/**
 * Helper to encode big-endian u64 length
 */
export function u64BigEndian(num: number): Uint8Array {
  const buf = new ArrayBuffer(8);
  const view = new DataView(buf);
  // JavaScript safe integers fit within lower 32-bits for length
  view.setUint32(0, 0, false);
  view.setUint32(4, num >>> 0, false);
  return new Uint8Array(buf);
}
