# ==============================================================================
# TEKMERION Multi-Stage Production Dockerfile
# Face → Discovery → Verification → Evidence → Blockchain
# ==============================================================================

# ------------------------------------------------------------------------------
# Stage 1: Rust Core & CLI Builder
# ------------------------------------------------------------------------------
FROM rust:bookworm AS rust-builder
WORKDIR /build

COPY Cargo.toml Cargo.lock rust-toolchain.toml ./
COPY crates/ ./crates/
COPY apps/ ./apps/

RUN cargo build --release -p tekmerion-tui

# ------------------------------------------------------------------------------
# Stage 2: Web UI Frontend Builder
# ------------------------------------------------------------------------------
FROM oven/bun:1 AS frontend-builder
WORKDIR /app

COPY frontend/package.json frontend/bun.lock* ./
RUN bun install --frozen-lockfile || bun install

COPY frontend/ ./
RUN bun run build

# ------------------------------------------------------------------------------
# Stage 3: TEKMERION Backend & Pipeline Engine
# ------------------------------------------------------------------------------
FROM python:3.11-slim-bookworm AS backend

ENV DEBIAN_FRONTEND=noninteractive \
    PYTHONUNBUFFERED=1 \
    BUN_INSTALL="/root/.bun" \
    PATH="/root/.bun/bin:${PATH}" \
    PORT=3001 \
    ETH_RPC_URL="https://rpc.sepolia.org" \
    EVIDENCE_CONTRACT_ADDRESS="0x712a7f0e014A895FdBdf0F46aA3199859fC8A79E" \
    FACE_WORKER_PATH="/app/workers/face/worker.py"

RUN apt-get update && apt-get install -y --no-install-recommends \
    curl \
    ca-certificates \
    libgomp1 \
    build-essential \
    g++ \
    git \
    unzip \
    && curl -fsSL https://bun.sh/install | bash \
    && rm -rf /var/lib/apt/lists/*

RUN pip install --no-cache-dir --upgrade pip setuptools wheel && \
    pip install --no-cache-dir \
    "insightface>=0.7.3" \
    "numpy>=1.24" \
    "onnxruntime>=1.16" \
    "opencv-python-headless>=4.8"

WORKDIR /app

COPY --from=rust-builder /build/target/release/tekmerion /usr/local/bin/tekmerion

COPY workers/ ./workers/
COPY assets/ ./assets/
COPY contracts/ ./contracts/
COPY frontend/package.json ./frontend/package.json
COPY frontend/bun.lock* ./frontend/bun.lock
COPY frontend/tsconfig.json ./frontend/tsconfig.json
COPY frontend/server.ts ./frontend/server.ts
COPY frontend/src/ ./frontend/src/

WORKDIR /app/frontend
RUN bun install

RUN mkdir -p /app/runs/temp_uploads

EXPOSE 3001
CMD ["bun", "server.ts"]

# ------------------------------------------------------------------------------
# Stage 4: Frontend SPA Web Server (Nginx)
# ------------------------------------------------------------------------------
FROM nginx:alpine AS frontend
COPY --from=frontend-builder /app/dist /usr/share/nginx/html
COPY docker/nginx.conf /etc/nginx/conf.d/default.conf
EXPOSE 80
CMD ["nginx", "-g", "daemon off;"]

# ------------------------------------------------------------------------------
# Stage 5: Interactive Terminal User Interface (TUI) & CLI Container
# ------------------------------------------------------------------------------
FROM backend AS tui
WORKDIR /app
ENTRYPOINT ["/usr/local/bin/tekmerion"]
CMD ["--help"]
