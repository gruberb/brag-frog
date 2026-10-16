// Injects the shared app shell (sidebar) so every mock page stays small.
// Usage: <body data-page="today"> ... <main class="main">...</main>
(function () {
  const items = [
    ['today', 'Today', 'today.html', 'G T'],
    ['logbook', 'Logbook', 'logbook.html', 'G L'],
    ['priorities', 'Priorities', 'priorities.html', 'G P'],
    ['review', 'Review', 'review.html', 'G V'],
    ['trends', 'Trends', 'trends.html', 'G D'],
  ];
  const page = document.body.dataset.page;
  const archived = document.body.dataset.archived === 'true';
  const side = document.createElement('aside');
  side.className = 'sidebar';
  side.innerHTML = `
    <div class="brand"><i></i> Brag Frog</div>
    <a class="cycle-switch" href="settings.html#cycles" title="Switch performance cycle">
      <span class="l">Cycle</span>
      <b><span>${archived ? 'H1 2026 (past)' : 'H2 2026'}</span><span>⌄</span></b>
    </a>
    <div class="cmdk"><span>Search or jump…</span><span class="mono">⌘K</span></div>
    <nav class="nav">${items
      .map(([id, label, href, k]) => `<a href="${href}" class="${id === page ? 'active' : ''}">${label}<span class="k">${k}</span></a>`)
      .join('')}</nav>
    <div class="sp"></div>
    <div class="sync"><span class="dot"></span> Synced 4 min ago <a class="muted" href="settings.html#integrations">· details</a></div>
    <div class="user-row"><span class="muted small">bgruber@mozilla.com</span>
      <a href="settings.html" class="gear ${page === 'settings' ? 'active' : ''}" title="Settings" aria-label="Settings">
        <svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round"><circle cx="12" cy="12" r="3"/><path d="M19.4 15a1.7 1.7 0 0 0 .3 1.8l.1.1a2 2 0 1 1-2.8 2.8l-.1-.1a1.7 1.7 0 0 0-1.8-.3 1.7 1.7 0 0 0-1 1.5V21a2 2 0 1 1-4 0v-.1a1.7 1.7 0 0 0-1.1-1.5 1.7 1.7 0 0 0-1.8.3l-.1.1a2 2 0 1 1-2.8-2.8l.1-.1a1.7 1.7 0 0 0 .3-1.8 1.7 1.7 0 0 0-1.5-1H3a2 2 0 1 1 0-4h.1a1.7 1.7 0 0 0 1.5-1.1 1.7 1.7 0 0 0-.3-1.8l-.1-.1a2 2 0 1 1 2.8-2.8l.1.1a1.7 1.7 0 0 0 1.8.3H9a1.7 1.7 0 0 0 1-1.5V3a2 2 0 1 1 4 0v.1a1.7 1.7 0 0 0 1 1.5 1.7 1.7 0 0 0 1.8-.3l.1-.1a2 2 0 1 1 2.8 2.8l-.1.1a1.7 1.7 0 0 0-.3 1.8V9a1.7 1.7 0 0 0 1.5 1H21a2 2 0 1 1 0 4h-.1a1.7 1.7 0 0 0-1.5 1z"/></svg>
      </a></div>`;
  const app = document.createElement('div');
  app.className = 'app';
  const main = document.querySelector('main');
  document.body.insertBefore(app, main);
  app.appendChild(side);
  app.appendChild(main);
  // Drawers start closed so the list stays readable; the highlighted row opens it.
  const drawer = document.querySelector('.drawer');
  if (drawer) {
    drawer.style.display = 'none';
    document.querySelectorAll('.entry, .prow, [data-open-drawer]').forEach((row) =>
      row.addEventListener('click', () => (drawer.style.display = 'flex')));
    drawer.querySelectorAll('[data-close]').forEach((el) =>
      el.addEventListener('click', () => (drawer.style.display = 'none')));
  }
})();
