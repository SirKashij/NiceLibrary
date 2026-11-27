import { useEffect, useRef, useState } from 'react';
import { useT } from './i18n';
import type { About as AboutApp, AboutPage, Operations } from './operations';

/**
 * About NiceLibrary, opened from the bottom of the sidebar: the version and
 * the license, the privacy notice, the Valve trademark notice and the
 * statement that the app is not affiliated with Valve. Shows no logo, the
 * Steam logo least of all.
 */
export function AboutButton({ operations }: { operations: Operations }) {
  const t = useT();
  const [open, setOpen] = useState(false);
  const button = useRef<HTMLButtonElement>(null);

  const close = () => {
    setOpen(false);
    button.current?.focus();
  };

  return (
    <>
      <button ref={button} type="button" className="about-button" onClick={() => setOpen(true)}>
        {t('about.title')}
      </button>
      {open && <AboutDialog operations={operations} onClose={close} />}
    </>
  );
}

function AboutDialog({ operations, onClose }: { operations: Operations; onClose: () => void }) {
  const t = useT();
  const [about, setAbout] = useState<AboutApp | 'loading' | 'failed'>('loading');
  const [openFailed, setOpenFailed] = useState(false);

  useEffect(() => {
    let active = true;
    operations.about().then(
      (value) => active && setAbout(value),
      () => active && setAbout('failed'),
    );
    return () => {
      active = false;
    };
  }, [operations]);

  const openPage = (page: AboutPage) => {
    setOpenFailed(false);
    operations.openAboutPage(page).catch(() => setOpenFailed(true));
  };

  return (
    <div className="about-backdrop">
      <div
        className="about"
        role="dialog"
        aria-modal="true"
        aria-labelledby="about-title"
        onKeyDown={(event) => {
          if (event.key === 'Escape') onClose();
        }}
      >
        <h2 id="about-title">{t('about.title')}</h2>
        {about === 'failed' ? (
          <p className="note" role="alert">
            {t('about.versionFailed')}
          </p>
        ) : (
          typeof about === 'object' && (
            <p className="about-facts">
              <span>{t('about.version', { version: about.version })}</span>
              <span>{t('about.license', { license: about.license })}</span>
            </p>
          )
        )}
        <div className="about-links">
          <button type="button" className="button" onClick={() => openPage('project')}>
            {t('about.projectPage')}
          </button>
          <button type="button" className="button" onClick={() => openPage('license')}>
            {t('about.licensePage')}
          </button>
        </div>
        {openFailed && (
          <p className="note" role="alert">
            {t('about.openFailed')}
          </p>
        )}
        <section className="about-section" aria-labelledby="about-privacy">
          <h3 id="about-privacy">{t('about.privacy')}</h3>
          <p className="note">{t('about.privacyConnections')}</p>
          <p className="note">{t('about.privacyStorage')}</p>
          <p className="note">{t('about.privacyNothingElse')}</p>
        </section>
        <section className="about-section" aria-labelledby="about-trademarks">
          <h3 id="about-trademarks">{t('about.trademarks')}</h3>
          <p className="note">{t('about.trademarkNotice')}</p>
          <p className="note">{t('about.notAffiliated')}</p>
        </section>
        <button type="button" className="button" autoFocus onClick={onClose}>
          {t('about.close')}
        </button>
      </div>
    </div>
  );
}
