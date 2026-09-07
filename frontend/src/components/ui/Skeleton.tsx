import React from 'react';

export interface SkeletonProps extends React.HTMLAttributes<HTMLDivElement> {
  width?: string | number;
  height?: string | number;
  variant?: 'text' | 'circle' | 'rect';
  lines?: number;
}

export const Skeleton: React.FC<SkeletonProps> = ({
  width,
  height,
  variant = 'text',
  lines = 1,
  className = '',
  ...props
}) => {
  if (variant === 'text' && lines > 1) {
    return (
      <div className={`ui-skeleton-group ${className}`.trim()} {...props}>
        {Array.from({ length: lines }).map((_, i) => (
          <div
            key={i}
            className="ui-skeleton ui-skeleton-text"
            style={{
              width: i === lines - 1 ? '60%' : '100%',
            }}
          />
        ))}
      </div>
    );
  }

  const variantClass = {
    text: 'ui-skeleton-text',
    circle: 'ui-skeleton-circle',
    rect: 'ui-skeleton-rect',
  }[variant];

  return (
    <div
      className={`ui-skeleton ${variantClass} ${className}`.trim()}
      style={{ width, height }}
      {...props}
    />
  );
};
