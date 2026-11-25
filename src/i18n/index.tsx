// The translation system. Every visible text goes through `useT`.
import { createContext, useContext, type ReactNode } from 'react';
import { de } from './de';
import { en, type MessageKey } from './en';

export type Lang = 'de' | 'en';
export type { MessageKey };

export const messages: Record<Lang, Record<MessageKey, string>> = { de, en };

/** The interface language the system asks for, falling back to English. */
export function systemLang(locale: string = navigator.language): Lang {
  return locale.toLowerCase().startsWith('de') ? 'de' : 'en';
}

type Translate = (key: MessageKey) => string;

const I18nContext = createContext<Translate>((key) => en[key]);

export function I18nProvider({ lang, children }: { lang: Lang; children: ReactNode }) {
  const table = messages[lang];
  return <I18nContext.Provider value={(key) => table[key]}>{children}</I18nContext.Provider>;
}

export function useT(): Translate {
  return useContext(I18nContext);
}
