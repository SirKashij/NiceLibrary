import { StrictMode } from 'react';
import { createRoot } from 'react-dom/client';
// Both typefaces are bundled as files with the app, never loaded from a CDN (ADR-0007).
import '@fontsource-variable/bricolage-grotesque';
import '@fontsource-variable/instrument-sans';
import './styles/tokens.css';
import './styles/app.css';
import { App } from './App';
import { systemLang } from './i18n';
import { tauriOperations } from './operations';

const lang = systemLang();
document.documentElement.lang = lang;

createRoot(document.getElementById('root')!).render(
  <StrictMode>
    <App operations={tauriOperations} lang={lang} />
  </StrictMode>,
);
