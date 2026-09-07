import React, { useMemo, useState } from 'react';
import { EvidenceBundle, EvidenceRecord, TamperState } from '../../types/forensic';
import { Fingerprint, Copy, Check } from 'lucide-react';
import { Sheet, Badge } from '../ui';
import { formatFloat } from '../../crypto/merkle';

interface MerkleViewProps {
  evidenceBundle: EvidenceBundle | null;
  evidenceRecord: EvidenceRecord | null;
  tamperState: TamperState;
}

interface NodeInfo {
  id: string;
  name: string;
  type: 'root' | 'node' | 'leaf';
  hash: string;
  subtitle: string;
  fields: { label: string; value: string }[];
}

const truncHash = (hash: string) =>
  `${hash.substring(0, 10)}…${hash.substring(hash.length - 8)}`;

const AFFECTED_PATHS: Record<string, string[]> = {
  IMAGE: ['leaf-image', 'node-0-1', 'node-0-3', 'root'],
  CONTENT: ['leaf-content', 'node-0-1', 'node-0-3', 'root'],
  METADATA: ['leaf-metadata', 'node-2-3', 'node-0-3', 'root'],
  FACE: ['leaf-face', 'node-2-3', 'node-0-3', 'root'],
  PROVENANCE: ['leaf-provenance', 'root'],
};

export const MerkleView: React.FC<MerkleViewProps> = ({
  evidenceBundle,
  evidenceRecord,
  tamperState,
}) => {
  const [selected, setSelected] = useState<NodeInfo | null>(null);
  const [copied, setCopied] = useState(false);

  const affected = useMemo(() => {
    if (!tamperState.isTampered || !tamperState.tamperedLeaf) return new Set<string>();
    const key =
      Object.keys(AFFECTED_PATHS).find((k) =>
        tamperState.tamperedLeaf!.toUpperCase().includes(k)
      ) || 'CONTENT';
    return new Set(AFFECTED_PATHS[key]);
  }, [tamperState.isTampered, tamperState.tamperedLeaf]);

  if (!evidenceBundle || !evidenceRecord) {
    return (
      <div className="proof-empty">
        <div className="proof-empty-icon">
          <Fingerprint size={28} strokeWidth={1.5} />
        </div>
        <h2 className="proof-empty-title">No fingerprint yet</h2>
        <p className="proof-empty-text">
          Run the verification pipeline to build the Merkle fingerprint for this record.
        </p>
      </div>
    );
  }

  const isTampered = tamperState.isTampered;
  const { tree } = evidenceBundle;
  const leaves = tree.leaves;

  const nodeInfo: Record<string, NodeInfo> = {
    root: {
      id: 'root',
      name: 'Root',
      type: 'root',
      hash: tree.root_hash,
      subtitle: 'Composite fingerprint of all five components',
      fields: [{ label: 'Run ID', value: evidenceRecord.run_id }],
    },
    'node-0-3': {
      id: 'node-0-3',
      name: 'Node 0–3',
      type: 'node',
      hash: tree.node_2_4,
      subtitle: 'Combines Node 0–1 and Node 2–3',
      fields: [],
    },
    'node-0-1': {
      id: 'node-0-1',
      name: 'Node 0–1',
      type: 'node',
      hash: tree.node_0_1,
      subtitle: 'Combines Image and Content',
      fields: [],
    },
    'node-2-3': {
      id: 'node-2-3',
      name: 'Node 2–3',
      type: 'node',
      hash: tree.node_2_3,
      subtitle: 'Combines Metadata and Face',
      fields: [],
    },
    'leaf-image': {
      id: 'leaf-image',
      name: 'Image',
      type: 'leaf',
      hash: leaves.image_hash,
      subtitle: 'Leaf #0 · Candidate media bytes',
      fields: [{ label: 'Image SHA-256', value: evidenceRecord.image_sha256 }],
    },
    'leaf-content': {
      id: 'leaf-content',
      name: 'Content',
      type: 'leaf',
      hash: leaves.content_hash,
      subtitle: 'Leaf #1 · Source title and post text',
      fields: [
        { label: 'Title', value: evidenceRecord.title },
        { label: 'Text', value: evidenceRecord.text },
      ],
    },
    'leaf-metadata': {
      id: 'leaf-metadata',
      name: 'Metadata',
      type: 'leaf',
      hash: leaves.metadata_hash,
      subtitle: 'Leaf #2 · Source, platform and retrieval time',
      fields: [
        { label: 'Source URL', value: evidenceRecord.source_url },
        { label: 'Domain', value: evidenceRecord.domain },
        { label: 'Retrieved at', value: evidenceRecord.retrieved_at },
      ],
    },
    'leaf-face': {
      id: 'leaf-face',
      name: 'Face',
      type: 'leaf',
      hash: leaves.face_hash,
      subtitle: 'Leaf #3 · Biometric match score and quality',
      fields: [
        { label: 'Similarity', value: formatFloat(evidenceRecord.face_similarity) },
        { label: 'Model', value: evidenceRecord.face_model },
      ],
    },
    'leaf-provenance': {
      id: 'leaf-provenance',
      name: 'Provenance',
      type: 'leaf',
      hash: leaves.provenance_hash,
      subtitle: 'Leaf #4 · Promoted directly to the root',
      fields: [
        { label: 'Provider', value: evidenceRecord.provider },
        { label: 'Platform', value: evidenceRecord.platform },
      ],
    },
  };

  const renderNode = (info: NodeInfo) => {
    const isHit = affected.has(info.id);
    const cls = [
      'mt-node',
      `mt-node-${info.type}`,
      isHit ? 'is-affected' : '',
      info.id === 'root' && isTampered ? 'is-tampered-root' : '',
    ]
      .filter(Boolean)
      .join(' ');

    return (
      <button
        type="button"
        className={cls}
        onClick={() => {
          setSelected(info);
          setCopied(false);
        }}
        aria-label={`Inspect ${info.name}`}
      >
        <span className="mt-node-name">{info.name}</span>
        <span className="mt-node-hash mono">{truncHash(info.hash)}</span>
        {info.id === 'root' && isTampered && (
          <span className="mt-root-status">
            <Badge variant="error" dot>Not Verified</Badge>
          </span>
        )}
      </button>
    );
  };

  const handleCopy = (hash: string) => {
    navigator.clipboard.writeText(hash);
    setCopied(true);
    setTimeout(() => setCopied(false), 2000);
  };

  return (
    <div className="merkle-view">
      <header className="proof-header">
        <div>
          <h1 className="proof-title">Merkle Fingerprint</h1>
          <p className="proof-subtitle">
            Each component hashes upward until they combine into a single root. Select a node to inspect it.
          </p>
        </div>
        {isTampered ? (
          <Badge variant="error" dot>Not Verified</Badge>
        ) : (
          <Badge variant="success" dot>Verified</Badge>
        )}
      </header>

      <div className="merkle-tree-card">
        <ul className="mt-tree">
          <li>
            {renderNode(nodeInfo.root)}
            <ul>
              <li>
                {renderNode(nodeInfo['node-0-3'])}
                <ul>
                  <li>
                    {renderNode(nodeInfo['node-0-1'])}
                    <ul>
                      <li>{renderNode(nodeInfo['leaf-image'])}</li>
                      <li>{renderNode(nodeInfo['leaf-content'])}</li>
                    </ul>
                  </li>
                  <li>
                    {renderNode(nodeInfo['node-2-3'])}
                    <ul>
                      <li>{renderNode(nodeInfo['leaf-metadata'])}</li>
                      <li>{renderNode(nodeInfo['leaf-face'])}</li>
                    </ul>
                  </li>
                </ul>
              </li>
              <li>{renderNode(nodeInfo['leaf-provenance'])}</li>
            </ul>
          </li>
        </ul>
      </div>

      <p className="merkle-footnote">
        Provenance (Leaf #4) is promoted to the root level without re-hashing, per the odd-node rule. When any
        value changes, only the affected leaf and the hashes above it change.
      </p>

      <Sheet
        isOpen={!!selected}
        onClose={() => setSelected(null)}
        title={selected ? selected.name : ''}
        side="right"
      >
        {selected && (
          <div className="merkle-sheet">
            <p className="merkle-sheet-subtitle">{selected.subtitle}</p>

            <div className="proof-section-label">SHA-256</div>
            <div className="evidence-sheet-hash-row">
              <span className="evidence-sheet-hash mono">{selected.hash}</span>
              <button className="proof-copy-btn" onClick={() => handleCopy(selected.hash)} aria-label="Copy hash">
                {copied ? <Check size={13} /> : <Copy size={13} />}
              </button>
            </div>

            {selected.fields.length > 0 && (
              <>
                <div className="proof-section-label">Includes</div>
                <div className="evidence-sheet-fields">
                  {selected.fields.map((f) => (
                    <div className="evidence-field" key={f.label}>
                      <span className="evidence-field-label">{f.label}</span>
                      <span className="evidence-field-value">{f.value}</span>
                    </div>
                  ))}
                </div>
              </>
            )}
          </div>
        )}
      </Sheet>
    </div>
  );
};