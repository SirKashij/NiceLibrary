import { createRoot } from 'react-dom/client';
import { App } from './App';
import '@fontsource-variable/bricolage-grotesque';
import '@fontsource-variable/instrument-sans';
import '@fontsource-variable/archivo/wdth.css';
import '@fontsource-variable/unbounded';
import '@fontsource-variable/manrope';
import './styles.css';
import './looks.css';

createRoot(document.getElementById('root')!).render(<App />);
