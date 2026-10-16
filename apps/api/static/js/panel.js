// Brag Frog — Slide-over panel system

function openPanel(title, contentHtml) {
    var scrim = document.getElementById('panel-scrim');
    var container = document.getElementById('panel-container');
    var titleEl = document.getElementById('panel-title');
    var bodyEl = document.getElementById('panel-body');
    if (!scrim || !container) return;
    titleEl.textContent = title || '';
    bodyEl.innerHTML = contentHtml || '';
    scrim.classList.add('active');
    container.classList.add('open');
    if (window.htmx) htmx.process(bodyEl);
}

function openPanelFromUrl(title, url) {
    var scrim = document.getElementById('panel-scrim');
    var container = document.getElementById('panel-container');
    var titleEl = document.getElementById('panel-title');
    var bodyEl = document.getElementById('panel-body');
    if (!scrim || !container) return;
    titleEl.textContent = title || '';
    bodyEl.innerHTML = '<div style="padding:20px;color:var(--color-gray-50)">Loading…</div>';
    scrim.classList.add('active');
    container.classList.add('open');
    if (window.htmx) {
        htmx.ajax('GET', url, {target: '#panel-body', swap: 'innerHTML'});
    }
}

function closePanel() {
    var scrim = document.getElementById('panel-scrim');
    var container = document.getElementById('panel-container');
    if (scrim) scrim.classList.remove('active');
    if (container) container.classList.remove('open');
}

function isPanelOpen() {
    var container = document.getElementById('panel-container');
    return container && container.classList.contains('open');
}
