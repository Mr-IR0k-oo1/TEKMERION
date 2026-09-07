import React, { useMemo, useState } from 'react';
import { EvidenceBundle, EvidenceRecord, TamperState } from '../../types/forensic';
import { ShieldCheck, AlertTriangle, RotateCcw, Copy, Check } from 'lucide-react';
import { Badge } from '../ui';

interface TamperLabProps {
  evidenceRecord: EvidenceRecord | null;
  evidenceBundle: EvidenceBundle | null;
  tamperState: TamperState;
  anchoredRoot: string;
  onApplyTamper: (mutatedRecord: Partial<EvidenceRecord>, leafName: string, fieldName: string) => void;
  onRestore: () => void;
}

const trunc = (s: string, pad = 10) => `${s.substring(0, pad)}…${s.substring(s.length - 8)}`;

const LEAF_PATHS: Record<string, string[]> = {
  IMAGE: ['leaf', 'node-0-1', 'node-0-3', 'root'],
  CONTENT: ['leaf', 'node-0-1', 'node-0-3', 'root'],
  METADATA: ['leaf', 'node-2-3', 'node-0-3', 'root'],
  FACE: ['leaf', 'node-2-3', 'node-0-3', 'root'],
  PROVENANCE: ['leaf', 'root'],
};

interface PathStep {
  key: string;
  name: string;
  hash: string;
}

export const TamperLab: React.FC<TamperLabProps> = ({
  evidenceRecord,
  evidenceBundle,
  tamperState,
  anchoredRoot,
  onApplyTamper,
  onRestore,
}) => {
  const [copied, setCopied] = useState(false);
  const isTampered = tamperState.isTampered;

  const pathSteps: PathStep[] = useMemo(() => {
    if (!evidenceBundle) return [];
    const tree = evidenceBundle.tree;
    const key =
      Object.keys(LEAF_PATHS).find((k) =>
        (tamperState.tamperedLeaf || 'CONTENT').toUpperCase().includes(k)
      ) || 'CONTENT';
    const names: Record<string, string> = {
      leaf: tamperState.tamperedLeaf || 'Content · Leaf #1',
      'node-0-1': 'Node 0–1',
      'node-2-3': 'Node 2–3',
      'node-0-3': 'Node 0–3',
      root: 'Root',
    };
    const hashes: Record<string, string> = {
      leaf: tamperState.tamperedLeafHash || tree.leaves.content_hash,
      'node-0-1': tree.node_0_1,
      'node-2-3': tree.node_2_3,
      'node-0-3': tree.node_2_4,
      root: tree.root_hash,
    };
    return LEAF_PATHS[key].map((step, idx) => ({
      key: step,
      name: names[step],
      hash: hashes[step],
    }));
  }, [evidenceBundle, tamperState]);

  if (!evidenceRecord || !evidenceBundle) {
    return (
      <div className="proof-empty">
        <div className="proof-empty-icon">
          <AlertTriangle size={28} strokeWidth={1.5} />
        </div>
        <h2 className="proof-empty-title">Nothing to compare yet</h2>
        <p className="proof-empty-text">
          Run the pipeline to generate an evidence record before testing tamper resistance.
        </p>
      </div>
    );
  }

  const handleCopy = () => {
    navigator.clipboard.writeText(evidenceBundle.root_hash);
    setCopied(true);
    setTimeout(() => setCopied(false), 2000);
  };

  return (
    <div className="tamper-view">
      <header className="proof-header">
        <div>
          <h1 className="proof-title">Tamper Demonstration</h1>
          <p className="proof-subtitle">
            Change a value in the local record. Because the root was anchored on-chain, the discrepancy is immediate.
          </p>
        </div>
        {isTampered ? (
          <Badge variant="error" dot>Not Verified</Badge>
        ) : (
          <Badge variant="success" dot>Verified</Badge>
        )}
      </header>

      <div className="tamper-controls">
        <button className="btn btn-tamper" onClick={() => onApplyTamper(
          { title: `${evidenceRecord.title} [UNAUTHORIZED ALTERATION]` },
          'CONTENT (Leaf #1)',
          'title'
        )}>
          Modify Content
        </button>
        <button className="btn btn-secondary" onClick={() => onApplyTamper(
          { face_similarity: 0.999999 },
          'FACE (Leaf #3)',
          'face_similarity'
        )}>
          Modify Face
        </button>
        <button className="btn btn-ghost" onClick={onRestore}>
          <RotateCcw size={14} /> Restore
        </button>
      </div>

      <div className="tamper-compare">
        <div className="tamper-column">
          <span className="tamper-column-title">Original Evidence</span>
          <span className="tamper-column-sub">
            <Badge variant="success" dot>Verified</Badge>
          </span>
          <div className="tamper-root">
            <span className="tamper-root-label">Root · registered on-chain</span>
            <span className="tamper-root-hash mono">{trunc(anchoredRoot)}</span>
          </div>
        </div>

        <div className={`tamper-column ${isTampered ? 'is-modified' : ''}`}>
          <span className="tamper-column-title">Modified Evidence</span>
          <span className="tamper-column-sub">
            {isTampered ? (
              <Badge variant="error" dot>Not Verified</Badge>
            ) : (
              <Badge variant="success" dot>Verified</Badge>
            )}
          </span>
          <div className="tamper-root">
            <span className="tamper-root-label">Root · current local record</span>
            <span className={`tamper-root-hash mono ${isTampered ? 'is-changed' : ''}`}>{trunc(evidenceBundle.root_hash)}</span>
            <button className="proof-copy-btn" onClick={handleCopy} aria-label="Copy current root">
              {copied ? <Check size={13} /> : <Copy size={13} />}
            </button>
          </div>
        </div>
      </div>

      {isTampered && (
        <div className="tamper-detail">
          <div className="tamper-detail-copy">
            <strong>The evidence fingerprint no longer matches the fingerprint registered on-chain.</strong>
          </div>

          <div className="tamper-component">
            <span className="tamper-component-label">Changed component</span>
            <span className="tamper-component-name">{tamperState.tamperedLeaf || 'Content · Leaf #1'}</span>
            {tamperState.tamperedField && (
              <span className="tamper-component-field mono">{tamperState.tamperedField}</span>
            )}
          </div>

          <div className="tamper-path" aria-label="Hash path from changed leaf to root">
            {pathSteps.map((step, idx) => (
              <React.Fragment key={step.key}>
                {idx > 0 && <span className="tamper-path-arrow" aria-hidden="true" />}
                <span className="tamper-path-node is-affected">
                  <span className="tamper-path-name">{step.name}</span>
                  <span className="tamper-path-hash mono">{trunc(step.hash, 8)}</span>
                </span>
              </React.Fragment>
            ))}
          </div>
        </div>
      )}
    </div>
  );
};