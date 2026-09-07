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
  AlertCircle,
  ShieldX,
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
  candidates: VerificationResult[];
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
  candidates,
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
  const topCandidate = candidates && candidates.length > 0 ? candidates[0] : null;

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
              Live Execution Profiles (Zero Mocks)
            </h4>
            {isDragging && <span className="badge badge-cyan">DROP IMAGE HERE</span>}
          </div>

          <div style={{ display: 'flex', flexDirection: 'column', gap: '8px' }}>

            <button
              className="btn btn-secondary"
              style={{
                justifyContent: 'center',
                padding: '12px 14px',
                borderStyle: 'dashed',
                background: 'var(--color-accent-dim)',
                borderColor: 'var(--color-rule-glow)',
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

        {/* Completion, No Match, Gate Rejection, or Tamper Alert Banners */}
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

        {status === 'no_match' && (
          <div className="stage-banner" style={{ background: 'var(--color-amber-dim)', border: '1px solid var(--color-amber)' }}>
            <div style={{ display: 'flex', alignItems: 'center', gap: '12px' }}>
              <AlertCircle size={26} color="var(--color-amber)" />
              <div>
                <h3 style={{ fontSize: '16px', fontWeight: 800, color: 'var(--color-amber)' }}>
                  NO BIOMETRIC MATCH FOUND (BELOW 75% THRESHOLD)
                </h3>
                <p style={{ fontSize: '13px', opacity: 0.9, color: 'var(--color-ink-2)' }}>
                  All candidate web assets scored below the required 75.0% biometric similarity threshold. Evidence bundle creation and Ethereum Sepolia anchoring were halted.
                </p>
              </div>
            </div>
          </div>
        )}

        {quality.status === 'fail' && (
          <div className="stage-banner" style={{ background: 'var(--color-crimson-dim)', border: '1px solid var(--color-crimson)' }}>
            <div style={{ display: 'flex', alignItems: 'center', gap: '12px' }}>
              <ShieldX size={26} color="var(--color-crimson)" />
              <div>
                <h3 style={{ fontSize: '16px', fontWeight: 800, color: 'var(--color-crimson)' }}>
                  FORENSIC INPUT GATE REJECTION ✗
                </h3>
                <p style={{ fontSize: '13px', opacity: 0.9, color: 'var(--color-ink-2)' }}>
                  {quality.reasons.join(', ') || 'Face detection or quality criteria not satisfied. Pipeline halted.'}
                </p>
              </div>
            </div>
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
            <div style={{ background: 'var(--color-paper)', padding: '12px', borderRadius: '6px' }}>
              <span style={{ color: 'var(--text-muted)' }}>Input File: </span>
              <strong style={{ color: 'var(--color-ink)' }}>{imageFileName}</strong>
              <br />
              <span style={{ color: 'var(--text-muted)' }}>Dimensions: </span>
              <span className="mono">{resolution}</span>
            </div>

            <div style={{ background: 'var(--color-paper)', padding: '12px', borderRadius: '6px' }}>
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
            <div style={{ background: 'var(--color-paper)', padding: '12px', borderRadius: '6px' }}>
              <span style={{ color: 'var(--text-muted)' }}>Detector Model:</span>
              <div style={{ fontWeight: 600, color: 'var(--color-ink)' }}>SCRFD-10G (ONNX CPU)</div>
            </div>
            <div style={{ background: 'var(--color-paper)', padding: '12px', borderRadius: '6px' }}>
              <span style={{ color: 'var(--text-muted)' }}>Embedding Architecture:</span>
              <div style={{ fontWeight: 600, color: 'var(--color-ink)' }}>ArcFace ResNet-100 (512-D)</div>
            </div>
            <div style={{ background: 'var(--color-paper)', padding: '12px', borderRadius: '6px' }}>
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
            {topCandidate && topCandidate.status === 'verified' && topCandidate.similarity >= 0.75 ? (
              <span className="badge badge-emerald">
                <CheckCircle2 size={12} /> VERIFIED MATCH
              </span>
            ) : topCandidate && topCandidate.status === 'below_threshold' ? (
              <span className="badge badge-amber">
                <AlertCircle size={12} /> BELOW THRESHOLD (NO MATCH)
              </span>
            ) : topCandidate && topCandidate.status === 'no_face' ? (
              <span className="badge badge-crimson">
                <ShieldX size={12} /> NO FACE DETECTED
              </span>
            ) : completedStages.includes('VERIFY') ? (
              <span className="badge badge-secondary">SCANNED</span>
            ) : (
              <span className="badge badge-cyan">READY</span>
            )}
          </div>

          {candidates && candidates.length > 0 ? (
            <div style={{ display: 'flex', flexDirection: 'column', gap: '8px' }}>
              <div style={{ display: 'grid', gridTemplateColumns: '30px 1fr 80px 100px', gap: '12px', padding: '0 12px 8px', fontSize: '11px', color: 'var(--text-muted)', borderBottom: '1px solid var(--color-rule-dim)' }}>
                <div>RANK</div>
                <div>CANDIDATE SOURCE</div>
                <div style={{ textAlign: 'right' }}>SIMILARITY</div>
                <div style={{ textAlign: 'right' }}>RESULT</div>
              </div>
              {candidates.slice(0, 4).map((c, i) => (
                <div key={i} style={{ display: 'grid', gridTemplateColumns: '30px 1fr 80px 100px', gap: '12px', alignItems: 'center', background: 'var(--color-paper)', padding: '10px 12px', borderRadius: '4px', fontSize: '13px' }}>
                  <div style={{ color: 'var(--text-muted)', fontWeight: 600 }}>#{i + 1}</div>
                  <div style={{ whiteSpace: 'nowrap', overflow: 'hidden', textOverflow: 'ellipsis' }}>
                    <a href={c.candidate.url} target="_blank" rel="noreferrer" style={{ color: 'var(--color-ink)', textDecoration: 'none' }}>
                      {c.candidate.domain}/...
                    </a>
                  </div>
                  <div style={{ textAlign: 'right', fontFamily: 'monospace' }}>
                    {(c.similarity * 100).toFixed(1)}%
                  </div>
                  <div style={{ textAlign: 'right' }}>
                    {c.status === 'verified' ? (
                      <span style={{ color: 'var(--emerald-verified)', fontWeight: 600, display: 'inline-flex', alignItems: 'center', gap: '4px' }}><CheckCircle2 size={12} /> MATCH</span>
                    ) : (
                      <span style={{ color: 'var(--crimson-tamper)', fontWeight: 600, display: 'inline-flex', alignItems: 'center', gap: '4px' }}><ShieldX size={12} /> REJECT</span>
                    )}
                  </div>
                </div>
              ))}
              
              {candidates[0]?.status === 'verified' && (
                <div style={{ marginTop: '12px', paddingTop: '12px', borderTop: '1px solid var(--color-rule-dim)' }}>
                   <div style={{ display: 'flex', gap: '16px', alignItems: 'center' }}>
                     <img src={candidates[0].candidate.thumbnail_url || candidates[0].candidate.image_url} alt="Top Match" style={{ width: '60px', height: '60px', borderRadius: '4px', objectFit: 'cover' }} />
                     <div>
                       <div style={{ fontSize: '12px', color: 'var(--text-muted)' }}>Best verified candidate</div>
                       <div style={{ fontSize: '14px', fontWeight: 600 }}>{candidates[0].candidate.title.slice(0, 40)}</div>
                       <div style={{ fontSize: '12px', color: 'var(--emerald-verified)', display: 'flex', alignItems: 'center', gap: '4px', marginTop: '4px' }}><CheckCircle2 size={12} /> VERIFIED</div>
                     </div>
                   </div>
                </div>
              )}
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
            ) : status === 'no_match' ? (
              <span className="badge badge-amber">HALTED (NO MATCH)</span>
            ) : (
              <span className="badge badge-cyan">PENDING</span>
            )}
          </div>

          <div style={{ display: 'grid', gridTemplateColumns: '1fr', gap: '14px', fontSize: '13px' }}>
            {evidenceBundle?.tree?.leaves && (
              <div style={{ background: 'var(--color-paper)', padding: '12px', borderRadius: '6px' }}>
                <span style={{ color: 'var(--text-muted)', display: 'block', marginBottom: '8px' }}>Merkle Tree Leaves:</span>
                <div style={{ display: 'grid', gridTemplateColumns: '120px 1fr', gap: '4px', fontSize: '11px' }} className="mono">
                  <div style={{ color: 'var(--text-muted)' }}>[0] Image Hash:</div>
                  <div style={{ color: 'var(--color-ink)', wordBreak: 'break-all' }}>{evidenceBundle.tree.leaves.image_hash}</div>
                  
                  <div style={{ color: 'var(--text-muted)' }}>[1] Content Hash:</div>
                  <div style={{ color: 'var(--color-ink)', wordBreak: 'break-all' }}>{evidenceBundle.tree.leaves.content_hash}</div>
                  
                  <div style={{ color: 'var(--text-muted)' }}>[2] Meta Hash:</div>
                  <div style={{ color: 'var(--color-ink)', wordBreak: 'break-all' }}>{evidenceBundle.tree.leaves.metadata_hash}</div>
                  
                  <div style={{ color: 'var(--text-muted)' }}>[3] Face Hash:</div>
                  <div style={{ color: 'var(--color-ink)', wordBreak: 'break-all' }}>{evidenceBundle.tree.leaves.face_hash}</div>
                  
                  <div style={{ color: 'var(--text-muted)' }}>[4] Prov Hash:</div>
                  <div style={{ color: 'var(--color-ink)', wordBreak: 'break-all' }}>{evidenceBundle.tree.leaves.provenance_hash}</div>
                </div>
              </div>
            )}
            
            <div style={{ display: 'grid', gridTemplateColumns: '1fr 1fr', gap: '14px' }}>
              <div style={{ background: 'var(--color-paper)', padding: '12px', borderRadius: '6px' }}>
                <span style={{ color: 'var(--text-muted)' }}>Local Evidence Root (RFC 8785):</span>
                <div
                  className="mono"
                  style={{
                    color: isTampered
                      ? 'var(--crimson-tamper)'
                      : evidenceBundle?.root_hash
                      ? 'var(--emerald-verified)'
                      : 'var(--text-muted)',
                    fontSize: '12px',
                    wordBreak: 'break-all',
                    marginTop: '4px',
                  }}
                >
                  {evidenceBundle?.root_hash || (status === 'no_match' ? '-- (Halted: Similarity < 75%)' : '--')}
                </div>
              </div>
  
              <div style={{ background: 'var(--color-paper)', padding: '12px', borderRadius: '6px' }}>
                <span style={{ color: 'var(--text-muted)' }}>Sepolia Anchored Root:</span>
                <div
                  className="mono"
                  style={{
                    color: blockchainRecord?.registered_root ? 'var(--violet-chain)' : 'var(--text-muted)',
                    fontSize: '12px',
                    wordBreak: 'break-all',
                    marginTop: '4px',
                  }}
                >
                  {blockchainRecord?.registered_root || (status === 'no_match' ? '-- (Registration halted)' : '--')}
                </div>
              </div>
            </div>
            
            {blockchainRecord?.registered_root && evidenceBundle?.root_hash && (
              <div style={{
                padding: '10px 14px',
                borderRadius: '4px',
                background: isTampered ? 'var(--color-crimson-dim)' : 'var(--color-emerald-dim)',
                border: isTampered ? '1px solid var(--color-crimson)' : '1px solid var(--color-emerald)',
                color: isTampered ? 'var(--crimson-tamper)' : 'var(--emerald-verified)',
                fontSize: '12px',
                display: 'flex',
                alignItems: 'center',
                gap: '8px',
                fontWeight: 600
              }}>
                {isTampered ? <ShieldX size={15} style={{ flexShrink: 0 }} /> : <CheckCircle2 size={15} style={{ flexShrink: 0 }} />}
                <span>
                  {isTampered 
                    ? 'INTEGRITY FAILURE: Local Merkle root does not match the Ethereum Sepolia anchor. Evidence has been tampered with!' 
                    : 'VERIFIED: Local Merkle root cryptographically matches the immutable on-chain Sepolia anchor.'}
                </span>
              </div>
            )}
          </div>

          {status === 'no_match' && (
            <div
              style={{
                marginTop: '12px',
                padding: '10px 14px',
                borderRadius: '4px',
                background: 'var(--color-amber-dim)',
                border: '1px solid oklch(78% 0.16 75 / 0.3)',
                color: 'var(--color-amber)',
                fontSize: '12px',
                display: 'flex',
                alignItems: 'center',
                gap: '8px',
              }}
            >
              <AlertCircle size={15} style={{ flexShrink: 0 }} />
              <span>
                Zero-Trust Forensics: Merkle evidence generation and Sepolia anchoring are strictly halted because candidate similarity is below the 75.0% threshold.
              </span>
            </div>
          )}

          {blockchainRecord && (
            <div style={{ display: 'flex', justifyContent: 'space-between', fontSize: '12px', marginTop: '12px', color: 'var(--text-secondary)' }}>
              <span>Transaction Hash: <span className="mono" style={{ color: 'var(--color-ink)' }}>{blockchainRecord.tx_hash.substring(0, 16)}...</span></span>
              <span>Block: <span className="mono" style={{ color: 'var(--color-ink)' }}>#{blockchainRecord.block_number}</span></span>
              <span>Confirmations: <span className="mono" style={{ color: 'var(--emerald-verified)' }}>{blockchainRecord.confirmations} blocks</span></span>
            </div>
          )}
        </div>
      </div>
    </div>
  );
};
