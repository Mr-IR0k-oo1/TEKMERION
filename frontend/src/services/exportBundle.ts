import JSZip from 'jszip';
import {
  AuditEvent,
  BlockchainRecord,
  EvidenceBundle,
  EvidenceRecord,
  VerificationResult,
} from '../types/forensic';

export interface BundleExportData {
  runId: string;
  imageFileName: string;
  imageHash: string;
  resolution: string;
  candidates: VerificationResult[];
  evidenceRecord: EvidenceRecord | null;
  evidenceBundle: EvidenceBundle | null;
  blockchainRecord: BlockchainRecord | null;
  auditEvents: AuditEvent[];
}

export async function downloadForensicZip(data: BundleExportData): Promise<void> {
  const zip = new JSZip();
  const root = zip.folder(`runs/${data.runId}`);
  if (!root) throw new Error('Failed to initialize zip folder structure');

  // 1. input/
  const inputDir = root.folder('input');
  if (inputDir) {
    const inputMeta = {
      name: data.imageFileName,
      resolution: data.resolution,
      sha256: data.imageHash,
      run_id: data.runId,
      recorded_at: new Date().toISOString(),
    };
    inputDir.file('input_metadata.json', JSON.stringify(inputMeta, null, 2));
  }

  // 2. discovery/
  const discDir = root.folder('discovery');
  if (discDir) {
    const candidates = data.candidates.map(c => c.candidate);
    discDir.file('candidates.json', JSON.stringify(candidates, null, 2));
  }

  // 3. verification/
  const verDir = root.folder('verification');
  if (verDir) {
    verDir.file('results.json', JSON.stringify(data.candidates, null, 2));
  }

  // 4. evidence/
  const evDir = root.folder('evidence');
  if (evDir) {
    if (data.evidenceRecord) {
      evDir.file('evidence.json', JSON.stringify(data.evidenceRecord, null, 2));
    }
    if (data.evidenceBundle) {
      evDir.file('leaves.json', JSON.stringify(data.evidenceBundle.tree.leaves, null, 2));
      evDir.file(
        'root.json',
        JSON.stringify(
          {
            root_hash: data.evidenceBundle.root_hash,
            generated_at: new Date().toISOString(),
          },
          null,
          2
        )
      );
    }
  }

  // 5. blockchain/
  const chainDir = root.folder('blockchain');
  if (chainDir && data.blockchainRecord) {
    chainDir.file('transaction.json', JSON.stringify(data.blockchainRecord, null, 2));
  }

  // 6. audit.jsonl
  let jsonlContent = '';
  for (const event of data.auditEvents) {
    jsonlContent += JSON.stringify(event) + '\n';
  }
  root.file('audit.jsonl', jsonlContent);

  // Generate ZIP blob and trigger download
  const blob = await zip.generateAsync({ type: 'blob' });
  const url = URL.createObjectURL(blob);
  const a = document.createElement('a');
  a.href = url;
  a.download = `tekmerion_${data.runId}_forensic_bundle.zip`;
  document.body.appendChild(a);
  a.click();
  document.body.removeChild(a);
  URL.revokeObjectURL(url);
}

export function downloadForensicJson(data: BundleExportData): void {
  const exportPayload = {
    tekmerion_version: '1.0.0',
    run_id: data.runId,
    exported_at: new Date().toISOString(),
    query_image: {
      name: data.imageFileName,
      resolution: data.resolution,
      sha256: data.imageHash,
    },
    candidates: data.candidates,
    evidence_record: data.evidenceRecord,
    evidence_bundle: data.evidenceBundle,
    blockchain_record: data.blockchainRecord,
    audit_trail: data.auditEvents,
  };

  const blob = new Blob([JSON.stringify(exportPayload, null, 2)], {
    type: 'application/json',
  });
  const url = URL.createObjectURL(blob);
  const a = document.createElement('a');
  a.href = url;
  a.download = `tekmerion_${data.runId}_report.json`;
  document.body.appendChild(a);
  a.click();
  document.body.removeChild(a);
  URL.revokeObjectURL(url);
}
