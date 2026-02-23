// Brag Frog — Entry detail/edit toggle helpers

function toggleEntryDetail(entryId) {
    var detail = document.getElementById('entry-detail-' + entryId.replace('entry-', ''));
    if (detail) {
        detail.classList.toggle('hidden');
    }
}

function toggleEntryEdit(entryId) {
    var v = document.getElementById('entry-meta-view-' + entryId);
    var e = document.getElementById('entry-meta-edit-' + entryId);
    var c = document.getElementById('entry-content-view-' + entryId);
    if (v) v.classList.add('hidden');
    if (c) c.classList.add('hidden');
    if (e) e.classList.remove('hidden');
}
