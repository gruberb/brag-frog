// Brag Frog — Button flash helper + toast notifications

function flashButtonResult(btn, event) {
    if (!btn) return;
    var label = btn.querySelector('.btn-label');
    if (!label) return;
    var original = label.textContent;
    var ok = event.detail.successful;
    label.textContent = ok ? '\u2713' : '\u2717';
    if (!ok) label.style.color = 'var(--color-error)';
    setTimeout(function() {
        label.textContent = original;
        label.style.color = '';
    }, 5000);
}

// Floating toast notification — appears at top center, auto-dismisses after 2s.
// type: 'success' (green) or 'error' (red)
function showToast(message, type) {
    var toast = document.createElement('div');
    toast.className = 'toast toast-' + (type || 'success');
    toast.textContent = message;
    document.body.appendChild(toast);
    setTimeout(function() {
        toast.classList.add('toast-exit');
        toast.addEventListener('animationend', function() { toast.remove(); });
    }, 2000);
}
