import React, { useEffect, useState } from 'react';

export interface ToastProps {
  message: string;
  variant?: 'info' | 'success' | 'warning' | 'error';
  duration?: number;
  onClose: () => void;
}

export const Toast: React.FC<ToastProps> = ({
  message,
  variant = 'info',
  duration = 4000,
  onClose,
}) => {
  const [visible, setVisible] = useState(false);

  useEffect(() => {
    requestAnimationFrame(() => setVisible(true));
    const timer = setTimeout(() => {
      setVisible(false);
      setTimeout(onClose, 200);
    }, duration);
    return () => clearTimeout(timer);
  }, [duration, onClose]);

  const variantClass = {
    info: 'ui-toast-info',
    success: 'ui-toast-success',
    warning: 'ui-toast-warning',
    error: 'ui-toast-error',
  }[variant];

  return (
    <div className={`ui-toast ${variantClass} ${visible ? 'ui-toast-visible' : ''}`} role="alert">
      <span className="ui-toast-message">{message}</span>
      <button className="ui-toast-close" onClick={() => { setVisible(false); setTimeout(onClose, 200); }} aria-label="Dismiss">
        <svg width="12" height="12" viewBox="0 0 12 12" fill="none">
          <path d="M1 1L11 11M11 1L1 11" stroke="currentColor" strokeWidth="1.5" strokeLinecap="round" />
        </svg>
      </button>
    </div>
  );
};
