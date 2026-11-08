import '@fontsource-variable/figtree';
import React from 'react';
import ReactDOM from 'react-dom/client';
import { App } from './App';
// Self-hosted so the page makes no third-party requests on load
import './index.css';

ReactDOM.createRoot(document.getElementById('root')!).render(
  <React.StrictMode>
    <App />
  </React.StrictMode>,
);
