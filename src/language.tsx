// The interface language. The texts of both languages live in src/i18n; this module only says
// which one shows: the one the user chose, or the system language until they choose.
import { createContext, useContext, useEffect, useLayoutEffect, useRef, useState, type ReactNode } from 'react';
import { I18nProvider, type Lang } from './i18n';
import type { Language, Operations } from './operations';

interface LanguageChoice {
  /** The language the user chose; `system` until they choose. */
  language: Language;
  /** Shows `language` at once and keeps it; answers whether it could be kept, and goes back if not. */
  choose(language: Language): Promise<boolean>;
}

const LanguageContext = createContext<LanguageChoice>({ language: 'system', choose: async () => false });

/**
 * Shows the interface in the language the user chose, and in `system` (the system language) until the
 * stored choice answers or when it cannot be read. Texts, dates, numbers and durations all follow it.
 */
export function LanguageProvider({
  operations,
  system,
  children,
}: {
  operations: Operations;
  system: Lang;
  children: ReactNode;
}) {
  const [language, setLanguage] = useState<Language>('system');
  // Once the user chose here, the stored choice that answers late no longer counts.
  const chosenHere = useRef(false);
  // Only the answer to the latest choice counts; an earlier one may answer later.
  const latestChoice = useRef(0);

  useEffect(() => {
    let active = true;
    operations.language().then(
      (stored) => active && !chosenHere.current && setLanguage(stored),
      () => {},
    );
    return () => {
      active = false;
    };
  }, [operations]);

  const shown: Lang = language === 'system' ? system : language;
  // Before the first paint, so screen readers and spell checking hear the language that shows.
  useLayoutEffect(() => {
    document.documentElement.lang = shown;
  }, [shown]);

  const choose = (next: Language) => {
    const id = ++latestChoice.current;
    const before = language;
    chosenHere.current = true;
    setLanguage(next);
    return operations.chooseLanguage(next).then(
      () => true,
      () => {
        if (id === latestChoice.current) setLanguage(before);
        return false;
      },
    );
  };

  return (
    <LanguageContext.Provider value={{ language, choose }}>
      <I18nProvider lang={shown}>{children}</I18nProvider>
    </LanguageContext.Provider>
  );
}

export function useLanguage(): LanguageChoice {
  return useContext(LanguageContext);
}
