#!/usr/bin/env python3

import json
import sys
import numpy as np
import cv2
from insightface.app import FaceAnalysis

# Initialize face analysis
app = FaceAnalysis(providers=['CPUExecutionProvider'])
app.prepare(ctx_id=0, det_size=(640, 640))

# Constants
EMBEDDING_DIMENSIONS = 512

# Process each line of input
for line in sys.stdin:
    try:
        # Parse the request
        request = json.loads(line.strip())
        request_id = request.get("request_id")
        operation = request.get("operation")
        image_path = request.get("image_path")

        if operation != "embed":
            response = {
                "request_id": request_id,
                "success": False,
                "error": "Unsupported operation"
            }
            print(json.dumps(response))
            continue

        # Read the image
        image = cv2.imread(image_path)
        if image is None:
            response = {
                "request_id": request_id,
                "success": False,
                "error": "Failed to read image"
            }
            print(json.dumps(response))
            continue

        # Detect faces
        faces = app.get(image)
        face_count = len(faces)

        if face_count == 0:
            response = {
                "request_id": request_id,
                "success": False,
                "error": "No faces detected"
            }
            print(json.dumps(response))
            continue

        if face_count > 1:
            # Return all detected faces
            faces_info = [
                {
                    "bbox": face.bbox.astype(int).tolist(),
                    "embedding": face.normed_embedding.astype(float).tolist()
                }
                for face in faces
            ]
            response = {
                "request_id": request_id,
                "success": False,
                "error": "Multiple faces detected",
                "faces": faces_info
            }
            print(json.dumps(response))
            continue

        # Process single face
        face = faces[0]
        embedding = face.normed_embedding.astype(float).tolist()
        bbox = face.bbox.astype(int).tolist()

        # Prepare response
        response = {
            "request_id": request_id,
            "success": True,
            "face_count": 1,
            "embedding": embedding,
            "bbox": bbox
        }

        print(json.dumps(response))

    except json.JSONDecodeError:
        response = {
            "request_id": "unknown",
            "success": False,
            "error": "Invalid JSON"
        }
        print(json.dumps(response))
    except Exception as e:
        response = {
            "request_id": "unknown",
            "success": False,
            "error": str(e)
        }
        print(json.dumps(response))
