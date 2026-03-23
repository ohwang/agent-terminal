// ---- State ----
let castFrames = [];     // [{time: float_seconds, data: string}]
let textFrames = [];     // [{timestamp_ms, text, cols, rows, cursor_row, cursor_col}]
let actions = [];        // [{timestamp_ms, command, args}]
let meta = null;
let mode = 'visual';     // 'visual' | 'text'
let playing = false;
let speed = 1;
let currentTime = 0;     // seconds
let totalDuration = 0;   // seconds
let animFrameId = null;
let lastRealTime = 0;

// ---- Init ----
(async function init() {
    const params = new URLSearchParams(window.location.search);
    const group = params.get('group');
    const name = params.get('name');
    if (!group || !name) {
        document.getElementById('player-title').textContent = 'Missing recording parameters';
        return;
    }
    const base = `/api/recording/${encodeURIComponent(group)}/${encodeURIComponent(name)}`;

    // Load all data in parallel
    const [metaRes, castRes, framesRes, actionsRes] = await Promise.all([
        fetch(`${base}/meta`),
        fetch(`${base}/cast`),
        fetch(`${base}/frames`),
        fetch(`${base}/actions`),
    ]);

    meta = await metaRes.json();

    // Parse .cast file
    const castText = await castRes.text();
    const castLines = castText.trim().split('\n');
    if (castLines.length > 1) {
        // Skip header (line 0), parse events
        for (let i = 1; i < castLines.length; i++) {
            try {
                const evt = JSON.parse(castLines[i]);
                castFrames.push({ time: evt[0], data: evt[2] });
            } catch(e) {}
        }
    }

    // Parse frames.jsonl
    const framesText = await framesRes.text();
    for (const line of framesText.trim().split('\n')) {
        if (!line) continue;
        try { textFrames.push(JSON.parse(line)); } catch(e) {}
    }

    // Parse actions.jsonl
    const actionsText = await actionsRes.text();
    for (const line of actionsText.trim().split('\n')) {
        if (!line) continue;
        try { actions.push(JSON.parse(line)); } catch(e) {}
    }

    // Determine total duration
    if (meta.duration_ms) {
        totalDuration = meta.duration_ms / 1000;
    } else if (castFrames.length > 0) {
        totalDuration = castFrames[castFrames.length - 1].time + 0.5;
    }

    // Update UI
    const label = meta.label ? ` [${meta.label}]` : '';
    document.getElementById('player-title').textContent = `${meta.session}${label}`;
    document.getElementById('player-meta').textContent =
        `Group: ${meta.group} | ${meta.cols}x${meta.rows} | ${meta.frame_count || 0} frames | ${formatTime(totalDuration)}`;

    // Render actions list
    renderActions();

    // Show first frame
    renderFrame(0);
    updateTimeDisplay();
})();

// ---- ANSI to HTML ----
const COLORS_BASIC = [
    '#000000','#cd3131','#0dbc79','#e5e510','#2472c8','#bc3fbc','#11a8cd','#e5e5e5'
];
const COLORS_BRIGHT = [
    '#666666','#f14c4c','#23d18b','#f5f543','#3b8eea','#d670d6','#29b8db','#ffffff'
];

function color256(n) {
    if (n < 8) return COLORS_BASIC[n];
    if (n < 16) return COLORS_BRIGHT[n - 8];
    if (n < 232) {
        n -= 16;
        const r = Math.floor(n / 36) * 51;
        const g = Math.floor((n % 36) / 6) * 51;
        const b = (n % 6) * 51;
        return `rgb(${r},${g},${b})`;
    }
    const v = 8 + (n - 232) * 10;
    return `rgb(${v},${v},${v})`;
}

function ansiToHtml(text) {
    let html = '';
    let fg = null, bg = null;
    let bold = false, dim = false, italic = false, underline = false, strikethrough = false, reverse = false;

    const lines = text.split('\n');
    for (let li = 0; li < lines.length; li++) {
        if (li > 0) html += '\n';
        const line = lines[li];
        let i = 0;
        while (i < line.length) {
            if (line[i] === '\x1b' && line[i+1] === '[') {
                // Parse CSI sequence
                let j = i + 2;
                while (j < line.length && !((line.charCodeAt(j) >= 0x40 && line.charCodeAt(j) <= 0x7e))) j++;
                if (j < line.length && line[j] === 'm') {
                    const params = line.substring(i + 2, j).split(';').map(Number);
                    applySGR(params);
                }
                i = j + 1;
            } else if (line[i] === '\x1b') {
                // Skip other escape sequences
                let j = i + 1;
                if (j < line.length && line[j] === ']') {
                    // OSC: skip until ST
                    while (j < line.length && line[j] !== '\x07' && !(line[j] === '\x1b' && line[j+1] === '\\')) j++;
                    i = j + 1;
                } else {
                    while (j < line.length && !((line.charCodeAt(j) >= 0x40 && line.charCodeAt(j) <= 0x7e))) j++;
                    i = j + 1;
                }
            } else {
                // Collect plain text run
                let j = i;
                while (j < line.length && line[j] !== '\x1b') j++;
                const chunk = line.substring(i, j);
                html += openSpan() + escHtml(chunk) + '</span>';
                i = j;
            }
        }
    }
    return html;

    function applySGR(params) {
        for (let k = 0; k < params.length; k++) {
            const p = params[k];
            if (p === 0 || isNaN(p)) { fg = bg = null; bold = dim = italic = underline = strikethrough = reverse = false; }
            else if (p === 1) bold = true;
            else if (p === 2) dim = true;
            else if (p === 3) italic = true;
            else if (p === 4) underline = true;
            else if (p === 7) reverse = true;
            else if (p === 9) strikethrough = true;
            else if (p === 22) { bold = false; dim = false; }
            else if (p === 23) italic = false;
            else if (p === 24) underline = false;
            else if (p === 27) reverse = false;
            else if (p === 29) strikethrough = false;
            else if (p >= 30 && p <= 37) fg = COLORS_BASIC[p - 30];
            else if (p === 38) {
                if (params[k+1] === 5) { fg = color256(params[k+2]); k += 2; }
                else if (params[k+1] === 2) { fg = `rgb(${params[k+2]},${params[k+3]},${params[k+4]})`; k += 4; }
            }
            else if (p === 39) fg = null;
            else if (p >= 40 && p <= 47) bg = COLORS_BASIC[p - 40];
            else if (p === 48) {
                if (params[k+1] === 5) { bg = color256(params[k+2]); k += 2; }
                else if (params[k+1] === 2) { bg = `rgb(${params[k+2]},${params[k+3]},${params[k+4]})`; k += 4; }
            }
            else if (p === 49) bg = null;
            else if (p >= 90 && p <= 97) fg = COLORS_BRIGHT[p - 90];
            else if (p >= 100 && p <= 107) bg = COLORS_BRIGHT[p - 100];
        }
    }

    function openSpan() {
        const classes = [];
        const styles = [];
        if (bold) classes.push('bold');
        if (dim) classes.push('dim');
        if (italic) classes.push('italic');
        if (underline) classes.push('underline');
        if (strikethrough) classes.push('strikethrough');
        if (reverse) classes.push('reverse');
        if (fg) styles.push(`color:${fg}`);
        if (bg) styles.push(`background-color:${bg}`);
        let s = '<span';
        if (classes.length) s += ` class="${classes.join(' ')}"`;
        if (styles.length) s += ` style="${styles.join(';')}"`;
        return s + '>';
    }
}

function escHtml(s) {
    return s.replace(/&/g, '&amp;').replace(/</g, '&lt;').replace(/>/g, '&gt;');
}

// ---- Rendering ----
function renderFrame(timeSec) {
    const terminal = document.getElementById('terminal');
    if (mode === 'visual') {
        // Find the latest cast frame at or before timeSec
        let frame = null;
        for (let i = castFrames.length - 1; i >= 0; i--) {
            if (castFrames[i].time <= timeSec) { frame = castFrames[i]; break; }
        }
        if (frame) {
            // Strip clear-screen + home prefix for rendering
            let data = frame.data;
            if (data.startsWith('\x1b[2J\x1b[H')) data = data.substring(7);
            terminal.innerHTML = ansiToHtml(data);
        } else if (castFrames.length > 0) {
            let data = castFrames[0].data;
            if (data.startsWith('\x1b[2J\x1b[H')) data = data.substring(7);
            terminal.innerHTML = ansiToHtml(data);
        } else {
            terminal.textContent = '(no frames)';
        }
    } else {
        // Text mode: find nearest text frame
        const timeMs = timeSec * 1000;
        let frame = null;
        for (let i = textFrames.length - 1; i >= 0; i--) {
            if (textFrames[i].timestamp_ms <= timeMs) { frame = textFrames[i]; break; }
        }
        if (frame) {
            terminal.textContent = frame.text;
        } else if (textFrames.length > 0) {
            terminal.textContent = textFrames[0].text;
        } else {
            terminal.textContent = '(no frames)';
        }
    }

    // Highlight current action
    highlightAction(timeSec * 1000);
}

function highlightAction(timeMs) {
    const items = document.querySelectorAll('.action-item');
    let activeIdx = -1;
    for (let i = actions.length - 1; i >= 0; i--) {
        if (actions[i].timestamp_ms <= timeMs) { activeIdx = i; break; }
    }
    items.forEach((el, i) => {
        el.classList.toggle('active', i === activeIdx);
    });
    // Scroll active into view
    if (activeIdx >= 0 && items[activeIdx]) {
        items[activeIdx].scrollIntoView({ block: 'nearest', behavior: 'smooth' });
    }
}

function renderActions() {
    const list = document.getElementById('actions-list');
    if (actions.length === 0) return;
    let html = '';
    for (const a of actions) {
        const t = formatTime(a.timestamp_ms / 1000);
        const args = a.args && a.args.length ? ' ' + a.args.map(escHtml).join(' ') : '';
        html += `<div class="action-item" onclick="seekTo(${a.timestamp_ms / 1000})">`;
        html += `<span class="action-time">${t}</span>`;
        html += `<span class="action-cmd">${escHtml(a.command)}</span>`;
        html += `<span class="action-args">${args}</span>`;
        html += `</div>`;
    }
    list.innerHTML = html;
}

// ---- Playback ----
function togglePlay() {
    if (playing) pause(); else play();
}

function play() {
    if (currentTime >= totalDuration) currentTime = 0;
    playing = true;
    lastRealTime = performance.now();
    document.getElementById('btn-play').textContent = 'Pause';
    tick();
}

function pause() {
    playing = false;
    document.getElementById('btn-play').textContent = 'Play';
    if (animFrameId) { cancelAnimationFrame(animFrameId); animFrameId = null; }
}

function tick() {
    if (!playing) return;
    const now = performance.now();
    const delta = (now - lastRealTime) / 1000 * speed;
    lastRealTime = now;
    currentTime += delta;

    if (currentTime >= totalDuration) {
        currentTime = totalDuration;
        renderFrame(currentTime);
        updateTimeDisplay();
        pause();
        return;
    }

    renderFrame(currentTime);
    updateTimeDisplay();
    animFrameId = requestAnimationFrame(tick);
}

function seekTimeline(e) {
    const rect = document.getElementById('timeline').getBoundingClientRect();
    const ratio = Math.max(0, Math.min(1, (e.clientX - rect.left) / rect.width));
    currentTime = ratio * totalDuration;
    renderFrame(currentTime);
    updateTimeDisplay();
}

function seekTo(timeSec) {
    currentTime = timeSec;
    renderFrame(currentTime);
    updateTimeDisplay();
}

function setSpeed(v) { speed = parseFloat(v); }

function setMode(m) {
    mode = m;
    document.getElementById('mode-visual').classList.toggle('active', m === 'visual');
    document.getElementById('mode-text').classList.toggle('active', m === 'text');
    renderFrame(currentTime);
}

function updateTimeDisplay() {
    const progress = totalDuration > 0 ? (currentTime / totalDuration) * 100 : 0;
    document.getElementById('timeline-progress').style.width = progress + '%';
    document.getElementById('time-display').textContent =
        `${formatTime(currentTime)} / ${formatTime(totalDuration)}`;
}

function formatTime(sec) {
    const m = Math.floor(sec / 60);
    const s = Math.floor(sec % 60);
    return `${m}:${s.toString().padStart(2, '0')}`;
}

// Keyboard shortcuts
document.addEventListener('keydown', (e) => {
    if (e.key === ' ') { e.preventDefault(); togglePlay(); }
    if (e.key === 'ArrowLeft') { currentTime = Math.max(0, currentTime - 1); renderFrame(currentTime); updateTimeDisplay(); }
    if (e.key === 'ArrowRight') { currentTime = Math.min(totalDuration, currentTime + 1); renderFrame(currentTime); updateTimeDisplay(); }
});
