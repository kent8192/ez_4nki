import React from 'react';
import ReactDOM from 'react-dom/client';
import { invoke } from '@tauri-apps/api/core';
import App from './App';
import type { Transport } from './types';

const transport: Transport = {
  command: (request) => invoke('command', { request }),
  pickCsv: (encoding) => invoke('pick_csv', { encoding }),
  exportBackup: (passphrase) => invoke('export_backup', { passphrase }),
  previewRestore: (passphrase) => invoke('preview_restore', { passphrase }),
};
ReactDOM.createRoot(document.getElementById('root')!).render(
  <React.StrictMode>
    <App transport={transport} />
  </React.StrictMode>,
);
