# TEKMERION Demonstration Guide & Video Recording Protocol

This document provides the exact script, timing timeline, test assets, CLI commands, and visual proof checkpoints for recording the TEKMERION golden run acceptance video.

---

## 1. Demonstration Timeline (00:00 - 02:35)

The video recording serves as an undeniable proof-of-work demonstration of the live end-to-end evidence pipeline:

| Timestamp | Phase | Action & Visual Output | Verification Checkpoint |
|-----------|-------|------------------------|-------------------------|
| **00:00 - 00:10** | **Initialization & Configuration** | Launch TEKMERION (`tekmerion run assets/query_face.jpg` or TUI/Web UI). Display validated runtime configuration with secret redaction (`ETH_RPC_URL`, Sepolia contract address). | Stage 0 Config Validated |
| **00:10 - 00:20** | **Input Security Validation** | Input image passes existence, magic byte inspection (JPEG `FF D8 FF`), size bounds check (52.4 KB), and SHA-256 fingerprinting. | `SHA256: 0e7826...` generated |
| **00:20 - 00:35** | **Face Detection & ArcFace Encoding** | SCRFD worker identifies single high-quality face (1.00 score, pose/blur within thresholds). 512-D ArcFace L2-normalized embedding extracted. | Quality gate passed (`1 FACE, HIGH_QUALITY`) |
| **00:35 - 00:55** | **Live Reverse-Image Discovery** | Discovery orchestrator queries search provider. Live candidates parsed, normalized, and deduplicated. Candidate image downloaded via secure HTTPS fetcher. | Real candidate URLs & metadata shown |
| **00:55 - 01:15** | **Candidate Face Verification** | ArcFace compares candidate face with query embedding. Cosine similarity computed: **`0.941`** (exceeds `0.80` threshold). Match selected. | Legitimate verified match |
| **01:15 - 01:30** | **Evidence Merkle Tree Construction** | Deterministic canonicalization (RFC 8785). 7 RFC 6962 leaves hashed. Binary Merkle tree computes root hash: `0x...`. Run persisted atomically in `runs/<run_id>/`. | Atomic run artifacts written |
| **01:30 - 01:55** | **Ethereum Sepolia Registration** | Alloy client submits EIP-1559 transaction to `EvidenceRegistry.sol` (`0x712a7f0e...`). Displays pending transaction hash, waits for block confirmation. | Sepolia Block confirmed (~12-15s) |
| **01:55 - 02:10** | **On-Chain Verification** | Reconstructs local evidence bundle, queries Sepolia smart contract `getEvidence(imageHash)`. Root A matches Root B. | `STATUS: VERIFIED ✓` |
| **02:10 - 02:35** | **Tamper Detection Demonstration** | Runs `tekmerion tamper-test <run-id>`. Modifies one local byte in evidence bundle, recomputes root, compares with Sepolia anchor. Pinpoints leaf divergence. | `RESULT: TAMPER DETECTED ⚠` |

---

## 2. Test Assets & Consented Data

To ensure ethical standards and avoid harvesting private individuals, the demonstration utilizes public domain and consented test portraits:

- **Query Image**: `assets/query_face.jpg`
  - Dimensions: `512x512`
  - Format: JPEG (MIME: `image/jpeg`)
  - Target: Standard consented benchmark face
- **Contract Address on Sepolia**: `0x712a7f0e014A895FdBdf0F46aA3199859fC8A79E`
- **RPC Endpoint**: Ethereum Sepolia via Infura / Alchemy (`https://rpc.sepolia.org`)

---

## 3. Command-Line Interface (CLI) Demonstration Commands

### Step 1: Inspect Blockchain Contract
```bash
tekmerion contract-info
```
*Expected Output:*
```text
=== TEKMERION BLOCKCHAIN CONTRACT STATUS ===
RPC Endpoint:     https://rpc.sepolia.org
Chain ID:         11155111 (Sepolia)
Contract Address: 0x712a7f0e014A895FdBdf0F46aA3199859fC8A79E
Submitter Address: 0x51E283594Bff94112921C260b0d621feFe93998b
Contract Code:    PRESENT (Deployed)
Latest Block:     #11652126
```

### Step 2: Execute End-to-End Pipeline
```bash
tekmerion run assets/query_face.jpg
```
*For structured machine output:*
```bash
tekmerion run assets/query_face.jpg --json
```

### Step 3: Inspect Run Artifacts
```bash
tekmerion inspect <run-id>
```
*Displays:*
- Query Image SHA-256 and input dimensions.
- Verified Candidate: Title, URL, Cosine Similarity (`0.941`).
- Merkle Root Hash (`0x...`).
- Blockchain Confirmation: TX hash, block number, Sepolia Etherscan link.
- Chronological Audit Log (`audit.jsonl`).

### Step 4: Verify Evidence Against Sepolia
```bash
tekmerion verify <run-id>
```
*Expected Output:*
```text
=== TEKMERION ON-CHAIN VERIFICATION ===
Recomputed Local Root: 0x5776d65c3b6d2e67df146d9a9f939e144a1bfaee5c93c407817eb65e8a60965d
On-Chain Stored Root:  0x5776d65c3b6d2e67df146d9a9f939e144a1bfaee5c93c407817eb65e8a60965d
STATUS: VERIFIED (Local evidence exactly matches immutable Sepolia anchor!)
```

### Step 5: Execute Tamper Verification Test
```bash
tekmerion tamper-test <run-id>
```
*Expected Output:*
```text
=== TEKMERION TAMPER DETECTION TEST ===
Original Root Hash: 0x5776d65c3b6d2e67df146d9a9f939e144a1bfaee5c93c407817eb65e8a60965d
On-Chain Root Hash:  0x5776d65c3b6d2e67df146d9a9f939e144a1bfaee5c93c407817eb65e8a60965d
[MUTATING EVIDENCE FIELD: candidate.similarity += 0.05]
Tampered Local Root: 0x9a842cf03b71946e3a0937a347eb10bfa816912384a8bc391a673b648f07aa22
RESULT: TAMPER DETECTED!
Local evidence divergence pinpointed. Cryptographic verification rejected!
```

---

## 4. Web UI Demonstration Protocol

The web application is accessible at `http://localhost:5174/` (backed by the production pipeline runner on port `3001`):

1. **Upload & Run**:
   - Drag and drop `assets/query_face.jpg` onto the Query Input dropzone.
   - Click **"Launch Evidence Pipeline"**.
2. **Real-time Pipeline Telemetry**:
   - Observe live status badges updating across the 18-stage pipeline.
   - Inspect the face detection bounding box and ArcFace quality gauges.
3. **Live Evidence Tree & Blockchain Explorer**:
   - View the 7 Merkle leaves rendered with cryptographic SHA-256 prefixes.
   - Click the Sepolia transaction link to inspect the on-chain receipt.
4. **Interactive Tamper Challenge**:
   - Click **"Run Tamper Challenge"** in the UI to demonstrate instant cryptographic invalidation when evidence metadata is modified.

---

## 5. Requirement Verification Checklist

- [x] SCRFD face detection with landmark alignment.
- [x] ArcFace 512-D L2-normalized embedding extraction.
- [x] Live web search query orchestration with deduplication.
- [x] Strict input validation boundary (`crates/security`).
- [x] Independent candidate face verification.
- [x] RFC 8785 canonical JSON serialization.
- [x] RFC 6962 Merkle tree with domain separation.
- [x] Atomic evidence bundle persistence (`crates/storage`).
- [x] Ethereum Sepolia smart contract registration (`EvidenceRegistry.sol`).
- [x] Dual verification: Local re-hashing vs. Sepolia on-chain anchor.
- [x] Mathematical tamper detection isolating leaf divergences.
