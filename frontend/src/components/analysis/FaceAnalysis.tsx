import React, { useState, useCallback } from 'react';
import { ZoomIn, ZoomOut, RotateCcw, ChevronLeft, ChevronRight, Info } from 'lucide-react';

interface FaceMetrics {
  quality: number; // extraction quality 0-100
  blur: 'Low' | 'Moderate' | 'High';
  exposure: 'Good' | 'Low' | 'High';
  pose: 'Suitable' | 'Slight' | 'Extreme';
}

interface FaceAnalysisProps {
  imageSrc: string;
  imageFileName: string;
  resolution?: string;
  faceCount: number;
  bbox: [number, number, number, number];
  extractionQuality: number; // 0-100
  blurVariance: number;
  brightness: number;
  status: 'pass' | 'fail' | 'warn' | 'idle';
}

const EXPLANATION =
  'Extraction quality describes suitability for feature extraction. It does not establish identity or authenticity.';

const deriveMetrics = (quality: number, blurVariance: number, brightness: number): FaceMetrics => {
  const blur = blurVariance < 100 ? 'High' : blurVariance < 180 ? 'Moderate' : 'Low';
  const exposure = brightness < 80 ? 'Low' : brightness > 180 ? 'High' : 'Good';
  const pose = quality > 70 ? 'Suitable' : quality > 40 ? 'Slight' : 'Extreme';
  return { quality, blur, exposure, pose };
};

export const FaceAnalysis: React.FC<FaceAnalysisProps> = ({
  imageSrc,
  imageFileName,
  resolution,
  faceCount,
  bbox,
  extractionQuality,
  blurVariance,
  brightness,
  status,
}) => {
  const [zoom, setZoom] = useState(1);
  const [activeFace, setActiveFace] = useState(0);
  const [showTip, setShowTip] = useState(false);

  const metrics = deriveMetrics(extractionQuality, blurVariance, brightness);

  const resetZoom = useCallback(() => setZoom(1), []);

  // Normalize bbox [0..1000] to percentage for overlay
  const [x1, y1, x2, y2] = bbox;
  const leftPct = (x1 / 1000) * 100;
  const topPct = (y1 / 1000) * 100;
  const widthPct = ((x2 - x1) / 1000) * 100;
  const heightPct = ((y2 - y1) / 1000) * 100;

  const faceCountCapped = Math.max(1, faceCount);
  const showOverlay = status !== 'idle' && faceCount > 0 && imageSrc;

  return (
    <div className="face-analysis">
      <div className="face-analysis-viewer">
        <div
          className="face-analysis-canvas"
          style={{ transform: `scale(${zoom})` }}
        >
          {imageSrc ? (
            <>
              <img src={imageSrc} alt={imageFileName} />
              {showOverlay && (
                <div
                  className="face-analysis-box"
                  style={{
                    left: `${leftPct}%`,
                    top: `${topPct}%`,
                    width: `${widthPct}%`,
                    height: `${heightPct}%`,
                  }}
                >
                  <span className="face-analysis-box-label">Face {String(activeFace + 1).padStart(2, '0')}</span>
                </div>
              )}
            </>
          ) : (
            <div className="face-analysis-empty">
              <div className="face-analysis-empty-icon" />
              <span>Select an image to begin analysis</span>
            </div>
          )}
        </div>

        {imageSrc && (
          <div className="face-analysis-toolbar">
            <button className="face-analysis-tool" onClick={() => setZoom((z) => Math.min(4, z * 1.25))} aria-label="Zoom in">
              <ZoomIn size={15} />
            </button>
            <button className="face-analysis-tool" onClick={() => setZoom((z) => Math.max(1, z / 1.25))} aria-label="Zoom out">
              <ZoomOut size={15} />
            </button>
            <button className="face-analysis-tool" onClick={resetZoom} aria-label="Reset zoom">
              <RotateCcw size={15} />
            </button>
            <div className="face-analysis-tool-sep" />
            <button
              className="face-analysis-tool"
              onClick={() => setActiveFace((f) => (f - 1 + faceCountCapped) % faceCountCapped)}
              disabled={faceCountCapped <= 1}
              aria-label="Previous face"
            >
              <ChevronLeft size={15} />
            </button>
            <span className="face-analysis-face-nav">{activeFace + 1} / {faceCountCapped}</span>
            <button
              className="face-analysis-tool"
              onClick={() => setActiveFace((f) => (f + 1) % faceCountCapped)}
              disabled={faceCountCapped <= 1}
              aria-label="Next face"
            >
              <ChevronRight size={15} />
            </button>
          </div>
        )}
      </div>

      <div className="face-analysis-info">
        <div className="face-analysis-info-header">
          <div>
            <span className="face-analysis-kicker">Face Analysis</span>
            <h3 className="face-analysis-title">Face {String(activeFace + 1).padStart(2, '0')}</h3>
          </div>
          <span className="face-analysis-tooltip-trigger">
            <Info
              size={15}
              onMouseEnter={() => setShowTip(true)}
              onMouseLeave={() => setShowTip(false)}
              onFocus={() => setShowTip(true)}
              onBlur={() => setShowTip(false)}
              tabIndex={0}
              aria-label="Show explanation"
            />
            {showTip && <span className="face-analysis-tooltip">{EXPLANATION}</span>}
          </span>
        </div>

        {imageSrc && (
          <div className="face-analysis-metrics">
            <div className="face-analysis-metric">
              <div className="face-analysis-metric-label">
                <span>Extraction Quality</span>
              </div>
              <div className="face-analysis-metric-bar">
                <div
                  className="face-analysis-metric-fill"
                  style={{ width: `${metrics.quality}%` }}
                />
              </div>
              <span className="face-analysis-metric-value">{Math.round(metrics.quality)}</span>
            </div>
            <div className="face-analysis-metric">
              <div className="face-analysis-metric-label">
                <span>Blur</span>
                <span className="face-analysis-metric-status">{metrics.blur}</span>
              </div>
            </div>
            <div className="face-analysis-metric">
              <div className="face-analysis-metric-label">
                <span>Exposure</span>
                <span className="face-analysis-metric-status">{metrics.exposure}</span>
              </div>
            </div>
            <div className="face-analysis-metric">
              <div className="face-analysis-metric-label">
                <span>Pose</span>
                <span className="face-analysis-metric-status">{metrics.pose}</span>
              </div>
            </div>
          </div>
        )}

        {!imageSrc && (
          <p className="face-analysis-empty-hint">
            Analysis details will appear here once you select an image.
          </p>
        )}

        <div className="face-analysis-meta">
          <span>{imageFileName}</span>
          {resolution && <span>{resolution}</span>}
        </div>
      </div>
    </div>
  );
};
