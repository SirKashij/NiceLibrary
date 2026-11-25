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

/** Values for the `{name}` placeholders in a text. */
type Params = Record<string, string>;

type Translate = (key: MessageKey, params?: Params) => string;

/** `text` with each `{name}` replaced by `params[name]`. */
function fill(text: string, params: Params = {}): string {
  return text.replace(/\{(\w+)\}/g, (placeholder, name: string) => params[name] ?? placeholder);
}

const I18nContext = createContext<Translate>((key, params) => fill(en[key], params));

export function I18nProvider({ lang, children }: { lang: Lang; children: ReactNode }) {
  const table = messages[lang];
  return <I18nContext.Provider value={(key, params) => fill(table[key], params)}>{children}</I18nContext.Provider>;
}

export function useT(): Translate {
  return useContext(I18nContext);
}
