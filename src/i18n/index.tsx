// The translation system. Every visible text goes through `useT`.
import { createContext, useContext, useMemo, type ReactNode } from 'react';
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
const LangContext = createContext<Lang>('en');

export function I18nProvider({ lang, children }: { lang: Lang; children: ReactNode }) {
  const table = messages[lang];
  return (
    <LangContext.Provider value={lang}>
      <I18nContext.Provider value={(key, params) => fill(table[key], params)}>{children}</I18nContext.Provider>
    </LangContext.Provider>
  );
}

export function useT(): Translate {
  return useContext(I18nContext);
}

/** Durations and times written the way the interface language writes them, in the computer's time zone. */
export interface Format {
  /** A Playtime: minutes below an hour, hours with one decimal above. */
  playtime(minutes: number): string;
  /** The day of a moment given in seconds since the Unix epoch. */
  date(seconds: number): string;
  /** The day and time of a moment given in seconds since the Unix epoch. */
  dateTime(seconds: number): string;
  /** A whole number, with the interface language's digit grouping. */
  count(n: number): string;
  /** A percent given from 0 to 100, with one decimal and the interface language's percent sign. */
  percent(percent: number): string;
  /** A whole percent from 0 to 100, such as a Progress, with the interface language's percent sign. */
  wholePercent(percent: number): string;
}

export function useFormat(): Format {
  const lang = useContext(LangContext);
  const t = useT();
  const { number, count, percent, wholePercent, date, dateTime } = useMemo(
    () => ({
      number: new Intl.NumberFormat(lang, { maximumFractionDigits: 1 }),
      count: new Intl.NumberFormat(lang, { maximumFractionDigits: 0 }),
      percent: new Intl.NumberFormat(lang, { style: 'percent', minimumFractionDigits: 1, maximumFractionDigits: 1 }),
      wholePercent: new Intl.NumberFormat(lang, { style: 'percent', maximumFractionDigits: 0 }),
      date: new Intl.DateTimeFormat(lang, { dateStyle: 'medium' }),
      dateTime: new Intl.DateTimeFormat(lang, { dateStyle: 'medium', timeStyle: 'short' }),
    }),
    [lang],
  );
  return {
    playtime: (minutes) =>
      minutes < 60
        ? t('format.minutes', { n: number.format(minutes) })
        : t('format.hours', { n: number.format(minutes / 60) }),
    date: (seconds) => date.format(seconds * 1000),
    dateTime: (seconds) => dateTime.format(seconds * 1000),
    count: (n) => count.format(n),
    percent: (n) => percent.format(n / 100),
    wholePercent: (n) => wholePercent.format(n / 100),
  };
}
