import { useState } from 'react';
import { useT, type MessageKey } from '../i18n';
import type { Theme } from '../operations';
import { useTheme } from '../theme';

const THEMES: { theme: Theme; label: MessageKey }[] = [
  { theme: 'system', label: 'theme.system' },
  { theme: 'dark', label: 'theme.dark' },
  { theme: 'light', label: 'theme.light' },
];

/** Switching between the dark and the light theme, or following the system. */
export function ThemeSettings() {
  const t = useT();
  const { theme, choose } = useTheme();
  const [failed, setFailed] = useState(false);

  const pick = (next: Theme) => {
    setFailed(false);
    void choose(next).then((kept) => setFailed(!kept));
  };

  return (
    <section className="settings-section" aria-labelledby="settings-theme">
      <h2 id="settings-theme">{t('settings.theme')}</h2>
      <div role="radiogroup" aria-labelledby="theme-choice" className="theme-choice">
        <p id="theme-choice" className="muted">
          {t('theme.choose')}
        </p>
        {THEMES.map((option) => (
          <label key={option.theme}>
            <input
              type="radio"
              name="theme"
              value={option.theme}
              checked={theme === option.theme}
              onChange={() => pick(option.theme)}
            />
            {t(option.label)}
          </label>
        ))}
      </div>
      {failed && (
        <p className="note" role="alert">
          {t('theme.saveFailed')}
        </p>
      )}
    </section>
  );
}
