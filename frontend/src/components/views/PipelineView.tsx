import React from 'react';
import {
  BlockchainRecord,
  EvidenceBundle,
  EvidenceRecord,
  FaceQualityAssessment,
  PipelineStageId,
  PipelineStatus,
  VerificationResult,
} from '../../types/forensic';
import { FaceHUD } from '../common/FaceHUD';
import {
  Play,
  StepForward,
  RotateCcw,
  AlertTriangle,
  CheckCircle2,
  ExternalLink,
  ShieldCheck,
  Search,
  Fingerprint,
  Link,
  Database,
  UploadCloud,
} from 'lucide-react';
import confetti from 'canvas-confetti';

interface PipelineViewProps {
  currentStage: PipelineStageId;
  completedStages: PipelineStageId[];
  status: PipelineStatus;
  runId: string;
  imageSrc: string;
  imageFileName: string;
  resolution: string;
  imageHash: string;
  quality: FaceQualityAssessment;
  evidenceRecord: EvidenceRecord | null;
  evidenceBundle: EvidenceBundle | null;
  blockchainRecord: BlockchainRecord | null;
  topCandidate: VerificationResult | null;
  candidatesCount: number;
  onRunPipeline: () => void;
  onStepNext: () => void;
  onReset: () => void;
  onTamper: () => void;
  onSelectSample: (sampleId: string) => void;
  onCustomImageUpload: (file: File) => void;
}

export const PipelineView: React.FC<PipelineViewProps> = ({
  currentStage,
  completedStages,
  status,
  runId,
  imageSrc,
  imageFileName,
  resolution,
  imageHash,
  quality,
  evidenceRecord,
  evidenceBundle,
  blockchainRecord,
  topCandidate,
  candidatesCount,
  onRunPipeline,
  onStepNext,
  onReset,
  onTamper,
  onSelectSample,
  onCustomImageUpload,
}) => {
  const isCompleted = status === 'completed';
  const isTampered = status === 'tampered';
  const isRunning = status === 'running';

  const triggerCelebration = () => {
    confetti({
      particleCount: 80,
      spread: 70,
      origin: { y: 0.6 },
      colors: ['#00f0ff', '#10b981', '#a855f7'],
    });
  };

  const fileInputRef = React.useRef<HTMLInputElement | null>(null);
  const [isDragging, setIsDragging] = React.useState(false);

  const handleFileChange = (e: React.ChangeEvent<HTMLInputElement>) => {
    const file = e.target.files?.[0];
    if (file) {
      onCustomImageUpload(file);
    }
    e.target.value = '';
  };

  const handleDragOver = (e: React.DragEvent) => {
    e.preventDefault();
    e.stopPropagation();
    setIsDragging(true);
  };

  const handleDragLeave = (e: React.DragEvent) => {
    e.preventDefault();
    e.stopPropagation();
    setIsDragging(false);
  };

  const handleDrop = (e: React.DragEvent) => {
    e.preventDefault();
    e.stopPropagation();
    setIsDragging(false);
    const file = e.dataTransfer.files?.[0];
    if (file) {
      onCustomImageUpload(file);
    }
  };

  return (
    <div className="pipeline-layout">
      {/* Left Column: Face HUD & Source Selection */}
      <div style={{ display: 'flex', flexDirection: 'column', gap: '20px' }}>
        <FaceHUD
          imageSrc={imageSrc}
          imageFileName={imageFileName}
          resolution={resolution}
          imageHash={imageHash}
          quality={quality}
          isScanning={isRunning}
        />

        {/* Investigation Sample Selector & Upload */}
        <div
          className={`card ${isDragging ? 'card-glowing-cyan' : ''}`}
          onDragOver={handleDragOver}
          onDragLeave={handleDragLeave}
          onDrop={handleDrop}
          style={{
            display: 'flex',
            flexDirection: 'column',
            gap: '12px',
            border: isDragging ? '2px dashed var(--cyan-bright)' : undefined,
            background: isDragging ? 'rgba(0, 240, 255, 0.08)' : undefined,
            transition: 'all 0.2s ease',
          }}
        >
          <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center' }}>
            <h4 style={{ fontSize: '13px', fontWeight: 700, color: 'var(--text-secondary)' }}>
              Investigation Case Profiles
            </h4>
            {isDragging && <span className="badge badge-cyan">DROP IMAGE HERE</span>}
          </div>

          <div style={{ display: 'flex', flexDirection: 'column', gap: '8px' }}>
            <button
              className={`btn btn-secondary ${imageFileName.includes('query_face') ? 'active' : ''}`}
              style={{ justifyContent: 'flex-start', textAlign: 'left', padding: '10px 14px' }}
              onClick={() => onSelectSample('case_single_face')}
            >
              <div>
                <div style={{ fontWeight: 600, color: '#fff' }}>Case 1: Single Subject (Jane Doe)</div>
                <div style={{ fontSize: '11px', color: 'var(--text-muted)' }}>
                  SCRFD single face • Passes blur & exposure gates
                </div>
              </div>
            </button>

            <button
              className={`btn btn-secondary ${imageFileName.includes('multi_face') ? 'active' : ''}`}
              style={{ justifyContent: 'flex-start', textAlign: 'left', padding: '10px 14px' }}
              onClick={() => onSelectSample('case_multi_face')}
            >
              <div>
                <div style={{ fontWeight: 600, color: '#fff' }}>Case 2: Crowd Photo (Multi-Face)</div>
                <div style={{ fontSize: '11px', color: 'var(--amber-warn)' }}>
                  Rejection Test: Strict MULTIPLE_FACES gate failure
                </div>
              </div>
            </button>

            <button
              className="btn btn-secondary"
              style={{
                justifyContent: 'center',
                padding: '12px 14px',
                borderStyle: 'dashed',
                background: 'rgba(0, 240, 255, 0.04)',
                borderColor: 'rgba(0, 240, 255, 0.3)',
              }}
              onClick={() => fileInputRef.current?.click()}
            >
              <UploadCloud size={16} color="var(--cyan-bright)" />
              <span style={{ color: 'var(--cyan-bright)' }}>Upload or Drag Forensic Image</span>
            </button>
            <input
              type="file"
              ref={fileInputRef}
              onChange={handleFileChange}
              accept="image/*"
              style={{ display: 'none' }}
            />
          </div>
        </div>
      </div>

      {/* Right Column: Execution Console */}
      <div style={{ display: 'flex', flexDirection: 'column', gap: '20px' }}>
        {/* Pipeline Control Dock */}
        <div className="control-dock">
          <button
            className="btn btn-primary"
            onClick={onRunPipeline}
            disabled={isRunning || quality.status === 'fail'}
          >
            <Play size={16} /> Run Golden Path
          </button>

          <button
            className="btn btn-secondary"
            onClick={onStepNext}
            disabled={isRunning || isCompleted || quality.status === 'fail'}
          >
            <StepForward size={16} /> Step Next Stage
          </button>

          <button className="btn btn-tamper" onClick={onTamper}>
            <AlertTriangle size={16} /> Simulate Tamper Test
          </button>

          <button className="btn btn-secondary" onClick={onReset} style={{ marginLeft: 'auto' }}>
            <RotateCcw size={16} /> Reset Pipeline
          </button>
        </div>

        {/* Completion or Tamper Alert Banners */}
        {isCompleted && (
          <div className="stage-banner stage-banner-verified">
            <div style={{ display: 'flex', alignItems: 'center', gap: '12px' }}>
              <ShieldCheck size={26} />
              <div>
                <h3 style={{ fontSize: '16px', fontWeight: 800 }}>
                  FORENSIC VERIFICATION COMPLETE ✓
                </h3>
                <p style={{ fontSize: '13px', opacity: 0.9 }}>
                  Local RFC 8785 Merkle root matches the immutable Ethereum Sepolia anchor with 0
                  discrepancies.
                </p>
              </div>
            </div>
            <button
              className="btn btn-emerald"
              onClick={triggerCelebration}
              style={{ padding: '6px 14px', fontSize: '12px' }}
            >
              Celebrate
            </button>
          </div>
        )}

        {isTampered && (
          <div className="stage-banner stage-banner-tampered">
            <div style={{ display: 'flex', alignItems: 'center', gap: '12px' }}>
              <AlertTriangle size={26} />
              <div>
                <h3 style={{ fontSize: '16px', fontWeight: 800 }}>
                  [MISMATCH ✗] TAMPER DETECTED
                </h3>
                <p style={{ fontSize: '13px', opacity: 0.9 }}>
                  Local evidence data was altered. Local Merkle root deviates from Sepolia contract
                  anchor.
                </p>
              </div>
            </div>
          </div>
        )}

        {/* Stage 1: INPUT Ingestion */}
        <div className="card">
          <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center', marginBottom: '14px' }}>
            <div style={{ display: 'flex', alignItems: 'center', gap: '10px' }}>
              <Database size={18} color="var(--cyan-bright)" />
              <h3 style={{ fontSize: '16px', fontWeight: 700 }}>Stage 1 — Input Ingestion & Hash profiling</h3>
            </div>
            {completedStages.includes('INPUT') || isCompleted ? (
              <span className="badge badge-emerald">PROFILED</span>
            ) : (
              <span className="badge badge-cyan">ACTIVE</span>
            )}
          </div>

          <div style={{ display: 'grid', gridTemplateColumns: '1fr 1fr', gap: '14px', fontSize: '13px' }}>
            <div style={{ background: '#060910', padding: '12px', borderRadius: '6px' }}>
              <span style={{ color: 'var(--text-muted)' }}>Input File: </span>
              <strong style={{ color: '#fff' }}>{imageFileName}</strong>
              <br />
              <span style={{ color: 'var(--text-muted)' }}>Dimensions: </span>
              <span className="mono">{resolution}</span>
            </div>

            <div style={{ background: '#060910', padding: '12px', borderRadius: '6px' }}>
              <span style={{ color: 'var(--text-muted)' }}>Cryptographic SHA-256 Digest:</span>
              <div className="mono" style={{ color: 'var(--cyan-bright)', fontSize: '12px', wordBreak: 'break-all', marginTop: '4px' }}>
                {imageHash}
              </div>
            </div>
          </div>
        </div>

        {/* Stage 2: Face Intelligence */}
        <div className="card">
          <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center', marginBottom: '14px' }}>
            <div style={{ display: 'flex', alignItems: 'center', gap: '10px' }}>
              <Fingerprint size={18} color="var(--cyan-bright)" />
              <h3 style={{ fontSize: '16px', fontWeight: 700 }}>Stage 2 — SCRFD Detection & ArcFace Embedding</h3>
            </div>
            {completedStages.includes('FACE') || isCompleted ? (
              <span className="badge badge-emerald">EXTRACTED</span>
            ) : (
              <span className="badge badge-cyan">STANDBY</span>
            )}
          </div>

          <div style={{ display: 'grid', gridTemplateColumns: 'repeat(3, 1fr)', gap: '12px', fontSize: '13px' }}>
            <div style={{ background: '#060910', padding: '12px', borderRadius: '6px' }}>
              <span style={{ color: 'var(--text-muted)' }}>Detector Model:</span>
              <div style={{ fontWeight: 600, color: '#fff' }}>SCRFD-10G (ONNX CPU)</div>
            </div>
            <div style={{ background: '#060910', padding: '12px', borderRadius: '6px' }}>
              <span style={{ color: 'var(--text-muted)' }}>Embedding Architecture:</span>
              <div style={{ fontWeight: 600, color: '#fff' }}>ArcFace ResNet-100 (512-D)</div>
            </div>
            <div style={{ background: '#060910', padding: '12px', borderRadius: '6px' }}>
              <span style={{ color: 'var(--text-muted)' }}>Face Quality Verdict:</span>
              <div style={{ fontWeight: 600, color: quality.status === 'pass' ? 'var(--emerald-verified)' : 'var(--crimson-tamper)' }}>
                {quality.status === 'pass' ? 'PASSED (Blur & Exposure OK)' : 'REJECTED (Rule Violation)'}
              </div>
            </div>
          </div>
        </div>

        {/* Stage 3 & 4: Web Discovery & Biometric Matching */}
        <div className="card">
          <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center', marginBottom: '14px' }}>
            <div style={{ display: 'flex', alignItems: 'center', gap: '10px' }}>
              <Search size={18} color="var(--cyan-bright)" />
              <h3 style={{ fontSize: '16px', fontWeight: 700 }}>Stage 3 & 4 — Discovery & Biometric Verification</h3>
            </div>
            {completedStages.includes('VERIFY') || isCompleted ? (
              <span className="badge badge-emerald">VERIFIED</span>
            ) : (
              <span className="badge badge-cyan">READY</span>
            )}
          </div>

          {topCandidate ? (
            <div style={{ background: '#060910', padding: '16px', borderRadius: '6px', display: 'flex', gap: '16px', alignItems: 'center' }}>
              <img
                src={topCandidate.candidate.thumbnail_url || topCandidate.candidate.image_url}
                alt="Candidate Match"
                style={{ width: '70px', height: '70px', borderRadius: '6px', objectFit: 'cover' }}
              />
              <div style={{ flex: 1 }}>
                <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center' }}>
                  <h4 style={{ fontSize: '15px', fontWeight: 700 }}>{topCandidate.candidate.title}</h4>
                  <span className="badge badge-emerald">
                    {(topCandidate.similarity * 100).toFixed(1)}% Biometric Match
                  </span>
                </div>
                <div style={{ fontSize: '12px', color: 'var(--cyan-bright)', marginTop: '2px' }}>
                  <a
                    href={topCandidate.candidate.url}
                    target="_blank"
                    rel="noreferrer"
                    style={{ color: 'inherit', textDecoration: 'none', display: 'inline-flex', alignItems: 'center', gap: '4px' }}
                  >
                    {topCandidate.candidate.url} <ExternalLink size={12} />
                  </a>
                </div>
                <p style={{ fontSize: '12px', color: 'var(--text-muted)', marginTop: '4px' }}>
                  {topCandidate.candidate.snippet}
                </p>
              </div>
            </div>
          ) : (
            <div style={{ color: 'var(--text-muted)', fontSize: '13px' }}>
              No candidates verified yet. Advance the pipeline to trigger reverse-image discovery.
            </div>
          )}
        </div>

        {/* Stage 5 & 6: Merkle Tree & Blockchain Anchor */}
        <div className="card">
          <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center', marginBottom: '14px' }}>
            <div style={{ display: 'flex', alignItems: 'center', gap: '10px' }}>
              <Link size={18} color="var(--violet-chain)" />
              <h3 style={{ fontSize: '16px', fontWeight: 700 }}>Stage 5 & 6 — Merkle Fingerprint & Sepolia Anchor</h3>
            </div>
            {blockchainRecord && (completedStages.includes('BLOCKCHAIN') || isCompleted) ? (
              <span className="badge badge-violet">ANCHORED ON-CHAIN</span>
            ) : (
              <span className="badge badge-cyan">PENDING</span>
            )}
          </div>

          <div style={{ display: 'grid', gridTemplateColumns: '1fr 1fr', gap: '14px', fontSize: '13px' }}>
            <div style={{ background: '#060910', padding: '12px', borderRadius: '6px' }}>
              <span style={{ color: 'var(--text-muted)' }}>Local Merkle Root (RFC 8785):</span>
              <div
                className="mono"
                style={{
                  color: isTampered ? 'var(--crimson-tamper)' : 'var(--emerald-verified)',
                  fontSize: '12px',
                  wordBreak: 'break-all',
                  marginTop: '4px',
                }}
              >
                {evidenceBundle?.root_hash || '--'}
              </div>
            </div>

            <div style={{ background: '#060910', padding: '12px', borderRadius: '6px' }}>
              <span style={{ color: 'var(--text-muted)' }}>Sepolia Anchored Root:</span>
              <div
                className="mono"
                style={{
                  color: 'var(--violet-chain)',
                  fontSize: '12px',
                  wordBreak: 'break-all',
                  marginTop: '4px',
                }}
              >
                {blockchainRecord?.registered_root || '--'}
              </div>
            </div>
          </div>

          {blockchainRecord && (
            <div style={{ display: 'flex', justifyContent: 'space-between', fontSize: '12px', marginTop: '12px', color: 'var(--text-secondary)' }}>
              <span>Transaction Hash: <span className="mono" style={{ color: '#fff' }}>{blockchainRecord.tx_hash.substring(0, 16)}...</span></span>
              <span>Block: <span className="mono" style={{ color: '#fff' }}>#{blockchainRecord.block_number}</span></span>
              <span>Confirmations: <span className="mono" style={{ color: 'var(--emerald-verified)' }}>{blockchainRecord.confirmations} blocks</span></span>
            </div>
          )}
        </div>
      </div>
    </div>
  );
};
