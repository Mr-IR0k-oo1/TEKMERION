import React from 'react';

export interface CardProps extends React.HTMLAttributes<HTMLDivElement> {
  padding?: boolean;
  hoverable?: boolean;
}

export const Card: React.FC<CardProps> = ({
  padding = true,
  hoverable = false,
  className = '',
  children,
  ...props
}) => {
  return (
    <div
      className={`ui-card ${padding ? 'ui-card-padded' : ''} ${hoverable ? 'ui-card-hoverable' : ''} ${className}`.trim()}
      {...props}
    >
      {children}
    </div>
  );
};
