import { sourceMeta } from '@/utils/entries';

const IMAGES: Record<string, string> = {
  phabricator: '/sources/favicon-phabricator.png',
  bugzilla: '/sources/favicon-bugzilla.svg',
  jira: '/sources/favicon-jira.ico',
  google_calendar: '/sources/favicon-gcalendar.png',
  google_drive: '/sources/favicon-gdocs.png',
};

const GITHUB_PATH =
  'M12 .5a11.5 11.5 0 0 0-3.64 22.41c.58.1.79-.25.79-.56v-2c-3.2.7-3.88-1.37-3.88-1.37-.52-1.33-1.28-1.69-1.28-1.69-1.04-.71.08-.7.08-.7 1.16.08 1.77 1.19 1.77 1.19 1.03 1.77 2.7 1.26 3.36.96.1-.75.4-1.26.73-1.55-2.55-.29-5.24-1.28-5.24-5.69 0-1.26.45-2.28 1.19-3.09-.12-.29-.52-1.46.11-3.05 0 0 .97-.31 3.17 1.18a11 11 0 0 1 5.78 0c2.2-1.49 3.17-1.18 3.17-1.18.63 1.59.23 2.76.11 3.05.74.81 1.19 1.83 1.19 3.09 0 4.42-2.69 5.39-5.25 5.68.41.36.78 1.06.78 2.14v3.17c0 .31.21.67.8.56A11.5 11.5 0 0 0 12 .5Z';

/** The provider's own logo, so entries are recognisable at a glance. Unknown sources get a neutral dot. */
export function SourceIcon({ source }: { source: string }) {
  const m = sourceMeta(source);
  const img = IMAGES[source];
  return (
    <span className="src-logo" title={m.label} role="img" aria-label={m.label}>
      {img && <img src={img} alt="" width={20} height={20} />}
      {source === 'github' && (
        <svg viewBox="0 0 24 24" width={20} height={20} aria-hidden="true"><path fill="#24292f" d={GITHUB_PATH} /></svg>
      )}
      {source === 'confluence' && (
        <svg viewBox="0 0 24 24" width={20} height={20} aria-hidden="true">
          <path fill="#1868db" d="M2.3 17.6c-.3.5-.6 1-.9 1.4a.9.9 0 0 0 .3 1.2l3.8 2.4a.9.9 0 0 0 1.2-.3l.7-1.2c1.4-2.3 2.8-2 5.3-.8l3.8 1.8a.9.9 0 0 0 1.2-.4l1.8-4a.9.9 0 0 0-.4-1.2c-.8-.4-2.4-1.1-3.8-1.8-5.2-2.5-9.7-2.3-13 3.1Z" />
          <path fill="#1868db" d="M21.7 6.4c.3-.5.6-1 .9-1.4a.9.9 0 0 0-.3-1.2L18.5 1.4a.9.9 0 0 0-1.2.3l-.7 1.2c-1.4 2.3-2.8 2-5.3.8L7.5 1.9a.9.9 0 0 0-1.2.4l-1.8 4a.9.9 0 0 0 .4 1.2c.8.4 2.4 1.1 3.8 1.8 5.2 2.5 9.7 2.3 13-2.9Z" />
        </svg>
      )}
      {source === 'manual' && (
        <svg viewBox="0 0 24 24" width={18} height={18} fill="none" stroke="var(--color-primary)" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round" aria-hidden="true"><path d="M12 20h9" /><path d="M16.5 3.5a2.1 2.1 0 0 1 3 3L7 19l-4 1 1-4Z" /></svg>
      )}
      {!img && !['github', 'confluence', 'manual'].includes(source) && <span className="dot gray" />}
    </span>
  );
}
