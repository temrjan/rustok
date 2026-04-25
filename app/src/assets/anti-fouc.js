// Anti-FOUC: applies the persisted light theme before the first paint so the
// initial render matches the user's preference. Loaded from an external file
// (not inline) to satisfy the Tauri CSP `script-src 'self'` policy.
(function () {
    try {
        var t = localStorage.getItem('rustok.theme');
        if (t === 'light') {
            document.documentElement.setAttribute('data-theme', 'light');
        }
    } catch (e) {}
})();
