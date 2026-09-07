import React, { useRef, useState } from 'react';
import { ImagePlus, FileJson, Clock, ChevronRight } from 'lucide-react';

export interface SelectedImage {
  fileName: string;
  dataUrl: string;
  dimensions: string;
  fileSize: string;
}

interface InvestigationHomeProps {
  onImageSelected: (image: SelectedImage) => void;
  onOpenBenchmark?: (sampleId: string) => void;
}

export const InvestigationHome: React.FC<InvestigationHomeProps> = ({ onImageSelected, onOpenBenchmark }) => {
  const [isDragging, setIsDragging] = useState(false);
  const fileInputRef = useRef<HTMLInputElement | null>(null);

  const readImage = (file: File) => {
    if (!file.type.startsWith('image/')) return;

    const reader = new FileReader();
    reader.onload = () => {
      const dataUrl = reader.result as string;
      const img = new Image();
      img.onload = () => {
        onImageSelected({
          fileName: file.name,
          dataUrl,
          dimensions: `${img.naturalWidth} × ${img.naturalHeight}`,
          fileSize: formatFileSize(file.size),
        });
      };
      img.src = dataUrl;
    };
    reader.readAsDataURL(file);
  };

  const formatFileSize = (bytes: number) => {
    if (bytes < 1024) return `${bytes} B`;
    if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
    return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
  };

  const handleFileChange = (e: React.ChangeEvent<HTMLInputElement>) => {
    const file = e.target.files?.[0];
    if (file) readImage(file);
    e.target.value = '';
  };

  const handleDragOver = (e: React.DragEvent) => {
    e.preventDefault();
    setIsDragging(true);
  };

  const handleDragLeave = (e: React.DragEvent) => {
    e.preventDefault();
    setIsDragging(false);
  };

  const handleDrop = (e: React.DragEvent) => {
    e.preventDefault();
    setIsDragging(false);
    const file = e.dataTransfer.files?.[0];
    if (file) readImage(file);
  };

  return (
    <div className="workspace-grid animate-scroll-entry">
      <div className="investigation-main-area" style={{ display: 'flex', flexDirection: 'column', gap: 'var(--space-4)' }}>
        <header className="investigation-home-header">
          <h1 style={{ marginBottom: 'var(--space-1)' }}>New Investigation</h1>
          <p style={{ color: 'var(--color-text-secondary)', fontSize: 'var(--text-base)' }}>
            Start a new evidence verification session.
          </p>
        </header>

        <div
          className={`investigation-dropzone card ${isDragging ? 'is-dragging' : ''}`}
          onDragOver={handleDragOver}
          onDragLeave={handleDragLeave}
          onDrop={handleDrop}
          onClick={() => fileInputRef.current?.click()}
          role="button"
          tabIndex={0}
          aria-label="Drop an image to begin"
          onKeyDown={(e) => {
            if (e.key === 'Enter' || e.key === ' ') {
              e.preventDefault();
              fileInputRef.current?.click();
            }
          }}
          style={{
            flex: 1,
            minHeight: '280px',
            display: 'flex',
            flexDirection: 'column',
            alignItems: 'center',
            justifyContent: 'center',
            gap: 'var(--space-3)',
            cursor: 'pointer',
            transition: 'all var(--duration-normal)',
            backgroundColor: isDragging ? 'var(--color-surface-tertiary)' : 'var(--color-surface-secondary)',
          }}
        >
          <div style={{ padding: 'var(--space-4)', background: 'var(--color-surface-elevated)', borderRadius: '50%', marginBottom: 'var(--space-2)' }}>
            <ImagePlus size={32} strokeWidth={1.5} color="var(--color-text)" />
          </div>
          
          <div style={{ textAlign: 'center' }}>
            <h2 style={{ fontSize: 'var(--text-xl)', marginBottom: 'var(--space-1)' }}>
              {isDragging ? 'Drop to upload' : 'Drop an image here'}
            </h2>
            <p style={{ color: 'var(--color-text-secondary)' }}>or choose from your files</p>
          </div>

          <span className="btn btn-primary btn-lg" style={{ marginTop: 'var(--space-2)' }}>Choose Image</span>
          
          <span style={{ fontSize: 'var(--text-sm)', color: 'var(--color-text-tertiary)', marginTop: 'var(--space-2)' }}>
            PNG · JPEG · Maximum 10 MB
          </span>
          
          <input
            type="file"
            ref={fileInputRef}
            onChange={handleFileChange}
            accept="image/png,image/jpeg"
            style={{ display: 'none' }}
          />
        </div>
      </div>

      <aside className="investigation-sidebar" style={{ display: 'flex', flexDirection: 'column', gap: 'var(--space-4)', animationDelay: '80ms' }}>
        <section>
          <h2 style={{ fontSize: 'var(--text-base)', marginBottom: 'var(--space-3)', display: 'flex', alignItems: 'center', gap: 'var(--space-2)' }}>
            <Clock size={16} strokeWidth={2} />
            Recent Investigations
          </h2>
          <div className="card" style={{ padding: 'var(--space-3)' }}>
            <p style={{ color: 'var(--color-text-secondary)', fontSize: 'var(--text-sm)', textAlign: 'center', padding: 'var(--space-3) 0' }}>
              No investigations yet
            </p>
          </div>
        </section>


      </aside>
    </div>
  );
};