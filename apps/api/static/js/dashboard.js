// Brag Frog — Dashboard: quick-add, OKR snapshot, chip system

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
