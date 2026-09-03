#!/usr/bin/env python3
"""Hermetic mock of the TEKMERION face worker, for Rust protocol tests.

This script implements the same JSON Lines protocol as `workers/face/worker.py`
but requires no InsightFace/ONNX runtime. It resolves behaviour deterministically
from the `image_path` value so tests are repeatable:

  * a real-existing file path  -> single-face success response
  * "__zero__"                 -> explicit zero-face success result
  * "__multi__"                -> explicit multi-face success result
  * "__timeout__"              -> sleeps, never answers (triggers client timeout)
  * "__crash__"                -> exits immediately without answering
  * "__badjson__"              -> writes an unparseable line then exits
  * "__missing__"              -> structured "missing file" error
  * "__swap_a__"/"__swap_b__"  -> answer B before A (proves id correlation)
  * operation != "analyze"     -> structured "unsupported operation" error
"""

import json
import os
import sys
import time

EMBEDDING = [1.0, 0.0, 0.0]
LANDMARKS = [[0.0, 0.0], [1.0, 1.0], [2.0, 2.0], [3.0, 3.0], [4.0, 4.0]]
POSE = {"pitch": 0.1, "yaw": 0.2, "roll": 0.3}
BBOX = [10.0, 20.0, 90.0, 120.0]
QUALITY = 0.9

_SWAP_A = None


def single_face():
    return {
        "bounding_box": BBOX,
        "embedding": EMBEDDING,
        "quality": QUALITY,
        "landmarks": LANDMARKS,
        "pose": POSE,
    }


def reply(obj):
    sys.stdout.write(json.dumps(obj) + "\n")
    sys.stdout.flush()


def error_response(request_id, message):
    reply(
        {
            "request_id": request_id,
            "success": False,
            "faces": [],
            "embedding": None,
            "quality": None,
            "errors": [message],
        }
    )


def handle(request):
    request_id = request.get("request_id")
    operation = request.get("operation")

    if operation != "analyze":
        error_response(request_id, "unsupported operation %r" % (operation,))
        return

    image_path = request.get("image_path")

    if image_path in ("__swap_a__", "__swap_b__"):
        global _SWAP_A

        def swap_face(embedding):
            face = single_face()
            face["embedding"] = embedding
            return face

        if image_path == "__swap_a__":
            _SWAP_A = request
            return
        # B arrives: answer B first, then A. Proves the client correlates by
        # request id rather than assuming responses arrive in order.
        reply(
            {
                "request_id": request_id,
                "success": True,
                "faces": [swap_face([0.0, 1.0, 0.0])],
                "embedding": [0.0, 1.0, 0.0],
                "quality": QUALITY,
                "errors": [],
            }
        )
        if _SWAP_A is not None:
            reply(
                {
                    "request_id": _SWAP_A.get("request_id"),
                    "success": True,
                    "faces": [swap_face([0.0, 0.0, 1.0])],
                    "embedding": [0.0, 0.0, 1.0],
                    "quality": QUALITY,
                    "errors": [],
                }
            )
        return

    if image_path == "__timeout__":
        time.sleep(60)
        return

    if image_path == "__crash__":
        os._exit(1)

    if image_path == "__badjson__":
        sys.stdout.write("this is not json\n")
        sys.stdout.flush()
        os._exit(0)

    if image_path == "__zero__":
        reply(
            {
                "request_id": request_id,
                "success": True,
                "faces": [],
                "embedding": None,
                "quality": None,
                "errors": [],
            }
        )
        return

    if image_path == "__multi__":
        reply(
            {
                "request_id": request_id,
                "success": True,
                "faces": [single_face(), single_face()],
                "embedding": None,
                "quality": None,
                "errors": [],
            }
        )
        return

    if image_path == "__missing__":
        error_response(request_id, "unsupported or missing image file: %s" % image_path)
        return

    if not os.path.isfile(image_path):
        error_response(request_id, "unsupported or missing image file: %s" % image_path)
        return

    reply(
        {
            "request_id": request_id,
            "success": True,
            "faces": [single_face()],
            "embedding": EMBEDDING,
            "quality": QUALITY,
            "errors": [],
        }
    )


def main():
    for raw in sys.stdin:
        line = raw.strip()
        if not line:
            continue
        try:
            request = json.loads(line)
        except json.JSONDecodeError as exc:
            reply(
                {
                    "request_id": None,
                    "success": False,
                    "faces": [],
                    "embedding": None,
                    "quality": None,
                    "errors": ["invalid JSON: %s" % exc],
                }
            )
            continue
        handle(request)


if __name__ == "__main__":
    main()
