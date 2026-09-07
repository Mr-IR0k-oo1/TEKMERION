# TEKMERION Containerization & Docker Deployment Guide

This guide details how to build, run, and orchestrate TEKMERION using Docker and Docker Compose. All components adhere strictly to the **Zero Mock Data Policy**: all inputs, face embeddings, similarity calculations, Merkle trees, and blockchain anchoring reflect genuine computations on true data.

---

## 1. System Architecture in Docker

The containerized stack is divided into three coordinated services:

```text
┌─────────────────────────────────────────────────────────────┐
│                    DOCKER HOST NETWORK                      │
└─────────────────────────────────────────────────────────────┘
          │                                        │
          ▼ (Port 5173)                            ▼ (Port 3001)
┌───────────────────────────┐            ┌───────────────────────────┐
│     tekmerion-frontend    │            │     tekmerion-backend     │
│       (Nginx Alpine)      │───/api/───►│ (Python 3.11 + Bun + Rust)│
│    Production React SPA   │            │  • InsightFace / ArcFace  │
└───────────────────────────┘            │  • Real Pipeline Runner   │
                                         │  • /usr/local/bin/tekmerion
                                         └─────────────┬─────────────┘
                                                       │
                                        Shared Volume: tekmerion_runs
                                                       │
                                         ┌─────────────▼─────────────┐
                                         │       tekmerion-tui       │
                                         │  Interactive Rust TUI/CLI │
                                         └───────────────────────────┘
```

1. **`backend`**:
   - Python 3.11 runtime with `insightface`, `onnxruntime`, `opencv-python-headless`.
   - Bun runtime running `server.ts` on port 3001.
   - Compiled Rust `tekmerion` binary in `/usr/local/bin/tekmerion`.
   - Mounts `tekmerion_runs` volume to store immutable run artifacts.

2. **`frontend`**:
   - Ultra-lightweight `nginx:alpine` container serving the compiled production SPA.
   - Reverse proxies `/api/*` and `/candidates/*` directly to `backend:3001`.
   - Accessible on `http://localhost:5173`.

3. **`tui`**:
   - Interactive CLI service (under Compose profile `cli`).
   - Runs `tekmerion run`, `verify`, `tamper-test`, `inspect`, and `contract-info` with access to the host's terminal.

---

## 2. Quickstart: Running with Docker Compose

### Step 1: Start Backend and Web Frontend
```bash
docker compose up --build -d
```

### Step 2: Verify Service Health
```bash
# Check container status
docker compose ps

# Test backend health endpoint
curl http://localhost:3001/api/health
```

Expected JSON response:
```json
{
  "status": "online",
  "service": "TEKMERION Forensic Backend API",
  "version": "1.0.0",
  "timestamp": "2026-09-07T..."
}
```

### Step 3: Open the Web UI
Open your browser to:
**`http://localhost:5173`**

You will be greeted with the clean forensic workspace. Drag and drop any image or click **"Run Benchmark: Single Face (query_face.jpg)"** to initiate live execution.

---

## 3. Running CLI & TUI Commands via Docker

The Rust CLI is fully available through the `tui` service without requiring local Rust or Python toolchains installed on the host machine:

### Query Blockchain Contract
```bash
docker compose run --rm tui contract-info
```

### Run Full Pipeline on an Image
```bash
docker compose run --rm tui run assets/query_face.jpg
```

For machine-readable JSON output:
```bash
docker compose run --rm tui run assets/query_face.jpg --json
```

### Inspect Stored Evidence Run
```bash
docker compose run --rm tui inspect <run-id>
```

### Re-verify Evidence Against Sepolia Anchor
```bash
docker compose run --rm tui verify <run-id>
```

### Execute Cryptographic Tamper Test
```bash
docker compose run --rm tui tamper-test <run-id>
```

---

## 4. Environment Variables Configuration

You can customize runtime parameters via a `.env` file in the workspace root or by passing environment variables:

| Variable | Description | Default |
|----------|-------------|---------|
| `PORT` | Backend HTTP API port | `3001` |
| `ETH_RPC_URL` | Ethereum Sepolia JSON-RPC endpoint | `https://rpc.sepolia.org` |
| `EVIDENCE_CONTRACT_ADDRESS` | Deployed `EvidenceRegistry.sol` address | `0x712a7f0e014A895FdBdf0F46aA3199859fC8A79E` |
| `ETH_PRIVATE_KEY` | Submitter account private key | (Optional for read verification) |
| `TEKMERION_SEARCH_API_KEY` | Upstream reverse-image search API token | (Optional) |
| `FACE_WORKER_PATH` | Path to Python face worker script | `/app/workers/face/worker.py` |

---

## 5. Building Standalone Docker Images

If you prefer building standalone Docker images instead of using Docker Compose:

### Build Backend:
```bash
docker build -t tekmerion-backend -f Dockerfile.backend .
docker run -p 3001:3001 tekmerion-backend
```

### Build Frontend:
```bash
docker build -t tekmerion-frontend -f Dockerfile.frontend .
docker run -p 5173:80 tekmerion-frontend
```

### Build TUI/CLI:
```bash
docker build --target tui -t tekmerion-cli .
docker run -it --rm tekmerion-cli run assets/query_face.jpg
```

---

## 6. Zero-Mock Policy Verification

In every Docker execution path:
1. **No Simulated Fallbacks**: Input images are processed by InsightFace SCRFD and ArcFace in the Python runtime.
2. **True Quality Scores**: Blur variance is computed from pixel-level Laplacian matrices and brightness from grayscale means.
3. **Real Biometric Verification**: Candidate images are decoded and compared using mathematical cosine similarity ($[-1.0, 1.0]$).
4. **RFC 8785 Merkle Tree**: 5 leaves are canonicalized and hashed via SHA-256 with domain separation ($0\text{x}00$ leaves, $0\text{x}01$ interior nodes).
5. **Live Blockchain Anchoring**: Transactions and block height verifications query the live Ethereum Sepolia testnet.
