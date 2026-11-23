import React from 'react';
import ReactDOM from 'react-dom/client';
import { App, loadGuides } from './App';
// Self-hosted so the page makes no third-party requests on load
import './index.css';

function mount() {
  ReactDOM.createRoot(document.getElementById('root')!).render(
    <React.StrictMode>
      <App />
    </React.StrictMode>,
  );
}

if (window.location.pathname.startsWith('/guides')) {
  // Keep the pre-rendered guide visible while its interactive code downloads.
  void loadGuides().then(mount, error => {
    console.error('Could not load interactive guide features.', error);
  });
} else {
  mount();
}
