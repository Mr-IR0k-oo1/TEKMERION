#!/usr/bin/env python3
"""TEKMERION local face-analysis worker.

Reads JSON Lines requests from stdin and writes JSON Lines responses to stdout.
Long-running analysis happens here (InsightFace + ONNX Runtime) so it never
blocks the Rust host. Logs go to stderr only; stdout carries the protocol
payload exclusively.

Protocol (request):

    {"request_id": "...", "operation": "analyze", "image_path": "..."}

Protocol (response): every response carries `request_id`, `success`, `faces`,
`embedding`, `quality` and `errors`:

    {
        "request_id": "...",
        "success": true|false,
        "faces": [ ... ],          # one entry per detected face
        "embedding": [...]|null,   # non-null only when exactly one face
        "quality": 0.0..1.0|null,  # non-null only when exactly one face
        "errors": []               # structured error summaries
    }

Each face object:

    {
        "bounding_box": [x1, y1, x2, y2],
        "embedding": [...],        # L2-normalized ArcFace embedding
        "quality": 0.0..1.0,
        "landmarks": [[x, y], ...] | null,   # where available
        "pose": {"pitch":..., "yaw":..., "roll":...} | null  # where available
    }

Zero detected faces is an explicit, non-failing result: `success` is true and
`faces` is an empty list. Multiple faces are all represented in `faces`; the
worker never silently selects one, so the top-level `embedding`/`quality` are
`null` when the face count is not exactly one.
"""

import contextlib
import json
import os
import sys

import numpy as np

try:
    import cv2
except Exception as exc:  # pragma: no cover - environment guard
    cv2 = None

try:
    from insightface.app import FaceAnalysis
except Exception as exc:  # pragma: no cover - environment guard
    FaceAnalysis = None

# Supported image extensions (lower-case, with and without leading dot).
SUPPORTED_EXTENSIONS = {
    ".jpg",
    ".jpeg",
    ".png",
    ".bmp",
    ".webp",
    ".tif",
    ".tiff",
}


def log(message):
    """Write a log line to stderr (never stdout, never image data)."""
    sys.stderr.write(message + "\n")
    sys.stderr.flush()


def l2_normalize(vector):
    """Return an L2-normalized copy of a 1-D float vector."""
    vector = np.asarray(vector, dtype=np.float64)
    norm = np.linalg.norm(vector)
    if norm < 1e-12:
        return np.zeros_like(vector)
    return (vector / norm).astype(np.float64)


def is_supported_image(path):
    """Check the extension and that the path exists and is a regular file."""
    base, ext = os.path.splitext(path)
    if ext.lower() not in SUPPORTED_EXTENSIONS:
        return False
    if not os.path.exists(path):
        return False
    if not os.path.isfile(path):
        return False
    return True


def build_request_error(request_id, message):
    """Return a structured failure response for a request-level error."""
    return {
        "request_id": request_id,
        "success": False,
        "faces": [],
        "embedding": None,
        "quality": None,
        "errors": [message],
    }


class FaceAnalyzer:
    """Lazy, process-wide InsightFace initializer."""

    def __init__(self):
        self.app = None
        self.available = FaceAnalysis is not None

    def prepare(self):
        if self.app is not None or not self.available:
            return
        with contextlib.redirect_stdout(sys.stderr):
            self.app = FaceAnalysis(providers=["CPUExecutionProvider"])
            self.app.prepare(ctx_id=0, det_size=(640, 640))
        log("insightface initialized")


ANALYZER = FaceAnalyzer()


def analyze_request(request):
    """Validate and run the analyze operation on a single request.

    Returns a response dict exactly matching the protocol above.
    """
    request_id = request.get("request_id")
    operation = request.get("operation")
    image_path = request.get("image_path")

    if operation != "analyze":
        return build_request_error(request_id, "unsupported operation %r" % (operation,))

    if not isinstance(image_path, str) or not image_path.strip():
        return build_request_error(request_id, "image_path is required")

    path = os.path.abspath(image_path)
    if not is_supported_image(path):
        return build_request_error(
            request_id,
            "unsupported or missing image file: %s" % path,
        )

    if cv2 is None or FaceAnalysis is None:
        return build_request_error(request_id, "face analysis dependencies unavailable")

    try:
        ANALYZER.prepare()
        if ANALYZER.app is None:
            return build_request_error(request_id, "face analysis engine not initialized")

        image = cv2.imread(path)
        if image is None:
            return build_request_error(request_id, "failed to decode image: %s" % path)

        with contextlib.redirect_stdout(sys.stderr):
            results = ANALYZER.app.get(image)
    except Exception as exc:
        log("analysis error for request %s: %s" % (request_id, exc))
        return build_request_error(request_id, "analysis failed: %s" % exc)

    faces = []
    for face in results:
        try:
            embedding = list(l2_normalize(face.normed_embedding))
        except Exception:
            embedding = None

        landmarks = None
        if hasattr(face, "landmark_2d_106") and face.landmark_2d_106 is not None:
            landmarks = [[float(x), float(y)] for x, y in face.landmark_2d_106]
        elif hasattr(face, "landmark") and face.landmark is not None:
            landmarks = [[float(x), float(y)] for x, y in face.landmark]

        pose = None
        if hasattr(face, "pose") and face.pose is not None:
            pose = {
                "pitch": float(face.pose[0]),
                "yaw": float(face.pose[1]),
                "roll": float(face.pose[2]),
            }

        quality = 1.0
        if hasattr(face, "det_score"):
            quality = float(face.det_score)

        faces.append(
            {
                "bounding_box": [float(v) for v in face.bbox],
                "embedding": embedding,
                "quality": quality,
                "landmarks": landmarks,
                "pose": pose,
            }
        )

    # Zero faces is an explicit result, not an error.
    if len(faces) == 0:
        return {
            "request_id": request_id,
            "success": True,
            "faces": [],
            "embedding": None,
            "quality": None,
            "errors": [],
        }

    # Multiple faces: represent every one; never silently choose.
    if len(faces) > 1:
        return {
            "request_id": request_id,
            "success": True,
            "faces": faces,
            "embedding": None,
            "quality": None,
            "errors": [],
        }

    single = faces[0]
    return {
        "request_id": request_id,
        "success": True,
        "faces": faces,
        "embedding": single["embedding"],
        "quality": single["quality"],
        "errors": [],
    }


def main():
    log("worker ready")
    for raw in sys.stdin:
        line = raw.strip()
        if not line:
            continue
        try:
            try:
                request = json.loads(line)
            except json.JSONDecodeError as exc:
                response = build_request_error(None, "invalid JSON: %s" % exc)
            else:
                if not isinstance(request, dict):
                    response = build_request_error(None, "request must be a JSON object")
                else:
                    response = analyze_request(request)
        except Exception as exc:
            # Never let a single request take the worker down.
            log("unexpected worker error: %s" % exc)
            response = build_request_error(None, "unexpected worker error")
        sys.stdout.write(json.dumps(response) + "\n")
        sys.stdout.flush()


if __name__ == "__main__":
    main()
