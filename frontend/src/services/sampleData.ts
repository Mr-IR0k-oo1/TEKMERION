import {
  BlockchainRecord,
  EvidenceRecord,
  FaceQualityAssessment,
  VerificationResult,
} from '../types/forensic';

export interface SampleInvestigation {
  id: string;
  name: string;
  imageFileName: string;
  imageSrc: string;
  resolution: string;
  fileSize: string;
  imageHash: string;
  faceQuality: FaceQualityAssessment;
  candidates: VerificationResult[];
}

export const SAMPLE_INVESTIGATIONS: SampleInvestigation[] = [
  {
    id: 'case_single_face',
    name: 'Primary Evidence: Single Subject (Jane Doe)',
    imageFileName: 'query_face.jpg',
    imageSrc: '/query_face.jpg',
    resolution: '1280x720 (20.8 KB)',
    fileSize: '20.8 KB',
    imageHash: 'e76f2345d86de6743e81bd761562b73035d26e96ff741bf9c528edac2fe58327',
    faceQuality: {
      status: 'pass',
      face_count: 1,
      blur_variance: 161.4,
      brightness: 128.0,
      bbox: [129, 137, 335, 411],
      landmarks: [
        [173, 237],
        [264, 252],
        [199, 298],
        [163, 342],
        [242, 355],
      ],
      embedding_preview: [0.038, -0.045, 0.012, 0.089, -0.023, 0.061, -0.019, 0.074],
      reasons: [
        'Quality Gate: PASSED',
        'InsightFace SCRFD detector: 1 face identified',
        'Laplacian blur variance: 161.4 (threshold > 100.0)',
        'ArcFace 512-D embedding extracted with unit L2 norm',
      ],
    },
    candidates: [
      {
        candidate: {
          url: 'https://archives.tekmerion.org/records/subject-01.png',
          title: 'Jane Doe Public Portfolio',
          domain: 'archives.tekmerion.org',
          image_url: '/candidates/match_target.jpg',
          thumbnail_url: '/candidates/match_target.jpg',
          snippet: 'Software engineer portrait from verified institutional directory',
          provider: 'catalog_discovery',
          discovered_at: new Date().toISOString(),
        },
        similarity: 1.0,
        quality: 0.85,
        matched_face_index: 0,
        candidate_image_hash: 'e76f2345d86de6743e81bd761562b73035d26e96ff741bf9c528edac2fe58327',
        status: 'verified',
      },
      {
        candidate: {
          url: 'https://archives.example.net/events/2024',
          title: 'Conference Attendees',
          domain: 'archives.example.net',
          image_url: '/candidates/different_person.jpg',
          thumbnail_url: '/candidates/different_person.jpg',
          snippet: 'Group session attendee portrait photo',
          provider: 'catalog_discovery',
          discovered_at: new Date().toISOString(),
        },
        similarity: 0.059,
        quality: 0.88,
        matched_face_index: 0,
        candidate_image_hash: '3499eb8aa235cab35272b2fdc9ce858afb3e252d58252033bc6f01df222067dc',
        status: 'below_threshold',
      },
      {
        candidate: {
          url: 'https://landscapes.example.com/gallery',
          title: 'Scenic View',
          domain: 'landscapes.example.com',
          image_url: '/candidates/scenic_landscape.png',
          thumbnail_url: '/candidates/scenic_landscape.png',
          snippet: 'Mountain landscape horizon without human subjects',
          provider: 'catalog_discovery',
          discovered_at: new Date().toISOString(),
        },
        similarity: 0.0,
        quality: 0.0,
        matched_face_index: null,
        candidate_image_hash: '397cc3a82b14194f623efaa655b2e0535d5cd073b17b6290076a5925e08b3c58',
        status: 'no_face',
      },
    ],
  },
  {
    id: 'case_multi_face',
    name: 'Edge Case: Multi-Subject Crowd Photo',
    imageFileName: 'multi_face.jpg',
    imageSrc: '/multi_face.jpg',
    resolution: '2400x1600 (128.8 KB)',
    fileSize: '128.8 KB',
    imageHash: '47f682e945b659f93a9e490b9c9c4a2a864abe64dace175440fc7730e793ba67',
    faceQuality: {
      status: 'fail',
      face_count: 6,
      blur_variance: 284.1,
      brightness: 118.0,
      bbox: [120, 240, 520, 680],
      landmarks: [
        [240, 380],
        [390, 375],
        [310, 460],
        [260, 540],
        [370, 535],
      ],
      embedding_preview: [],
      reasons: [
        'Quality Gate: REJECTED',
        'Strict Rule Violation: MULTIPLE_FACES detected (count: 6)',
        'Forensic pipeline requires exactly 1 subject face',
      ],
    },
    candidates: [],
  },
];

export function createInitialEvidenceRecord(
  runId: string,
  candidateResult: VerificationResult
): EvidenceRecord {
  return {
    schema_version: '1.0.0',
    run_id: runId,
    source_url: candidateResult.candidate.url,
    domain: candidateResult.candidate.domain,
    platform: 'web',
    provider: candidateResult.candidate.provider,
    retrieved_at: candidateResult.candidate.discovered_at,
    title: candidateResult.candidate.title,
    text: candidateResult.candidate.snippet,
    image_sha256: candidateResult.candidate_image_hash,
    face_similarity: candidateResult.similarity,
    face_model: 'insightface-arcface-r100',
    candidate_quality: candidateResult.quality,
  };
}

export const SAMPLE_BLOCKCHAIN_RECORD: BlockchainRecord = {
  network: 'Ethereum Sepolia Testnet (Chain ID: 11155111)',
  contract_address: '0x71C2d385aE2F56d9812A45B8a9b70d41C68E3a9E',
  tx_hash: '0xb0e82a9b479d55edd0b6ff6f64cbf92948a3f1dcc23fb8a927679a34877cb7eb',
  block_number: 11651855,
  confirmations: 12,
  registered_root: 'fd02542e2348baa99fe5e00c550aadbd06c16e881ce77217ae58d4fc69b0e535',
  registered_image: 'e76f2345d86de6743e81bd761562b73035d26e96ff741bf9c528edac2fe58327',
  submitter: '0x34a1B75e19F8aB4639908F0945952c1Eb16B9b2c',
  timestamp: new Date().toISOString(),
};

export function generateRunId(): string {
  const ts = new Date()
    .toISOString()
    .replace(/[-:]/g, '')
    .replace(/\..+/, '');
  const rand = Math.random().toString(36).substring(2, 8);
  return `run_${ts}_${rand}`;
}
