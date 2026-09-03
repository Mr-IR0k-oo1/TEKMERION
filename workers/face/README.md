# TEKMERION Face Worker

Local Python face-analysis worker for the TEKMERION pipeline. The Rust host
spawns this process and talks to it over JSON Lines (JSONL) on stdin/stdout, so
long-running inference (InsightFace + ONNX Runtime) never blocks the host or the
TUI. **This is not a web API.**

## Architecture

```
Rust
  |  stdin/stdout JSON Lines
  v
Python worker  (this directory)
  |
  v
InsightFace -> ONNX Runtime -> face detection + ArcFace embedding
  |
  v  JSON Lines
Rust
```

## Requirements

- Python 3.9+
- `pip install -r requirements.txt` (insightface, numpy, onnxruntime, opencv-python)

## Running

```bash
# From the repo root
python workers/face/worker.py
```

## Protocol

The worker reads one JSON object per line on stdin and writes one JSON object
per line on stdout. Logs go to **stderr** only; stdout carries protocol payloads.

### Request

```json
{"request_id": "abc-123", "operation": "analyze", "image_path": "/abs/or/rel/path.jpg"}
```

### Response

Every response includes `request_id`, `success`, `faces`, `embedding`, `quality`
and `errors`:

```json
{
  "request_id": "abc-123",
  "success": true,
  "faces": [
    {
      "bounding_box": [141, 42, 218, 124],
      "embedding": [0.012, -0.09, ...],
      "quality": 0.93,
      "landmarks": [[141, 70], [192, 68], ...],
      "pose": {"pitch": -0.12, "yaw": 0.31, "roll": 0.04}
    }
  ],
  "embedding": [0.012, -0.09, ...],
  "quality": 0.93,
  "errors": []
}
```

Behavior notes:

- **Zero faces is an explicit result.** `success` is `true`, `faces` is `[]`, and
  the top-level `embedding`/`quality` are `null`.
- **Multiple faces are all represented** in `faces`. The worker never silently
  chooses a face, so the top-level `embedding`/`quality` are `null` when the face
  count is not exactly one.
- Each face carries `bounding_box`, `embedding`, `quality`, and — when the
  backend provides them — `landmarks` and `pose`.
- Embeddings are **L2-normalized consistently** before returning.
- Image paths are validated; unsupported file types and missing files yield a
  structured error (`success: false`, `errors` populated).

### Errors

On failure `success` is `false` and `errors` contains one or more message
strings, e.g. `["unsupported or missing image file: /x.png"]`.

## Security & hygiene

- **Never logs image pixel contents.** Only request ids, operations and output
  counts/path names are logged, to stderr.
- Unsupported operations / files are rejected with structured errors.
- A malformed request never takes the worker down; it is answered with a
  structured error and the loop continues.
