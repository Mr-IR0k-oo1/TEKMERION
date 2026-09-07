import React, { useState } from 'react';
import { EvidenceBundle, EvidenceRecord } from '../../types/forensic';
import { Image, FileText, Info, ScanFace, History, ChevronRight, Copy, Check } from 'lucide-react';
import { Sheet, Badge } from '../ui';
import { formatFloat } from '../../crypto/merkle';

interface EvidenceViewProps {
  evidenceBundle: EvidenceBundle | null;
  evidenceRecord: EvidenceRecord | null;
  verified: boolean;
}

interface EvidenceItem {
  id: string;
  name: string;
  leaf: string;
  hash: string;
  note: string;
  icon: React.ReactNode;
  fields: { label: string; value: string }[];
}

export const EvidenceView: React.FC<EvidenceViewProps> = ({
  evidenceBundle,
  evidenceRecord,
  verified,
}) => {
  const [selected, setSelected] = useState<EvidenceItem | null>(null);
  const [copied, setCopied] = useState(false);

  if (!evidenceBundle || !evidenceRecord) {
    return (
      <div className="proof-empty">
        <div className="proof-empty-icon">
          <FileText size={28} strokeWidth={1.5} />
        </div>
        <h2 className="proof-empty-title">No evidence yet</h2>
        <p className="proof-empty-text">
          Run the verification pipeline to build the evidence record and its cryptographic fingerprint.
        </p>
      </div>
    );
  }

  const leaves = evidenceBundle.tree.leaves;

  const items: EvidenceItem[] = [
    {
      id: 'image',
      name: 'Image',
      leaf: 'Leaf #0',
      hash: leaves.image_hash,
      note: 'Candidate media bytes',
      icon: <Image size={16} strokeWidth={1.75} />,
      fields: [{ label: 'Image SHA-256', value: evidenceRecord.image_sha256 }],
    },
    {
      id: 'content',
      name: 'Content',
      leaf: 'Leaf #1',
      hash: leaves.content_hash,
      note: 'Source title & post text',
      icon: <FileText size={16} strokeWidth={1.75} />,
      fields: [
        { label: 'Title', value: evidenceRecord.title },
        { label: 'Text', value: evidenceRecord.text },
      ],
    },
    {
      id: 'metadata',
      name: 'Metadata',
      leaf: 'Leaf #2',
      hash: leaves.metadata_hash,
      note: 'Source, platform & retrieval time',
      icon: <Info size={16} strokeWidth={1.75} />,
      fields: [
        { label: 'Schema version', value: evidenceRecord.schema_version },
        { label: 'Run ID', value: evidenceRecord.run_id },
        { label: 'Source URL', value: evidenceRecord.source_url },
        { label: 'Domain', value: evidenceRecord.domain },
        { label: 'Platform', value: evidenceRecord.platform },
        { label: 'Provider', value: evidenceRecord.provider },
        { label: 'Retrieved at', value: evidenceRecord.retrieved_at },
      ],
    },
    {
      id: 'face',
      name: 'Face',
      leaf: 'Leaf #3',
      hash: leaves.face_hash,
      note: 'Biometric match score & quality',
      icon: <ScanFace size={16} strokeWidth={1.75} />,
      fields: [
        { label: 'Similarity', value: formatFloat(evidenceRecord.face_similarity) },
        { label: 'Model', value: evidenceRecord.face_model },
        { label: 'Candidate quality', value: formatFloat(evidenceRecord.candidate_quality) },
      ],
    },
    {
      id: 'provenance',
      name: 'Provenance',
      leaf: 'Leaf #4',
      hash: leaves.provenance_hash,
      note: 'Audit trail for this run',
      icon: <History size={16} strokeWidth={1.75} />,
      fields: [
        { label: 'Run ID', value: evidenceRecord.run_id },
        { label: 'Provider', value: evidenceRecord.provider },
        { label: 'Platform', value: evidenceRecord.platform },
        { label: 'Retrieved at', value: evidenceRecord.retrieved_at },
      ],
    },
  ];

  const handleCopy = (hash: string) => {
    navigator.clipboard.writeText(hash);
    setCopied(true);
    setTimeout(() => setCopied(false), 2000);
  };

  const shortHash = (hash: string) =>
    `${hash.substring(0, 12)}…${hash.substring(hash.length - 9)}`;

  return (
    <div className="evidence-view">
      <header className="proof-header">
        <div>
          <h1 className="proof-title">Evidence</h1>
          <p className="proof-subtitle">
            Five components are fingerprinted into this record. Select a component to inspect its hash.
          </p>
        </div>
        <Badge variant={verified ? 'success' : 'error'} dot>
          {verified ? 'Verified' : 'Not Verified'}
        </Badge>
      </header>

      <div className="evidence-list">
        {items.map((item) => (
          <button
            key={item.id}
            className="evidence-row"
            onClick={() => {
              setSelected(item);
              setCopied(false);
            }}
          >
            <span className="evidence-row-icon">{item.icon}</span>
            <span className="evidence-row-main">
              <span className="evidence-row-name">{item.name}</span>
              <span className="evidence-row-note">{item.note}</span>
            </span>
            <span className="evidence-row-hash mono">{shortHash(item.hash)}</span>
            <ChevronRight size={15} className="evidence-row-chevron" />
          </button>
        ))}
      </div>

      <Sheet
        isOpen={!!selected}
        onClose={() => setSelected(null)}
        title={selected ? `${selected.name} · ${selected.leaf}` : ''}
        side="right"
      >
        {selected && (
          <div className="evidence-sheet">
            <div className="evidence-sheet-status">
              <Badge variant={verified ? 'success' : 'error'} dot>
                {verified ? 'Verified' : 'Not Verified'}
              </Badge>
            </div>

            <div className="proof-section-label">SHA-256</div>
            <div className="evidence-sheet-hash-row">
              <span className="evidence-sheet-hash mono">{selected.hash}</span>
              <button className="proof-copy-btn" onClick={() => handleCopy(selected.hash)} aria-label="Copy hash">
                {copied ? <Check size={13} /> : <Copy size={13} />}
              </button>
            </div>

            <div className="proof-section-label">Includes</div>
            <div className="evidence-sheet-fields">
              {selected.fields.map((f) => (
                <div className="evidence-field" key={f.label}>
                  <span className="evidence-field-label">{f.label}</span>
                  <span className="evidence-field-value">{f.value}</span>
                </div>
              ))}
            </div>

            <p className="evidence-sheet-note">
              This SHA-256 is the component's contribution to the Merkle root anchored on-chain.
            </p>
          </div>
        )}
      </Sheet>
    </div>
  );
};