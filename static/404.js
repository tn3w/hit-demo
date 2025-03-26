document.addEventListener('DOMContentLoaded', function() {
    HighlightIt.init();

    const themePreference = localStorage.getItem('highlightit-theme-preference') || 'auto';
    const stylePreference = localStorage.getItem('highlightit-style-preference') || 'default';
    
    document.documentElement.classList.remove('highlightit-theme-light', 'highlightit-theme-dark', 'highlightit-theme-auto');
    document.documentElement.classList.add(`highlightit-theme-${themePreference}`);
    
    if (themePreference === 'auto') {
        applySystemPreference();
        
        if (window.matchMedia) {
            const mediaQuery = window.matchMedia('(prefers-color-scheme: dark)');
            if (mediaQuery.addEventListener) {
                mediaQuery.addEventListener('change', applySystemPreference);
            } else {
                try {
                    mediaQuery.addListener(applySystemPreference);
                } catch (err) {
                    console.warn('Could not add media query change listener:', err);
                }
            }
        }
    }
    
    if (stylePreference !== 'default') {
        loadCustomStyle(stylePreference);
    }
    
    function applySystemPreference() {
        if (document.documentElement.classList.contains('highlightit-theme-auto')) {
            const prefersDark = window.matchMedia && window.matchMedia('(prefers-color-scheme: dark)').matches;
            document.documentElement.classList.remove('system-light-theme', 'system-dark-theme');
            document.documentElement.classList.add(prefersDark ? 'system-dark-theme' : 'system-light-theme');
        }
    }
    
    function loadCustomStyle(style) {
        const existingStyle = document.getElementById('highlight-style');
        if (existingStyle) {
            try {
                existingStyle.parentNode.removeChild(existingStyle);
            } catch (err) {
                console.warn('Failed to remove existing style:', err);
            }
        }
        
        const styleLink = document.createElement('link');
        styleLink.rel = 'stylesheet';
        styleLink.href = `https://cdn.jsdelivr.net/npm/highlight-it@VERSION/dist/styles/${style}.min.css`;
        styleLink.id = 'highlight-style';
        document.head.appendChild(styleLink);
    }
});