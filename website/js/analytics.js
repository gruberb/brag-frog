// Analytics placeholder — no-op by default.
// To add analytics, create website/js/analytics.local.js with your snippet.
// That file is gitignored and will be loaded automatically.
(function() {
  var s = document.createElement('script');
  s.src = 'js/analytics.local.js';
  s.async = true;
  s.onerror = function() {}; // silently ignore if local file doesn't exist
  document.head.appendChild(s);
})();
