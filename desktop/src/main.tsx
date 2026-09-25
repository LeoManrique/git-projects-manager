import React from 'react';
import ReactDOM from 'react-dom/client';
import App from './App';
import { describeError, log } from './lib/log';
import './index.css';

// Errors nothing caught would otherwise reach only the devtools console, which
// a release build does not open (FRONTEND.md §6.4).
window.addEventListener('error', (event) => {
  // The stack says where, which the message alone does not.
  const detail = event.error instanceof Error ? event.error.stack : undefined;
  log('error', `uncaught: ${detail ?? describeError(event.error ?? event.message)}`);
});
window.addEventListener('unhandledrejection', (event) => {
  log('error', `unhandled rejection: ${describeError(event.reason)}`);
});

ReactDOM.createRoot(document.getElementById('root')!).render(
  <React.StrictMode>
    <App />
  </React.StrictMode>,
);
