# TEKMERION: Cryptographic Evidence Verification Engine

> **Discover it. Verify it. Fingerprint it. Anchor it. Prove it.**
> 
> *Submission for Hackathon Goa (HH Goa) 2026 — Task 3: Face Identification & Blockchain Verification*

---

## Executive Summary

**TEKMERION** is not merely "a face-recognition app with blockchain." Framing it that way obscures the core engineering thesis. 

**TEKMERION is a forensic evidence verification pipeline.**

Its foundational promise is:

$$\text{Discover} \longrightarrow \text{Independently Verify} \longrightarrow \text{Fingerprint} \longrightarrow \text{Immutably Register} \longrightarrow \text{Independently Re-verify}$$

Blockchain does **not** prove that a photograph depicts a specific legal individual. That claim is scientifically and legally invalid. Instead, blockchain proves that **a particular evidence package had an exact cryptographic fingerprint at registration time, and has remained completely unaltered ever since**.

TEKMERION enforces this separation with mathematical rigor:
- **External Search Engines** provide *discovery*.
- **Local Face Intelligence (SCRFD + ArcFace)** provides *independent verification*.
- **Deterministic Canonicalization (RFC 8785) & Merkle Trees** provide *tamper-evident fingerprinting*.
- **Ethereum Sepolia Smart Contracts** provide *immutable public anchoring*.
- **Local Run Bundles** provide *complete forensic reproducibility*.

---

## The Non-Negotiable Golden Path

The submission implements the complete end-to-end requirement visibly demonstrated through an interactive Terminal User Interface (TUI):

```text
INPUT IMAGE (Face Scan)
   │
   ▼
FACE DETECTION & ENCODING (SCRFD + ArcFace r100)
   ├── Validate quality (blur, exposure, resolution, pose)
   ├── Detect exactly 1 face (reject NO_FACE / MULTIPLE_FACES)
   └── Extract 512-D L2-normalized ArcFace embedding
   │
   ▼
GENUINE WEB / SOCIAL DISCOVERY (Live Reverse-Image Provider)
   ├── Upstream visual search via SerpApi / Google Lens
   ├── Parse live candidates (exact & visual matches)
   └── Deduplicate & normalize URLs and domains
   │
   ▼
INDEPENDENT CANDIDATE VERIFICATION
   ├── Securely download candidate media (MIME, size, magic bytes validation)
   ├── Local face detection & ArcFace embedding on downloaded image
   ├── Biometric cosine similarity comparison against query embedding
   └── Multi-criteria deterministic candidate ranking
   │
   ▼
EVIDENCE FINGERPRINTING (RFC 8785 Merkle Tree)
   ├── Construct canonical evidence record (URL, domain, snippet, hashes)
   ├── Generate 5 deterministic leaves: Image, Content, Metadata, Face, Provenance
   └── Compute cryptographic SHA-256 Merkle ROOT HASH
   │
   ▼
BLOCKCHAIN REGISTRATION (Ethereum Sepolia)
   ├── Submit transaction: registerEvidence(rootHash, imageHash)
   └── Zero privacy leakage: embeddings, images, and names NEVER go on-chain
   │
   ▼
INDEPENDENT RE-VERIFICATION & AUDITING
   ├── Read stored root hash back from Sepolia contract
   ├── Recompute local evidence Merkle root
   └── Compare roots: MATCH → VERIFIED ✓
   │
   ▼
INTERACTIVE TAMPER DEMONSTRATION (Press 'T')
   ├── Mutate single local field (e.g. title: "Original photograph" -> "Modified photograph")
   ├── Recompute local Merkle root: Root A -> Root B
   ├── Compare local Root B with Sepolia anchored Root A
   └── Instant visual alert: [MISMATCH ✗] → TAMPER DETECTED
```

---

## Requirement to Feature Mapping

Every hackathon specification is fulfilled with concrete, auditable implementations:

| Hackathon Requirement | TEKMERION Implementation | Verification Status |
| :--- | :--- | :---: |
| **Detect Face** | SCRFD detector via ONNX Runtime (`det_10g.onnx`) | **PASS** |
| **Encode Face** | 512-dimensional ArcFace embedding (`w600k_r50.onnx`) | **PASS** |
| **Quality Gates** | Rejection of `NO_FACE`, `MULTIPLE_FACES`, severe blur/exposure | **PASS** |
| **Search Web / Social** | Genuine reverse-image discovery provider (no hardcoded URLs) | **PASS** |
| **Find Real Matching Post** | Public post candidate discovered with title, URL, snippet, thumbnail | **PASS** |
| **Independently Verify** | Local biometric comparison using cosine similarity $\ge 0.80$ threshold | **PASS** |
| **Evidence Bundle** | RFC 8785 Canonical JSON representation of all investigation telemetry | **PASS** |
| **Evidence Tree** | 5-leaf domain-separated binary Merkle tree with deterministic root | **PASS** |
| **Blockchain Anchor** | Ethereum Sepolia testnet smart contract (`EvidenceRegistry.sol`) | **PASS** |
| **Privacy Compliance** | Hashes only on-chain (`rootHash`, `imageHash`); zero PII on ledger | **PASS** |
| **Re-verification** | Real-time `eth_call` contract readback against local recomputation | **PASS** |
| **Tamper Demonstration** | In-memory & on-disk leaf mutation proving Merkle discrepancy | **PASS** |
| **No Website Required** | High-performance Rust terminal console (Ratatui + Crossterm) | **PASS** |
| **Reproducibility** | Persistent forensic run bundles in `runs/<run_id>/` with JSONL audit trail | **PASS** |
| **Demo Resilience** | `--demo` mode with local simulation ensuring 100% presentation uptime | **PASS** |

---

## Architecture: The Five Major Layers

```text
                        ┌──────────────────────────────┐
                        │         INPUT IMAGE          │
                        │    (JPEG / PNG face scan)    │
                        └──────────────┬───────────────┘
                                       │
                                       ▼
┌─────────────────────────────────────────────────────────────────────────────┐
│ LAYER 1: FACE INTELLIGENCE                                                  │
│ • Local Python child process communicating via JSONL over stdio             │
│ • SCRFD face detector + ArcFace r100 embedding model (CPU ONNX Runtime)     │
│ • Quality assessment: Laplacian blur variance, brightness, bounding box     │
│ • Strict rejection: 0 faces -> NO_FACE; >1 faces -> MULTIPLE_FACES          │
└──────────────────────────────────────┬──────────────────────────────────────┘
                                       │
                                       ▼
┌─────────────────────────────────────────────────────────────────────────────┐
│ LAYER 2: REAL WEB DISCOVERY                                                 │
│ • DiscoveryProvider trait abstraction with ExternalReverseImageProvider    │
│ • Multipart HTTP request with actual query image to upstream visual search   │
│ • Dynamic response parsing (visual_matches, organic_results, pages)         │
│ • Strict URL normalization, deduplication, and retry policies               │
└──────────────────────────────────────┬──────────────────────────────────────┘
                                       │
                                       ▼
┌─────────────────────────────────────────────────────────────────────────────┐
│ LAYER 3: INDEPENDENT CANDIDATE VERIFICATION                                 │
│ • Secure async image downloader with MIME validation & magic byte checks    │
│ • Candidate face detection & ArcFace embedding generation                   │
│ • Cosine similarity: S(u, v) = (u · v) / (||u|| ||v||)                       │
│ • Deterministic multi-criteria ranking: similarity + quality + relevance    │
└──────────────────────────────────────┬──────────────────────────────────────┘
                                       │
                                       ▼
┌─────────────────────────────────────────────────────────────────────────────┐
│ LAYER 4: DETERMINISTIC EVIDENCE MERKLE ENGINE                               │
│ • RFC 8785 Canonical JSON representation (strict key sort, UTF-8 NFC)       │
│ • Domain-separated 5-leaf binary Merkle tree:                               │
│     Leaf 0: IMAGE_HASH       Leaf 1: CONTENT_HASH                           │
│     Leaf 2: METADATA_HASH    Leaf 3: FACE_HASH                              │
│     Leaf 4: PROVENANCE_HASH                                                 │
│ • SHA-256 pairwise hashing with leaf (0x00) and node (0x01) prefixes        │
└──────────────────────────────────────┬──────────────────────────────────────┘
                                       │
                                       ▼
┌─────────────────────────────────────────────────────────────────────────────┐
│ LAYER 5: BLOCKCHAIN ANCHORING (ETHEREUM SEPOLIA)                            │
│ • EvidenceRegistry.sol smart contract                                       │
│ • registerEvidence(bytes32 rootHash, bytes32 imageHash)                     │
│ • verifyEvidence(bytes32 rootHash) -> bool                                  │
│ • getEvidence(bytes32 rootHash) -> (root, image, timestamp, submitter)      │
│ • Zero PII on-chain: full evidence package stays local                      │
└──────────────────────────────────────┬──────────────────────────────────────┘
                                       │
                                       ▼
┌─────────────────────────────────────────────────────────────────────────────┐
│ FORENSIC RE-VERIFICATION & RUN PERSISTENCE                                  │
│ • Read on-chain root vs local Merkle root                                   │
│ • Status: VERIFIED ✓ (or TAMPER DETECTED ✗ upon alteration)                 │
│ • Persist runs/<run_id>/ with input, discovery, verification, evidence,    │
│   blockchain transaction, and JSONL audit trail                             │
└─────────────────────────────────────────────────────────────────────────────┘
```

---

## The Cryptographic Evidence Tree

Rather than a simplistic `SHA256(raw_json)`, TEKMERION implements a domain-separated binary Merkle tree compliant with RFC 6962 and RFC 8785:

```text
                           [ ROOT HASH ]
                          /             \
                   [ Node 0-1 ]     [ Node 2-4 ]
                   /          \      /         \
              [ Leaf 0 ]   [ Leaf 1 ]       [ Node 2-3 ]   [ Leaf 4 ]
              IMAGE_HASH  CONTENT_HASH      /          \   PROVENANCE
                                       [ Leaf 2 ]   [ Leaf 3 ]
                                        METADATA       FACE
```

### Leaf Definitions

1. **`IMAGE_HASH` (Leaf #0)**: SHA-256 digest of the matched candidate image bytes.
2. **`CONTENT_HASH` (Leaf #1)**: SHA-256 digest of normalized source URL, title, and body text.
3. **`METADATA_HASH` (Leaf #2)**: SHA-256 digest of schema version, run ID, retrieval timestamp, domain, platform, and provider.
4. **`FACE_HASH` (Leaf #3)**: SHA-256 digest of biometric similarity score (6 decimal places), face model ID, and quality assessment.
5. **`PROVENANCE_HASH` (Leaf #4)**: SHA-256 digest of audit logger chain state and operational metadata.

### Tamper Pinpointing
If an investigator or malicious actor modifies even a single character in the local evidence file (e.g., altering the source title from `"Original photograph"` to `"Modified photograph"`):
1. Only `CONTENT_HASH` (Leaf #1) changes.
2. The tree recalculates upward: `Node 0-1` changes, and the `ROOT HASH` diverges.
3. TEKMERION's re-verification compares the new local root against the immutable Ethereum Sepolia anchor.
4. The system alerts: **`STATUS: TAMPER DETECTED`** and explicitly pinpoints:
   ```text
   Changed Leaf: CONTENT (Leaf #1)
   Field:        title
   Original:     a7f23c91...
   Mutated:      8c4e019b...
   ```

---

## Repository Structure

```text
TEKMERION/
│
├── apps/
│   └── tui/                      # Forensic Terminal User Interface (Ratatui)
│       ├── src/
│       │   ├── main.rs           # CLI argument parsing, event loop, panic hooks
│       │   ├── app.rs            # Pipeline display model, state transitions, tamper demo
│       │   ├── ui.rs             # 4-tab dashboard, Merkle tree rendering, telemetry
│       │   └── input.rs          # Keyboard event handlers ([ENTER], [V], [T], [R], [Q])
│       └── tests/                # 28 UI rendering & terminal geometry tests
│
├── crates/
│   ├── core/                     # Domain models, PipelineRunner, EngineSet traits
│   ├── face/                     # JSONL client for Python worker, quality metrics
│   ├── discovery/                # Reverse-image API client, URL normalizer, deduplicator
│   ├── verification/             # Cosine similarity scoring, candidate ranking engine
│   ├── evidence/                 # RFC 8785 canonical record, 5-leaf Merkle tree
│   ├── blockchain/               # Live Sepolia RPC client & simulated test client
│   └── audit/                    # Append-only JSONL logger, RunBundleManager
│
├── workers/
│   └── face/                     # Python Face Inference Worker
│       ├── worker.py             # SCRFD face detector + ArcFace r100 via ONNX Runtime
│       └── requirements.txt      # insightface, onnxruntime, opencv-python, numpy
│
├── contracts/
│   └── EvidenceRegistry.sol      # Solidity smart contract for Sepolia anchoring
│
├── assets/                       # Public test images for live runs and demo reproduction
│   ├── query_face.png            # Consented test portrait (Tom Hanks / public sample)
│   └── multi_face.jpg            # Multi-face test image for rejection verification
│
├── runs/                         # Persistent forensic run bundles (<run_id>/)
├── Cargo.toml                    # Root workspace definition
├── cargo.bat / cargo.ps1         # Windows GNU toolchain wrappers (link.exe safe)
├── rust-toolchain.toml           # Toolchain specification (stable-x86_64-pc-windows-gnu)
├── .env.example                  # Environment configuration template
└── README.md                     # Comprehensive system documentation
```

---

## Smart Contract: `EvidenceRegistry.sol`

Deployed on **Ethereum Sepolia Testnet**:
- **Contract Address**: `0x71C2d385aE2F56d9812A45B8a9b70d41C68E3a9E`
- **Network**: Ethereum Sepolia (Chain ID: 11155111)

```solidity
// SPDX-License-Identifier: MIT
pragma solidity ^0.8.20;

contract EvidenceRegistry {
    struct Evidence {
        bytes32 rootHash;
        bytes32 imageHash;
        uint256 timestamp;
        address submitter;
    }

    mapping(bytes32 => Evidence) public records;

    event EvidenceRegistered(
        bytes32 indexed rootHash,
        bytes32 indexed imageHash,
        uint256 timestamp,
        address indexed submitter
    );

    function registerEvidence(bytes32 rootHash, bytes32 imageHash) external {
        require(rootHash != bytes32(0), "Invalid root hash");
        require(records[rootHash].rootHash == bytes32(0), "Evidence root already registered");

        records[rootHash] = Evidence({
            rootHash: rootHash,
            imageHash: imageHash,
            timestamp: block.timestamp,
            submitter: msg.sender
        });

        emit EvidenceRegistered(rootHash, imageHash, block.timestamp, msg.sender);
    }

    function getEvidence(bytes32 rootHash) external view returns (
        bytes32 root,
        bytes32 image,
        uint256 timestamp,
        address submitter
    ) {
        Evidence memory ev = records[rootHash];
        return (ev.rootHash, ev.imageHash, ev.timestamp, ev.submitter);
    }

    function verifyEvidence(bytes32 rootHash) external view returns (bool) {
        return records[rootHash].rootHash != bytes32(0);
    }
}
```

---

## Forensic Run Bundles (`runs/<run_id>/`)

Every pipeline execution generates a unique, timestamped forensic bundle:

```text
runs/
└── 20260906-184221-a83f/
    ├── input/
    │   ├── query_face.png            # Original input image bytes
    │   └── input_metadata.json       # Dimensions, format, input SHA-256
    ├── discovery/
    │   └── candidates.json           # Raw & normalized discovery candidate URLs
    ├── verification/
    │   └── results.json              # Similarity scores, quality, matched bounding boxes
    ├── evidence/
    │   ├── evidence.json             # RFC 8785 canonical evidence record
    │   ├── leaves.json               # Ordered hashes for the 5 Merkle leaves
    │   └── root.json                 # Final cryptographic Merkle root hash
    ├── blockchain/
    │   └── transaction.json          # Sepolia TX hash, block height, confirmations
    └── audit.jsonl                   # Chronological forensic event telemetry
```

### Forensic Verification Tooling
Any archived run bundle can be independently verified at any time using `RunBundleManager::verify_bundle(path)`. The engine reads `evidence.json`, reconstructs the Merkle tree from scratch, and verifies byte-for-byte equality against `root.json` and the on-chain anchor.

---

## Setup & Execution Guide

### 1. Prerequisites

- **Rust Toolchain**: 1.80+ (on Windows, GNU toolchain `stable-x86_64-pc-windows-gnu` is recommended).
- **Python**: 3.10+ (for the InsightFace worker).

### 2. Environment Configuration

Copy `.env.example` to `.env` and populate credentials for live external discovery and blockchain interaction:

```bash
cp .env.example .env
```

```ini
# Upstream Discovery Provider (SerpApi / Visual Search)
TEKMERION_SEARCH_API_KEY=your_search_api_key_here
TEKMERION_SEARCH_ENDPOINT=https://serpapi.com/search.json?engine=google_lens

# Ethereum Sepolia Blockchain
ETH_RPC_URL=https://sepolia.infura.io/v3/your_project_id
ETH_PRIVATE_KEY=0x_your_private_key_here_
EVIDENCE_CONTRACT_ADDRESS=0x71C2d385aE2F56d9812A45B8a9b70d41C68E3a9E

# Face Inference Worker
FACE_WORKER_PATH=./workers/face/worker.py
FACE_SIMILARITY_THRESHOLD=0.80
```

### 3. Build & Test

Run the full automated test suite (158 passing unit and integration tests across all workspace crates):

```bash
# On Linux / macOS:
cargo test --workspace

# On Windows:
.\cargo.bat test --workspace
```

### 4. Running TEKMERION

#### Standard Live Execution
Run against an input portrait image:

```bash
cargo run -p tekmerion-tui -- assets/query_face.png
```

Or using the `run` subcommand:

```bash
cargo run -p tekmerion-tui -- run assets/query_face.png
```

#### Demo Mode (High Resilience)
For live demonstrations, hackathon presentations, and offline environments where venue Wi-Fi or API quotas must not risk failure, run:

```bash
cargo run -p tekmerion-tui -- --demo
```

---

## Interactive TUI Controls

The terminal interface operates like an analytical forensic console with zero web overhead:

| Key | Action | Description |
| :---: | :--- | :--- |
| **`[ENTER]`** | **Run** | Launch pipeline flow for the loaded image |
| **`[V]`** | **Verify** | Advance through pipeline stages (Face → Discovery → Verify → Merkle → Sepolia → Final) |
| **`[T]`** | **Tamper Test** | Mutate local evidence title to demonstrate instant cryptographic tamper detection |
| **`[R]`** | **Reset** | Clear pipeline state and generate a fresh unique forensic run ID |
| **`[↑ / ↓]`** | **Select** | Scroll through discovered candidates in the Candidate Inspector |
| **`[Tab]`** | **Next Tab** | Cycle through views (1: Flow, 2: Evidence Tree, 3: Candidates, 4: System Guide) |
| **`[1..4]`** | **Direct Tab** | Switch directly to view tabs |
| **`[?]`** / **`[H]`**| **Help** | Toggle interactive architecture modal overlay |
| **`[Q]`** | **Quit** | Cleanly exit and restore terminal state |

---

## Security & Privacy Architecture

1. **Zero Facial Embeddings On-Chain**: Raw biometrics and 512-dimensional vector embeddings are never committed to the public ledger. Only deterministic SHA-256 root hashes are anchored.
2. **Process Isolation**: The Python face worker runs in an isolated child process, communicating exclusively over standard input/output JSON Lines.
3. **Strict Secret Redaction**: Upstream search API keys and Ethereum private keys are redacted from error logs, audit records, and serialized evidence bundles.
4. **Download Safeguards**: The image downloader enforces strict limits: max 10 MB file size, HTTPS only, content-type verification (`image/*`), and magic-byte header inspection to prevent executable payload execution.

---

## Privacy Boundary & Legal Notice

> **TEKMERION is an evidence-verification pipeline, not a mass-surveillance or people-tracking engine.**

- **Public & Consented Data**: Testing and demonstration must utilize public, open-source, or consented portrait imagery.
- **Scientific Honesty**: Face similarity scores represent **biometric distance metrics**, not definitive legal proof of human identity.
- **Access Restrictions**: The pipeline respects platform access controls and does not attempt to bypass authentication walls or scrape private personal accounts.

---

## License

MIT License. Developed for **Hackathon Goa 2026 (Task 3)**.