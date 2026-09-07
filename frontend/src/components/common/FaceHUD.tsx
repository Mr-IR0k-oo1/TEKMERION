import React from 'react';
import { FaceQualityAssessment } from '../../types/forensic';
import { Scan, AlertOctagon, CheckCircle, ShieldAlert } from 'lucide-react';

interface FaceHUDProps {
  imageSrc: string;
  imageFileName: string;
  resolution: string;
  imageHash: string;
  quality: FaceQualityAssessment;
  isScanning?: boolean;
}

export const FaceHUD: React.FC<FaceHUDProps> = ({
  imageSrc,
  imageFileName,
  resolution,
  imageHash,
  quality,
  isScanning = false,
}) => {
  // Convert bbox coordinates assuming a coordinate space [0..1000]
  const [x1, y1, x2, y2] = quality.bbox;
  const leftPct = (x1 / 1000) * 100;
  const topPct = (y1 / 1000) * 100;
  const widthPct = ((x2 - x1) / 1000) * 100;
  const heightPct = ((y2 - y1) / 1000) * 100;

  const isMultipleFaces = quality.face_count > 1;
  const isNoFace = quality.face_count === 0;
  const isBlurFailed = quality.blur_variance < 100;

  return (
    <div className="card face-hud-panel">
      <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center' }}>
        <h3 style={{ fontSize: '15px', fontWeight: 700, display: 'flex', alignItems: 'center', gap: '8px' }}>
          <Scan size={18} color="var(--cyan-bright)" /> Face Intelligence HUD
        </h3>
        {quality.status === 'pass' ? (
          <span className="badge badge-emerald">
            <CheckCircle size={12} /> GATE PASSED
          </span>
        ) : (
          <span className="badge badge-crimson">
            <AlertOctagon size={12} /> REJECTED
          </span>
        )}
      </div>

      {/* Interactive HUD Viewport */}
      <div className="hud-viewport">
        <img src={imageSrc} alt="Input Query" onError={(e) => {
          // Fallback if local asset isn't loaded
          (e.target as HTMLImageElement).src = 'https://images.unsplash.com/photo-1534528741775-53994a69daeb?w=600&auto=format&fit=crop&q=80';
        }} />

        {isScanning && <div className="hud-laser" />}

        {/* Bounding Box HUD */}
        {!isNoFace && (
          <div
            className="hud-bbox"
            style={{
              left: `${leftPct}%`,
              top: `${topPct}%`,
              width: `${widthPct}%`,
              height: `${heightPct}%`,
              borderColor: isMultipleFaces ? 'var(--crimson-tamper)' : 'var(--cyan-bright)',
            }}
          >
            <div className="hud-corner-tl" />
            <div className="hud-corner-tr" />
            <div className="hud-corner-bl" />
            <div className="hud-corner-br" />
          </div>
        )}

        {/* 5 Facial Landmarks */}
        {!isNoFace && quality.landmarks.map((lm, idx) => (
          <div
            key={idx}
            className="hud-landmark-dot"
            style={{
              left: `${(lm[0] / 1000) * 100}%`,
              top: `${(lm[1] / 1000) * 100}%`,
              backgroundColor: isMultipleFaces ? 'var(--crimson-tamper)' : '#fff',
            }}
          />
        ))}
      </div>

      {/* Input Metadata Row */}
      <div style={{ display: 'flex', flexDirection: 'column', gap: '4px', fontSize: '12px' }}>
        <div style={{ display: 'flex', justifyContent: 'space-between', color: 'var(--text-secondary)' }}>
          <span>Source File:</span>
          <span className="mono" style={{ color: 'var(--text-primary)' }}>{imageFileName}</span>
        </div>
        <div style={{ display: 'flex', justifyContent: 'space-between', color: 'var(--text-secondary)' }}>
          <span>Resolution:</span>
          <span className="mono" style={{ color: 'var(--text-primary)' }}>{resolution}</span>
        </div>
        <div style={{ display: 'flex', justifyContent: 'space-between', color: 'var(--text-secondary)' }}>
          <span>SHA-256 Digest:</span>
          <span className="mono" style={{ color: 'var(--cyan-bright)', fontSize: '11px' }}>
            {imageHash.substring(0, 10)}...{imageHash.substring(imageHash.length - 8)}
          </span>
        </div>
      </div>

      {/* Forensic Quality Gates Breakdown */}
      <div className="quality-gates-grid">
        <div className="gate-metric">
          <div className="gate-metric-title">
            <span>Face Count</span>
            <span style={{ color: quality.face_count === 1 ? 'var(--emerald-verified)' : 'var(--crimson-tamper)' }}>
              {quality.face_count === 1 ? 'VALID' : 'REJECT'}
            </span>
          </div>
          <div className="gate-metric-val">{quality.face_count} face(s)</div>
          <div className="gate-progress-bar">
            <div
              className="gate-progress-fill"
              style={{
                width: quality.face_count === 1 ? '100%' : '20%',
                background: quality.face_count === 1 ? 'var(--emerald-verified)' : 'var(--crimson-tamper)',
              }}
            />
          </div>
        </div>

        <div className="gate-metric">
          <div className="gate-metric-title">
            <span>Laplacian Blur</span>
            <span style={{ color: isBlurFailed ? 'var(--crimson-tamper)' : 'var(--emerald-verified)' }}>
              {isBlurFailed ? 'BLURRY' : 'SHARP'}
            </span>
          </div>
          <div className="gate-metric-val">{quality.blur_variance.toFixed(1)} / 100.0</div>
          <div className="gate-progress-bar">
            <div
              className="gate-progress-fill"
              style={{
                width: `${Math.min(100, (quality.blur_variance / 500) * 100)}%`,
                background: isBlurFailed ? 'var(--crimson-tamper)' : 'var(--cyan-bright)',
              }}
            />
          </div>
        </div>
      </div>

      {/* ArcFace 512-D Embedding Preview */}
      <div style={{ background: '#050810', padding: '12px', borderRadius: '6px', border: '1px solid var(--border-dim)' }}>
        <div style={{ display: 'flex', justifyContent: 'space-between', fontSize: '11px', marginBottom: '6px', color: 'var(--text-muted)' }}>
          <span>ArcFace r100 L2 Embedding</span>
          <span className="mono">512-D Normalized Vector</span>
        </div>
        <div style={{ display: 'flex', alignItems: 'flex-end', gap: '4px', height: '28px' }}>
          {quality.embedding_preview.map((val, idx) => {
            const heightPct = Math.max(15, Math.abs(val) * 200);
            return (
              <div
                key={idx}
                style={{
                  flex: 1,
                  height: `${heightPct}%`,
                  background: val >= 0 ? 'var(--cyan-bright)' : 'var(--violet-chain)',
                  borderRadius: '2px',
                  opacity: 0.85,
                }}
                title={`Dim ${idx}: ${val.toFixed(4)}`}
              />
            );
          })}
        </div>
      </div>

      {/* Quality Rejection Warning if applicable */}
      {quality.status === 'fail' && (
        <div style={{
          background: 'rgba(239, 68, 68, 0.1)',
          border: '1px solid var(--crimson-tamper)',
          padding: '10px 14px',
          borderRadius: '6px',
          color: '#ff6b6b',
          fontSize: '12px',
          display: 'flex',
          gap: '8px',
          alignItems: 'flex-start',
        }}>
          <ShieldAlert size={16} style={{ flexShrink: 0, marginTop: '2px' }} />
          <div>
            <strong>Forensic Gate Rejection:</strong>
            <ul style={{ paddingLeft: '16px', marginTop: '4px' }}>
              {quality.reasons.map((r, i) => (
                <li key={i}>{r}</li>
              ))}
            </ul>
          </div>
        </div>
      )}
    </div>
  );
};
