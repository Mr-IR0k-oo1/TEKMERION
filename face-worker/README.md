# Face Worker

This is the local Python face worker for the Face Identification & Blockchain Verification project.

## Requirements

- Python 3.8 or higher
- InsightFace
- NumPy
- OpenCV
- ONNX Runtime

## Installation

1. Create a virtual environment:

```bash
python -m venv venv
```

2. Activate the virtual environment:

- On Windows:

```bash
venv\Scripts\activate
```

- On macOS/Linux:

```bash
source venv/bin/activate
```

3. Install the requirements:

```bash
pip install -r requirements.txt
```

## Usage

The worker accepts JSON Lines from stdin and outputs JSON Lines to stdout.

Example request:

```json
{
  "request_id": "123",
  "operation": "embed",
  "image_path": "/path/to/image.jpg"
}
```

Example response:

```json
{
  "request_id": "123",
  "success": true,
  "face_count": 1,
  "embedding": [...],
  "bbox": [x1, y1, x2, y2]
}
```

## Error Handling

The worker handles various error cases and returns structured error responses.
