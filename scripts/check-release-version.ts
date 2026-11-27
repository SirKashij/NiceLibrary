// Fails if the release tag differs from the version in the manifests.
// Run from the repository root: node scripts/check-release-version.ts v0.1.0
import { readFileSync } from 'node:fs';
import { parse } from 'smol-toml';
import { releaseVersionErrors } from './release-version.ts';

const tag = process.argv[2];
if (!tag) {
  console.error('Usage: node scripts/check-release-version.ts <tag>');
  process.exit(1);
}

const versionIn = (path: string) => (JSON.parse(readFileSync(path, 'utf8')) as { version: string }).version;
const cargo = parse(readFileSync('Cargo.toml', 'utf8')) as { workspace: { package: { version: string } } };

const errors = releaseVersionErrors(tag, {
  'package.json': versionIn('package.json'),
  'src-tauri/tauri.conf.json': versionIn('src-tauri/tauri.conf.json'),
  'Cargo.toml': cargo.workspace.package.version,
});

for (const error of errors) console.error(`::error::${error}`);
if (errors.length > 0) process.exit(1);
console.log(`The tag ${tag} matches the version of every manifest.`);
