export type PipelineStageId =
  | 'INPUT'
  | 'FACE'
  | 'DISCOVERY'
  | 'VERIFY'
  | 'EVIDENCE'
  | 'BLOCKCHAIN'
  | 'FINAL_VERIFY';

export interface PipelineStageInfo {
  id: PipelineStageId;
  label: string;
  name: string;
  description: string;
}

export const PIPELINE_STAGES: PipelineStageInfo[] = [
  {
    id: 'INPUT',
    label: 'STAGE 1',
    name: 'Input Ingestion',
    description: 'Cryptographic SHA-256 digesting of raw query image & resolution profiling',
  },
  {
    id: 'FACE',
    label: 'STAGE 2',
    name: 'Face Intelligence',
    description: 'SCRFD detection, Laplacian blur/exposure gates & ArcFace 512-D embedding',
  },
  {
    id: 'DISCOVERY',
    label: 'STAGE 3',
    name: 'Web Discovery',
    description: 'Reverse-image web & social search, URL normalization and candidate gathering',
  },
  {
    id: 'VERIFY',
    label: 'STAGE 4',
    name: 'Biometric Verification',
    description: 'Independent candidate download, ArcFace inference & cosine similarity matching (≥ 0.80)',
  },
  {
    id: 'EVIDENCE',
    label: 'STAGE 5',
    name: 'Merkle Fingerprint',
    description: 'RFC 8785 canonical JSON serialization & 5-leaf domain-separated Merkle tree root',
  },
  {
    id: 'BLOCKCHAIN',
    label: 'STAGE 6',
    name: 'Sepolia Anchor',
    description: 'Public immutable anchoring on Ethereum Sepolia smart contract (registerEvidence)',
  },
  {
    id: 'FINAL_VERIFY',
    label: 'STAGE 7',
    name: 'Audit Re-verification',
    description: 'Contract eth_call root readback against local Merkle recalculation',
  },
];

export type PipelineStatus = 'idle' | 'running' | 'completed' | 'tampered' | 'error';

export type ViewTab = 'pipeline' | 'merkle' | 'tamper' | 'candidates' | 'audit';

export interface FaceQualityAssessment {
  status: 'pass' | 'fail' | 'warn';
  face_count: number;
  blur_variance: number;
  brightness: number;
  bbox: [number, number, number, number]; // [x1, y1, x2, y2]
  landmarks: [number, number][]; // 5 keypoints
  embedding_preview: number[];
  reasons: string[];
}

export interface SearchCandidate {
  url: string;
  title: string;
  domain: string;
  image_url: string;
  thumbnail_url?: string;
  snippet: string;
  provider: string;
  discovered_at: string;
}

export type VerificationStatus = 'verified' | 'below_threshold' | 'no_face' | 'error';

export interface VerificationResult {
  candidate: SearchCandidate;
  similarity: number;
  quality: number;
  matched_face_index: number | null;
  candidate_image_hash: string;
  status: VerificationStatus;
  error_message?: string;
}

export interface EvidenceRecord {
  schema_version: string;
  run_id: string;
  source_url: string;
  domain: string;
  platform: string;
  provider: string;
  retrieved_at: string;
  title: string;
  text: string;
  image_sha256: string;
  face_similarity: number;
  face_model: string;
  candidate_quality: number;
}

export interface MerkleLeaves {
  image_hash: string;      // Leaf #0
  content_hash: string;    // Leaf #1
  metadata_hash: string;   // Leaf #2
  face_hash: string;       // Leaf #3
  provenance_hash: string; // Leaf #4
}

export interface MerkleTreeData {
  root_hash: string;
  node_0_1: string;
  node_2_4: string;
  node_2_3: string;
  leaves: MerkleLeaves;
}

export interface EvidenceBundle {
  schema_version: string;
  run_id: string;
  root_hash: string;
  tree: MerkleTreeData;
  record: EvidenceRecord;
}

export interface BlockchainRecord {
  network: string;
  contract_address: string;
  tx_hash: string;
  block_number: number;
  confirmations: number;
  registered_root: string;
  registered_image: string;
  submitter: string;
  timestamp: string;
}

export interface AuditEvent {
  id: string;
  event: string;
  timestamp: string;
  run_id: string;
  level: 'info' | 'warn' | 'error' | 'success';
  detail?: string;
}

export interface TamperState {
  isTampered: boolean;
  tamperedLeaf: string | null;
  tamperedField: string | null;
  originalValue: string | null;
  tamperedValue: string | null;
  originalLeafHash: string | null;
  tamperedLeafHash: string | null;
  originalRoot: string | null;
  tamperedRoot: string | null;
}
