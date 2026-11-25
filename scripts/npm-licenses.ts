// Evaluates SPDX license expressions against the allow list (ADR-0004).
// Grammar: or := and ("OR" and)* ; and := term ("AND" term)* ;
//          term := "(" or ")" | id ("WITH" id)?
// Operators and ids are matched without regard to case. A license with an
// exception counts only if the allow list names that exact pair.

const OPERATORS = new Set(['and', 'or', 'with']);

/** Whether an SPDX expression permits use under the allowed licenses only. Unreadable expressions are not allowed. */
export function isAllowed(expression: string, allow: readonly string[]): boolean {
  const allowed = new Set(allow.map(normalize));
  const tokens = expression.match(/\(|\)|[^\s()]+/g) ?? [];
  let position = 0;

  const peek = (): string | undefined => tokens[position]?.toLowerCase();
  const next = (): string | undefined => tokens[position++];

  function expect(token: string): void {
    if (peek() !== token) throw new SyntaxError(`expected "${token}" in "${expression}"`);
    next();
  }

  function id(): string {
    const token = next();
    if (token === undefined || token === '(' || token === ')' || OPERATORS.has(token.toLowerCase())) {
      throw new SyntaxError(`expected a license id in "${expression}"`);
    }
    return token;
  }

  function parseOr(): boolean {
    let result = parseAnd();
    while (peek() === 'or') {
      next();
      const right = parseAnd();
      result = result || right;
    }
    return result;
  }

  function parseAnd(): boolean {
    let result = parseTerm();
    while (peek() === 'and') {
      next();
      const right = parseTerm();
      result = result && right;
    }
    return result;
  }

  function parseTerm(): boolean {
    if (peek() === '(') {
      next();
      const result = parseOr();
      expect(')');
      return result;
    }
    let license = id();
    if (peek() === 'with') {
      next();
      license = `${license} WITH ${id()}`;
    }
    return allowed.has(normalize(license));
  }

  try {
    const result = parseOr();
    if (position < tokens.length) throw new SyntaxError(`unexpected "${tokens[position]}" in "${expression}"`);
    return result;
  } catch (error) {
    if (error instanceof SyntaxError) return false;
    throw error;
  }
}

/** The parts of a `package-lock.json` (lockfile version 2 or 3) this check reads. */
export interface Lockfile {
  packages: Record<string, LockedPackage>;
}

interface LockedPackage {
  name?: string;
  version?: string;
  license?: string;
  link?: boolean;
}

export interface Violation {
  name: string;
  version: string | undefined;
  /** The declared SPDX expression, or `undefined` if the package declares none. */
  license: string | undefined;
}

/**
 * Lists every locked dependency, on every platform and including dev
 * dependencies, whose license is missing or not allowed. The project itself
 * (the "" entry) and links, which point at an entry of their own, are skipped.
 */
export function findViolations(lockfile: Lockfile, allow: readonly string[]): Violation[] {
  return Object.entries(lockfile.packages)
    .filter(([path, entry]) => path !== '' && !entry.link)
    .filter(([, entry]) => entry.license === undefined || !isAllowed(entry.license, allow))
    .map(([path, entry]) => ({
      name: entry.name ?? path.slice(path.lastIndexOf('node_modules/') + 'node_modules/'.length),
      version: entry.version,
      license: entry.license,
    }));
}

function normalize(license: string): string {
  return license.trim().replace(/\s+/g, ' ').toLowerCase();
}
