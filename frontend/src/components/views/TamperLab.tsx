import React, { useState } from 'react';
import { EvidenceBundle, EvidenceRecord, TamperState } from '../../types/forensic';
import {
  AlertTriangle,
  RotateCcw,
  ShieldCheck,
  Zap,
  ArrowRight,
  ShieldAlert,
} from 'lucide-react';

interface TamperLabProps {
  evidenceRecord: EvidenceRecord | null;
  evidenceBundle: EvidenceBundle | null;
  tamperState: TamperState;
  anchoredRoot: string;
  onApplyTamper: (mutatedRecord: Partial<EvidenceRecord>, leafName: string, fieldName: string) => void;
  onRestore: () => void;
}

export const TamperLab: React.FC<TamperLabProps> = ({
  evidenceRecord,
  evidenceBundle,
  tamperState,
  anchoredRoot,
  onApplyTamper,
  onRestore,
}) => {
  if (!evidenceRecord || !evidenceBundle) {
    return (
      <div className="card" style={{ textAlign: 'center', padding: '60px 20px' }}>
        <AlertTriangle size={40} color="var(--text-muted)" style={{ margin: '0 auto 16px' }} />
        <h3 style={{ fontSize: '18px', fontWeight: 700 }}>No Evidence to Tamper Yet</h3>
        <p style={{ color: 'var(--text-muted)', marginTop: '8px' }}>
          Run the pipeline to generate an initial evidence bundle before testing tamper resistance.
        </p>
      </div>
    );
  }

  // Form State
  const [title, setTitle] = useState(evidenceRecord.title);
  const [text, setText] = useState(evidenceRecord.text);
  const [similarity, setSimilarity] = useState(evidenceRecord.face_similarity.toString());
  const [imageHash, setImageHash] = useState(evidenceRecord.image_sha256);

  const isTampered = tamperState.isTampered;

  const handleTamperTitle = () => {
    const mutated = `${title} [UNAUTHORIZED ALTERATION]`;
    setTitle(mutated);
    onApplyTamper({ title: mutated }, 'CONTENT (Leaf #1)', 'title');
  };

  const handleTamperSimilarity = () => {
    const mutated = 0.999999;
    setSimilarity(mutated.toString());
    onApplyTamper({ face_similarity: mutated }, 'FACE (Leaf #3)', 'face_similarity');
  };

  const handleCustomApply = () => {
    const simNum = parseFloat(similarity) || evidenceRecord.face_similarity;
    onApplyTamper(
      {
        title,
        text,
        face_similarity: simNum,
        image_sha256: imageHash,
      },
      title !== evidenceRecord.title ? 'CONTENT (Leaf #1)' : 'CUSTOM MUTATION',
      'title'
    );
  };

  const handleResetForm = () => {
    setTitle(evidenceRecord.title);
    setText(evidenceRecord.text);
    setSimilarity(evidenceRecord.face_similarity.toString());
    setImageHash(evidenceRecord.image_sha256);
    onRestore();
  };

  return (
    <div className="tamper-lab-container">
      {/* Introduction Banner */}
      <div className="tamper-intro-card">
        <div className="tamper-intro-header">
          <div style={{ display: 'flex', alignItems: 'center', gap: '10px' }}>
            <AlertTriangle size={24} color="var(--crimson-tamper)" />
            <h2 style={{ fontSize: '18px', fontWeight: 800 }}>
              Tamper Pinpoint Laboratory (Interactive Stress Test)
            </h2>
          </div>
          <span className="badge badge-crimson">ADVERSARIAL SIMULATION</span>
        </div>
        <p style={{ fontSize: '13px', color: 'var(--text-secondary)', lineHeight: 1.5 }}>
          In a forensic dispute, a bad actor may alter the investigation record locally (e.g. changing the source title,
          exaggerating face similarity, or substituting media). Because the original Merkle root was permanently anchored
          on Ethereum Sepolia, <strong>any local modification immediately causes a root discrepancy</strong>.
        </p>
      </div>

      <div className="tamper-grid">
        {/* Left Column: Field Mutator Form */}
        <div className="mutator-card">
          <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center' }}>
            <h3 style={{ fontSize: '16px', fontWeight: 700, display: 'flex', alignItems: 'center', gap: '8px' }}>
              <Zap size={18} color="var(--amber-warn)" /> Evidence Mutation Controls
            </h3>
            {isTampered && (
              <span className="badge badge-crimson" style={{ fontSize: '10px' }}>MUTATED</span>
            )}
          </div>

          <div className="form-group">
            <div className="form-label">
              <span>Source Post Title (Leaf #1: Content)</span>
              <button
                className="btn btn-tamper"
                style={{ padding: '2px 8px', fontSize: '11px' }}
                onClick={handleTamperTitle}
              >
                Tamper Title
              </button>
            </div>
            <input
              type="text"
              className="form-input mono"
              value={title}
              onChange={(e) => setTitle(e.target.value)}
            />
          </div>

          <div className="form-group">
            <div className="form-label">
              <span>Body Text / Snippet (Leaf #1: Content)</span>
            </div>
            <textarea
              className="form-textarea mono"
              value={text}
              onChange={(e) => setText(e.target.value)}
            />
          </div>

          <div className="form-group">
            <div className="form-label">
              <span>Biometric Similarity Score (Leaf #3: Face)</span>
              <button
                className="btn btn-tamper"
                style={{ padding: '2px 8px', fontSize: '11px' }}
                onClick={handleTamperSimilarity}
              >
                Tamper to 0.999999
              </button>
            </div>
            <input
              type="number"
              step="0.000001"
              className="form-input mono"
              value={similarity}
              onChange={(e) => setSimilarity(e.target.value)}
            />
          </div>

          <div className="form-group">
            <div className="form-label">
              <span>Candidate Media SHA-256 (Leaf #0: Image)</span>
            </div>
            <input
              type="text"
              className="form-input mono"
              value={imageHash}
              onChange={(e) => setImageHash(e.target.value)}
            />
          </div>

          <div style={{ display: 'flex', gap: '12px', marginTop: '8px' }}>
            <button className="btn btn-tamper" onClick={handleCustomApply} style={{ flex: 1 }}>
              <AlertTriangle size={15} /> Recalculate & Test Merkle Discrepancy
            </button>
            <button className="btn btn-secondary" onClick={handleResetForm}>
              <RotateCcw size={15} /> Restore Authenticity
            </button>
          </div>
        </div>

        {/* Right Column: Real-time Verification Comparison */}
        <div className={`comparison-card ${isTampered ? 'mismatch-active' : ''}`}>
          <h3 style={{ fontSize: '16px', fontWeight: 700 }}>
            Cryptographic Integrity Audit
          </h3>

          {/* Status Banner */}
          {isTampered ? (
            <div className="mismatch-banner mismatch-banner-red">
              <ShieldAlert size={24} />
              <div>
                <div style={{ fontSize: '15px', fontWeight: 800 }}>
                  [MISMATCH ✗] TAMPER DETECTED
                </div>
                <div style={{ fontSize: '12px', opacity: 0.9, marginTop: '2px' }}>
                  Local Merkle root deviates from Sepolia contract anchor.
                </div>
              </div>
            </div>
          ) : (
            <div className="mismatch-banner mismatch-banner-green">
              <ShieldCheck size={24} />
              <div>
                <div style={{ fontSize: '15px', fontWeight: 800 }}>
                  [MATCH ✓] EVIDENCE INTACT & UNALTERED
                </div>
                <div style={{ fontSize: '12px', opacity: 0.9, marginTop: '2px' }}>
                  Local root matches the on-chain Sepolia anchor bit-for-bit.
                </div>
              </div>
            </div>
          )}

          {/* Root Hashes Comparison */}
          <div className="roots-compare-box">
            <div className="root-entry">
              <div className="root-entry-header">
                <span>Local Recomputed Merkle Root:</span>
                <span className="mono" style={{ color: isTampered ? 'var(--crimson-tamper)' : 'var(--emerald-verified)' }}>
                  {isTampered ? 'Root B (Tampered)' : 'Root A (Original)'}
                </span>
              </div>
              <div className={`root-entry-hash ${isTampered ? 'tampered-hash' : 'anchored-hash'}`}>
                {evidenceBundle.root_hash}
              </div>
            </div>

            <div style={{ textAlign: 'center', color: 'var(--text-muted)', fontSize: '11px', display: 'flex', alignItems: 'center', justifyContent: 'center', gap: '8px' }}>
              <span>Comparison against public ledger anchor</span>
              <ArrowRight size={13} />
            </div>

            <div className="root-entry">
              <div className="root-entry-header">
                <span>Ethereum Sepolia Anchored Root:</span>
                <span className="mono" style={{ color: 'var(--violet-chain)' }}>Root A (Immutable Contract)</span>
              </div>
              <div className="root-entry-hash anchored-hash">
                {anchoredRoot}
              </div>
            </div>
          </div>

          {/* Pinpointed Discrepancy Breakdown */}
          {isTampered && (
            <div style={{ background: 'rgba(239, 68, 68, 0.08)', padding: '16px', borderRadius: '6px', border: '1px solid rgba(239, 68, 68, 0.3)' }}>
              <div style={{ fontSize: '12px', fontWeight: 700, color: 'var(--crimson-tamper)', textTransform: 'uppercase', marginBottom: '8px' }}>
                Forensic Discrepancy Isolation:
              </div>
              <div style={{ display: 'flex', flexDirection: 'column', gap: '6px', fontSize: '12px' }}>
                <div>
                  <span style={{ color: 'var(--text-muted)' }}>Compromised Leaf: </span>
                  <strong style={{ color: '#fff' }}>{tamperState.tamperedLeaf || 'CONTENT (Leaf #1)'}</strong>
                </div>
                <div>
                  <span style={{ color: 'var(--text-muted)' }}>Mutated Field: </span>
                  <span className="mono" style={{ color: 'var(--amber-warn)' }}>{tamperState.tamperedField || 'title'}</span>
                </div>
                {tamperState.originalLeafHash && tamperState.tamperedLeafHash && (
                  <div style={{ marginTop: '4px' }}>
                    <div style={{ color: 'var(--text-muted)', fontSize: '11px' }}>Leaf Hash Transition:</div>
                    <div className="mono" style={{ fontSize: '11px', color: 'var(--emerald-verified)' }}>
                      Original: {tamperState.originalLeafHash.substring(0, 16)}...
                    </div>
                    <div className="mono" style={{ fontSize: '11px', color: 'var(--crimson-tamper)' }}>
                      Mutated:  {tamperState.tamperedLeafHash.substring(0, 16)}...
                    </div>
                  </div>
                )}
              </div>
            </div>
          )}

          {/* Mathematical Guarantee Note */}
          <div style={{ fontSize: '12px', color: 'var(--text-muted)', lineHeight: 1.4 }}>
            💡 <strong>Avalanche Effect:</strong> SHA-256 ensures that changing even 1 bit in a title or score
            changes ~50% of the bits in the leaf hash, which cascades up the Merkle tree to completely alter the root hash.
          </div>
        </div>
      </div>
    </div>
  );
};
