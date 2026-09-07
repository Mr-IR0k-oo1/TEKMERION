import React, { useState, useRef, useCallback } from 'react';

export interface TooltipProps {
  content: React.ReactNode;
  position?: 'top' | 'bottom' | 'left' | 'right';
  children: React.ReactElement;
}

export const Tooltip: React.FC<TooltipProps> = ({
  content,
  position = 'top',
  children,
}) => {
  const [visible, setVisible] = useState(false);
  const timeoutRef = useRef<ReturnType<typeof setTimeout> | undefined>(undefined);

  const show = useCallback(() => {
    clearTimeout(timeoutRef.current);
    timeoutRef.current = setTimeout(() => setVisible(true), 400);
  }, []);

  const hide = useCallback(() => {
    clearTimeout(timeoutRef.current);
    timeoutRef.current = setTimeout(() => setVisible(false), 100);
  }, []);

  return (
    <span
      className="ui-tooltip-wrapper"
      onMouseEnter={show}
      onMouseLeave={hide}
      onFocus={show}
      onBlur={hide}
    >
      {children}
      {visible && (
        <span className={`ui-tooltip ui-tooltip-${position}`} role="tooltip">
          {content}
        </span>
      )}
    </span>
  );
};
