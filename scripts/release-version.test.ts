import { describe, expect, it } from 'vitest';
import { releaseVersionErrors } from './release-version.ts';

const files = (version: string) => ({
  'package.json': version,
  'src-tauri/tauri.conf.json': version,
  'Cargo.toml': version,
});

describe('releaseVersionErrors', () => {
  it('accepts a tag that names the version every manifest carries', () => {
    expect(releaseVersionErrors('v0.1.0', files('0.1.0'))).toEqual([]);
  });

  it('names each manifest whose version differs from the tag', () => {
    const versions = { ...files('0.1.0'), 'src-tauri/tauri.conf.json': '0.2.0' };
    expect(releaseVersionErrors('v0.1.0', versions)).toEqual([
      'src-tauri/tauri.conf.json has version 0.2.0, but the tag is v0.1.0.',
    ]);
  });

  it('refuses a tag that is not v followed by a version', () => {
    for (const tag of ['0.1.0', 'v0.1', 'release-1', 'v0.1.0-']) {
      expect(releaseVersionErrors(tag, files('0.1.0'))).toEqual([
        `The tag ${tag} is not "v" followed by a version such as v0.1.0.`,
      ]);
    }
  });

  it('accepts a pre-release tag when the manifests carry the same pre-release version', () => {
    expect(releaseVersionErrors('v0.2.0-beta.1', files('0.2.0-beta.1'))).toEqual([]);
  });
});
