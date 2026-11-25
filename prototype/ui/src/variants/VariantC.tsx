// PROTOTYPE - Variant C: dense productivity layout. Slim icon rail, compact game list in the
// middle, permanent detail pane on the right with tabs. Nothing opens or closes; one click per game.
import { useState } from 'react';
import { computeStats, percent } from '../data';
import {
  AchievementList, Bar, Cover, Icon, PlayButton, Ring, SessionList, SettingsSections, SORTS, STATUSES, SourceTag, StatusBadge,
  VProps, WeekChart, fmtNum, fmtRel, fmtTime, useLibrary,
} from '../shared';

export function VariantC(p: VProps) {
  const { games, view, go, gameId, openGame, t, lang, launch } = p;
  const lib = useLibrary(games, 'list');
  const [tab, setTab] = useState<'overview' | 'achievements' | 'sessions'>('overview');
  const game = games.find((g) => g.id === gameId) ?? lib.result[0] ?? games[0];
  const x = percent(game);

  return (
    <div className="c-root">
      <nav className="c-rail">
        <span className="logo" />
        {(['library', 'stats', 'settings'] as const).map((v) => (
          <button key={v} className={view === v ? 'on' : ''} onClick={() => go(v)} title={t(v)} aria-label={t(v)}>
            <Icon name={v} size={20} />
          </button>
        ))}
        <span className="grow" />
        <button title={t('syncNow')} aria-label={t('syncNow')}><Icon name="sync" size={18} /></button>
      </nav>

      {view === 'library' && (
        <>
          <section className="c-list">
            <div className="c-listhead">
              <label className="searchbox">
                <Icon name="search" size={16} />
                <input value={lib.query} onChange={(e) => lib.setQuery(e.target.value)} placeholder={t('search')} />
              </label>
              <button className="iconbtn" title={t('addGame')} aria-label={t('addGame')}><Icon name="plus" /></button>
            </div>
            <div className="c-filters">
              {(['all', 'steam', 'local'] as const).map((s) => (
                <button key={s} className={`chip ${lib.source === s ? 'on' : ''}`} onClick={() => lib.setSource(s)}>{t(s)}</button>
              ))}
              <button className={`chip ${lib.onlyFav ? 'on' : ''}`} onClick={() => lib.setOnlyFav(!lib.onlyFav)}><Icon name="star" size={12} /> {t('favorites')}</button>
              <button className={`chip ${lib.onlyInstalled ? 'on' : ''}`} onClick={() => lib.setOnlyInstalled(!lib.onlyInstalled)}>{t('installed')}</button>
            </div>
            <div className="c-filters two">
              <select value={lib.status} onChange={(e) => lib.setStatus(e.target.value as any)} aria-label={t('status')}>
                <option value="all">{t('status')}: {t('all')}</option>
                {STATUSES.map((s) => <option key={s} value={s}>{t(s)}</option>)}
              </select>
              <select value={lib.sort} onChange={(e) => lib.setSort(e.target.value as any)} aria-label={t('sort')}>
                {SORTS.map(([k, l]) => <option key={k} value={k}>{t(l)}</option>)}
              </select>
              <span className="seg icon">
                <button className={lib.mode === 'list' ? 'on' : ''} onClick={() => lib.setMode('list')} aria-label={t('list')}><Icon name="list" size={15} /></button>
                <button className={lib.mode === 'grid' ? 'on' : ''} onClick={() => lib.setMode('grid')} aria-label={t('grid')}><Icon name="grid" size={15} /></button>
              </span>
            </div>
            <div className="c-count">{lib.result.length} {t('games')}</div>
            {lib.result.length === 0 && (
              <div className="empty-state"><p>{t('noResults')}</p><button className="btn" onClick={lib.reset}>{t('resetFilters')}</button></div>
            )}
            {lib.mode === 'grid' ? (
              <div className="c-grid">
                {lib.result.map((g) => (
                  <button key={g.id} className={`${g.id === game.id ? 'on' : ''} ${g.installed ? '' : 'dim'}`} onClick={() => openGame(g.id)} title={g.title}>
                    <Cover game={g} />
                  </button>
                ))}
              </div>
            ) : (
              <ul className="c-rows">
                {lib.result.map((g) => (
                  <li key={g.id}>
                    <button className={`${g.id === game.id ? 'on' : ''} ${g.installed ? '' : 'dim'}`} onClick={() => openGame(g.id)}>
                      <Cover game={g} className="thumb" />
                      <span className="c-rowmain">
                        <b>{g.title}{g.favorite && <Icon name="star" size={12} fill />}</b>
                        <small>{t(g.source)} · {fmtTime(g.minutes, lang)} · {fmtRel(g.lastPlayed, lang, t)}</small>
                        <Bar value={percent(g)} />
                      </span>
                      <em>{percent(g) === null ? '–' : `${percent(g)} %`}</em>
                    </button>
                  </li>
                ))}
              </ul>
            )}
          </section>

          <section className="c-pane">
            <header>
              <Cover game={game} />
              <div className="c-panetitle">
                <div className="a-tags"><SourceTag source={game.source} t={t} /><StatusBadge status={game.status} t={t} /><span className="tag">{t(game.installed ? 'installed' : 'notInstalled')}</span></div>
                <h1>{game.title}</h1>
                <p>{fmtTime(game.minutes, lang)} · {t('lastPlayed')}: {fmtRel(game.lastPlayed, lang, t)}</p>
              </div>
              <PlayButton game={game} t={t} launch={launch} big />
            </header>
            <div className="c-tabs" role="tablist">
              {(['overview', 'achievements', 'sessions'] as const).map((k) => (
                <button key={k} role="tab" aria-selected={tab === k} className={tab === k ? 'on' : ''} onClick={() => setTab(k)}>
                  {t(k)}
                  {k === 'achievements' && game.achievements.length > 0 && <em>{game.achievements.filter((a) => a.unlocked).length}/{game.achievements.length}</em>}
                  {k === 'sessions' && <em>{game.sessions.length}</em>}
                </button>
              ))}
            </div>
            <div className="c-panebody">
              {tab === 'overview' && (
                <>
                  <div className="c-facts">
                    <Ring value={x} size={96} />
                    <dl>
                      <div><dt>{t('playtime')}</dt><dd>{fmtTime(game.minutes, lang)}</dd></div>
                      <div><dt>{t('lastPlayed')}</dt><dd>{fmtRel(game.lastPlayed, lang, t)}</dd></div>
                      <div><dt>{t('achievements')}</dt><dd>{game.achievements.filter((a) => a.unlocked).length} / {game.achievements.length}</dd></div>
                      <div><dt>{t('sessions')}</dt><dd>{game.sessions.length}</dd></div>
                      <div><dt>{t('collections')}</dt><dd>{game.collections.join(', ') || '–'}</dd></div>
                      <div><dt>{t('source')}</dt><dd>{t(game.source)}</dd></div>
                    </dl>
                  </div>
                  <h3>{t('sessions')}</h3>
                  <SessionList game={{ ...game, sessions: game.sessions.slice(0, 5) }} t={t} lang={lang} />
                </>
              )}
              {tab === 'achievements' && <AchievementList game={game} t={t} lang={lang} />}
              {tab === 'sessions' && <SessionList game={game} t={t} lang={lang} />}
            </div>
          </section>
        </>
      )}

      {view === 'stats' && <StatsC {...p} />}

      {view === 'settings' && (
        <section className="c-wide c-settings">
          <nav>
            <h1>{t('settings')}</h1>
            {[['set-steam', 'steamSection'], ['set-tracking', 'tracking'], ['set-appearance', 'appearance'], ['set-updates', 'updates'], ['set-about', 'about']].map(([id, k]) => (
              <a key={id} href={`#${id}`} onClick={(e) => { e.preventDefault(); document.getElementById(id)?.scrollIntoView({ block: 'start' }); }}>{t(k)}</a>
            ))}
          </nav>
          <div className="c-setbody"><SettingsSections {...p} /></div>
        </section>
      )}
    </div>
  );
}

function StatsC({ games, t, lang, openGame, go }: VProps) {
  const s = computeStats(games);
  const rows = [...games].sort((a, b) => b.minutes - a.minutes);
  return (
    <section className="c-wide c-stats">
      <div className="c-kpis">
        <div><small>{t('totalPlaytime')}</small><b>{fmtNum(Math.round(s.totalMinutes / 60), lang)} h</b></div>
        <div><small>{t('completionRate')}</small><b>{Math.round((s.achUnlocked / s.achTotal) * 100)} %</b></div>
        <div><small>{t('achievements')}</small><b>{s.achUnlocked} / {s.achTotal}</b></div>
        <div><small>{t('perfectGames')}</small><b>{s.perfect}</b></div>
        {(Object.keys(s.statusCounts) as (keyof typeof s.statusCounts)[]).map((k) => (
          <div key={k}><small>{t(k)}</small><b>{s.statusCounts[k]}</b></div>
        ))}
      </div>
      <div className="c-statgrid">
        <div>
          <h3>{t('last8Weeks')}</h3>
          <WeekChart games={games} t={t} lang={lang} height={180} />
        </div>
        <div className="c-stattable">
          <h3>{t('allGames')}</h3>
          <table>
            <thead><tr><th>{t('game')}</th><th>{t('status')}</th><th className="num">{t('playtime')}</th><th className="num">{t('sessions')}</th><th>{t('progress')}</th></tr></thead>
            <tbody>
              {rows.map((g) => (
                <tr key={g.id} onClick={() => { openGame(g.id); go('library'); }}>
                  <td><b>{g.title}</b></td>
                  <td><StatusBadge status={g.status} t={t} /></td>
                  <td className="num">{fmtTime(g.minutes, lang)}</td>
                  <td className="num">{g.sessions.length}</td>
                  <td className="a-prog"><Bar value={percent(g)} /><span>{percent(g) === null ? '–' : `${percent(g)} %`}</span></td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      </div>
    </section>
  );
}
