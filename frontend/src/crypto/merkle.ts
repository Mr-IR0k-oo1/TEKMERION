import { EvidenceRecord, MerkleLeaves, MerkleTreeData } from '../types/forensic';
import { normalizeUtf8, normalizeUrl } from './canonicalize';
import { hashLeaf, hashParent, sha256, u64BigEndian } from './sha256';

/**
 * Format floating-point numbers with deterministic fixed 6-decimal precision
 * matching Rust format!("{:.6}", value)
 */
export function formatFloat(val: number): string {
  if (!Number.isFinite(val)) {
    throw new Error('Non-finite float not allowed in canonical evidence hashing');
  }
  return val.toFixed(6);
}

/**
 * Compute SHA-256 for component buffers with prefixed headers and big-endian lengths
 */
async function hashComponent(header: string, fields: (string | Uint8Array)[]): Promise<string> {
  const encoder = new TextEncoder();
  const headerBytes = encoder.encode(header);
  
  let totalLength = headerBytes.length;
  const processed: Uint8Array[] = [headerBytes];

  for (const f of fields) {
    const bytes = typeof f === 'string' ? encoder.encode(f) : f;
    const lenPrefix = u64BigEndian(bytes.length);
    processed.push(lenPrefix);
    processed.push(bytes);
    totalLength += lenPrefix.length + bytes.length;
  }

  const merged = new Uint8Array(totalLength);
  let offset = 0;
  for (const part of processed) {
    merged.set(part, offset);
    offset += part.length;
  }

  return sha256(merged);
}

/**
 * Compute Leaf #0: IMAGE_HASH
 */
export async function computeImageHash(imageSha256: string): Promise<string> {
  const clean = normalizeUtf8(imageSha256).toLowerCase();
  return hashComponent('image_component:v1\n', [clean]);
}

/**
 * Compute Leaf #1: CONTENT_HASH
 */
export async function computeContentHash(title: string, text: string): Promise<string> {
  const nfcTitle = normalizeUtf8(title);
  const nfcText = normalizeUtf8(text);
  return hashComponent('content_component:v1\n', [nfcTitle, nfcText]);
}

/**
 * Compute Leaf #2: METADATA_HASH
 */
export async function computeMetadataHash(
  schemaVersion: string,
  runId: string,
  sourceUrl: string,
  domain: string,
  platform: string,
  provider: string,
  retrievedAt: string
): Promise<string> {
  return hashComponent('metadata_component:v1\n', [
    normalizeUtf8(schemaVersion),
    normalizeUtf8(runId),
    normalizeUrl(sourceUrl),
    normalizeUtf8(domain).toLowerCase(),
    normalizeUtf8(platform).toLowerCase(),
    normalizeUtf8(provider),
    retrievedAt,
  ]);
}

/**
 * Compute Leaf #3: FACE_HASH
 */
export async function computeFaceHash(
  similarity: number,
  model: string,
  quality: number
): Promise<string> {
  const simStr = formatFloat(similarity);
  const qualStr = formatFloat(quality);
  const modelStr = normalizeUtf8(model);
  return hashComponent('face_result_component:v1\n', [simStr, modelStr, qualStr]);
}

/**
 * Compute Leaf #4: PROVENANCE_HASH
 */
export async function computeProvenanceHash(
  runId: string,
  provider: string,
  platform: string,
  retrievedAt: string
): Promise<string> {
  return hashComponent('provenance_component:v1\n', [
    normalizeUtf8(runId),
    normalizeUtf8(provider),
    normalizeUtf8(platform).toLowerCase(),
    retrievedAt,
  ]);
}

/**
 * Compute all 5 leaves for an EvidenceRecord
 */
export async function computeEvidenceLeaves(record: EvidenceRecord): Promise<MerkleLeaves> {
  const [image_hash, content_hash, metadata_hash, face_hash, provenance_hash] = await Promise.all([
    computeImageHash(record.image_sha256),
    computeContentHash(record.title, record.text),
    computeMetadataHash(
      record.schema_version,
      record.run_id,
      record.source_url,
      record.domain,
      record.platform,
      record.provider,
      record.retrieved_at
    ),
    computeFaceHash(record.face_similarity, record.face_model, record.candidate_quality),
    computeProvenanceHash(record.run_id, record.provider, record.platform, record.retrieved_at),
  ]);

  return {
    image_hash,
    content_hash,
    metadata_hash,
    face_hash,
    provenance_hash,
  };
}

/**
 * Build the complete domain-separated Merkle tree according to RFC 6962 odd-node rules
 */
export async function buildMerkleTree(leaves: MerkleLeaves): Promise<MerkleTreeData> {
  const leaf0 = await hashLeaf(leaves.image_hash);
  const leaf1 = await hashLeaf(leaves.content_hash);
  const leaf2 = await hashLeaf(leaves.metadata_hash);
  const leaf3 = await hashLeaf(leaves.face_hash);
  const leaf4 = await hashLeaf(leaves.provenance_hash);

  // Layer 1:
  // Node 0-1 = Parent(leaf0, leaf1)
  // Node 2-3 = Parent(leaf2, leaf3)
  // leaf4 promoted
  const node_0_1 = await hashParent(leaf0, leaf1);
  const node_2_3 = await hashParent(leaf2, leaf3);
  const promoted_4 = leaf4;

  // Layer 2:
  // Node 0-3 = Parent(node_0_1, node_2_3)
  // promoted_4 promoted
  const node_0_3 = await hashParent(node_0_1, node_2_3);

  // Layer 3:
  // Root = Parent(node_0_3, promoted_4)
  const root_hash = await hashParent(node_0_3, promoted_4);

  return {
    root_hash,
    node_0_1,
    node_2_4: node_0_3, // node covering 2-4 branch / internal
    node_2_3,
    leaves,
  };
}
