import { describe, expect, it } from 'vitest';
import { messages, systemLang, type Lang } from '.';

describe('translations', () => {
  it('ship every text in German and English', () => {
    const keys = (lang: Lang) => Object.keys(messages[lang]).sort();
    expect(keys('de')).toEqual(keys('en'));

    for (const lang of Object.keys(messages) as Lang[]) {
      for (const [key, text] of Object.entries(messages[lang])) {
        expect(text.trim(), `${lang}: ${key}`).not.toBe('');
      }
    }
  });

  it('follow the system language, with English for anything but German', () => {
    expect(systemLang('de-DE')).toBe('de');
    expect(systemLang('de-AT')).toBe('de');
    expect(systemLang('en-US')).toBe('en');
    expect(systemLang('fr-FR')).toBe('en');
  });
});
