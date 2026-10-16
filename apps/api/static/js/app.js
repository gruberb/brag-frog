// Brag Frog — Keyboard shortcuts (capture phase to fire before hx-boost)

document.addEventListener('keydown', function(event) {
    // Escape closes command palette, then panel, then dropdown
    if (event.key === 'Escape') {
        if (isCommandPaletteOpen()) {
            closeCommandPalette();
            event.preventDefault();
            return;
        }
        if (isPanelOpen()) {
            closePanel();
            event.preventDefault();
            return;
        }
        var dropdown = document.getElementById('user-dropdown');
        if (dropdown) dropdown.classList.remove('open');
    }

    // Cmd/Ctrl+K — command palette
    if ((event.metaKey || event.ctrlKey) && event.key === 'k') {
        event.preventDefault();
        event.stopPropagation();
        if (isCommandPaletteOpen()) {
            closeCommandPalette();
        } else {
            openCommandPalette();
        }
    }
}, true);
