import React from 'react';
import ReactDOM from 'react-dom/client';
import App from './App';
import './index.css';
import './styles/pipeline.css';
import './styles/merkle.css';
import './styles/tamper.css';
import './styles/inspector.css';
import './styles/audit.css';

ReactDOM.createRoot(document.getElementById('root')!).render(
  <React.StrictMode>
    <App />
  </React.StrictMode>
);
