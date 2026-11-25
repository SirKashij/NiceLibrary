// PROTOTYPE - Variant B: storefront feel. Top navigation, a hero for the game you are playing,
// horizontal shelves instead of filters first, game detail as a slide-over panel on top of any view.
import { computeStats, Game, percent } from '../data';
import {
  AchievementList, Bar, Cover, Icon, PlayButton, Ring, SessionList, SettingsSections, SORTS, STATUSES, SourceTag, StatusBadge,
  VProps, WeekChart, fmtNum, fmtRel, fmtTime, useLibrary,
} from '../shared';

export function VariantB(p: VProps) {
  const { games, view, go, gameId, openGame, t, lang, launch } = p;
  const lib = useLibrary(games);
  const game = games.find((g) => g.id === gameId) ?? null;
  const recent = [...games].filter((g) => g.lastPlayed).sort((a, b) => b.lastPlayed!.localeCompare(a.lastPlayed!));
  const hero = recent[0];
  const filtering = lib.query !== '' || lib.source !== 'all' || lib.status !== 'all' || lib.onlyInstalled;

  const shelves: [string, Game[]][] = [
    ['recently', recent.slice(1, 9)],
    ['almostDone', games.filter((g) => { const x = percent(g); return x !== null && x >= 60 && x < 100; })],
    ['favorites', games.filter((g) => g.favorite)],
    ['backlog', games.filter((g) => g.status === 'unplayed')],
  ];

  return (
    <div className="b-root">
      <header className="b-top">
        <div className="b-brand"><span className="logo" />NiceLibrary</div>
        <nav>
          {(['library', 'stats', 'settings'] as const).map((v) => (
            <button key={v} className={view === v ? 'on' : ''} onClick={() => go(v)}>{t(v)}</button>
          ))}
        </nav>
        <label className="searchbox">
          <Icon name="search" size={16} />
          <input value={lib.query} onChange={(e) => { lib.setQuery(e.target.value); go('library'); }} placeholder={t('search')} />
        </label>
      </header>

      <main className="b-main">
        {view === 'library' && (
          <>
            {!filtering && (
              <>
                <section className="b-hero" style={{ background: `linear-gradient(110deg, hsl(${hero.hue} 60% 30%), hsl(${(hero.hue + 45) % 360} 55% 14%))` }}>
                  <Cover game={hero} />
                  <div>
                    <small>{t('recently')} · {fmtRel(hero.lastPlayed, lang, t)}</small>
                    <h1>{hero.title}</h1>
                    <p>{fmtTime(hero.minutes, lang)} · {hero.achievements.filter((a) => a.unlocked).length} / {hero.achievements.length} {t('achievements')}</p>
                    <div className="b-hero-bar"><Bar value={percent(hero)} /><b>{percent(hero)} %</b></div>
                    <div className="b-hero-actions">
                      <PlayButton game={hero} t={t} launch={launch} big label={t('resume')} />
                      <button className="btn big onhero" onClick={() => openGame(hero.id)}>{t('overview')}</button>
                    </div>
                  </div>
                </section>
                {shelves.filter(([, list]) => list.length > 0).map(([key, list]) => (
                  <section key={key} className="b-shelf">
                    <h2>{t(key)} <span>{list.length}</span></h2>
                    <div className="b-row">
                      {list.map((g) => <Tile key={g.id} g={g} p={p} />)}
                    </div>
                  </section>
                ))}
              </>
            )}
            <section className="b-all">
              <div className="b-allhead">
                <h2>{t('allGames')} <span>{lib.result.length}</span></h2>
                <span className="grow" />
                <button className="btn"><Icon name="plus" size={15} /> {t('addGame')}</button>
              </div>
              <div className="b-chips">
                {(['all', 'steam', 'local'] as const).map((s) => (
                  <button key={s} className={`chip ${lib.source === s ? 'on' : ''}`} onClick={() => lib.setSource(s)}>{t(s)}</button>
                ))}
                <i />
                {STATUSES.map((s) => (
                  <button key={s} className={`chip ${lib.status === s ? 'on' : ''}`} onClick={() => lib.setStatus(lib.status === s ? 'all' : s)}>{t(s)}</button>
                ))}
                <i />
                <button className={`chip ${lib.onlyInstalled ? 'on' : ''}`} onClick={() => lib.setOnlyInstalled(!lib.onlyInstalled)}>{t('onlyInstalled')}</button>
                <span className="grow" />
                <select value={lib.sort} onChange={(e) => lib.setSort(e.target.value as any)} aria-label={t('sort')}>
                  {SORTS.map(([k, l]) => <option key={k} value={k}>{t(l)}</option>)}
                </select>
                <span className="seg icon">
                  <button className={lib.mode === 'grid' ? 'on' : ''} onClick={() => lib.setMode('grid')} aria-label={t('grid')}><Icon name="grid" size={16} /></button>
                  <button className={lib.mode === 'list' ? 'on' : ''} onClick={() => lib.setMode('list')} aria-label={t('list')}><Icon name="list" size={16} /></button>
                </span>
              </div>
              {lib.result.length === 0 && (
                <div className="empty-state"><p>{t('noResults')}</p><button className="btn" onClick={lib.reset}>{t('resetFilters')}</button></div>
              )}
              {lib.mode === 'grid' ? (
                <div className="b-grid">{lib.result.map((g) => <Tile key={g.id} g={g} p={p} />)}</div>
              ) : (
                <ul className="b-list">
                  {lib.result.map((g) => (
                    <li key={g.id} onClick={() => openGame(g.id)} className={g.installed ? '' : 'dim'}>
                      <Cover game={g} className="thumb" />
                      <span className="b-list-title"><b>{g.title}</b><small>{t(g.source)} · {t(g.status)}</small></span>
                      <span>{fmtTime(g.minutes, lang)}</span>
                      <span className="muted">{fmtRel(g.lastPlayed, lang, t)}</span>
                      <span className="b-list-prog"><Bar value={percent(g)} /><b>{percent(g) === null ? '–' : `${percent(g)} %`}</b></span>
                      <PlayButton game={g} t={t} launch={launch} />
                    </li>
                  ))}
                </ul>
              )}
            </section>
          </>
        )}

        {view === 'stats' && <StatsB {...p} />}

        {view === 'settings' && (
          <div className="b-settings">
            <h1>{t('settings')}</h1>
            <div className="b-setgrid"><SettingsSections {...p} /></div>
          </div>
        )}
      </main>

      {game && (
        <>
          <div className="b-scrim" onClick={() => openGame(null)} />
          <aside className="b-panel" role="dialog" aria-label={game.title}>
            <div className="b-panel-head" style={{ background: `linear-gradient(140deg, hsl(${game.hue} 60% 34%), hsl(${(game.hue + 45) % 360} 55% 16%))` }}>
              <button className="iconbtn" onClick={() => openGame(null)} aria-label={t('close')}><Icon name="close" /></button>
              <Cover game={game} />
              <div>
                <h2>{game.title}</h2>
                <div className="a-tags"><SourceTag source={game.source} t={t} /><StatusBadge status={game.status} t={t} /></div>
                <PlayButton game={game} t={t} launch={launch} big />
              </div>
            </div>
            <div className="b-panel-body">
              <div className="b-facts">
                <Ring value={percent(game)} />
                <div><small>{t('playtime')}</small><b>{fmtTime(game.minutes, lang)}</b></div>
                <div><small>{t('lastPlayed')}</small><b>{fmtRel(game.lastPlayed, lang, t)}</b></div>
                <div><small>{t('achievements')}</small><b>{game.achievements.filter((a) => a.unlocked).length} / {game.achievements.length}</b></div>
              </div>
              <h3>{t('achievements')}</h3>
              <AchievementList game={game} t={t} lang={lang} />
              <h3>{t('sessions')}</h3>
              <SessionList game={game} t={t} lang={lang} />
            </div>
          </aside>
        </>
      )}
    </div>
  );
}

function Tile({ g, p }: { g: Game; p: VProps }) {
  const x = percent(g);
  return (
    <button className={`b-tile ${g.installed ? '' : 'dim'}`} onClick={() => p.openGame(g.id)}>
      <Cover game={g} />
      <span className="b-tile-over">
        <b>{g.title}</b>
        <small>{fmtTime(g.minutes, p.lang)}{x === null ? '' : ` · ${x} %`}</small>
        <Bar value={x} />
      </span>
    </button>
  );
}

function StatsB({ games, t, lang, openGame }: VProps) {
  const s = computeStats(games);
  const maxTop = s.top[0].minutes;
  return (
    <div className="b-stats">
      <div className="b-bignums">
        <div><b>{fmtNum(Math.round(s.totalMinutes / 60), lang)}<i>h</i></b><small>{t('totalPlaytime')}</small></div>
        <div><b>{Math.round((s.achUnlocked / s.achTotal) * 100)}<i>%</i></b><small>{t('completionRate')}</small></div>
        <div><b>{s.achUnlocked}</b><small>{t('achievements')}</small></div>
        <div><b>{s.perfect}</b><small>{t('perfectGames')}</small></div>
        <div><b>{s.count}</b><small>{t('inLibrary')}</small></div>
      </div>
      <section className="b-card"><h2>{t('last8Weeks')}</h2><WeekChart games={games} t={t} lang={lang} height={220} /></section>
      <div className="b-statcols">
        <section className="b-card">
          <h2>{t('mostPlayed')}</h2>
          <ul className="toplist">
            {s.top.map((g, i) => (
              <li key={g.id} onClick={() => openGame(g.id)}>
                <em>{i + 1}</em><Cover game={g} className="thumb" /><b>{g.title}</b>
                <span className="sess-bar"><i style={{ width: `${(g.minutes / maxTop) * 100}%` }} /></span>
                <span>{fmtTime(g.minutes, lang)}</span>
              </li>
            ))}
          </ul>
        </section>
        <section className="b-card">
          <h2>{t('byStatus')}</h2>
          <div className="b-statusstack">
            {(Object.keys(s.statusCounts) as (keyof typeof s.statusCounts)[]).map((k) => (
              <span key={k} className={`st-${k}`} style={{ flex: s.statusCounts[k] }} title={t(k)} />
            ))}
          </div>
          <ul className="b-legend">
            {(Object.keys(s.statusCounts) as (keyof typeof s.statusCounts)[]).map((k) => (
              <li key={k}><i className={`st-${k}`} />{t(k)}<b>{s.statusCounts[k]}</b></li>
            ))}
          </ul>
        </section>
      </div>
    </div>
  );
}
