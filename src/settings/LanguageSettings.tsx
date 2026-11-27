import { useState } from 'react';
import { useT, type MessageKey } from '../i18n';
import { useLanguage } from '../language';
import type { Language } from '../operations';

const LANGUAGES: { language: Language; label: MessageKey; lang?: string }[] = [
  { language: 'system', label: 'language.system' },
  // Each language by its own name, so it can be found whichever language shows.
  { language: 'de', label: 'language.de', lang: 'de' },
  { language: 'en', label: 'language.en', lang: 'en' },
];

/** Switching the interface between German and English, or following the system language. */
export function LanguageSettings() {
  const t = useT();
  const { language, choose } = useLanguage();
  const [failed, setFailed] = useState(false);

  const pick = (next: Language) => {
    setFailed(false);
    void choose(next).then((kept) => setFailed(!kept));
  };

  return (
    <section className="settings-section" aria-labelledby="settings-language">
      <h2 id="settings-language">{t('settings.language')}</h2>
      <div role="radiogroup" aria-labelledby="language-choice" className="language-choice">
        <p id="language-choice" className="muted">
          {t('language.choose')}
        </p>
        {LANGUAGES.map((option) => (
          <label key={option.language} lang={option.lang}>
            <input
              type="radio"
              name="language"
              value={option.language}
              checked={language === option.language}
              onChange={() => pick(option.language)}
            />
            {t(option.label)}
          </label>
        ))}
      </div>
      {failed && (
        <p className="note" role="alert">
          {t('language.saveFailed')}
        </p>
      )}
    </section>
  );
}
