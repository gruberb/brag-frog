// Brag Frog — User dropdown toggle + click-outside close

function toggleUserDropdown() {
    var dropdown = document.getElementById('user-dropdown');
    dropdown.classList.toggle('open');
}

document.addEventListener('click', function(event) {
    var dropdown = document.getElementById('user-dropdown');
    if (dropdown && !dropdown.contains(event.target)) {
        dropdown.classList.remove('open');
    }
});
