import React, { useState } from 'react';
import { VerificationResult, VerificationStatus } from '../../types/forensic';
import {
  Users,
  ExternalLink,
  CheckCircle2,
  AlertCircle,
  ShieldX,
  FileQuestion,
  Filter,
} from 'lucide-react';

interface CandidateInspectorProps {
  candidates: VerificationResult[];
  queryImageSrc: string;
}

export const CandidateInspector: React.FC<CandidateInspectorProps> = ({
  candidates,
  queryImageSrc,
}) => {
  const [filter, setFilter] = useState<'all' | VerificationStatus>('all');

  const filtered = candidates.filter((c) => {
    if (filter === 'all') return true;
    return c.status === filter;
  });

  return (
    <div className="inspector-container">
      {/* Header & Filter Controls */}
      <div className="card" style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center', flexWrap: 'wrap', gap: '16px' }}>
        <div>
          <h2 style={{ fontSize: '18px', fontWeight: 800, display: 'flex', alignItems: 'center', gap: '8px' }}>
            <Users size={20} color="var(--cyan-bright)" /> Public Candidate Inspector
          </h2>
          <p style={{ fontSize: '12px', color: 'var(--text-muted)', marginTop: '4px' }}>
            Discovered through reverse-image discovery providers and independently verified locally via ArcFace embeddings.
          </p>
        </div>

        {/* Filter Badges */}
        <div style={{ display: 'flex', alignItems: 'center', gap: '8px' }}>
          <Filter size={15} color="var(--text-muted)" />
          <button
            className={`btn btn-secondary ${filter === 'all' ? 'active' : ''}`}
            style={{ padding: '4px 10px', fontSize: '12px' }}
            onClick={() => setFilter('all')}
          >
            All ({candidates.length})
          </button>
          <button
            className={`btn btn-secondary ${filter === 'verified' ? 'active' : ''}`}
            style={{ padding: '4px 10px', fontSize: '12px' }}
            onClick={() => setFilter('verified')}
          >
            Verified ({candidates.filter((c) => c.status === 'verified').length})
          </button>
          <button
            className={`btn btn-secondary ${filter === 'below_threshold' ? 'active' : ''}`}
            style={{ padding: '4px 10px', fontSize: '12px' }}
            onClick={() => setFilter('below_threshold')}
          >
            Below Threshold
          </button>
          <button
            className={`btn btn-secondary ${filter === 'no_face' ? 'active' : ''}`}
            style={{ padding: '4px 10px', fontSize: '12px' }}
            onClick={() => setFilter('no_face')}
          >
            No Face
          </button>
        </div>
      </div>

      {/* Grid of Candidates */}
      {filtered.length === 0 ? (
        <div className="card" style={{ textAlign: 'center', padding: '60px 20px', color: 'var(--text-muted)' }}>
          No candidates found matching the selected filter.
        </div>
      ) : (
        <div className="candidates-grid">
          {filtered.map((item, idx) => {
            const { candidate, similarity, quality, status, error_message, candidate_image_hash } = item;
            const isVerified = status === 'verified';
            const pct = Math.round(similarity * 100);

            return (
              <div key={idx} className={`candidate-card ${status.replace('_', '-')}`}>
                {/* Header: Title, Domain & Status Badge */}
                <div className="candidate-header">
                  <div>
                    <h3 className="candidate-title">{candidate.title || 'Untitled Candidate'}</h3>
                    <div className="candidate-domain">
                      <span>{candidate.domain}</span>
                      <a
                        href={candidate.url}
                        target="_blank"
                        rel="noreferrer"
                        style={{ color: 'inherit', display: 'inline-flex', alignItems: 'center' }}
                      >
                        <ExternalLink size={12} />
                      </a>
                    </div>
                  </div>

                  <div>
                    {isVerified ? (
                      <span className="badge badge-emerald">
                        <CheckCircle2 size={12} /> VERIFIED
                      </span>
                    ) : status === 'below_threshold' ? (
                      <span className="badge badge-amber">
                        <AlertCircle size={12} /> LOW SIMILARITY
                      </span>
                    ) : status === 'no_face' ? (
                      <span className="badge badge-crimson">
                        <ShieldX size={12} /> NO FACE
                      </span>
                    ) : (
                      <span className="badge badge-crimson">
                        <FileQuestion size={12} /> ERROR
                      </span>
                    )}
                  </div>
                </div>

                {/* Biometric Comparison Gauge */}
                <div className="biometric-gauge-box">
                  <div className="gauge-row">
                    <span className="gauge-label">ArcFace Biometric Cosine Similarity:</span>
                    <span
                      className="gauge-value"
                      style={{ color: isVerified ? 'var(--emerald-verified)' : 'var(--amber-warn)' }}
                    >
                      {(similarity * 100).toFixed(2)}%
                    </span>
                  </div>

                  <div className="similarity-bar-container">
                    <div
                      className={`similarity-fill ${isVerified ? 'pass' : 'fail'}`}
                      style={{ width: `${Math.min(100, Math.max(0, pct))}%` }}
                    />
                    {/* Fixed 0.80 (80%) Threshold line */}
                    <div className="threshold-marker" title="Forensic Acceptance Threshold: 80%" />
                  </div>

                  <div style={{ display: 'flex', justifyContent: 'space-between', fontSize: '10px', color: 'var(--text-muted)' }}>
                    <span>0% Non-Match</span>
                    <span style={{ color: '#fff', fontWeight: 600 }}>Threshold: 80.0%</span>
                    <span>100% Exact</span>
                  </div>
                </div>

                {/* Candidate Media Row & Side-by-Side Crop */}
                <div className="candidate-media-row">
                  <div style={{ display: 'flex', gap: '6px', alignItems: 'center' }}>
                    <div title="Query Face">
                      <img src={queryImageSrc} alt="Query Face" className="candidate-thumb" />
                    </div>
                    <span style={{ color: 'var(--text-muted)', fontSize: '11px' }}>vs</span>
                    <div title="Candidate Post Media">
                      <img
                        src={candidate.thumbnail_url || candidate.image_url}
                        alt="Candidate Post"
                        className="candidate-thumb"
                        onError={(e) => {
                          (e.target as HTMLImageElement).src =
                            'https://images.unsplash.com/photo-1534528741775-53994a69daeb?w=200&auto=format&fit=crop&q=80';
                        }}
                      />
                    </div>
                  </div>

                  <div className="candidate-snippet">
                    {error_message ? (
                      <span style={{ color: 'var(--crimson-tamper)' }}>{error_message}</span>
                    ) : (
                      candidate.snippet || 'No snippet description available.'
                    )}
                  </div>
                </div>

                {/* Footer: Candidate Image SHA-256 Digest */}
                <div className="candidate-footer">
                  <span style={{ color: 'var(--text-muted)' }}>Quality Score: {(quality * 100).toFixed(1)}%</span>
                  <span className="mono" style={{ color: 'var(--cyan-bright)' }}>
                    SHA: {candidate_image_hash.substring(0, 8)}...{candidate_image_hash.substring(candidate_image_hash.length - 6)}
                  </span>
                </div>
              </div>
            );
          })}
        </div>
      )}
    </div>
  );
};
