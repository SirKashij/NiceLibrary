// PROTOTYPE - floating bar to flip between variants. Not part of the design being judged.
import { useEffect } from 'react';

interface Props {
  variants: { key: string; name: string }[];
  current: string;
  onChange: (key: string) => void;
  state: string;
  theme: string;
  lang: string;
  onTheme: () => void;
  onLang: () => void;
}

export function PrototypeSwitcher({ variants, current, onChange, state, theme, lang, onTheme, onLang }: Props) {
  const i = variants.findIndex((v) => v.key === current);
  const step = (d: number) => onChange(variants[(i + d + variants.length) % variants.length].key);

  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      const el = document.activeElement as HTMLElement | null;
      if (el && (el.tagName === 'INPUT' || el.tagName === 'TEXTAREA' || el.tagName === 'SELECT' || el.isContentEditable)) return;
      if (e.key === 'ArrowLeft') step(-1);
      if (e.key === 'ArrowRight') step(1);
    };
    window.addEventListener('keydown', onKey);
    return () => window.removeEventListener('keydown', onKey);
  });

  return (
    <div className="proto-bar">
      <button onClick={() => step(-1)} aria-label="Vorherige Variante">←</button>
      <span className="proto-label">
        <b>{current}</b> {variants[i].name}
        <small>{state}</small>
      </span>
      <button onClick={() => step(1)} aria-label="Nächste Variante">→</button>
      <i />
      <button onClick={onTheme}>{theme === 'dark' ? '☾ dark' : '☀ light'}</button>
      <button onClick={onLang}>{lang.toUpperCase()}</button>
    </div>
  );
}
