# TEKMERION Architecture Specification

```text
┌─────────────────────────────────────────────────────────────────────┐
│                         TEKMERION                                  │
│            Face → Discovery → Verification → Evidence → Chain     │
└─────────────────────────────────────────────────────────────────────┘
```

## 1. System Subsystems & Boundaries

The codebase is organized into isolated, single-responsibility crates with strictly controlled dependency directions:

```text
apps/
└── tui/                  (Terminal UI, interactive dashboard, CLI dispatch)

crates/
├── core/                 (Pure domain models, state machine, pipeline events, interfaces)
├── config/               (Stage 0: Environment validation, RuntimeConfig, secret isolation)
├── security/             (Stage 1: Input validation, magic bytes, dimension bounds, SHA-256)
├── face/                 (Stage 2-3: SCRFD + ArcFace IPC client, face quality gate)
├── discovery/            (Stage 4-6: Provider abstraction, normalization, deduplication)
├── verification/         (Stage 7-10: Secure fetcher, candidate ArcFace matching, ranking engine)
├── evidence/             (Stage 11-13: RFC 8785 Canonical JSON, RFC 6962 binary Merkle tree)
├── storage/              (Stage 14, 23: Atomic run bundle persistence & .cache manager)
├── blockchain/           (Stage 15-16: Alloy/RPC client, Ethereum Sepolia contract verification)
└── audit/                (Stage 18: Structured audit logger & event stream)

workers/
└── face/                 (Isolated Python process running InsightFace SCRFD & ArcFace)

contracts/
└── EvidenceRegistry.sol  (On-chain Sepolia smart contract anchoring evidence roots)
```

### Dependency Graph

```text
             ┌─────────────┐
             │     TUI     │
             └──────┬──────┘
                    │
                    ▼
             ┌─────────────┐
             │   Pipeline  │
             └──────┬──────┘
                    │
       ┌────────────┼────────────┐
       ▼            ▼            ▼
     Face       Discovery    Verification
       │            │            │
       └────────────┼────────────┘
                    ▼
                 Evidence
                    │
             ┌──────┴──────┐
             ▼             ▼
          Storage      Blockchain
```

`tekmerion-core` owns the domain types and interfaces. It contains **no dependencies** on Ratatui, SerpApi, Ethereum, or Python, making the core domain 100% testable in isolation.

---

## 2. End-to-End Pipeline Stages

### Stage 0: Configuration (`crates/config`)
- Loads from `.env` and system environment variables into a validated `RuntimeConfig`.
- Sanitizes and redacts all secrets (`search_api_key`, `eth_private_key`) so secrets never leak into logs, evidence, console outputs, or Git.

### Stage 1: Input Validation (`crates/security`)
- Validates file existence, regular file properties, and read permissions.
- Whitelists file extensions (`.jpg`, `.jpeg`, `.png`) and verifies binary magic bytes (`FF D8 FF` for JPEG, `89 50 4E 47` for PNG).
- Enforces strict file size limits (<= 10MB) and resolution boundaries (min 64x64, max 4096x4096px).
- Generates `SHA-256(input_image)` as the immutable initial evidence identity.

### Stage 2: Face Analysis (`crates/face`, `workers/face`)
- Dispatches IPC request to the Python worker running InsightFace SCRFD and ArcFace.
- Detects face bounding boxes, 106-point landmarks, and computes unit-norm 512-D ArcFace embedding.
- Analyzes image quality (Laplacian blur variance, brightness, pose angles).

### Stage 3: Face Quality Gate
- Rejects inputs with 0 faces (`NO_FACE`).
- Rejects inputs with >1 faces (`MULTIPLE_FACES`) to guarantee single-subject forensic integrity.
- Rejects low-quality or blurry inputs (`LOW_FACE_QUALITY`).
- Halts pipeline execution before invoking external search APIs.

### Stage 4-6: Discovery & Normalization (`crates/discovery`)
- Queries reverse-image search providers via the `DiscoveryProvider` trait.
- Normalizes URLs (lowercasing host, removing default ports, sorting query parameters).
- Deduplicates results by canonical URL, image URL, and candidate image hash.

### Stage 7-10: Candidate Verification & Ranking (`crates/verification`)
- Securely downloads candidate assets with streaming byte limits and Content-Type validation.
- Extracts ArcFace embeddings for each candidate face.
- Computes cosine similarity against the query subject:
  $$\text{sim}(u, v) = \frac{u \cdot v}{\|u\|_2 \|v\|_2}$$
- Evaluates composite ranking:
  $$\text{Score} = 0.5 \times \text{Similarity} + 0.3 \times \text{Quality} + 0.2 \times \text{Relevance}$$

### Stage 11-13: Evidence Engine & Merkle Tree (`crates/evidence`)
- Formats evidence records with RFC 8785 Canonical JSON (deterministic key sorting, fixed-precision floats, NFC UTF-8).
- Builds 5 domain-separated leaves:
  - Leaf 0 (`IDENTIFIER`): `run_id`, `schema_version`, `platform`
  - Leaf 1 (`CONTENT`): `domain`, `source_url`, `text`, `title`
  - Leaf 2 (`PROVENANCE`): `provider`, `retrieved_at`
  - Leaf 3 (`ASSET`): `face_model`, `image_sha256`
  - Leaf 4 (`BIOMETRIC`): `candidate_quality`, `face_similarity`
- Leaf hashing with `0x00` domain separation:
  $$H_{\text{leaf}} = \text{SHA256}(0x00 \mathbin{\Vert} \text{CanonicalJSON})$$
- Internal node hashing with `0x01` domain separation:
  $$H_{\text{node}} = \text{SHA256}(0x01 \mathbin{\Vert} H_{\text{left}} \mathbin{\Vert} H_{\text{right}})$$

### Stage 14: Evidence Persistence (`crates/storage`)
- Atomically writes run bundles using temporary file creation and atomic rename:
  ```text
  runs/<run_id>/
  ├── input/
  │   ├── input.jpg
  │   └── input_metadata.json
  ├── discovery/
  │   └── candidates.json
  ├── verification/
  │   └── results.json
  ├── evidence/
  │   ├── evidence.json
  │   ├── leaves.json
  │   └── root.json
  ├── blockchain/
  │   └── transaction.json
  └── audit.jsonl
  ```

### Stage 15-17: Blockchain Anchoring & Verification (`crates/blockchain`)
- Submits or simulates registration on `EvidenceRegistry.sol` on Ethereum Sepolia.
- Reads contract state via JSON-RPC to verify that the local Merkle root matches the on-chain anchor bit-for-bit.
- Proves tamper resistance by altering local records and demonstrating root divergence.

---

## 3. Production State Machine

The pipeline progresses through 21 formal states with deterministic failure transitions:

```text
IDLE
 │
 ▼
INPUT_VALIDATING
 │
 ▼
INPUT_READY
 │
 ▼
FACE_ANALYSIS
 │
 ├──── failure ───→ ERROR
 │
 ▼
FACE_VERIFIED
 │
 ▼
SEARCHING
 │
 ├──── failure ───→ SEARCH_FAILURE
 │
 ▼
CANDIDATES_FOUND
 │
 ▼
DOWNLOADING
 │
 ▼
VERIFYING_CANDIDATES
 │
 ├──── no match ──→ NO_VERIFIED_MATCH
 │
 ▼
MATCH_FOUND
 │
 ▼
EVIDENCE_CREATING
 │
 ▼
ROOT_COMPUTED
 │
 ▼
BLOCKCHAIN_SUBMITTING
 │
 ├──── failure ───→ TRANSACTION_FAILURE
 │
 ▼
BLOCKCHAIN_CONFIRMED
 │
 ▼
ONCHAIN_VERIFYING
 │
 ├──── mismatch ──→ ONCHAIN_MISMATCH
 │
 ▼
VERIFIED
```

---

## 4. Concurrency & Resource Bounding

To prevent resource exhaustion during multi-candidate verification:
- `MAX_CONCURRENT_DOWNLOADS`: Bounded to 4 parallel HTTP streams.
- `MAX_CONCURRENT_FACE_JOBS`: Bounded to 2 worker tasks to prevent GPU/CPU RAM spikes.
- `MAX_DOWNLOAD_BYTES`: Capped at 10 MB per image.
- `HTTP_TIMEOUT_SECONDS`: Strict 30-second socket timeout with cancellation.
