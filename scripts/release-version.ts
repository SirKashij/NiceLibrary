// Checks a release tag against the version the manifests carry, so a release
// is never published under a tag that differs from the version inside the
// installer and the app.

/** "v" and a version: major.minor.patch, optionally a pre-release such as -beta.1. */
const TAG = /^v(\d+\.\d+\.\d+(?:-[0-9A-Za-z.-]*[0-9A-Za-z])?)$/;

/** What is wrong with releasing `tag`, given each manifest's path and version; empty if nothing is. */
export function releaseVersionErrors(tag: string, versions: Readonly<Record<string, string>>): string[] {
  const version = TAG.exec(tag)?.[1];
  if (version === undefined) return [`The tag ${tag} is not "v" followed by a version such as v0.1.0.`];
  return Object.entries(versions)
    .filter(([, found]) => found !== version)
    .map(([path, found]) => `${path} has version ${found}, but the tag is ${tag}.`);
}
