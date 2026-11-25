// The content security policy restricts what the interface may load (spec: "Outbound connections").
// Covers reach the interface through the desktop shell's `cover` URL scheme, never straight from the web.
import { readFileSync } from 'node:fs';
import { join } from 'node:path';
import { describe, expect, it } from 'vitest';

const config = JSON.parse(readFileSync(join(__dirname, '..', 'src-tauri', 'tauri.conf.json'), 'utf8'));

/** The sources the policy allows for one directive, e.g. `img-src`. */
function sources(directive: string): string[] {
  const policy: string = config.app.security.csp;
  const entry = policy
    .split(';')
    .map((part) => part.trim().split(/\s+/))
    .find(([name]) => name === directive);
  return entry ? entry.slice(1) : [];
}

/** The `cover` URL scheme: `cover://localhost/...`, on Windows `http://cover.localhost/...`. */
const COVER_SCHEME = ['cover:', 'http://cover.localhost'];
/** The app itself: its own files, images inlined into them, and the Covers the core serves. */
const THE_APP = ["'self'", 'data:', ...COVER_SCHEME];

describe('content security policy', () => {
  // The issue allows Steam's artwork hosts too; the interface needs none, because the core
  // downloads and caches Covers from them. The end-to-end test checks that direct loads fail.
  it('allows images only from the app itself', () => {
    expect(sources('img-src').filter((source) => !THE_APP.includes(source))).toEqual([]);
  });

  it('lets the interface load Covers through the cover URL scheme', () => {
    expect(sources('img-src')).toEqual(expect.arrayContaining(COVER_SCHEME));
  });

  it('allows nothing else from the web by default', () => {
    expect(sources('default-src')).toEqual(["'self'"]);
  });
});
