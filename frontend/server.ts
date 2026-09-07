import http from 'http';
import fs from 'fs';
import path from 'path';
import crypto from 'crypto';
import { analyzeImageWithWorker } from './src/server/workerBridge';

const PORT = process.env.PORT ? parseInt(process.env.PORT, 10) : 3001;
const UPLOAD_DIR = path.resolve(process.cwd(), '..', 'runs', 'temp_uploads');

if (!fs.existsSync(UPLOAD_DIR)) {
  fs.mkdirSync(UPLOAD_DIR, { recursive: true });
}

const server = http.createServer(async (req, res) => {
  // CORS Headers
  res.setHeader('Access-Control-Allow-Origin', '*');
  res.setHeader('Access-Control-Allow-Methods', 'GET, POST, OPTIONS');
  res.setHeader('Access-Control-Allow-Headers', 'Content-Type, Authorization');

  if (req.method === 'OPTIONS') {
    res.writeHead(204);
    res.end();
    return;
  }

  const url = new URL(req.url || '/', `http://${req.headers.host}`);

  // Health Endpoint
  if (req.method === 'GET' && url.pathname === '/api/health') {
    res.writeHead(200, { 'Content-Type': 'application/json' });
    res.end(
      JSON.stringify({
        status: 'online',
        service: 'TEKMERION Forensic Backend API',
        version: '1.0.0',
        timestamp: new Date().toISOString(),
      })
    );
    return;
  }

  // Analyze Image Endpoint
  if (req.method === 'POST' && url.pathname === '/api/analyze') {
    const chunks: Buffer[] = [];
    req.on('data', (chunk) => chunks.push(chunk));
    req.on('end', async () => {
      try {
        const bodyBuffer = Buffer.concat(chunks);
        let imageBuffer: Buffer;
        let originalName = 'uploaded_image.jpg';

        const contentType = req.headers['content-type'] || '';
        if (contentType.includes('application/json')) {
          const parsed = JSON.parse(bodyBuffer.toString('utf-8'));
          if (parsed.image_base64) {
            const cleanBase64 = parsed.image_base64.replace(/^data:image\/\w+;base64,/, '');
            imageBuffer = Buffer.from(cleanBase64, 'base64');
            originalName = parsed.filename || originalName;
          } else {
            throw new Error('Missing image_base64 in JSON payload');
          }
        } else {
          imageBuffer = bodyBuffer;
        }

        const sha256Digest = crypto.createHash('sha256').update(imageBuffer).digest('hex');
        const tempFilePath = path.join(UPLOAD_DIR, `${sha256Digest}.jpg`);
        fs.writeFileSync(tempFilePath, imageBuffer);

        console.log(`[Backend API] Analyzing image: ${tempFilePath} (Size: ${imageBuffer.length} bytes)`);
        const analysis = await analyzeImageWithWorker(tempFilePath);

        res.writeHead(200, { 'Content-Type': 'application/json' });
        res.end(
          JSON.stringify({
            success: true,
            sha256: sha256Digest,
            filename: originalName,
            size_bytes: imageBuffer.length,
            analysis,
          })
        );
      } catch (err: any) {
        console.error('[Backend API] Analysis failed:', err);
        res.writeHead(500, { 'Content-Type': 'application/json' });
        res.end(
          JSON.stringify({
            success: false,
            error: err.message || 'Failed to process image analysis',
          })
        );
      }
    });
    return;
  }

  // Run Full Real Pipeline Endpoint
  if (req.method === 'POST' && url.pathname === '/api/pipeline/run') {
    const chunks: Buffer[] = [];
    req.on('data', (chunk) => chunks.push(chunk));
    req.on('end', async () => {
      try {
        const bodyBuffer = Buffer.concat(chunks);
        let imageBuffer: Buffer | null = null;
        let originalName = 'query_face.jpg';

        const contentType = req.headers['content-type'] || '';
        if (bodyBuffer.length > 0) {
          if (contentType.includes('application/json')) {
            try {
              const parsed = JSON.parse(bodyBuffer.toString('utf-8'));
              if (parsed.image_base64) {
                const cleanBase64 = parsed.image_base64.replace(/^data:image\/\w+;base64,/, '');
                imageBuffer = Buffer.from(cleanBase64, 'base64');
                originalName = parsed.filename || originalName;
              } else if (parsed.image_path) {
                const resolved = path.isAbsolute(parsed.image_path)
                  ? parsed.image_path
                  : path.resolve(process.cwd(), '..', parsed.image_path);
                if (fs.existsSync(resolved)) {
                  imageBuffer = fs.readFileSync(resolved);
                  originalName = path.basename(resolved);
                }
              } else if (parsed.filename) {
                const assetPath = path.resolve(process.cwd(), '..', 'assets', parsed.filename);
                if (fs.existsSync(assetPath)) {
                  imageBuffer = fs.readFileSync(assetPath);
                  originalName = parsed.filename;
                }
              }
            } catch (e) {
              console.warn('[Backend API] Could not parse JSON body, checking binary fallback');
            }
          } else {
            imageBuffer = bodyBuffer;
          }
        }

        // If no image was sent, use the actual default query asset
        if (!imageBuffer || imageBuffer.length === 0) {
          const defaultAsset = path.resolve(process.cwd(), '..', 'assets', 'query_face.jpg');
          if (fs.existsSync(defaultAsset)) {
            imageBuffer = fs.readFileSync(defaultAsset);
            originalName = 'query_face.jpg';
          } else {
            throw new Error('No image payload supplied and default assets/query_face.jpg not found');
          }
        }

        console.log(`[Backend API] Executing real forensic pipeline for ${originalName} (${imageBuffer.length} bytes)...`);
        const { executeRealPipeline } = await import('./src/server/pipelineRunner');
        const pipelineResult = await executeRealPipeline(imageBuffer, originalName);

        res.writeHead(200, { 'Content-Type': 'application/json' });
        res.end(JSON.stringify(pipelineResult));
      } catch (err: any) {
        console.error('[Backend API] Pipeline execution failed:', err);
        res.writeHead(500, { 'Content-Type': 'application/json' });
        res.end(
          JSON.stringify({
            success: false,
            error: err.message || 'Forensic pipeline execution error',
          })
        );
      }
    });
    return;
  }

  res.writeHead(404, { 'Content-Type': 'application/json' });
  res.end(JSON.stringify({ error: 'Endpoint not found' }));
});

server.listen(PORT, () => {
  console.log(`🚀 TEKMERION Backend API running on http://localhost:${PORT}`);
  console.log(`   Health:  http://localhost:${PORT}/api/health`);
  console.log(`   Analyze: http://localhost:${PORT}/api/analyze`);
});
