// The documents a public project needs (spec: "Contributors and maintainer", "Data, privacy
// and trust"): a README in English and German that carries the same notices as the About
// dialog, a CONTRIBUTING.md with the terms for contributions, a code of conduct and issue
// templates, none of them with personal data.
import { existsSync, readFileSync } from 'node:fs';
import { join } from 'node:path';
import { describe, expect, it } from 'vitest';
import { de } from '../src/i18n/de';
import { en } from '../src/i18n/en';

const root = join(__dirname, '..');
const read = (path: string) => readFileSync(join(root, path), 'utf8');

const readme = read('README.md');
const [english, german = ''] = readme.split(/^## Deutsch$/m);

/** The headings of one level in a part of the README, e.g. `##`. */
const headings = (text: string, level: string) =>
  text.split('\n').filter((line) => line.startsWith(`${level} `));

// The notices of the About dialog (`about.*` in src/i18n), which the README repeats word for word.
const NOTICE_KEYS = [
  'about.privacyConnections',
  'about.privacyStorage',
  'about.privacyNothingElse',
  'about.trademarkNotice',
  'about.notAffiliated',
] as const;
const NOTICES = {
  english: NOTICE_KEYS.map((key) => en[key]),
  german: NOTICE_KEYS.map((key) => de[key]),
};

describe('README', () => {
  it('has an English part first and a German part second', () => {
    expect(german).not.toBe('');
    expect(english).toContain('[Deutsch](#deutsch)');
  });

  it('gives the German part a section for each English one', () => {
    const englishSections = headings(english, '##');
    const germanSections = headings(german, '###');
    expect(englishSections.length).toBeGreaterThan(0);
    expect(germanSections).toHaveLength(englishSections.length);
  });

  it('carries the privacy notice, the Valve trademark notice and the non-affiliation statement in English', () => {
    for (const notice of NOTICES.english) expect(english).toContain(notice);
  });

  it('carries the same notices in German', () => {
    for (const notice of NOTICES.german) expect(german).toContain(notice);
  });

  it('explains the Windows SmartScreen warning in both languages', () => {
    expect(english).toContain('SmartScreen');
    expect(german).toContain('SmartScreen');
  });

  it('shows no image from the web, so no game artwork reaches it', () => {
    const images = [...readme.matchAll(/!\[[^\]]*\]\(([^)]+)\)|<img[^>]+src="([^"]+)"/g)].map((m) => m[1] ?? m[2]);
    expect(images.filter((src) => /^[a-z]+:/i.test(src))).toEqual([]);
  });
});

describe('contributing', () => {
  it('accepts contributions under MIT without a contributor license agreement', () => {
    const contributing = read('CONTRIBUTING.md');
    expect(contributing).toMatch(/MIT/);
    expect(contributing).toMatch(/no contributor license agreement/i);
  });

  it('has a code of conduct and issue templates for bugs and feature requests', () => {
    for (const path of [
      'CODE_OF_CONDUCT.md',
      '.github/ISSUE_TEMPLATE/bug_report.yml',
      '.github/ISSUE_TEMPLATE/feature_request.yml',
    ]) {
      expect(existsSync(join(root, path)), path).toBe(true);
    }
  });
});

describe('public documents', () => {
  const documents = [
    'README.md',
    'CONTRIBUTING.md',
    'CODE_OF_CONDUCT.md',
    'SECURITY.md',
    '.github/ISSUE_TEMPLATE/bug_report.yml',
    '.github/ISSUE_TEMPLATE/feature_request.yml',
    '.github/ISSUE_TEMPLATE/config.yml',
  ].filter((path) => existsSync(join(root, path)));

  it.each(documents)('%s names no email address but GitHub noreply ones', (path) => {
    const emails = read(path).match(/[\w.+-]+@[\w-]+(\.[\w-]+)+/g) ?? [];
    expect(emails.filter((email) => !email.endsWith('@users.noreply.github.com'))).toEqual([]);
  });

  it.each(documents)('%s names no path inside a Windows user folder', (path) => {
    expect(read(path)).not.toMatch(/[A-Z]:[\\/]Users[\\/]/i);
  });
});
