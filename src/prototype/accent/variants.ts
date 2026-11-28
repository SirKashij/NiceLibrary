// PROTOTYPE, throwaway: each variant is a CSS layer over tokens.css and app.css. The real app is untouched.

const mark = (stroke: string) =>
  `url("data:image/svg+xml,${encodeURIComponent(
    `<svg xmlns="http://www.w3.org/2000/svg" viewBox="200 160 620 700"><defs><linearGradient id="g" x1="0" y1="0" x2="1" y2="1"><stop offset="0" stop-color="#e4ff7a"/><stop offset="1" stop-color="#7fd66b"/></linearGradient></defs><path d="M290 770 V520 A135 135 0 0 1 560 520 V770 M730 250 V770" fill="none" stroke="${stroke}" stroke-width="124" stroke-linecap="round" stroke-linejoin="round"/></svg>`,
  )}")`;
const plateMark = `url("data:image/svg+xml,${encodeURIComponent(
  `<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 1024 1024"><defs><linearGradient id="g" x1="0" y1="0" x2="1" y2="1"><stop offset="0" stop-color="#e4ff7a"/><stop offset="1" stop-color="#7fd66b"/></linearGradient></defs><rect width="1024" height="1024" rx="224" fill="#121314"/><path d="M290 770 V520 A135 135 0 0 1 560 520 V770 M730 250 V770" fill="none" stroke="url(#g)" stroke-width="124" stroke-linecap="round" stroke-linejoin="round"/></svg>`,
)}")`;

// Lime roles: a fill (behind dark ink) and a line/text colour that holds contrast on the theme's background.
const BASE = `
:root { --lime-fill: #c8f06a; --lime-ink: #121314; --lime-line: #c8f06a; --lime-soft: rgb(200 240 106 / 0.10); }
:root[data-theme='light'] { --lime-fill: #b9e85a; --lime-ink: #1a1b1c; --lime-line: #3f6b0c; --lime-soft: rgb(63 107 12 / 0.10); }
.brand-mark { width: 18px; height: 20px; border-radius: 0; background: center / contain no-repeat ${mark('url(#g)')}; }
:root[data-theme='light'] .brand-mark { background-image: ${mark('#3f6b0c')}; }
`;

// Variant E as roles: what uses fill follows --color-accent, what uses line follows --color-accent-line.
const E_ROLES = `
:root, :root[data-theme='light'] { --color-accent: var(--lime-fill); --color-accent-ink: var(--lime-ink); --color-accent-line: var(--lime-line); }
.brand-mark, :root[data-theme='light'] .brand-mark { width: 22px; height: 22px; background-image: ${plateMark}; }
.sidebar { background: linear-gradient(180deg, var(--lime-soft), transparent 55%); }
.sidebar nav button[aria-current='page'] { color: var(--lime-ink); background: var(--lime-fill); border-radius: 999px; box-shadow: none; }
.page h1 { background: linear-gradient(120deg, #e4ff7a, #7fd66b); -webkit-background-clip: text; background-clip: text; color: transparent; }
:root[data-theme='light'] .page h1 { background-image: linear-gradient(120deg, #4f8a10, #2f6b2a); }
.game-detail h1 { background: none; color: var(--color-text); }
.game:hover .cover { outline: 2px solid var(--lime-fill); outline-offset: 2px; }
:root[data-theme='light'] .game:hover .cover { outline-color: var(--lime-line); }
.segmented button[aria-pressed='true'] { color: var(--color-accent-ink); background: var(--color-accent); }
button:focus-visible, input:focus-visible { outline-color: var(--color-accent-line); }
input[type='checkbox'], input[type='radio'], .toolbar-toggle input { accent-color: var(--color-accent-line); }
.button:hover { border-color: var(--color-accent-line); }
`;

export interface Variant {
  key: string;
  name: string;
  where: string;
  css: string;
}

export const VARIANTS: Variant[] = [
  {
    key: 'A',
    name: 'Nur das Logo',
    where: 'nur im Monogramm neben „NiceLibrary“. Alles andere bleibt Graphit; Farbe kommt weiter nur von den Covern.',
    css: BASE,
  },
  {
    key: 'B',
    name: 'Aktionen',
    where: 'auf allem, was etwas tut: „Spielen“ auf dem Cover, Fokusring, Checkboxen und Radios. Navigation und Daten bleiben neutral. Die Detailseite behält ihre Cover-Tönung.',
    css: `${BASE}
:root, :root[data-theme='light'] { --color-accent: var(--lime-fill); --color-accent-ink: var(--lime-ink); }
button:focus-visible, input:focus-visible { outline-color: var(--lime-line); }
input[type='checkbox'], input[type='radio'], .toolbar-toggle input { accent-color: var(--lime-line); }
`,
  },
  {
    key: 'C',
    name: 'Orientierung',
    where: 'dort, wo du gerade bist: aktiver Menüpunkt mit Balken, gewählte Sammlung, gedrückte Filter-Segmente, die aktuelle Woche in der Statistik. Buttons bleiben weiß/schwarz.',
    css: `${BASE}
.sidebar nav button[aria-current='page'] { color: var(--lime-line); box-shadow: inset 3px 0 0 var(--lime-line); }
.collection-name[aria-current='page'] { color: var(--lime-line); }
.segmented button[aria-pressed='true'] { color: var(--lime-line); box-shadow: inset 0 -2px 0 var(--lime-line); }
.week-chart li[aria-current='date'] .week-bar { background: var(--lime-line); }
.week-chart li[aria-current='date'] .week-label { color: var(--lime-line); }
`,
  },
  {
    key: 'D',
    name: 'Fortschritt',
    where: 'nur in deinen Spieldaten: Fortschritt unter den Covern, 100 % (ersetzt das bisherige Grün), Status „Spiele ich“, alle Balken der Statistik. Lime heißt hier: „das hast du gespielt“.',
    css: `${BASE}
:root, :root[data-theme='light'] { --color-complete: var(--lime-line); }
.game-progress { color: var(--lime-line); opacity: 0.75; }
.game-progress[data-complete='true'], .progress-value[data-complete='true'] { opacity: 1; font-weight: 600; }
.status-badge[data-status='playing'] { color: var(--lime-line); }
.week-chart .week-bar { background: var(--lime-line); opacity: 0.45; }
.week-chart li[aria-current='date'] .week-bar { opacity: 1; }
.most-played-share > span { background: var(--lime-line); }
`,
  },
  {
    key: 'E',
    name: 'Lime-Bühne',
    where: 'kräftig und überall: Seitenleiste leicht lime getönt, aktiver Menüpunkt als gefüllte Pille, Seitentitel im Logo-Verlauf, Logo mit Platte, Cover-Hover mit Rahmen, „Spielen“ in Lime, Fokusring.',
    css: `${BASE}
:root, :root[data-theme='light'] { --color-accent: var(--lime-fill); --color-accent-ink: var(--lime-ink); }
.brand-mark, :root[data-theme='light'] .brand-mark { width: 22px; height: 22px; background-image: ${plateMark}; }
.sidebar { background: linear-gradient(180deg, var(--lime-soft), transparent 55%); }
.sidebar nav button[aria-current='page'] { color: var(--lime-ink); background: var(--lime-fill); border-radius: 999px; box-shadow: none; }
.page h1 { background: linear-gradient(120deg, #e4ff7a, #7fd66b); -webkit-background-clip: text; background-clip: text; color: transparent; }
:root[data-theme='light'] .page h1 { background-image: linear-gradient(120deg, #4f8a10, #2f6b2a); }
.game:hover .cover { outline: 2px solid var(--lime-fill); outline-offset: 2px; }
.segmented button[aria-pressed='true'] { color: var(--lime-ink); background: var(--lime-fill); }
button:focus-visible, input:focus-visible { outline-color: var(--lime-line); }
input[type='checkbox'], input[type='radio'], .toolbar-toggle input { accent-color: var(--lime-line); }
.button:hover { border-color: var(--lime-line); }
`,
  },
  // #94 triage: E with the accent split into roles, to judge the detail page. Fill and ink: --color-accent(-ink),
  // which the Cover tint already overrides inline. Line: --color-accent-line, lime by default.
  {
    key: 'E-A',
    name: 'E, Detailseite ganz vom Cover',
    where: 'wie E. Auf der Detailseite eines Spiels kommen ALLE Akzentrollen vom Cover: „Spielen“, gedrücktes Status-Segment, Fokusring (Tab drücken), Sammlungs-Checkboxen, Button-Hover. Kein Lime im Inhalt der Detailseite.',
    css: `${BASE}${E_ROLES}
.game-detail[data-tint='cover'] { --color-accent-line: var(--color-accent); }
`,
  },
  {
    key: 'E-B',
    name: 'E, Detailseite: Linien bleiben Lime',
    where: 'wie E. Auf der Detailseite kommen „Spielen“ und das gedrückte Status-Segment vom Cover; Fokusring (Tab drücken), Sammlungs-Checkboxen und Button-Hover bleiben Lime.',
    css: `${BASE}${E_ROLES}`,
  },
];
