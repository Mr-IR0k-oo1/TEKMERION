import React from 'react';

export interface PanelProps extends React.HTMLAttributes<HTMLDivElement> {
  title?: string;
  subtitle?: string;
  headerRight?: React.ReactNode;
}

export const Panel: React.FC<PanelProps> = ({
  title,
  subtitle,
  headerRight,
  className = '',
  children,
  ...props
}) => {
  return (
    <div className={`ui-panel ${className}`.trim()} {...props}>
      {(title || headerRight) && (
        <div className="ui-panel-header">
          <div>
            {title && <h3 className="ui-panel-title">{title}</h3>}
            {subtitle && <p className="ui-panel-subtitle">{subtitle}</p>}
          </div>
          {headerRight && <div className="ui-panel-header-right">{headerRight}</div>}
        </div>
      )}
      <div className="ui-panel-body">{children}</div>
    </div>
  );
};
