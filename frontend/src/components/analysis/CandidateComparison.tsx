import React, { useState } from 'react';
import { VerificationResult } from '../../types/forensic';
import { Info } from 'lucide-react';

const EMBEDDING_NOTE =
  'This measures visual embedding similarity and is not proof of identity.';

interface CandidateComparisonProps {
  queryImageSrc: string;
  candidate: VerificationResult;
}

export const CandidateComparison: React.FC<CandidateComparisonProps> = ({
  queryImageSrc,
  candidate,
}) => {
  const [showNote, setShowNote] = useState(false);
  const similarity = candidate.similarity;

  const relevance = Math.min(100, Math.round(similarity * 100));
  const imageQuality = Math.min(100, Math.round((candidate.quality || 0) * 100));
  const ranking = candidate.status === 'verified' ? 'High' : candidate.status === 'below_threshold' ? 'Moderate' : 'Low';

  return (
    <div className="comparison">
      <div className="comparison-stage">
        <div className="comparison-image">
          <img src={queryImageSrc} alt="Original image" />
          <span className="comparison-image-label">Original</span>
        </div>
        <div className="comparison-image">
          <img
            src={candidate.candidate.thumbnail_url || candidate.candidate.image_url}
            alt={candidate.candidate.title}
            onError={(e) => {
              (e.target as HTMLImageElement).style.display = 'none';
            }}
          />
          <span className="comparison-image-label">Candidate</span>
        </div>
      </div>

      <div className="comparison-similarity">
        <div className="comparison-similarity-header">
          <span className="comparison-similarity-label">Embedding Similarity</span>
          <span className="comparison-tooltip-trigger">
            <Info
              size={13}
              tabIndex={0}
              onMouseEnter={() => setShowNote(true)}
              onMouseLeave={() => setShowNote(false)}
              onFocus={() => setShowNote(true)}
              onBlur={() => setShowNote(false)}
              aria-label="Show explanation"
            />
            {showNote && <span className="comparison-tooltip">{EMBEDDING_NOTE}</span>}
          </span>
        </div>
        <div className="comparison-similarity-value">{similarity.toFixed(3)}</div>
      </div>

      <div className="comparison-detail">
        <div className="comparison-detail-row">
          <span className="comparison-detail-label">Image Quality</span>
          <div className="comparison-detail-track">
            <div
              className={`comparison-detail-fill ${imageQuality > 70 ? 'is-high' : imageQuality > 40 ? 'is-mid' : 'is-low'}`}
              style={{ width: `${imageQuality}%` }}
            />
          </div>
          <span className="comparison-detail-value">{imageQuality}</span>
        </div>
        <div className="comparison-detail-row">
          <span className="comparison-detail-label">Relevance</span>
          <div className="comparison-detail-track">
            <div
              className={`comparison-detail-fill ${relevance > 70 ? 'is-high' : relevance > 40 ? 'is-mid' : 'is-low'}`}
              style={{ width: `${relevance}%` }}
            />
          </div>
          <span className="comparison-detail-value">{relevance}</span>
        </div>
        <div className="comparison-detail-row">
          <span className="comparison-detail-label">Ranking</span>
          <div className="comparison-detail-track">
            <div
              className={`comparison-detail-fill ${ranking === 'High' ? 'is-high' : ranking === 'Moderate' ? 'is-mid' : 'is-low'}`}
              style={{ width: ranking === 'High' ? '100%' : ranking === 'Moderate' ? '60%' : '30%' }}
            />
          </div>
          <span className="comparison-detail-value">{ranking}</span>
        </div>
      </div>
    </div>
  );
};
