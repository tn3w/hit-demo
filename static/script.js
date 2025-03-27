document.addEventListener('DOMContentLoaded', () => {
    const THEME_STORAGE_KEY = 'highlightit-theme-preference';
    const STYLE_STORAGE_KEY = 'highlightit-style-preference';
    
    const storedTheme = localStorage.getItem(THEME_STORAGE_KEY) || 'auto';
    const storedStyle = localStorage.getItem(STYLE_STORAGE_KEY) || 'default';
    
    HighlightIt.init({
        debounceTime: 40
    });

    const styleSelector = document.getElementById('style-selector');
    const currentStyleInfo = document.getElementById('current-style-info');
    let currentStyleLink = null;
    
    function applySystemTheme() {
        const prefersDark = window.matchMedia && window.matchMedia('(prefers-color-scheme: dark)').matches;
        if (document.documentElement.classList.contains('highlightit-theme-auto')) {
            console.log(`Auto theme mode: System prefers ${prefersDark ? 'dark' : 'light'} mode`);
            
            document.documentElement.classList.remove('system-light-theme', 'system-dark-theme');
            document.documentElement.classList.add(prefersDark ? 'system-dark-theme' : 'system-light-theme');
        }
    }
    
    function applyStoredTheme(theme) {
        document.documentElement.classList.remove(
            'highlightit-theme-light',
            'highlightit-theme-dark',
            'highlightit-theme-auto'
        );
        
        document.documentElement.classList.add(`highlightit-theme-${theme}`);
        
        const themeButtons = document.querySelectorAll('.theme-selector button');
        themeButtons.forEach(btn => {
            btn.classList.remove('active');
            if (btn.getAttribute('data-theme') === theme) {
                btn.classList.add('active');
            }
        });
        
        if (theme === 'auto') {
            applySystemTheme();
        }
    }
    
    function applyStoredStyle(style) {
        styleSelector.value = style;
        
        loadHighlightStyle(style);
    }
    
    applyStoredTheme(storedTheme);
    
    if (document.documentElement.classList.contains('highlightit-theme-auto')) {
        console.log('Auto theme detected, using system preference');
        applySystemTheme();

        document.documentElement.classList.add('theme-updating');
        setTimeout(() => {
            document.documentElement.classList.remove('theme-updating');
        }, 10);
    }
    
    if (window.matchMedia) {
        const colorSchemeQuery = window.matchMedia('(prefers-color-scheme: dark)');
        
        if (colorSchemeQuery.addEventListener) {
            colorSchemeQuery.addEventListener('change', function(e) {
                handleThemeChange();
            });
        } 
        else {
            try {
                colorSchemeQuery.addEventListener('change', handleThemeChange);
            } catch (e) {
                console.warn('Could not add media query change listener:', e);
            }
        }
    }
    
    function handleThemeChange() {
        if (document.documentElement.classList.contains('highlightit-theme-auto')) {
            console.log('System preference changed, updating auto theme');
            applySystemTheme();
            document.documentElement.classList.add('theme-updating');
            setTimeout(() => {
                document.documentElement.classList.remove('theme-updating');
            }, 10);
            
            updateImportCode(styleSelector.value);
        }
    }
    
    window.addEventListener('load', function() {
        if (document.documentElement.classList.contains('highlightit-theme-auto')) {
            applySystemTheme();
        }
    });
    
    function loadHighlightStyle(styleName) {
        if (currentStyleLink) {
            try {
                if (currentStyleLink.parentNode) {
                    currentStyleLink.parentNode.removeChild(currentStyleLink);
                } else {
                    const existingLink = document.getElementById('highlight-style');
                    if (existingLink && existingLink.parentNode) {
                        existingLink.parentNode.removeChild(existingLink);
                    }
                }
            } catch (e) {
                console.warn('Failed to remove previous style:', e);
                const existingLink = document.getElementById('highlight-style');
                if (existingLink && existingLink.parentNode) {
                    existingLink.parentNode.removeChild(existingLink);
                }
            }
        }
        
        const importCodeContainer = document.getElementById('import-code-container');
        
        if (styleName === 'default') {
            currentStyleInfo.textContent = 'Currently using: Default style';
            if (importCodeContainer) {
                importCodeContainer.style.display = 'block';
            }
        } else {
            const styleLink = document.createElement('link');
            styleLink.rel = 'stylesheet';
            styleLink.href = `https://cdn.jsdelivr.net/npm/highlight-it@VERSION/dist/styles/${styleName}.min.css`;
            styleLink.id = 'highlight-style';
            
            document.head.appendChild(styleLink);
            currentStyleLink = styleLink;
            
            currentStyleInfo.textContent = `Currently using: ${styleName} style`;
        }
        
        localStorage.setItem(STYLE_STORAGE_KEY, styleName);
        
        updateImportCode(styleName);
    }
    
    function updateImportCode(styleName) {
        const importCodeDisplay = document.getElementById('import-code-display');
        if (!importCodeDisplay) return;
        
        let currentTheme;
        if (document.documentElement.classList.contains('highlightit-theme-light')) {
            currentTheme = 'light';
        } else if (document.documentElement.classList.contains('highlightit-theme-dark')) {
            currentTheme = 'dark';
        } else {
            currentTheme = 'auto';
        }
        
        let styleLink = ``
        
        if (styleName !== 'default') {
            styleLink = `<link rel="stylesheet" href="https://cdn.jsdelivr.net/npm/highlight-it@VERSION/dist/styles/${styleName}.min.css">\n`;
        }

        let htmlCode = `${styleLink}<script src="https://cdn.jsdelivr.net/npm/highlight-it@VERSION/dist/highlight-it-min.js"><\/script>
<script>
    window.addEventListener('load', () => {
        HighlightIt.init({
            theme: '${currentTheme}',
            // Your other options here
            debounceTime: 40
        });
    });
</script>

<pre class="highlight-it" data-language="javascript">
    // Your code will be automatically highlighted
</pre>`;
        
        importCodeDisplay.textContent = htmlCode;
    }
    
    styleSelector.addEventListener('change', function() {
        loadHighlightStyle(this.value);
    });
    
    const existingStyleLink = document.getElementById('highlight-style');
    if (existingStyleLink) {
        try {
            existingStyleLink.parentNode.removeChild(existingStyleLink);
        } catch (e) {
            console.warn('Failed to remove existing style on init:', e);
        }
    }
    
    applyStoredStyle(storedStyle);
    
    const themeButtons = document.querySelectorAll('.theme-selector button')

    themeButtons.forEach((button) => {
        button.addEventListener('click', () => {
            themeButtons.forEach((btn) => btn.classList.remove('active'))

            button.classList.add('active')

            const theme = button.getAttribute('data-theme')

            document.documentElement.classList.remove(
                'highlightit-theme-light',
                'highlightit-theme-dark',
                'highlightit-theme-auto'
            )

            document.documentElement.classList.add(`highlightit-theme-${theme}`)
            
            localStorage.setItem(THEME_STORAGE_KEY, theme);
            
            if (theme === 'auto') {
                applySystemTheme();
                document.documentElement.classList.add('theme-updating');
                setTimeout(() => {
                    document.documentElement.classList.remove('theme-updating');
                }, 10);
            }

            updateImportCode(styleSelector.value);
        })
    })
    
    const pythonCode = `# Python Fibonacci Sequence Generator

def fibonacci(n):
    """Generate the Fibonacci sequence up to n terms."""
    fib_sequence = [0, 1]
    
    # Generate the sequence
    for i in range(2, n):
        fib_sequence.append(fib_sequence[i-1] + fib_sequence[i-2])
    
    return fib_sequence

# Test the function with different values
def test_fibonacci():
    # Test with n = 10
    result = fibonacci(10)
    print(f"First 10 Fibonacci numbers: {result}")
    
    # Calculate sum of the sequence
    total = sum(result)
    print(f"Sum of the first 10 Fibonacci numbers: {total}")
    
    # Get the ratio of consecutive terms (approaches golden ratio)
    for i in range(len(result) - 1, 0, -1):
        ratio = result[i] / result[i-1] if result[i-1] != 0 else 0
        print(f"Ratio of term {i+1}/{i}: {ratio:.6f}")

# Main execution
if __name__ == "__main__":
    test_fibonacci()
    print("Fibonacci sequence calculation complete!")`;

    const lines = pythonCode.split('\n');
    const totalLines = lines.length;
    let currentLine = 0;
    let interval;
    let isPaused = false;
    
    let streamSpeed = parseInt(document.getElementById('speed-selector').value);
    
    const startBtn = document.getElementById('startBtn');
    const pauseBtn = document.getElementById('pauseBtn');
    const resetBtn = document.getElementById('resetBtn');
    const statusEl = document.getElementById('status');
    const progressBar = document.getElementById('progress');
    const currentLineEl = document.getElementById('current-line');
    const speedSelector = document.getElementById('speed-selector');
    
    function updateLineCounter() {
        currentLineEl.textContent = `Line: ${currentLine}/${totalLines}`;
        
        const progressPercent = (currentLine / totalLines) * 100;
        progressBar.style.width = `${progressPercent}%`;
    }
    
    speedSelector.addEventListener('change', function() {
        streamSpeed = parseInt(this.value);
        
        if (!isPaused && interval) {
            clearInterval(interval);
            startStreaming();
        }
    });
    
    resetStream();
    
    function getCodeElement() {
        return document.getElementById('python-code');
    }
    
    function startStreaming() {
        interval = setInterval(function() {
            if (currentLine < totalLines) {
                const codeElement = getCodeElement();
                let content = codeElement.textContent || '';
                
                if (currentLine === 0) {
                    content = lines[currentLine];
                } else {
                    content += '\n' + lines[currentLine];
                }
                
                codeElement.textContent = content;
                
                currentLine++;
                updateLineCounter();
                statusEl.textContent = `Streaming line ${currentLine} of ${totalLines}...`;
            } else {
                clearInterval(interval);
                statusEl.textContent = 'Streaming complete!';
                startBtn.disabled = true;
                pauseBtn.disabled = true;
            }
        }, streamSpeed);
    }
    
    startBtn.addEventListener('click', function() {
        if (currentLine >= totalLines) {
            resetStream();
        }
        
        startBtn.disabled = true;
        pauseBtn.disabled = false;
        isPaused = false;
        
        statusEl.textContent = 'Streaming...';
        startStreaming();
    });
    
    pauseBtn.addEventListener('click', function() {
        clearInterval(interval);
        isPaused = true;
        startBtn.disabled = false;
        pauseBtn.disabled = true;
        statusEl.textContent = 'Streaming paused.';
    });
    
    resetBtn.addEventListener('click', resetStream);
    
    function resetStream() {
        clearInterval(interval);
        currentLine = 0;
        
        const codeElement = getCodeElement();
        if (codeElement) {
            codeElement.textContent = '# Python code will be streamed here';
        }
        
        startBtn.disabled = false;
        pauseBtn.disabled = true;
        statusEl.textContent = 'Ready to stream.';
        updateLineCounter();
    }
    
    const addElementBtn = document.getElementById('addElementBtn');
    const clearElementsBtn = document.getElementById('clearElementsBtn');
    const dynamicContainer = document.getElementById('dynamicContainer');
    
    let counter = 1;
    
    const codeSnippets = [
        {
            language: 'javascript',
            code: `// JavaScript Example
function factorial(n) {
    if (n <= 1) return 1;
    return n * factorial(n - 1);
}

console.log(factorial(5)); // 120`
        },
        {
            language: 'python',
            code: `# Python Example
def quick_sort(arr):
    if len(arr) <= 1:
        return arr
    pivot = arr[len(arr) // 2]
    left = [x for x in arr if x < pivot]
    middle = [x for x in arr if x == pivot]
    right = [x for x in arr if x > pivot]
    return quick_sort(left) + middle + quick_sort(right)

print(quick_sort([3, 6, 8, 10, 1, 2, 1]))`
        },
        {
            language: 'html',
            code: `<!-- HTML Example -->
<!DOCTYPE html>
<html>
    <head>
        <title>Sample Page</title>
    </head>
    <body>
        <h1>Hello World</h1>
        <p>This is a paragraph.</p>
    </body>
</html>`
        },
        {
            language: 'css',
            code: `/* CSS Example */
.container {
    display: flex;
    justify-content: center;
    align-items: center;
    height: 100vh;
    background-color: #f0f0f0;
}

.box {
    width: 200px;
    height: 200px;
    background-color: #3498db;
    border-radius: 8px;
    box-shadow: 0 4px 8px rgba(0, 0, 0, 0.1);
}`
        }
    ];
    
    addElementBtn.addEventListener('click', function() {
        const snippet = codeSnippets[Math.floor(Math.random() * codeSnippets.length)];
        
        const preElement = document.createElement('pre');
        const uniqueId = `dynamic-code-${counter++}`;
        preElement.id = uniqueId;
        preElement.textContent = snippet.code;
        
        dynamicContainer.appendChild(preElement);
        
        HighlightIt.highlight(preElement, {
            language: snippet.language,
            withLines: true,
            withReload: true,
            addCopyButton: true,
            showLanguage: true,
            autoDetect: true
        });
        
        const message = document.createElement('div');
        message.style.marginBottom = '20px';
        message.style.fontSize = '14px';
        message.style.color = 'var(--hl-text, #d4d4d4)';
        message.innerHTML = `<strong>Element created:</strong> ID: ${uniqueId} with ${snippet.language} highlighting`;
        
        const updateButton = document.createElement('button');
        updateButton.textContent = 'Update Content';
        updateButton.style.marginLeft = '10px';
        updateButton.style.fontSize = '12px';
        updateButton.style.padding = '3px 8px';
        updateButton.style.backgroundColor = 'var(--hl-header-bg, #2d2d2d)';
        updateButton.style.color = 'var(--hl-text, #d4d4d4)';
        updateButton.style.border = '1px solid var(--hl-border, #3d3d3d)';
        updateButton.style.borderRadius = '4px';
        updateButton.style.cursor = 'pointer';
        updateButton.addEventListener('click', function() {
            const originalElement = document.getElementById(uniqueId);
            if (originalElement) {
                const timestamp = new Date().toLocaleTimeString();
                const updatedCode = snippet.code + `\n\n// Updated at: ${timestamp}`;
                originalElement.textContent = updatedCode;
                
                updateButton.textContent = 'Updated!';
                setTimeout(() => {
                    updateButton.textContent = 'Update Content';
                }, 1000);
            }
        });
        
        message.appendChild(updateButton);
        dynamicContainer.appendChild(message);
    });
    
    clearElementsBtn.addEventListener('click', function() {
        dynamicContainer.innerHTML = '';
        counter = 1;
    });

    const globalConfigDemo = document.getElementById('globalConfigDemo');
    
    const configurations = [
        {
            title: "Default Configuration",
            description: "addHeader: true, addCopyButton: true, addLines: false",
            options: {
                addHeader: true,
                addCopyButton: true,
                addLines: false
            },
            code: "// This is default configuration\nfunction example() {\n    return 'Default config';\n}"
        },
        {
            title: "No Header Configuration",
            description: "addHeader: false, addCopyButton: true, addLines: false",
            options: {
                addHeader: false,
                addCopyButton: true,
                addLines: false
            },
            code: "// This has no header but has a floating copy button\nfunction example() {\n    return 'No header config';\n}"
        },
        {
            title: "With Line Numbers Configuration",
            description: "addHeader: true, addCopyButton: true, addLines: true",
            options: {
                addHeader: true,
                addCopyButton: true,
                addLines: true
            },
            code: "// This has line numbers enabled\nfunction example() {\n    return 'With line numbers';\n}"
        },
        {
            title: "No Copy Button Configuration",
            description: "addHeader: true, addCopyButton: false, addLines: false",
            options: {
                addHeader: true,
                addCopyButton: false,
                addLines: false
            },
            code: "// This has no copy button\nfunction example() {\n    return 'No copy button';\n}"
        },
        {
            title: "Minimal Configuration",
            description: "addHeader: false, addCopyButton: false, addLines: false",
            options: {
                addHeader: false,
                addCopyButton: false,
                addLines: false
            },
            code: "// This has no header and no copy button\nfunction example() {\n    return 'Minimal config';\n}"
        },
        {
            title: "Full Configuration",
            description: "addHeader: true, addCopyButton: true, addLines: true",
            options: {
                addHeader: true,
                addCopyButton: true,
                addLines: true
            },
            code: "// This has everything enabled\nfunction example() {\n    return 'Full config';\n}"
        }
    ];
    
    configurations.forEach(config => {
        const container = document.createElement('div');
        container.style.marginBottom = '30px';
        
        const title = document.createElement('h3');
        title.textContent = config.title;
        title.style.marginBottom = '10px';
        
        const description = document.createElement('p');
        description.textContent = config.description;
        description.style.marginBottom = '10px';
        
        const codeElement = document.createElement('pre');
        codeElement.textContent = config.code;
        
        container.appendChild(title);
        container.appendChild(description);
        container.appendChild(codeElement);
        
        globalConfigDemo.appendChild(container);
        
        HighlightIt.highlight(codeElement, {
            language: 'javascript',
            ...config.options
        });
    });
})