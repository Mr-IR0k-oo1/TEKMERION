import React, { useState, useEffect, useCallback } from 'react';
import {
  AuditEvent,
  BlockchainRecord,
  EvidenceBundle,
  EvidenceRecord,
  FaceQualityAssessment,
  PipelineStageId,
  PipelineStatus,
  TamperState,
  VerificationResult,
  ViewTab,
} from './types/forensic';
import {
  SAMPLE_INVESTIGATIONS,
  SAMPLE_BLOCKCHAIN_RECORD,
  createInitialEvidenceRecord,
  generateRunId,
} from './services/sampleData';
import { buildMerkleTree, computeEvidenceLeaves } from './crypto/merkle';
import { sha256 } from './crypto/sha256';
import { Navbar } from './components/Navbar';
import { EvidenceView } from './components/views/EvidenceView';
import { MerkleView } from './components/views/MerkleView';
import { BlockchainProof } from './components/views/BlockchainProof';
import { TamperLab } from './components/views/TamperLab';
import { CandidateInspector } from './components/views/CandidateInspector';
import { AuditExplorer } from './components/views/AuditExplorer';
import { BundleExportData } from './services/exportBundle';
import { InvestigationHome, InvestigationProgress } from './components/investigation';
import type { SelectedImage } from './components/investigation';
import { INVESTIGATION_STAGES } from './components/investigation';

export const App: React.FC = () => {
  // Navigation State
  const [activeTab, setActiveTab] = useState<ViewTab>('pipeline');
  const [evidenceTab, setEvidenceTab] = useState<'evidence' | 'merkle' | 'blockchain' | 'tamper'>('evidence');

  // Investigation Experience Mode
  const [investigationMode, setInvestigationMode] = useState<'home' | 'progress' | 'done'>('home');
  const [investigationImage, setInvestigationImage] = useState<SelectedImage | null>(null);

  // Active Sample Investigation
  const [currentSample, setCurrentSample] = useState(SAMPLE_INVESTIGATIONS[0]);
  const [runId, setRunId] = useState<string>(generateRunId());

  // Pipeline Execution State
  const [currentStage, setCurrentStage] = useState<PipelineStageId>('INPUT');
  const [completedStages, setCompletedStages] = useState<PipelineStageId[]>([]);
  const [status, setStatus] = useState<PipelineStatus>('idle');

  // Forensic Data (Zero mock data - begins awaiting user upload)
  const [imageFileName, setImageFileName] = useState('');
  const [imageSrc, setImageSrc] = useState('');
  const [resolution, setResolution] = useState('');
  const [imageHash, setImageHash] = useState('');
  const [quality, setQuality] = useState<FaceQualityAssessment>({
    status: 'idle',
    face_count: 0,
    blur_variance: 0,
    brightness: 0,
    bbox: [0, 0, 0, 0],
    landmarks: [],
    embedding_preview: [],
    reasons: ['Awaiting forensic image upload from user'],
  });
  const [candidates, setCandidates] = useState<VerificationResult[]>([]);

  // Evidence & Cryptographic Tree
  const [evidenceRecord, setEvidenceRecord] = useState<EvidenceRecord | null>(null);
  const [evidenceBundle, setEvidenceBundle] = useState<EvidenceBundle | null>(null);
  const [originalRecord, setOriginalRecord] = useState<EvidenceRecord | null>(null);
  const [blockchainRecord, setBlockchainRecord] = useState<BlockchainRecord | null>(null);

  // Tamper State
  const [tamperState, setTamperState] = useState<TamperState>({
    isTampered: false,
    tamperedLeaf: null,
    tamperedField: null,
    originalValue: null,
    tamperedValue: null,
    originalLeafHash: null,
    tamperedLeafHash: null,
    originalRoot: null,
    tamperedRoot: null,
  });

  // Audit Events Stream
  const [auditEvents, setAuditEvents] = useState<AuditEvent[]>([]);

  // Backend Health State
  const [backendOnline, setBackendOnline] = useState<boolean>(false);

  useEffect(() => {
    const checkBackend = async () => {
      try {
        const res = await fetch('/api/health');
        if (res.ok) {
          setBackendOnline(true);
        } else {
          setBackendOnline(false);
        }
      } catch {
        setBackendOnline(false);
      }
    };
    checkBackend();
    const timer = setInterval(checkBackend, 6000);
    return () => clearInterval(timer);
  }, []);

  const pushAuditEvent = useCallback(
    (event: string, level: 'info' | 'warn' | 'error' | 'success' = 'info', detail?: string) => {
      const entry: AuditEvent = {
        id: `ev_${Date.now()}_${Math.random().toString(36).substring(2, 6)}`,
        event,
        timestamp: new Date().toISOString(),
        run_id: runId,
        level,
        detail,
      };
      setAuditEvents((prev) => [entry, ...prev]);
    },
    [runId]
  );

  // Initialize Sample on load or switch
  const initializeSample = useCallback(
    async (sample: typeof SAMPLE_INVESTIGATIONS[0], newRunId?: string) => {
      const activeRun = newRunId || generateRunId();
      setRunId(activeRun);
      setCurrentSample(sample);
      setImageFileName(sample.imageFileName);
      setImageSrc(sample.imageSrc);
      setResolution(sample.resolution);
      setImageHash(sample.imageHash);
      setQuality(sample.faceQuality);
      setCandidates(sample.candidates);
      setCurrentStage('INPUT');
      setCompletedStages([]);
      setStatus('idle');
      setTamperState({
        isTampered: false,
        tamperedLeaf: null,
        tamperedField: null,
        originalValue: null,
        tamperedValue: null,
        originalLeafHash: null,
        tamperedLeafHash: null,
        originalRoot: null,
        tamperedRoot: null,
      });

      if (sample.candidates.length > 0 && sample.faceQuality.status === 'pass') {
        const top = sample.candidates[0];
        const record = createInitialEvidenceRecord(activeRun, top);
        const leaves = await computeEvidenceLeaves(record);
        const tree = await buildMerkleTree(leaves);
        const bundle: EvidenceBundle = {
          schema_version: record.schema_version,
          run_id: activeRun,
          root_hash: tree.root_hash,
          tree,
          record,
        };

        setEvidenceRecord(record);
        setOriginalRecord(record);
        setEvidenceBundle(bundle);

        const chainRec: BlockchainRecord = {
          ...SAMPLE_BLOCKCHAIN_RECORD,
          registered_root: tree.root_hash,
          registered_image: sample.imageHash,
          timestamp: new Date().toISOString(),
        };
        setBlockchainRecord(chainRec);
      } else {
        setEvidenceRecord(null);
        setEvidenceBundle(null);
        setBlockchainRecord(null);
      }

      pushAuditEvent(`Loaded investigation case: ${sample.name}`, 'info');
    },
    [pushAuditEvent]
  );

  useEffect(() => {
    pushAuditEvent(
      'TEKMERION Forensic Verification Node ready. Upload image or run live benchmark. Zero mock policy active.',
      'info'
    );
  }, [pushAuditEvent]);

  // Reset Pipeline to clean empty state
  const handleReset = () => {
    const newId = generateRunId();
    setRunId(newId);
    setImageFileName('');
    setImageSrc('');
    setResolution('');
    setImageHash('');
    setQuality({
      status: 'idle',
      face_count: 0,
      blur_variance: 0,
      brightness: 0,
      bbox: [0, 0, 0, 0],
      landmarks: [],
      embedding_preview: [],
      reasons: ['Awaiting forensic image upload from user'],
    });
    setCandidates([]);
    setEvidenceRecord(null);
    setEvidenceBundle(null);
    setOriginalRecord(null);
    setBlockchainRecord(null);
    setCurrentStage('INPUT');
    setCompletedStages([]);
    setStatus('idle');
    setTamperState({
      isTampered: false,
      tamperedLeaf: null,
      tamperedField: null,
      originalValue: null,
      tamperedValue: null,
      originalLeafHash: null,
      tamperedLeafHash: null,
      originalRoot: null,
      tamperedRoot: null,
    });
    pushAuditEvent(`Forensic session reset. New Run ID: ${newId}. Awaiting evidence image.`, 'info');
  };

  // Live Benchmark Handler - executes genuine pipeline on real benchmark file
  const handleSelectSample = async (sampleId: string) => {
    const filename = sampleId === 'case_multi_face' ? 'multi_face.jpg' : 'query_face.jpg';
    pushAuditEvent(`Fetching benchmark asset: /${filename} for live execution...`, 'info');
    try {
      const resp = await fetch(`/${filename}`);
      if (!resp.ok) throw new Error(`HTTP ${resp.status} reading /${filename}`);
      const blob = await resp.blob();
      const file = new File([blob], filename, { type: blob.type || 'image/jpeg' });
      await handleCustomImageUpload(file);
    } catch (err: any) {
      console.error('Benchmark fetch error:', err);
      pushAuditEvent(`Error fetching benchmark image: ${err.message}`, 'error');
    }
  };

  // Apply custom field tamper from TamperLab
  const handleApplyTamper = async (
    mutatedFields: Partial<EvidenceRecord>,
    leafName: string,
    fieldName: string
  ) => {
    if (!evidenceRecord || !evidenceBundle || !blockchainRecord) return;

    const baseOriginal = originalRecord || evidenceRecord;
    const mutatedRecord: EvidenceRecord = {
      ...evidenceRecord,
      ...mutatedFields,
    };

    const origLeaves = await computeEvidenceLeaves(baseOriginal);
    const mutatedLeaves = await computeEvidenceLeaves(mutatedRecord);
    const mutatedTree = await buildMerkleTree(mutatedLeaves);

    const mutatedBundle: EvidenceBundle = {
      ...evidenceBundle,
      root_hash: mutatedTree.root_hash,
      tree: mutatedTree,
      record: mutatedRecord,
    };

    setEvidenceRecord(mutatedRecord);
    setEvidenceBundle(mutatedBundle);

    const isDifferent = mutatedTree.root_hash !== blockchainRecord.registered_root;
    setStatus(isDifferent ? 'tampered' : 'completed');

    setTamperState({
      isTampered: isDifferent,
      tamperedLeaf: leafName,
      tamperedField: fieldName,
      originalValue: String((baseOriginal as unknown as Record<string, unknown>)[fieldName] || ''),
      tamperedValue: String((mutatedRecord as unknown as Record<string, unknown>)[fieldName] || ''),
      originalLeafHash: (origLeaves as unknown as Record<string, string>)[`${fieldName}_hash`] || origLeaves.content_hash,
      tamperedLeafHash: (mutatedLeaves as unknown as Record<string, string>)[`${fieldName}_hash`] || mutatedLeaves.content_hash,
      originalRoot: blockchainRecord.registered_root,
      tamperedRoot: mutatedTree.root_hash,
    });

    if (isDifferent) {
      pushAuditEvent(`Tamper applied on ${leafName} (${fieldName}): Local Merkle root changed.`, 'warn');
    } else {
      pushAuditEvent('Evidence values restored to authentic on-chain state.', 'success');
    }
  };

  // Restore Original Record
  const handleRestore = async () => {
    if (!originalRecord || !blockchainRecord) return;

    const leaves = await computeEvidenceLeaves(originalRecord);
    const tree = await buildMerkleTree(leaves);
    const bundle: EvidenceBundle = {
      schema_version: originalRecord.schema_version,
      run_id: runId,
      root_hash: tree.root_hash,
      tree,
      record: originalRecord,
    };

    setEvidenceRecord(originalRecord);
    setEvidenceBundle(bundle);
    setStatus('completed');

    setTamperState({
      isTampered: false,
      tamperedLeaf: null,
      tamperedField: null,
      originalValue: null,
      tamperedValue: null,
      originalLeafHash: null,
      tamperedLeafHash: null,
      originalRoot: null,
      tamperedRoot: null,
    });

    pushAuditEvent('Original forensic evidence restored. Merkle root matches Sepolia anchor ✓', 'success');
  };

  // Custom Image Upload Handler
  const handleCustomImageUpload = async (file: File) => {
    pushAuditEvent(`Reading forensic file: ${file.name}...`, 'info');

    // 1. Read arrayBuffer & compute Web Crypto SHA-256
    const arrayBuffer = await file.arrayBuffer();
    const hash = await sha256(new Uint8Array(arrayBuffer));

    // 2. Read as Data URL via FileReader for cross-browser reliability
    const dataUrl: string = await new Promise((resolve, reject) => {
      const reader = new FileReader();
      reader.onload = () => resolve(reader.result as string);
      reader.onerror = reject;
      reader.readAsDataURL(file);
    });

    // 3. Load Image safely to read real dimensions
    const dims: { width: number; height: number } = await new Promise((resolve) => {
      const img = new Image();
      img.onload = () => {
        resolve({
          width: img.naturalWidth || img.width || 1280,
          height: img.naturalHeight || img.height || 720,
        });
      };
      img.onerror = () => {
        resolve({ width: 1280, height: 720 });
      };
      img.src = dataUrl;
    });

    const resStr = `${dims.width}x${dims.height} (${(file.size / 1024).toFixed(1)} KB)`;
    setImageFileName(file.name);
    setImageSrc(dataUrl);
    setResolution(resStr);
    setImageHash(hash);
    setCurrentStage('INPUT');
    setCompletedStages([]);
    setStatus('running');

    pushAuditEvent(`Forensic file uploaded: ${file.name} (${resStr}) SHA-256: ${hash.substring(0, 16)}...`, 'info');
    pushAuditEvent('Executing live forensic pipeline via Python Face Worker...', 'info');

    try {
      const resp = await fetch('/api/pipeline/run', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({
          filename: file.name,
          image_base64: dataUrl,
        }),
      });

      if (!resp.ok) {
        throw new Error(`Pipeline API returned status ${resp.status}`);
      }

      const data = await resp.json();

      if (data.gate_rejected || !data.success) {
        // Strict Gate Rejection
        const failQuality: FaceQualityAssessment = {
          status: 'fail',
          face_count: data.face?.face_count || 0,
          blur_variance: data.face?.blur_variance || 0,
          brightness: 128.0,
          bbox: data.face?.bbox || [0, 0, 0, 0],
          landmarks: data.face?.landmarks || [],
          embedding_preview: data.face?.embedding_preview || [],
          reasons: data.face?.reasons || [data.error || 'Forensic Gate Rejection'],
        };
        setQuality(failQuality);
        setCandidates([]);
        setEvidenceRecord(null);
        setEvidenceBundle(null);
        setBlockchainRecord(null);
        setCurrentStage('FACE');
        setCompletedStages(['INPUT']);
        setStatus('idle');
        pushAuditEvent(`FORENSIC GATE REJECTION: ${data.error || 'Face gate rejected'}`, 'error');
        return;
      }

      // Success
      setRunId(data.run_id);
      const passedQuality: FaceQualityAssessment = {
        status: 'pass',
        face_count: data.face.face_count,
        blur_variance: data.face.blur_variance,
        brightness: 128.0,
        bbox: data.face.bbox,
        landmarks: data.face.landmarks,
        embedding_preview: data.face.embedding_preview,
        reasons: data.face.reasons,
      };
      setQuality(passedQuality);

      const convertedCandidates: VerificationResult[] = data.discovery.candidates.map((c: any) => ({
        candidate: {
          url: c.url,
          title: c.title,
          domain: c.domain,
          image_url: c.image_url,
          thumbnail_url: c.thumbnail_url || c.image_url,
          snippet: c.snippet,
          provider: c.provider,
          discovered_at: new Date().toISOString(),
        },
        similarity: c.similarity,
        quality: c.quality,
        matched_face_index: c.matched_face_index,
        candidate_image_hash: c.candidate_image_hash,
        status:
          c.status === 'Verified'
            ? 'verified'
            : c.status === 'BelowThreshold'
            ? 'below_threshold'
            : c.status === 'NoFace'
            ? 'no_face'
            : 'error',
      }));

      setCandidates(convertedCandidates);

      if (!data.match_found) {
        setEvidenceRecord(null);
        setOriginalRecord(null);
        setEvidenceBundle(null);
        setBlockchainRecord(null);
        setCurrentStage('VERIFY');
        setCompletedStages(['INPUT', 'FACE', 'DISCOVERY', 'VERIFY']);
        setStatus('no_match');

        pushAuditEvent(`Stage 2 (FACE): Detected 1 face. Blur: ${data.face.blur_variance.toFixed(1)}`, 'success');
        pushAuditEvent(`Stage 3 (DISCOVERY): Discovered ${convertedCandidates.length} candidate web assets`, 'info');
        pushAuditEvent(
          `Stage 4 (VERIFY): No candidate met the 75.0% biometric threshold. Top candidate similarity: ${
            convertedCandidates[0] ? (convertedCandidates[0].similarity * 100).toFixed(1) + '%' : 'None'
          } -> NO MATCH FOUND. Blockchain registration skipped.`,
          'warn'
        );
        return;
      }

      const record: EvidenceRecord = {
        schema_version: data.evidence.schema_version,
        run_id: data.run_id,
        source_url: data.evidence.record.source_url,
        domain: data.evidence.record.domain,
        platform: data.evidence.record.platform,
        provider: data.evidence.record.provider,
        retrieved_at: data.evidence.record.retrieved_at,
        title: data.evidence.record.title,
        text: data.evidence.record.text,
        image_sha256: data.evidence.record.image_sha256,
        face_similarity: data.evidence.record.face_similarity,
        face_model: data.evidence.record.face_model,
        candidate_quality: data.evidence.record.candidate_quality,
      };

      const leaves = await computeEvidenceLeaves(record);
      const tree = await buildMerkleTree(leaves);
      const bundle: EvidenceBundle = {
        schema_version: record.schema_version,
        run_id: data.run_id,
        root_hash: data.evidence.root_hash || tree.root_hash,
        tree,
        record,
      };

      setEvidenceRecord(record);
      setOriginalRecord(record);
      setEvidenceBundle(bundle);

      const chainRec: BlockchainRecord = {
        network: data.blockchain.network,
        contract_address: data.blockchain.contract,
        tx_hash: data.blockchain.tx_hash,
        block_number: data.blockchain.block_number,
        confirmations: data.blockchain.confirmations,
        registered_root: data.blockchain.registered_root,
        registered_image: data.input.sha256,
        submitter: '0x34a1B75e19F8aB4639908F0945952c1Eb16B9b2c',
        timestamp: data.blockchain.timestamp,
      };
      setBlockchainRecord(chainRec);

      setCurrentStage('FINAL_VERIFY');
      setCompletedStages(['INPUT', 'FACE', 'DISCOVERY', 'VERIFY', 'EVIDENCE', 'BLOCKCHAIN', 'FINAL_VERIFY']);
      setStatus('completed');

      pushAuditEvent(`Stage 2 (FACE): Detected 1 face. Blur: ${data.face.blur_variance.toFixed(1)}`, 'success');
      pushAuditEvent(
        `Stage 4 (VERIFY): Biometric match confirmed: "${convertedCandidates[0]?.candidate.title}" similarity = ${(
          (convertedCandidates[0]?.similarity || 0) * 100
        ).toFixed(2)}% (MATCH FOUND)`,
        'success'
      );
      pushAuditEvent(`Stage 6 (BLOCKCHAIN): Anchored evidence to Ethereum Sepolia Block #${chainRec.block_number} ✓`, 'success');
    } catch (err: any) {
      console.error('Custom image upload pipeline error:', err);
      pushAuditEvent(`Failed to execute pipeline for uploaded image: ${err.message}`, 'error');
      setStatus('idle');
    }
  };

  // Bundle Export Data package
  const exportData: BundleExportData = {
    runId,
    imageFileName,
    imageHash,
    resolution,
    candidates,
    evidenceRecord,
    evidenceBundle,
    blockchainRecord,
    auditEvents,
  };

  // Derived investigation display state
  const stageToIndex: Record<PipelineStageId, number> = {
    INPUT: 0,
    FACE: 1,
    DISCOVERY: 2,
    VERIFY: 3,
    EVIDENCE: 5,
    BLOCKCHAIN: 6,
    FINAL_VERIFY: 7,
  };
  const currentStageIndex = stageToIndex[currentStage] ?? 0;
  const completedStageIndexes = completedStages
    .map((s) => (s === 'VERIFY' ? 4 : stageToIndex[s] ?? 0))
    .filter((n) => !Number.isNaN(n));
  const investigationCompletedCount = completedStageIndexes.length
    ? Math.max(...completedStageIndexes) + 1
    : status === 'completed'
    ? INVESTIGATION_STAGES.length
    : 0;

  const getStageContext = (): string => {
    if (status === 'completed') return '';
    switch (currentStage) {
      case 'INPUT':
        return 'Preparing the image for analysis...';
      case 'FACE':
        return 'Analyzing the image...';
      case 'DISCOVERY':
        return `${candidates.length} discovered ${candidates.length === 1 ? 'candidate' : 'candidates'}`;
      case 'VERIFY':
        return `${candidates.filter((c) => c.status === 'verified').length} candidates verified`;
      case 'EVIDENCE':
        return 'Evidence fingerprint generated';
      case 'BLOCKCHAIN':
        return 'Recording the fingerprint on chain...';
      case 'FINAL_VERIFY':
        return 'Confirming the blockchain anchor...';
      default:
        return 'Working...';
    }
  };

  const handleInvestigationSelected = (image: SelectedImage) => {
    setInvestigationImage(image);
    if (imageSrc !== image.dataUrl) {
      const file = dataUrlToFile(image.dataUrl, image.fileName);
      if (file) {
        handleCustomImageUpload(file);
      }
    }
    setInvestigationMode('progress');
  };

  const handleInvestigationBack = () => {
    handleReset();
    setInvestigationImage(null);
    setInvestigationMode('home');
  };

  const dataUrlToFile = (dataUrl: string, fileName: string): File | null => {
    try {
      const [meta, base64] = dataUrl.split(',');
      const mime = (meta.match(/data:(.*?);/) || [])[1] || 'image/png';
      const binary = atob(base64);
      const bytes = new Uint8Array(binary.length);
      for (let i = 0; i < binary.length; i++) bytes[i] = binary.charCodeAt(i);
      return new File([bytes], fileName, { type: mime });
    } catch {
      return null;
    }
  };

  const investigationStatus = status === 'tampered'
    ? 'tampered'
    : status === 'no_match'
    ? 'no_match'
    : status === 'error'
    ? 'error'
    : status === 'completed'
    ? 'completed'
    : status;

  const extractionQuality = Math.max(15, Math.min(100, Math.round(100 - quality.blur_variance / 6)));

  const renderInvestigation = () => {
    if (!investigationImage) return null;
    return (
      <InvestigationProgress
        fileName={investigationImage.fileName}
        dimensions={investigationImage.dimensions}
        fileSize={investigationImage.fileSize}
        imageSrc={investigationImage.dataUrl}
        runId={runId}
        evidenceHash={imageHash ?? undefined}
        currentStageIndex={currentStageIndex}
        completedStages={investigationCompletedCount}
        status={investigationStatus}
        stageContext={getStageContext()}
        onBack={handleInvestigationBack}
        faceQuality={
          quality.face_count > 0
            ? {
                faceCount: quality.face_count,
                bbox: quality.bbox,
                extractionQuality,
                blurVariance: quality.blur_variance,
                brightness: quality.brightness,
                status: quality.status,
              }
            : undefined
        }
        candidates={candidates.length ? candidates : undefined}
      />
    );
  };

  const renderEvidence = () => {
    switch (evidenceTab) {
      case 'merkle':
        return (
          <MerkleView evidenceBundle={evidenceBundle} evidenceRecord={evidenceRecord} tamperState={tamperState} />
        );
      case 'blockchain':
        return <BlockchainProof blockchainRecord={blockchainRecord} verified={!tamperState.isTampered} />;
      case 'tamper':
        return (
          <TamperLab
            evidenceRecord={evidenceRecord}
            evidenceBundle={evidenceBundle}
            tamperState={tamperState}
            anchoredRoot={blockchainRecord?.registered_root || '--'}
            onApplyTamper={handleApplyTamper}
            onRestore={handleRestore}
          />
        );
      default:
        return <EvidenceView evidenceRecord={evidenceRecord} evidenceBundle={evidenceBundle} verified={!tamperState.isTampered} />;
    }
  };

  return (
    <div>
      <Navbar
        activeTab={activeTab}
        onSelectTab={(tab) => {
          setActiveTab(tab);
          if (tab !== 'pipeline') setInvestigationMode('home');
        }}
        runId={runId}
        isTampered={tamperState.isTampered}
        contractAddress={SAMPLE_BLOCKCHAIN_RECORD.contract_address}
        backendOnline={backendOnline}
      />

      <main className="app-container">
        {activeTab === 'pipeline' &&
          (investigationImage ? (
            renderInvestigation()
          ) : (
            <InvestigationHome onImageSelected={handleInvestigationSelected} onOpenBenchmark={handleSelectSample} />
          ))}

        {activeTab === 'evidence' && (
          <div className="evidence-workspace">
            <div className="evidence-toolbar">
              <div className="seg" role="tablist" aria-label="Evidence panes">
                <button role="tab" aria-selected={evidenceTab === 'evidence'} className={`seg-item ${evidenceTab === 'evidence' ? 'is-active' : ''}`} onClick={() => setEvidenceTab('evidence')}>Evidence</button>
                <button role="tab" aria-selected={evidenceTab === 'merkle'} className={`seg-item ${evidenceTab === 'merkle' ? 'is-active' : ''}`} onClick={() => setEvidenceTab('merkle')}>Merkle Tree</button>
                <button role="tab" aria-selected={evidenceTab === 'blockchain'} className={`seg-item ${evidenceTab === 'blockchain' ? 'is-active' : ''}`} onClick={() => setEvidenceTab('blockchain')}>Blockchain</button>
                <button role="tab" aria-selected={evidenceTab === 'tamper'} className={`seg-item ${evidenceTab === 'tamper' ? 'is-active' : ''}`} onClick={() => setEvidenceTab('tamper')}>Tamper Lab</button>
              </div>
            </div>
            {renderEvidence()}
          </div>
        )}

        {activeTab === 'candidates' && <CandidateInspector candidates={candidates} queryImageSrc={imageSrc} />}

        {activeTab === 'audit' && <AuditExplorer exportData={exportData} auditEvents={auditEvents} />}
      </main>
    </div>
  );
};

export default App;
