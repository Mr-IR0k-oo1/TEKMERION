# TEKMERION Security Architecture & Hardening Specification

This document details the security posture, threat model, boundaries, cryptographic standards, and hardening measures implemented across the TEKMERION Evidence Pipeline.

---

## 1. Threat Model & Attack Surface

TEKMERION operates in an untrusted web environment where it accepts user-supplied query images, queries external search engines, retrieves arbitrary third-party web content, and notarizes evidentiary records onto a public blockchain.

### Primary Threat Vectors

1. **Malicious / Corrupted Query Imagery**:
   - Decompression bombs (Zip bombs / GZIP bombs) disguised as images.
   - Polyglot binaries (e.g. valid JPEG header followed by executable shellcode).
   - Extreme dimension attacks (e.g. 100,000 x 100,000 px headers designed to trigger out-of-memory allocation).
   - Low-quality or non-face noise intended to exhaust downstream neural network resources.

2. **Untrusted Web Candidate Retrieval**:
   - SSRF (Server-Side Request Forgery) attacks via malicious reverse-image search redirects targeting loopback (`127.0.0.1`) or cloud metadata endpoints (`169.254.169.254`).
   - Infinite HTTP redirect loops or slowloris candidate endpoints stalling worker threads.
   - Excessive response payloads attempting RAM exhaustion.
   - Remote code execution attempts via image format parsing vulnerabilities (libpng/libjpeg zero-days).

3. **Evidence Tampering & Forgery**:
   - Local post-capture tampering of timestamps, URLs, face match scores, or candidate metadata.
   - Second preimage attacks on Merkle trees where interior nodes are presented as leaves or vice versa.
   - Non-deterministic JSON serialization causing identical evidence records to generate mismatched root hashes.
   - Unauthorized on-chain notarization and replay attacks.

4. **Credential & Privacy Exfiltration**:
   - Accidental leakage of Ethereum private keys or search API tokens in logs, audits, or smart contract transaction calldata.
   - PII/biometric exfiltration of face embeddings into public blockchain logs.

---

## 2. Security Boundaries

### Stage 1: Input Boundary (`crates/security`)

All user images entering the pipeline must pass through the `InputValidator` before touching any AI runtime or Python worker:

```text
Image Path
    │
    ▼
File Existence & Readable Check  ──(FAIL)──► INVALID_FILE
    │
    ▼
File Extension Allowlist         ──(FAIL)──► UNSUPPORTED_FORMAT
    │
    ▼
File Size Boundary [1KB - 10MB]  ──(FAIL)──► FILE_TOO_LARGE / IMAGE_TOO_SMALL
    │
    ▼
Magic Byte Verification          ──(FAIL)──► UNSUPPORTED_FORMAT
(PNG: 89 50 4E 47 0D 0A 1A 0A, JPEG: FF D8 FF)
    │
    ▼
Header Dimension Decoding        ──(FAIL)──► IMAGE_DECODE_FAILURE
(Min: 64x64, Max: 4096x4096)
    │
    ▼
SHA-256 Computation
    │
    ▼
FACE_ANALYSIS READY
```

### Candidate Fetcher Boundary (`crates/discovery`)

Candidate URLs returned by live search providers are fetched under strict resource sandboxing:
- **HTTPS Enforcement**: Non-encrypted HTTP links are rejected by default.
- **Streaming Byte Cap**: An atomic byte-counter terminates any candidate download exceeding `MAX_DOWNLOAD_BYTES` (default 10 MB) during stream consumption.
- **Redirect Limitation**: Maximum of 3 redirects allowed. Redirection targets are validated against private IP blocks (SSRF protection).
- **Execution Isolation**: Downloaded images are stored in temporary files, decoded into memory as uncompressed pixel buffers, and never executed as code.

### Face Inference Worker Boundary (`workers/face`)

- Communication occurs via standard JSON-RPC over stdin/stdout pipes.
- Worker process runs with isolated working directories and zero access to system environment variables containing Ethereum private keys.
- Worker execution is wrapped in a strict timeout (`FACE_WORKER_TIMEOUT_SECS`). If the worker stalls or hangs, the Rust supervisor forcefully terminates the process tree, emits `WORKER_TIMEOUT`, and fails cleanly without orphan/zombie processes.

---

## 3. Secret Hygiene & Zero-Leakage Policy

TEKMERION enforces a zero-tolerance policy for secret exposure across all stages:

```rust
// crates/config/src/lib.rs
impl RuntimeConfig {
    pub fn redacted_view(&self) -> String {
        // Redacts eth_private_key and search_api_key before display/logging
    }
}
```

1. **Environment Variable Ingestion**:
   - `ETH_PRIVATE_KEY` and `TEKMERION_SEARCH_API_KEY` are parsed into private memory structs.
   - Standard `Debug` and `Display` implementations for configuration redact secret fields as `"[REDACTED]"`.
2. **Exclusion from Evidence Bundles**:
   - `evidence.json`, `leaves.json`, `root.json`, and `audit.jsonl` contain no authorization headers, API keys, or private key materials.
3. **Smart Contract Calldata Sanitization**:
   - The on-chain contract `EvidenceRegistry.sol` only accepts `(bytes32 rootHash, bytes32 imageHash)`. No biometric embeddings or PII are ever stored on-chain.

---

## 4. Cryptographic Standards & Merkle Determinism

### Deterministic JSON Canonicalization (RFC 8785 JCS)

In standard JSON serialization, key ordering, whitespace, and floating-point representations can vary across platforms, resulting in different SHA-256 hashes for logically identical objects.

TEKMERION addresses this via strict canonicalization:
- Dictionary keys are lexicographically sorted by UTF-16 code units.
- Whitespace outside strings is completely stripped.
- Floating-point numbers are formatted with deterministic precision.
- Text encodings are enforced as UTF-8.

### Merkle Tree Domain Separation (RFC 6962)

To protect against second preimage attacks (where an attacker crafts an internal node that collides with a leaf node), domain separation prefixes are applied prior to SHA-256 hashing:

```text
Leaf Hash = SHA-256("TEKMERION:LEAF:" || LeafType || LeafData)
Node Hash = SHA-256("TEKMERION:NODE:" || LeftChildHash || RightChildHash)
```

The evidence tree hierarchy comprises 7 deterministic leaves:
1. `IMAGE_HASH`: SHA-256 of raw query image.
2. `TEXT_HASH`: SHA-256 of candidate title and extracted text.
3. `URL_HASH`: SHA-256 of canonical source URL.
4. `SOURCE_HASH`: SHA-256 of domain and provider metadata.
5. `METADATA_HASH`: SHA-256 of retrieval timestamps and pipeline version.
6. `FACE_RESULT_HASH`: SHA-256 of ArcFace similarity metrics and bounding boxes.
7. `PROVENANCE_HASH`: SHA-256 of raw discovery provider response.

---

## 5. Tamper Detection & Integrity Verification

Local evidence integrity is verified using a cryptographic roundtrip:

```text
Stored Evidence Bundle
       │
       ▼
Canonical JCS Serialization
       │
       ▼
Merkle Tree Reconstruction
       │
       ▼
Computed Root Hash (A) ───────┐
                              ▼
                        Compare (A == B)
                              ▲
Ethereum Sepolia RPC ─────────┘
`getEvidence(imageHash)`
       │
       ▼
On-Chain Anchor Root (B)
```

If any actor modifies a single character in `evidence.json` (such as adjusting `similarity: 0.941` to `0.999` or altering the `canonical_url`):
1. The corresponding leaf hash changes completely due to the avalanche effect.
2. The Merkle root recalculates to an entirely different `bytes32` hash.
3. `tekmerion verify` detects `ONCHAIN_MISMATCH`.
4. `tekmerion tamper-test` isolates the exact divergence between local evidence and the immutable Ethereum Sepolia ledger.
