import type { ReactNode } from 'react';
import { NavLink } from 'react-router';
import { paths } from '@/config/paths';

const items = [
  { to: paths.today, label: 'Today' },
  { to: paths.logbook, label: 'Logbook' },
  { to: paths.priorities, label: 'Priorities' },
  { to: '/review', label: 'Review' },
  { to: paths.trends, label: 'Trends' },
];

const GearIcon = () => (
  <svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.8" strokeLinecap="round" strokeLinejoin="round" aria-hidden>
    <circle cx="12" cy="12" r="3" />
    <path d="M19.4 15a1.7 1.7 0 0 0 .3 1.8l.1.1a2 2 0 1 1-2.8 2.8l-.1-.1a1.7 1.7 0 0 0-1.8-.3 1.7 1.7 0 0 0-1 1.5V21a2 2 0 1 1-4 0v-.1a1.7 1.7 0 0 0-1.1-1.5 1.7 1.7 0 0 0-1.8.3l-.1.1a2 2 0 1 1-2.8-2.8l.1-.1a1.7 1.7 0 0 0 .3-1.8 1.7 1.7 0 0 0-1.5-1H3a2 2 0 1 1 0-4h.1a1.7 1.7 0 0 0 1.5-1.1 1.7 1.7 0 0 0-.3-1.8l-.1-.1a2 2 0 1 1 2.8-2.8l.1.1a1.7 1.7 0 0 0 1.8.3H9a1.7 1.7 0 0 0 1-1.5V3a2 2 0 1 1 4 0v.1a1.7 1.7 0 0 0 1 1.5 1.7 1.7 0 0 0 1.8-.3l.1-.1a2 2 0 1 1 2.8 2.8l-.1.1a1.7 1.7 0 0 0-.3 1.8V9a1.7 1.7 0 0 0 1.5 1H21a2 2 0 1 1 0 4h-.1a1.7 1.7 0 0 0-1.5 1z" />
  </svg>
);

type Props = {
  /** Slots are composed by the app layer so this component stays free of feature imports. */
  cycleSwitcher: ReactNode;
  syncIndicator: ReactNode;
  userEmail: string;
  onOpenPalette: () => void;
  banner?: ReactNode;
  /** Search string such as `?cycle=3`, preserved across nav links so past cycles stay shareable. */
  search?: string;
  children: ReactNode;
};

export function AppShell({ cycleSwitcher, syncIndicator, userEmail, onOpenPalette, banner, search = '', children }: Props) {
  return (
    <div className="app">
      <aside className="sidebar">
        <div className="brand">
          <img src="/favicon.svg" alt="" /> Brag Frog
        </div>
        {cycleSwitcher}
        <button type="button" className="cmdk" onClick={onOpenPalette} aria-label="Search or jump">
          <span>Search or jump…</span>
          <span className="mono">⌘K</span>
        </button>
        <nav className="nav" aria-label="Main">
          {items.map((i) => (
            <NavLink key={i.to} to={{ pathname: i.to, search }} className={({ isActive }) => (isActive ? 'active' : '')}>
              {i.label}
            </NavLink>
          ))}
        </nav>
        <div className="sp" />
        {syncIndicator}
        <div className="user-row">
          <span className="muted small clamp">{userEmail}</span>
          <NavLink to={{ pathname: paths.settings(), search }} className={({ isActive }) => `gear ${isActive ? 'active' : ''}`} title="Settings" aria-label="Settings">
            <GearIcon />
          </NavLink>
        </div>
      </aside>
      <main className="main">
        {banner}
        {children}
      </main>
    </div>
  );
}
