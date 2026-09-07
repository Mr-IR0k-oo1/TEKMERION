import React, { useState } from 'react';
import { EvidenceBundle, EvidenceRecord, TamperState } from '../../types/forensic';
import { canonicalizeJson } from '../../crypto/canonicalize';
import { GitFork, ShieldAlert, CheckCircle2, ChevronRight, Copy, Check } from 'lucide-react';

interface MerkleViewProps {
  evidenceBundle: EvidenceBundle | null;
  evidenceRecord: EvidenceRecord | null;
  tamperState: TamperState;
}

export const MerkleView: React.FC<MerkleViewProps> = ({
  evidenceBundle,
  evidenceRecord,
  tamperState,
}) => {
  const [selectedNode, setSelectedNode] = useState<{
    title: string;
    type: 'root' | 'node' | 'leaf';
    hash: string;
    canonicalJson?: string;
    details?: string;
  } | null>(null);

  const [copied, setCopied] = useState(false);

  if (!evidenceBundle || !evidenceRecord) {
    return (
      <div className="card" style={{ textAlign: 'center', padding: '60px 20px' }}>
        <GitFork size={40} color="var(--text-muted)" style={{ margin: '0 auto 16px' }} />
        <h3 style={{ fontSize: '18px', fontWeight: 700 }}>No Evidence Tree Generated Yet</h3>
        <p style={{ color: 'var(--text-muted)', marginTop: '8px' }}>
          Execute the verification pipeline past Stage 5 (EVIDENCE) to construct the RFC 8785 Merkle tree.
        </p>
      </div>
    );
  }

  const { tree } = evidenceBundle;
  const leaves = tree.leaves;
  const isTampered = tamperState.isTampered;

  const handleCopyHash = (hash: string) => {
    navigator.clipboard.writeText(hash);
    setCopied(true);
    setTimeout(() => setCopied(false), 2000);
  };

  return (
    <div className="merkle-studio-container">
      {/* Header Info */}
      <div className="merkle-header-card">
        <div>
          <h2 style={{ fontSize: '18px', fontWeight: 800, display: 'flex', alignItems: 'center', gap: '8px' }}>
            <GitFork size={20} color="var(--cyan-bright)" /> Cryptographic Evidence Tree (RFC 6962 & RFC 8785)
          </h2>
          <p style={{ fontSize: '12px', color: 'var(--text-muted)', marginTop: '4px' }}>
            Domain-separated binary Merkle tree. Odd nodes promoted directly without duplicate hashing (prevents CVE-2012-2459).
          </p>
        </div>

        <div>
          {isTampered ? (
            <span className="badge badge-crimson" style={{ animation: 'flashRed 1.8s infinite' }}>
              <ShieldAlert size={14} /> TAMPER DETECTED: {tamperState.tamperedLeaf}
            </span>
          ) : (
            <span className="badge badge-emerald">
              <CheckCircle2 size={14} /> 5 LEAVES ANCHORED & VERIFIED
            </span>
          )}
        </div>
      </div>

      {/* Visual Merkle Tree Graphic */}
      <div className="merkle-canvas-card">
        {/* Level 0: Root */}
        <div className="tree-level tree-level-root">
          <div
            className={`merkle-node node-root ${isTampered ? 'tampered' : ''}`}
            onClick={() =>
              setSelectedNode({
                title: 'ROOT HASH (Composite Fingerprint)',
                type: 'root',
                hash: tree.root_hash,
                details: 'Computed as SHA-256(0x01 || Node 0-3 || Leaf 4 Promoted)',
                canonicalJson: canonicalizeJson({
                  schema_version: evidenceBundle.schema_version,
                  run_id: evidenceBundle.run_id,
                  root_hash: tree.root_hash,
                  leaves: tree.leaves,
                }),
              })
            }
          >
            <div className="node-title-row">
              <span className="node-label">ROOT HASH</span>
              <span className="badge badge-cyan" style={{ fontSize: '10px' }}>RFC 8785</span>
            </div>
            <div className="node-hash mono">
              {tree.root_hash.substring(0, 16)}...{tree.root_hash.substring(tree.root_hash.length - 16)}
            </div>
            <div className="node-desc">Anchored to Ethereum Sepolia EvidenceRegistry contract</div>
          </div>
        </div>

        {/* Level 1: Intermediate Nodes */}
        <div className="tree-level tree-level-intermediate">
          {/* Node 0-3 */}
          <div
            className={`merkle-node ${isTampered ? 'tampered' : ''}`}
            style={{ width: '320px' }}
            onClick={() =>
              setSelectedNode({
                title: 'NODE 0-3 (Left Subtree Parent)',
                type: 'node',
                hash: tree.node_2_4,
                details: 'Computed as SHA-256(0x01 || Node 0-1 || Node 2-3)',
              })
            }
          >
            <div className="node-title-row">
              <span className="node-label">Node 0-3</span>
              <span style={{ fontSize: '11px', color: 'var(--text-muted)' }}>Prefix: 0x01</span>
            </div>
            <div className="node-hash mono">
              {tree.node_2_4.substring(0, 12)}...{tree.node_2_4.substring(tree.node_2_4.length - 12)}
            </div>
            <div className="node-desc">Aggregates Image, Content, Metadata & Face</div>
          </div>

          {/* Leaf 4 Promoted */}
          <div
            className="merkle-node"
            style={{ width: '280px', opacity: 0.85 }}
            onClick={() =>
              setSelectedNode({
                title: 'PROMOTED LEAF #4 (Odd-Node Promotion)',
                type: 'node',
                hash: leaves.provenance_hash,
                details: 'RFC 6962 Odd-Node rule: directly promoted to root level without synthetic padding',
              })
            }
          >
            <div className="node-title-row">
              <span className="node-label">Promoted #4</span>
              <span className="badge badge-amber" style={{ fontSize: '10px' }}>ODD PROMOTION</span>
            </div>
            <div className="node-hash mono">
              {leaves.provenance_hash.substring(0, 10)}...{leaves.provenance_hash.substring(leaves.provenance_hash.length - 10)}
            </div>
            <div className="node-desc">Provenance Leaf carried forward</div>
          </div>
        </div>

        {/* Level 2: Sub-parents */}
        <div className="tree-level tree-level-intermediate">
          {/* Node 0-1 */}
          <div
            className={`merkle-node ${isTampered && tamperState.tamperedLeaf?.includes('CONTENT') ? 'tampered' : ''}`}
            style={{ width: '280px' }}
            onClick={() =>
              setSelectedNode({
                title: 'NODE 0-1 (Image & Content Subtree)',
                type: 'node',
                hash: tree.node_0_1,
                details: 'Computed as SHA-256(0x01 || Leaf 0 Image || Leaf 1 Content)',
              })
            }
          >
            <div className="node-title-row">
              <span className="node-label">Node 0-1</span>
              <span style={{ fontSize: '11px', color: 'var(--text-muted)' }}>Prefix: 0x01</span>
            </div>
            <div className="node-hash mono">
              {tree.node_0_1.substring(0, 10)}...{tree.node_0_1.substring(tree.node_0_1.length - 10)}
            </div>
            <div className="node-desc">Leaves #0 and #1</div>
          </div>

          {/* Node 2-3 */}
          <div
            className={`merkle-node ${isTampered && tamperState.tamperedLeaf?.includes('FACE') ? 'tampered' : ''}`}
            style={{ width: '280px' }}
            onClick={() =>
              setSelectedNode({
                title: 'NODE 2-3 (Metadata & Face Subtree)',
                type: 'node',
                hash: tree.node_2_3,
                details: 'Computed as SHA-256(0x01 || Leaf 2 Metadata || Leaf 3 Face)',
              })
            }
          >
            <div className="node-title-row">
              <span className="node-label">Node 2-3</span>
              <span style={{ fontSize: '11px', color: 'var(--text-muted)' }}>Prefix: 0x01</span>
            </div>
            <div className="node-hash mono">
              {tree.node_2_3.substring(0, 10)}...{tree.node_2_3.substring(tree.node_2_3.length - 10)}
            </div>
            <div className="node-desc">Leaves #2 and #3</div>
          </div>
        </div>

        {/* Level 3: The 5 Domain-Separated Leaves */}
        <div className="tree-level-leaves">
          {/* Leaf 0: IMAGE */}
          <div
            className={`merkle-node node-leaf ${isTampered && tamperState.tamperedLeaf?.includes('IMAGE') ? 'tampered' : ''}`}
            onClick={() =>
              setSelectedNode({
                title: 'LEAF #0: IMAGE_HASH',
                type: 'leaf',
                hash: leaves.image_hash,
                details: 'SHA-256(0x00 || image_component:v1\\n || len || clean_image_sha256)',
                canonicalJson: canonicalizeJson({
                  image_sha256: evidenceRecord.image_sha256,
                }),
              })
            }
          >
            <div className="node-title-row">
              <span className="node-label">Leaf #0: IMAGE</span>
              <span style={{ fontSize: '10px', color: 'var(--text-muted)' }}>Prefix: 0x00</span>
            </div>
            <div className="node-hash mono">
              {leaves.image_hash.substring(0, 8)}...{leaves.image_hash.substring(leaves.image_hash.length - 8)}
            </div>
            <div className="node-desc">Candidate image bytes digest</div>
          </div>

          {/* Leaf 1: CONTENT */}
          <div
            className={`merkle-node node-leaf ${isTampered && tamperState.tamperedLeaf?.includes('CONTENT') ? 'tampered' : ''}`}
            onClick={() =>
              setSelectedNode({
                title: 'LEAF #1: CONTENT_HASH',
                type: 'leaf',
                hash: leaves.content_hash,
                details: 'SHA-256(0x00 || content_component:v1\\n || len(title) || title || len(text) || text)',
                canonicalJson: canonicalizeJson({
                  title: evidenceRecord.title,
                  text: evidenceRecord.text,
                }),
              })
            }
          >
            <div className="node-title-row">
              <span className="node-label" style={{ color: isTampered ? 'var(--crimson-tamper)' : 'var(--cyan-bright)' }}>
                Leaf #1: CONTENT
              </span>
              <span style={{ fontSize: '10px', color: 'var(--text-muted)' }}>Prefix: 0x00</span>
            </div>
            <div className={`node-hash mono ${isTampered ? 'tampered-hash' : ''}`}>
              {leaves.content_hash.substring(0, 8)}...{leaves.content_hash.substring(leaves.content_hash.length - 8)}
            </div>
            <div className="node-desc">Source title & post text</div>
          </div>

          {/* Leaf 2: METADATA */}
          <div
            className={`merkle-node node-leaf ${isTampered && tamperState.tamperedLeaf?.includes('METADATA') ? 'tampered' : ''}`}
            onClick={() =>
              setSelectedNode({
                title: 'LEAF #2: METADATA_HASH',
                type: 'leaf',
                hash: leaves.metadata_hash,
                details: 'SHA-256(0x00 || metadata_component:v1\\n || schema || run_id || url || domain || platform || provider || ts)',
                canonicalJson: canonicalizeJson({
                  schema_version: evidenceRecord.schema_version,
                  run_id: evidenceRecord.run_id,
                  source_url: evidenceRecord.source_url,
                  domain: evidenceRecord.domain,
                  platform: evidenceRecord.platform,
                  provider: evidenceRecord.provider,
                  retrieved_at: evidenceRecord.retrieved_at,
                }),
              })
            }
          >
            <div className="node-title-row">
              <span className="node-label">Leaf #2: METADATA</span>
              <span style={{ fontSize: '10px', color: 'var(--text-muted)' }}>Prefix: 0x00</span>
            </div>
            <div className="node-hash mono">
              {leaves.metadata_hash.substring(0, 8)}...{leaves.metadata_hash.substring(leaves.metadata_hash.length - 8)}
            </div>
            <div className="node-desc">Provenance URL, domain & time</div>
          </div>

          {/* Leaf 3: FACE */}
          <div
            className={`merkle-node node-leaf ${isTampered && tamperState.tamperedLeaf?.includes('FACE') ? 'tampered' : ''}`}
            onClick={() =>
              setSelectedNode({
                title: 'LEAF #3: FACE_HASH',
                type: 'leaf',
                hash: leaves.face_hash,
                details: 'SHA-256(0x00 || face_result_component:v1\\n || similarity || model || quality)',
                canonicalJson: canonicalizeJson({
                  face_similarity: evidenceRecord.face_similarity.toFixed(6),
                  face_model: evidenceRecord.face_model,
                  candidate_quality: evidenceRecord.candidate_quality.toFixed(6),
                }),
              })
            }
          >
            <div className="node-title-row">
              <span className="node-label">Leaf #3: FACE</span>
              <span style={{ fontSize: '10px', color: 'var(--text-muted)' }}>Prefix: 0x00</span>
            </div>
            <div className="node-hash mono">
              {leaves.face_hash.substring(0, 8)}...{leaves.face_hash.substring(leaves.face_hash.length - 8)}
            </div>
            <div className="node-desc">ArcFace score & quality</div>
          </div>

          {/* Leaf 4: PROVENANCE */}
          <div
            className={`merkle-node node-leaf ${isTampered && tamperState.tamperedLeaf?.includes('PROVENANCE') ? 'tampered' : ''}`}
            onClick={() =>
              setSelectedNode({
                title: 'LEAF #4: PROVENANCE_HASH',
                type: 'leaf',
                hash: leaves.provenance_hash,
                details: 'SHA-256(0x00 || provenance_component:v1\\n || run_id || provider || platform || ts)',
                canonicalJson: canonicalizeJson({
                  run_id: evidenceRecord.run_id,
                  provider: evidenceRecord.provider,
                  platform: evidenceRecord.platform,
                  retrieved_at: evidenceRecord.retrieved_at,
                }),
              })
            }
          >
            <div className="node-title-row">
              <span className="node-label">Leaf #4: PROVENANCE</span>
              <span style={{ fontSize: '10px', color: 'var(--text-muted)' }}>Prefix: 0x00</span>
            </div>
            <div className="node-hash mono">
              {leaves.provenance_hash.substring(0, 8)}...{leaves.provenance_hash.substring(leaves.provenance_hash.length - 8)}
            </div>
            <div className="node-desc">Audit trail chain state</div>
          </div>
        </div>
      </div>

      {/* Selected Node Details Drawer */}
      {selectedNode && (
        <div className="inspector-drawer">
          <div className="drawer-header">
            <div>
              <h3 style={{ fontSize: '16px', fontWeight: 800, color: 'var(--cyan-bright)' }}>
                {selectedNode.title}
              </h3>
              <p style={{ fontSize: '12px', color: 'var(--text-muted)', marginTop: '2px' }}>
                {selectedNode.details}
              </p>
            </div>
            <button
              className="btn btn-secondary"
              style={{ fontSize: '12px', padding: '6px 12px' }}
              onClick={() => handleCopyHash(selectedNode.hash)}
            >
              {copied ? <Check size={14} color="var(--emerald-verified)" /> : <Copy size={14} />} Copy Hash
            </button>
          </div>

          <div style={{ marginBottom: '14px' }}>
            <span style={{ fontSize: '11px', color: 'var(--text-muted)', textTransform: 'uppercase' }}>
              SHA-256 Node Hash
            </span>
            <div className="mono" style={{ background: '#060910', padding: '8px 12px', borderRadius: '4px', color: 'var(--cyan-bright)', marginTop: '4px', wordBreak: 'break-all' }}>
              {selectedNode.hash}
            </div>
          </div>

          {selectedNode.canonicalJson && (
            <div>
              <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center', marginBottom: '6px' }}>
                <span style={{ fontSize: '11px', color: 'var(--text-muted)', textTransform: 'uppercase' }}>
                  RFC 8785 Canonical JSON Serialization Payload
                </span>
                <span style={{ fontSize: '10px', color: 'var(--text-muted)' }}>UTF-8 NFC Normalized • Strict Key Sort</span>
              </div>
              <pre className="json-code-block">{selectedNode.canonicalJson}</pre>
            </div>
          )}
        </div>
      )}
    </div>
  );
};
