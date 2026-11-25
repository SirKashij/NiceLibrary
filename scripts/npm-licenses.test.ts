import { describe, expect, it } from 'vitest';
import { findViolations, isAllowed } from './npm-licenses.ts';

const allow = ['MIT', 'Apache-2.0', 'ISC'];

describe('isAllowed', () => {
  it('accepts a single allowed license and rejects one that is not listed', () => {
    expect(isAllowed('MIT', allow)).toBe(true);
    expect(isAllowed('GPL-3.0-only', allow)).toBe(false);
  });

  it('accepts a choice when at least one option is allowed', () => {
    expect(isAllowed('GPL-3.0-only OR MIT', allow)).toBe(true);
    expect(isAllowed('GPL-3.0-only OR LGPL-2.1-only', allow)).toBe(false);
  });

  it('requires every license of a combination to be allowed', () => {
    expect(isAllowed('MIT AND ISC', allow)).toBe(true);
    expect(isAllowed('MIT AND GPL-3.0-only', allow)).toBe(false);
  });

  it('respects parentheses and binds AND tighter than OR', () => {
    expect(isAllowed('(MIT OR GPL-3.0-only) AND ISC', allow)).toBe(true);
    expect(isAllowed('(MIT OR Apache-2.0) AND GPL-3.0-only', allow)).toBe(false);
    // Reads as MIT OR (ISC AND GPL-3.0-only).
    expect(isAllowed('MIT OR ISC AND GPL-3.0-only', allow)).toBe(true);
    expect(isAllowed('GPL-3.0-only AND ISC OR LGPL-2.1-only', allow)).toBe(false);
  });

  it('accepts a license with an exception only when that exact pair is listed', () => {
    const withException = [...allow, 'Apache-2.0 WITH LLVM-exception'];
    expect(isAllowed('Apache-2.0 WITH LLVM-exception', withException)).toBe(true);
    expect(isAllowed('MIT OR Apache-2.0 WITH LLVM-exception', allow)).toBe(true);
    expect(isAllowed('Apache-2.0 WITH LLVM-exception', allow)).toBe(false);
    expect(isAllowed('GPL-2.0-only WITH Classpath-exception-2.0', allow)).toBe(false);
  });

  it('matches license ids and operators regardless of case', () => {
    expect(isAllowed('mit', allow)).toBe(true);
    expect(isAllowed('GPL-3.0-only or apache-2.0', allow)).toBe(true);
  });

  it('rejects expressions it cannot read', () => {
    for (const unreadable of ['', 'MIT OR', '(MIT', 'MIT)', 'MIT ISC', 'AND MIT', 'SEE LICENSE IN LICENSE.md']) {
      expect(isAllowed(unreadable, allow), unreadable).toBe(false);
    }
  });
});

describe('findViolations', () => {
  it('names every package whose license is not allowed, with its version and license', () => {
    const lockfile = {
      packages: {
        '': { name: 'nicelibrary', license: 'MIT' },
        'node_modules/react': { version: '19.3.1', license: 'MIT' },
        'node_modules/copyleft': { version: '1.2.3', license: 'GPL-3.0-only' },
        'node_modules/react/node_modules/nested': { version: '0.1.0', license: 'AGPL-3.0-only' },
        'node_modules/@scope/dual': { version: '2.0.0', license: 'MIT OR GPL-2.0-only' },
      },
    };

    expect(findViolations(lockfile, allow)).toEqual([
      { name: 'copyleft', version: '1.2.3', license: 'GPL-3.0-only' },
      { name: 'nested', version: '0.1.0', license: 'AGPL-3.0-only' },
    ]);
  });

  it('reports a package that declares no license', () => {
    const lockfile = { packages: { 'node_modules/@scope/silent': { version: '1.0.0' } } };

    expect(findViolations(lockfile, allow)).toEqual([{ name: '@scope/silent', version: '1.0.0', license: undefined }]);
  });

  it('reports a package whose license is not an SPDX string, such as the legacy object form', () => {
    const lockfile = { packages: { 'node_modules/old': { version: '0.0.1', license: { type: 'MIT' } } } };

    expect(findViolations(lockfile, allow)).toEqual([{ name: 'old', version: '0.0.1', license: '{"type":"MIT"}' }]);
  });

  it('skips links to local workspace packages, which are checked at their own entry', () => {
    const lockfile = {
      packages: {
        'node_modules/local': { resolved: 'packages/local', link: true },
        'packages/local': { name: 'local', version: '0.0.1', license: 'MIT' },
      },
    };

    expect(findViolations(lockfile, allow)).toEqual([]);
  });
});
