// Brag Frog — Reports: edit-toggle for the Latest Updates section,
// and a shared "copy rendered markdown to clipboard" helper for both tabs.
//
// The tab switcher lives inline in pages/reports.html so it runs before
// these handlers attach — this file only needs to care about interactions
// inside an already-visible section.

(function () {
    // Delegated click handler — survives HTMX swaps that replace the
    // Latest Updates section wholesale after a Save/Generate.
    document.addEventListener('click', function (event) {
        var target = event.target.closest('[data-action]');
        if (!target) return;

        var section = target.closest('#report-latest-updates');
        if (!section) return;

        var action = target.getAttribute('data-action');
        if (action === 'toggle-edit') {
            enterEditMode(section);
        } else if (action === 'cancel-edit') {
            leaveEditMode(section);
        }
    });

    function enterEditMode(section) {
        section.setAttribute('data-mode', 'edit');
        var textarea = section.querySelector('.report-editor-textarea');
        if (textarea) {
            // Focus at the end of existing content so the user can append.
            textarea.focus();
            var len = textarea.value.length;
            textarea.setSelectionRange(len, len);
        }
    }

    function leaveEditMode(section) {
        var mode = section.querySelector('.report-section-markdown[data-role="view"]')
            ? 'view'
            : 'empty';
        section.setAttribute('data-mode', mode);
    }
})();

// Shared copy helper used by both Last Week and Latest Updates sections.
// Grabs the rendered markdown text (innerText, so bullets/headings come
// across as plain text suitable for pasting into Slack / Lattice / docs).
function copyReportMarkdown(btn) {
    var section = btn.closest('.report-section');
    if (!section) return;
    var body = section.querySelector('.report-section-markdown');
    if (!body) return;
    var text = body.innerText.trim();
    navigator.clipboard.writeText(text).then(function () {
        var orig = btn.textContent;
        btn.textContent = 'Copied!';
        setTimeout(function () {
            btn.textContent = orig;
        }, 1500);
    });
}
