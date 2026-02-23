// Brag Frog — Command palette: open/close/filter/execute
// Uses event delegation so listeners survive hx-boost navigation.

function openCommandPalette() {
    var overlay = document.getElementById('cmd-palette-overlay');
    if (!overlay) return;
    overlay.style.display = 'flex';
    var input = document.getElementById('cmd-palette-input');
    if (input) { input.value = ''; input.focus(); }
    filterCommandActions('');
}

function closeCommandPalette() {
    var overlay = document.getElementById('cmd-palette-overlay');
    if (overlay) overlay.style.display = 'none';
}

function isCommandPaletteOpen() {
    var overlay = document.getElementById('cmd-palette-overlay');
    return overlay && overlay.style.display !== 'none';
}

function filterCommandActions(query) {
    var actions = document.querySelectorAll('.cmd-palette-action');
    var q = query.toLowerCase().trim();
    var activeSet = false;
    actions.forEach(function(action) {
        var text = action.textContent.toLowerCase();
        var match = !q || text.indexOf(q) !== -1;
        action.style.display = match ? 'flex' : 'none';
        if (match && !activeSet) {
            action.classList.add('active');
            activeSet = true;
        } else {
            action.classList.remove('active');
        }
    });
}

function executeCommandAction(action) {
    var href = action.getAttribute('data-href');
    var actionType = action.getAttribute('data-action');
    closeCommandPalette();
    if (actionType === 'navigate' && href) {
        window.location.href = href;
    } else if (actionType === 'sync' && href) {
        if (window.htmx) {
            htmx.ajax('POST', href, {swap: 'none'});
        }
    }
}

// Event delegation — survives hx-boost DOM replacements
document.addEventListener('input', function(event) {
    if (event.target.id === 'cmd-palette-input') {
        filterCommandActions(event.target.value);
    }
});

document.addEventListener('keydown', function(event) {
    if (event.target.id === 'cmd-palette-input') {
        if (event.key === 'Enter') {
            var active = document.querySelector('.cmd-palette-action.active');
            if (active) executeCommandAction(active);
            event.preventDefault();
        }
        if (event.key === 'ArrowDown' || event.key === 'ArrowUp') {
            event.preventDefault();
            var visible = Array.from(document.querySelectorAll('.cmd-palette-action')).filter(function(a) {
                return a.style.display !== 'none';
            });
            var idx = visible.findIndex(function(a) { return a.classList.contains('active'); });
            visible.forEach(function(a) { a.classList.remove('active'); });
            if (event.key === 'ArrowDown') idx = Math.min(idx + 1, visible.length - 1);
            else idx = Math.max(idx - 1, 0);
            if (visible[idx]) visible[idx].classList.add('active');
        }
    }
});

document.addEventListener('click', function(event) {
    // Action click
    var action = event.target.closest('.cmd-palette-action');
    if (action) {
        executeCommandAction(action);
        return;
    }
    // Scrim close
    if (event.target.id === 'cmd-palette-overlay') {
        closeCommandPalette();
        return;
    }
    // Badge click
    if (event.target.closest('.cmd-k-badge')) {
        openCommandPalette();
    }
});
