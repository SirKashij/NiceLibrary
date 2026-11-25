// PROTOTYPE - Variant A: classic launcher shell. Fixed sidebar with navigation and collections,
// toolbar on top, cover grid as the main surface, game detail as its own full page.
import { useState } from 'react';
import { COLLECTIONS, computeStats, percent } from '../data';
import {
  AchievementList, Bar, Cover, Icon, PlayButton, SessionList, SettingsSections, SORTS, STATUSES, SourceTag, StatusBadge,
  VProps, WeekChart, fmtNum, fmtRel, fmtTime, useLibrary,
} from '../shared';

export function VariantA(p: VProps) {
  const { games, view, go, gameId, openGame, t, lang, launch, look } = p;
  const lib = useLibrary(games);
  const game = games.find((g) => g.id === gameId) ?? null;
  const nav = (v: typeof view) => { openGame(null); go(v); };
  // Look 3 (Konsole): a selection frame that travels to the hovered or focused cover.
  const [cur, setCur] = useState<{ x: number; y: number; w: number; h: number } | null>(null);
  const moveCursor = (card: HTMLElement) => {
    const cover = card.querySelector('.cover') as HTMLElement;
    setCur({ x: card.offsetLeft + cover.offsetLeft - 7, y: card.offsetTop + cover.offsetTop - 7, w: cover.offsetWidth + 14, h: cover.offsetHeight + 14 });
  };
  const tinted = view === 'library' && game;

  return (
    <div className="a-root">
      <aside className="a-side">
        <div className="a-brand"><span className="logo" />NiceLibrary</div>
        <nav>
          {(['library', 'stats', 'settings'] as const).map((v) => (
            <button key={v} className={view === v ? 'on' : ''} onClick={() => nav(v)}>
              <Icon name={v} /> {t(v)}
            </button>
          ))}
        </nav>
        <h4>{t('collections')}</h4>
        <nav className="a-coll">
          <button className={lib.onlyFav ? 'on' : ''} onClick={() => { nav('library'); lib.setCollection(null); lib.setOnlyFav(!lib.onlyFav); }}>
            <Icon name="star" size={16} /> {t('favorites')} <em>{games.filter((g) => g.favorite).length}</em>
          </button>
          {COLLECTIONS.map((c) => (
            <button key={c} className={lib.collection === c ? 'on' : ''} onClick={() => { nav('library'); lib.setOnlyFav(false); lib.setCollection(lib.collection === c ? null : c); }}>
              <span className="dot" /> {c} <em>{games.filter((g) => g.collections.includes(c)).length}</em>
            </button>
          ))}
        </nav>
        <div className="a-sync"><Icon name="sync" size={14} /> {t('lastSync')}</div>
      </aside>

      <main className={`a-main ${tinted ? 'tinted' : ''}`} style={tinted ? ({ '--gh': game.hue } as any) : undefined}>
        {view === 'library' && !game && (
          <>
            <div className="a-toolbar">
              <label className="searchbox">
                <Icon name="search" size={16} />
                <input value={lib.query} onChange={(e) => lib.setQuery(e.target.value)} placeholder={t('search')} />
              </label>
              <span className="seg">
                {(['all', 'steam', 'local'] as const).map((s) => (
                  <button key={s} className={lib.source === s ? 'on' : ''} onClick={() => lib.setSource(s)}>{t(s)}</button>
                ))}
              </span>
              <select value={lib.status} onChange={(e) => lib.setStatus(e.target.value as any)} aria-label={t('status')}>
                <option value="all">{t('status')}: {t('all')}</option>
                {STATUSES.map((s) => <option key={s} value={s}>{t(s)}</option>)}
              </select>
              <label className="check"><input type="checkbox" checked={lib.onlyInstalled} onChange={(e) => lib.setOnlyInstalled(e.target.checked)} /> {t('onlyInstalled')}</label>
              <span className="grow" />
              <select value={lib.sort} onChange={(e) => lib.setSort(e.target.value as any)} aria-label={t('sort')}>
                {SORTS.map(([k, l]) => <option key={k} value={k}>{t(l)}</option>)}
              </select>
              <span className="seg icon">
                <button className={lib.mode === 'grid' ? 'on' : ''} onClick={() => lib.setMode('grid')} aria-label={t('grid')}><Icon name="grid" size={16} /></button>
                <button className={lib.mode === 'list' ? 'on' : ''} onClick={() => lib.setMode('list')} aria-label={t('list')}><Icon name="list" size={16} /></button>
              </span>
            </div>
            <div className="a-head">
              <h1>{lib.collection ?? (lib.onlyFav ? t('favorites') : t('library'))}</h1>
              <span>{lib.result.length} {t('games')}</span>
              <span className="grow" />
              <button className="btn"><Icon name="plus" size={15} /> {t('addGame')}</button>
            </div>
            {lib.result.length === 0 && (
              <div className="empty-state"><p>{t('noResults')}</p><button className="btn" onClick={lib.reset}>{t('resetFilters')}</button></div>
            )}
            {lib.mode === 'grid' ? (
              <div className="a-grid" onMouseLeave={() => setCur(null)}>
                {lib.result.map((g) => (
                  <button key={g.id} className={`a-card ${g.installed ? '' : 'dim'}`} onClick={() => openGame(g.id)} onMouseEnter={(e) => moveCursor(e.currentTarget)} onFocus={(e) => moveCursor(e.currentTarget)}>
                    <Cover game={g} />
                    <span className={`sticker st-${g.status}`}>{t(g.status)}</span>
                    <i className="board" />
                    {g.favorite && <span className="fav"><Icon name="star" size={14} fill /></span>}
                    <b>{g.title}</b>
                    <small className="a-meta"><span>{fmtTime(g.minutes, lang)}</span><span>{percent(g) === null ? '–' : `${percent(g)} %`}</span></small>
                    <Bar value={percent(g)} />
                  </button>
                ))}
                <span className="k-cursor" style={cur ? { width: cur.w, height: cur.h, transform: `translate(${cur.x}px, ${cur.y}px)` } : { opacity: 0 }} />
              </div>
            ) : look === '2' ? (
              <div className="shelf">
                {lib.result.map((g) => (
                  <button
                    key={g.id}
                    className={`spine ${g.installed ? '' : 'dim'}`}
                    title={`${g.title} – ${fmtTime(g.minutes, lang)}`}
                    onClick={() => openGame(g.id)}
                    style={{ width: g.minutes > 1500 ? 62 : 48, height: 200 + (g.hue % 56), background: `linear-gradient(90deg, hsl(${g.hue} 55% 27%), hsl(${g.hue} 60% 42%) 35%, hsl(${(g.hue + 30) % 360} 55% 24%))` }}
                  >
                    <span>{g.title}</span>
                    <em>{percent(g) === null ? '–' : `${percent(g)}%`}</em>
                  </button>
                ))}
                <i className="shelf-end" />
              </div>
            ) : (
              <table className="a-table">
                <thead><tr><th>{t('game')}</th><th>{t('source')}</th><th>{t('status')}</th><th>{t('playtime')}</th><th>{t('lastPlayed')}</th><th>{t('progress')}</th><th /></tr></thead>
                <tbody>
                  {lib.result.map((g) => (
                    <tr key={g.id} onClick={() => openGame(g.id)} className={g.installed ? '' : 'dim'}>
                      <td><span className="a-rowtitle"><Cover game={g} className="thumb" /><b>{g.title}</b></span></td>
                      <td><SourceTag source={g.source} t={t} /></td>
                      <td><StatusBadge status={g.status} t={t} /></td>
                      <td>{fmtTime(g.minutes, lang)}</td>
                      <td>{fmtRel(g.lastPlayed, lang, t)}</td>
                      <td className="a-prog"><Bar value={percent(g)} /><span>{percent(g) === null ? '–' : `${percent(g)} %`}</span></td>
                      <td><PlayButton game={g} t={t} launch={launch} /></td>
                    </tr>
                  ))}
                </tbody>
              </table>
            )}
          </>
        )}

        {view === 'library' && game && (
          <div className="a-detail">
            <button className="btn ghost" onClick={() => openGame(null)}><Icon name="back" size={16} /> {t('library')}</button>
            <header>
              <Cover game={game} />
              <div>
                <div className="a-tags"><SourceTag source={game.source} t={t} /><StatusBadge status={game.status} t={t} /><span className="tag">{t(game.installed ? 'installed' : 'notInstalled')}</span></div>
                <h1>{game.title}</h1>
                <PlayButton game={game} t={t} launch={launch} big />
                <div className="a-tiles">
                  <div><small>{t('playtime')}</small><b>{fmtTime(game.minutes, lang)}</b></div>
                  <div><small>{t('lastPlayed')}</small><b>{fmtRel(game.lastPlayed, lang, t)}</b></div>
                  <div><small>{t('progress')}</small><b>{percent(game) === null ? '–' : `${percent(game)} %`}</b><Bar value={percent(game)} /></div>
                  <div><small>{t('achievements')}</small><b>{game.achievements.filter((a) => a.unlocked).length} / {game.achievements.length}</b></div>
                </div>
              </div>
            </header>
            <div className="a-cols">
              <section><h2>{t('achievements')}</h2><AchievementList game={game} t={t} lang={lang} /></section>
              <section><h2>{t('sessions')}</h2><SessionList game={game} t={t} lang={lang} /></section>
            </div>
          </div>
        )}

        {view === 'stats' && <StatsA {...p} />}

        {view === 'settings' && (
          <div className="a-settings">
            <h1>{t('settings')}</h1>
            <SettingsSections {...p} />
          </div>
        )}
      </main>
    </div>
  );
}

function StatsA({ games, t, lang, openGame, go }: VProps) {
  const s = computeStats(games);
  const maxTop = s.top[0].minutes;
  return (
    <div className="a-stats">
      <h1>{t('stats')}</h1>
      <div className="a-tiles wide">
        <div><small>{t('totalPlaytime')}</small><b>{fmtNum(Math.round(s.totalMinutes / 60), lang)} h</b></div>
        <div><small>{t('completionRate')}</small><b>{Math.round((s.achUnlocked / s.achTotal) * 100)} %</b><Bar value={Math.round((s.achUnlocked / s.achTotal) * 100)} /></div>
        <div><small>{t('achievements')}</small><b>{s.achUnlocked} / {s.achTotal}</b></div>
        <div><small>{t('perfectGames')}</small><b>{s.perfect}</b></div>
      </div>
      <div className="a-cols">
        <section><h2>{t('last8Weeks')}</h2><WeekChart games={games} t={t} lang={lang} /></section>
        <section>
          <h2>{t('byStatus')}</h2>
          <ul className="statusbars">
            {(Object.keys(s.statusCounts) as (keyof typeof s.statusCounts)[]).map((k) => (
              <li key={k}><span>{t(k)}</span><span className="sess-bar"><i className={`st-${k}`} style={{ width: `${(s.statusCounts[k] / s.count) * 100}%` }} /></span><b>{s.statusCounts[k]}</b></li>
            ))}
          </ul>
        </section>
      </div>
      <section>
        <h2>{t('mostPlayed')}</h2>
        <ul className="toplist">
          {s.top.map((g, i) => (
            <li key={g.id} onClick={() => { go('library'); openGame(g.id); }}>
              <em>{i + 1}</em><Cover game={g} className="thumb" /><b>{g.title}</b>
              <span className="sess-bar"><i style={{ width: `${(g.minutes / maxTop) * 100}%` }} /></span>
              <span>{fmtTime(g.minutes, lang)}</span>
            </li>
          ))}
        </ul>
      </section>
    </div>
  );
}
