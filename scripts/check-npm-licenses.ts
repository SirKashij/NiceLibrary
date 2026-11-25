// Fails if an npm dependency's license is not on the allow list in deny.toml,
// the one list that cargo-deny also uses for the Rust dependencies (ADR-0004).
// Run from the repository root: node scripts/check-npm-licenses.ts
import { readFileSync } from 'node:fs';
import { parse } from 'smol-toml';
import { findViolations, type Lockfile } from './npm-licenses.ts';

const config = parse(readFileSync('deny.toml', 'utf8')) as { licenses?: { allow?: string[] } };
const allow = config.licenses?.allow;
if (!allow?.length) {
  console.error('::error file=deny.toml::deny.toml has no [licenses] allow list.');
  process.exit(1);
}

const lockfile = JSON.parse(readFileSync('package-lock.json', 'utf8')) as Lockfile;
const violations = findViolations(lockfile, allow);

for (const { name, version, license } of violations) {
  const declared = license === undefined ? 'declares no license' : `is licensed under "${license}"`;
  console.error(`::error file=package-lock.json::npm package ${name}@${version ?? '?'} ${declared}, which is not allowed by deny.toml.`);
}

if (violations.length > 0) {
  console.error(`${violations.length} npm package(s) with a disallowed license. See ADR-0004.`);
  process.exit(1);
}
console.log('All npm packages have allowed licenses.');
