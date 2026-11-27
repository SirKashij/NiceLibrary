// The theme of the interface. The colours of both themes live in src/styles/tokens.css; this
// module only says which one shows, through `data-theme` on the document element.
import { createContext, useContext, useEffect, useLayoutEffect, useRef, useState, type ReactNode } from 'react';
import type { Operations, Theme } from './operations';

/** A theme the token file defines. */
type ShownTheme = 'dark' | 'light';

/** The operating system's setting as the webview reports it. */
const SYSTEM_LIGHT = '(prefers-color-scheme: light)';

/** Dark or light, as the operating system is set to; dark where it cannot tell. */
function systemTheme(): ShownTheme {
  return typeof window.matchMedia === 'function' && window.matchMedia(SYSTEM_LIGHT).matches ? 'light' : 'dark';
}

/** The operating system's dark or light setting, following it while the app runs. */
function useSystemTheme(): ShownTheme {
  const [theme, setTheme] = useState(systemTheme);
  useEffect(() => {
    if (typeof window.matchMedia !== 'function') return;
    const query = window.matchMedia(SYSTEM_LIGHT);
    const changed = () => setTheme(systemTheme());
    query.addEventListener('change', changed);
    return () => query.removeEventListener('change', changed);
  }, []);
  return theme;
}

/** Shows the system's theme, without asking for the stored choice: for a start without a database. */
export function SystemTheme({ children }: { children: ReactNode }) {
  const shown = useSystemTheme();
  useLayoutEffect(() => {
    document.documentElement.dataset.theme = shown;
  }, [shown]);
  return <>{children}</>;
}

interface ThemeChoice {
  /** The theme the user chose; `system` until they choose. */
  theme: Theme;
  /** Shows `theme` at once and keeps it; answers whether it could be kept, and goes back if not. */
  choose(theme: Theme): Promise<boolean>;
}

const ThemeContext = createContext<ThemeChoice>({ theme: 'system', choose: async () => false });

/** Shows the theme the user chose, and the system's until the stored choice answers or when it cannot be read. */
export function ThemeProvider({ operations, children }: { operations: Operations; children: ReactNode }) {
  const system = useSystemTheme();
  const [theme, setTheme] = useState<Theme>('system');
  // Once the user chose here, the stored choice that answers late no longer counts.
  const chosenHere = useRef(false);
  // Only the answer to the latest choice counts; an earlier one may answer later.
  const latestChoice = useRef(0);

  useEffect(() => {
    let active = true;
    operations.theme().then(
      (stored) => active && !chosenHere.current && setTheme(stored),
      () => {},
    );
    return () => {
      active = false;
    };
  }, [operations]);

  const shown = theme === 'system' ? system : theme;
  // Before the first paint, so the interface never shows in the other theme first.
  useLayoutEffect(() => {
    document.documentElement.dataset.theme = shown;
  }, [shown]);

  const choose = (next: Theme) => {
    const id = ++latestChoice.current;
    const before = theme;
    chosenHere.current = true;
    setTheme(next);
    return operations.chooseTheme(next).then(
      () => true,
      () => {
        if (id === latestChoice.current) setTheme(before);
        return false;
      },
    );
  };

  return <ThemeContext.Provider value={{ theme, choose }}>{children}</ThemeContext.Provider>;
}

export function useTheme(): ThemeChoice {
  return useContext(ThemeContext);
}
