// Brag Frog — Dashboard: focus items, entry autocomplete, chip system

// ─── Quick-add result handler ───────────────────────────────
function handleQuickAdd(form, event) {
    if (event.detail.successful) {
        var xhr = event.detail.xhr;
        var text = xhr ? xhr.responseText : '';
        // Extract message from the flash div
        var match = text.match(/>([^<]+)</);
        var msg = match ? match[1] : 'Entry logged';
        showToast(msg, 'success');
        form.reset();
        // Restore today's date (reset clears it)
        var dateInput = form.querySelector('input[name="occurred_at"]');
        if (dateInput && !dateInput.value) {
            dateInput.value = new Date().toISOString().slice(0, 10);
        }
    } else {
        var xhr = event.detail.xhr;
        var msg = (xhr && xhr.responseText) || 'Something went wrong';
        showToast(msg, 'error');
    }
}

// ─── Focus helpers ──────────────────────────────────────────
function toggleFocusExpand(focusId) {
    var card = document.getElementById('focus-card-' + focusId);
    var body = document.getElementById('focus-body-' + focusId);
    if (card && body) {
        card.classList.toggle('expanded');
        body.classList.toggle('hidden');
    }
}

function toggleFocusEdit(focusId) {
    var form = document.getElementById('focus-edit-' + focusId);
    if (form) form.classList.toggle('hidden');
}

function handleFocusAddClick() {
    var form = document.getElementById('focus-add-form');
    var btn = document.getElementById('focus-add-btn');
    if (form) form.classList.remove('hidden');
    if (btn) btn.classList.add('btn--greyed');
}

function cancelFocusAdd() {
    var form = document.getElementById('focus-add-form');
    var btn = document.getElementById('focus-add-btn');
    if (form) form.classList.add('hidden');
    if (btn && !btn.disabled) btn.classList.remove('btn--greyed');
}

function toggleFocusAdd() {
    var form = document.getElementById('focus-add-form');
    if (form) form.classList.toggle('hidden');
}

// ─── Entry autocomplete for focus items ─────────────────────
function showEntryResults(acId) {
    var results = document.getElementById('entry-ac-results-' + acId);
    if (!results || !results.classList.contains('hidden')) return;
    results.classList.remove('hidden');
    var container = document.getElementById('entry-ac-' + acId);
    // Use mousedown so we can detect clicks outside before blur fires
    setTimeout(function() {
        document.addEventListener('mousedown', function handler(e) {
            if (!container || !container.contains(e.target)) {
                results.classList.add('hidden');
                document.removeEventListener('mousedown', handler);
            }
        });
    }, 0);
}

function filterEntryResults(input, acId) {
    var results = document.getElementById('entry-ac-results-' + acId);
    if (!results) return;
    results.classList.remove('hidden');
    var q = input.value.toLowerCase().trim();
    var options = results.querySelectorAll('.entry-ac-option');
    options.forEach(function(opt) {
        var text = opt.textContent.toLowerCase();
        opt.style.display = text.indexOf(q) !== -1 ? '' : 'none';
    });
}

function toggleLinkedEntry(option, acId) {
    var id = option.getAttribute('data-id');
    var title = option.getAttribute('data-title');
    var selected = document.getElementById('entry-ac-selected-' + acId);
    if (!selected) return;

    // Toggle: remove if already selected, add if not
    var existing = selected.querySelector('.entry-ac-chip[data-id="' + id + '"]');
    if (existing) {
        existing.remove();
        option.classList.remove('selected');
    } else {
        var chip = document.createElement('span');
        chip.className = 'entry-ac-chip';
        chip.setAttribute('data-id', id);
        chip.innerHTML = '<span class="entry-ac-chip-label">' + title.replace(/</g, '&lt;') + '</span> <button type="button" onclick="removeLinkedEntry(this, \'' + acId + '\')">&times;</button>';
        selected.appendChild(chip);
        option.classList.add('selected');
    }
    syncEntryIds(acId);
}

function removeLinkedEntry(btn, acId) {
    var chip = btn.parentElement;
    if (chip) {
        var id = chip.getAttribute('data-id');
        chip.remove();
        // Un-highlight in dropdown
        var results = document.getElementById('entry-ac-results-' + acId);
        if (results) {
            var opt = results.querySelector('.entry-ac-option[data-id="' + id + '"]');
            if (opt) opt.classList.remove('selected');
        }
    }
    syncEntryIds(acId);
}

function syncEntryIds(acId) {
    var selected = document.getElementById('entry-ac-selected-' + acId);
    var container = document.getElementById('entry-ac-' + acId);
    if (!selected || !container) return;
    // Update or create a single hidden input with comma-separated IDs
    var hidden = container.querySelector('input[type="hidden"][name="entry_ids"]');
    if (!hidden) {
        hidden = document.createElement('input');
        hidden.type = 'hidden';
        hidden.name = 'entry_ids';
        container.appendChild(hidden);
    }
    var chips = selected.querySelectorAll('.entry-ac-chip');
    hidden.value = Array.from(chips).map(function(c) { return c.getAttribute('data-id'); }).join(',');
}

function filterFocusEntries(input, dropdownId) {
    var dd = document.getElementById(dropdownId);
    if (!dd) return;
    var q = input.value.toLowerCase().trim();
    var options = dd.querySelectorAll('.chip-option');
    options.forEach(function(opt) {
        var label = opt.querySelector('.chip-label');
        if (!label) return;
        var text = label.textContent.toLowerCase();
        opt.style.display = text.indexOf(q) !== -1 ? '' : 'none';
    });
}

// ─── OKR snapshot — collapsible goals with localStorage ──────
function toggleDashboardGoal(goalId) {
    var el = document.getElementById('dash-goal-' + goalId);
    if (!el) return;
    el.classList.toggle('collapsed');
    // Persist state
    var key = 'dashGoalCollapsed';
    var stored = {};
    try { stored = JSON.parse(localStorage.getItem(key) || '{}'); } catch(e) {}
    if (el.classList.contains('collapsed')) {
        stored[goalId] = true;
    } else {
        delete stored[goalId];
    }
    localStorage.setItem(key, JSON.stringify(stored));
}

// Restore collapsed state on page load
(function() {
    var key = 'dashGoalCollapsed';
    var stored = {};
    try { stored = JSON.parse(localStorage.getItem(key) || '{}'); } catch(e) {}
    Object.keys(stored).forEach(function(goalId) {
        var el = document.getElementById('dash-goal-' + goalId);
        if (el) el.classList.add('collapsed');
    });
})();

// ─── Chip dropdown helpers ───────────────────────────────────
function toggleDropdown(id) {
    var el = document.getElementById(id);
    if (!el) return;
    var panel = el.querySelector('.chip-dropdown-panel');
    if (!panel) return;
    var isOpen = panel.classList.contains('open');

    // Close all other open dropdowns first
    document.querySelectorAll('.chip-dropdown-panel.open').forEach(function(p) {
        p.classList.remove('open');
        p.classList.add('hidden');
    });

    if (!isOpen) {
        panel.classList.remove('hidden');
        panel.classList.add('open');
        // Close when clicking outside
        setTimeout(function() {
            document.addEventListener('click', function handler(e) {
                if (!el.contains(e.target)) {
                    panel.classList.remove('open');
                    panel.classList.add('hidden');
                    document.removeEventListener('click', handler);
                }
            });
        }, 0);
    }
}

function updateChipField(prefix) {
    var container = document.getElementById(prefix + '-chips');
    if (!container) return;
    var checked = container.querySelectorAll('input[type="checkbox"]:checked');
    var values = Array.from(checked).map(function(cb) { return cb.value; });
    var hidden = document.getElementById(prefix + '-value');
    if (hidden) hidden.value = values.join(',');
    var label = document.getElementById(prefix + '-label');
    if (label) {
        if (values.length > 0) {
            label.textContent = values.length + ' selected';
        } else {
            label.textContent = label.getAttribute('data-default') || 'Select';
        }
    }
}

function filterChips(input, containerId) {
    var query = input.value.toLowerCase();
    var container = document.getElementById(containerId);
    if (!container) return;
    container.querySelectorAll('.chip-option').forEach(function(label) {
        var text = label.querySelector('.chip-label').textContent.toLowerCase();
        label.style.display = text.includes(query) ? '' : 'none';
    });
}

function addChip(event, prefix) {
    if (event.key !== 'Enter') return;
    event.preventDefault();
    var input = event.target;
    var val = input.value.trim();
    if (!val) return;
    var container = document.getElementById(prefix + '-chips');
    if (!container) return;
    // Avoid duplicates
    var existing = container.querySelector('input[value="' + CSS.escape(val) + '"]');
    if (existing) { input.value = ''; return; }
    var option = document.createElement('label');
    option.className = 'chip-option';
    option.innerHTML = '<input type="checkbox" value="' + val.replace(/"/g, '&quot;') + '" checked onchange="updateChipField(\'' + prefix + '\')"> <span class="chip-label">' + val.replace(/</g, '&lt;') + '</span>';
    container.appendChild(option);
    input.value = '';
    updateChipField(prefix);
}
