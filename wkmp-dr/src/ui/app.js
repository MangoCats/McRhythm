// WKMP Database Review - Client Application
// [REQ-DR-F-090]: Save favorite searches
// [REQ-DR-F-100]: User preference persistence (localStorage)

const STORAGE_KEY = 'wkmp-dr-preferences';

// ============================================================================
// API Authentication Functions
// Per SPEC007 API-AUTH-025, API-AUTH-027, API-AUTH-028-A
// ============================================================================

/**
 * Convert JavaScript object to canonical JSON string
 * (sorted keys, no whitespace)
 */
function toCanonicalJSON(obj) {
    if (obj === null) return 'null';
    if (typeof obj === 'boolean') return obj.toString();
    if (typeof obj === 'number') return obj.toString();
    if (typeof obj === 'string') {
        const escaped = obj.replace(/\\/g, '\\\\').replace(/"/g, '\\"');
        return `"${escaped}"`;
    }
    if (Array.isArray(obj)) {
        const items = obj.map(toCanonicalJSON);
        return `[${items.join(',')}]`;
    }
    if (typeof obj === 'object') {
        const keys = Object.keys(obj).sort();
        const pairs = keys.map(k => `"${k}":${toCanonicalJSON(obj[k])}`);
        return `{${pairs.join(',')}}`;
    }
    return 'null';
}

/**
 * Calculate SHA-256 hash for API authentication
 * Per SPEC007 API-AUTH-027
 */
async function calculateHash(jsonObj, sharedSecret) {
    // Step 1: Clone object and replace hash with dummy hash (64 zeros)
    const objWithDummyHash = { ...jsonObj };
    objWithDummyHash.hash = '0000000000000000000000000000000000000000000000000000000000000000';

    // Step 2: Convert to canonical JSON (sorted keys, no whitespace)
    const canonical = toCanonicalJSON(objWithDummyHash);

    // Step 3: Append shared secret as decimal string
    const toHash = canonical + sharedSecret.toString();

    // Step 4: Calculate SHA-256 using SubtleCrypto API
    const encoder = new TextEncoder();
    const data = encoder.encode(toHash);
    const hashBuffer = await crypto.subtle.digest('SHA-256', data);

    // Step 5: Convert to 64 hex characters
    const hashArray = Array.from(new Uint8Array(hashBuffer));
    const hashHex = hashArray.map(b => b.toString(16).padStart(2, '0')).join('');

    return hashHex;
}

/**
 * Wrapper for fetch() that adds timestamp and hash authentication
 * Per SPEC007 API-AUTH-025
 */
async function authenticatedFetch(url, options = {}) {
    // Check if authentication is disabled (shared_secret = 0)
    if (API_SHARED_SECRET === "0") {
        // Bypass mode: send dummy hash
        const timestamp = Date.now();
        const dummyHash = '0000000000000000000000000000000000000000000000000000000000000000';

        if (options.method && (options.method === 'POST' || options.method === 'PUT')) {
            // Add to body for POST/PUT
            if (options.body) {
                const bodyObj = JSON.parse(options.body);
                bodyObj.timestamp = timestamp;
                bodyObj.hash = dummyHash;
                options.body = JSON.stringify(bodyObj);
            } else {
                options.body = JSON.stringify({ timestamp, hash: dummyHash });
            }
        } else {
            // Add to query string for GET/DELETE
            const separator = url.includes('?') ? '&' : '?';
            url = `${url}${separator}timestamp=${timestamp}&hash=${dummyHash}`;
        }

        return fetch(url, options);
    }

    // Normal authenticated mode
    const timestamp = Date.now();

    if (options.method && (options.method === 'POST' || options.method === 'PUT')) {
        // POST/PUT: Add timestamp and hash to body
        let bodyObj = {};
        if (options.body) {
            bodyObj = JSON.parse(options.body);
        }
        bodyObj.timestamp = timestamp;

        // Calculate hash
        const hash = await calculateHash(bodyObj, API_SHARED_SECRET);
        bodyObj.hash = hash;

        options.body = JSON.stringify(bodyObj);
        options.headers = options.headers || {};
        options.headers['Content-Type'] = 'application/json';

        return fetch(url, options);
    } else {
        // GET/DELETE: Add timestamp and hash to query string
        const queryObj = { timestamp, hash: 'dummy' };
        const hash = await calculateHash(queryObj, API_SHARED_SECRET);

        const separator = url.includes('?') ? '&' : '?';
        url = `${url}${separator}timestamp=${timestamp}&hash=${hash}`;

        return fetch(url, options);
    }
}

// ============================================================================
// Application State and UI
// ============================================================================

// Current state
let currentData = null;
let currentPage = 1;
let currentView = {
    type: 'table',
    params: { table: 'songs' }
};

// Preference Management
class PreferenceManager {
    constructor() {
        this.preferences = this.load();
    }

    load() {
        try {
            const stored = localStorage.getItem(STORAGE_KEY);
            if (stored) {
                return JSON.parse(stored);
            }
        } catch (e) {
            console.error('Failed to load preferences:', e);
        }
        return {
            savedSearches: [],
            lastView: null,
            version: 1
        };
    }

    save() {
        try {
            localStorage.setItem(STORAGE_KEY, JSON.stringify(this.preferences));
            return true;
        } catch (e) {
            console.error('Failed to save preferences:', e);
            showStatus('Failed to save preferences', 'error');
            return false;
        }
    }

    addSavedSearch(name, viewConfig) {
        const search = {
            id: Date.now().toString(),
            name: name,
            view: viewConfig,
            savedAt: new Date().toISOString()
        };

        // Check for duplicate names
        const existing = this.preferences.savedSearches.findIndex(s => s.name === name);
        if (existing >= 0) {
            this.preferences.savedSearches[existing] = search;
        } else {
            this.preferences.savedSearches.push(search);
        }

        this.save();
        renderFavorites();
        showStatus(`Saved search "${name}"`, 'success');
    }

    removeSavedSearch(id) {
        this.preferences.savedSearches = this.preferences.savedSearches.filter(s => s.id !== id);
        this.save();
        renderFavorites();
    }

    clearAll() {
        if (confirm('Clear all saved searches?')) {
            this.preferences.savedSearches = [];
            this.save();
            renderFavorites();
            showStatus('All saved searches cleared', 'success');
        }
    }

    exportPreferences() {
        const dataStr = JSON.stringify(this.preferences, null, 2);
        const dataBlob = new Blob([dataStr], { type: 'application/json' });
        const url = URL.createObjectURL(dataBlob);
        const link = document.createElement('a');
        link.href = url;
        link.download = 'wkmp-dr-preferences.json';
        link.click();
        URL.revokeObjectURL(url);
        showStatus('Preferences exported', 'success');
    }
}

const prefs = new PreferenceManager();

// UI Helpers
function showStatus(message, type = 'success') {
    const statusEl = document.getElementById('status');
    statusEl.textContent = message;
    statusEl.className = `status ${type}`;
    statusEl.classList.remove('hidden');
    setTimeout(() => {
        statusEl.classList.add('hidden');
    }, 3000);
}

function renderFavorites() {
    const container = document.getElementById('favoriteButtons');
    const searches = prefs.preferences.savedSearches;

    if (searches.length === 0) {
        container.innerHTML = '<em style="color: var(--secondary-color);">No saved searches yet</em>';
        return;
    }

    container.innerHTML = searches.map(search => `
        <button class="favorite-btn" data-id="${search.id}">${search.name}</button>
    `).join('');

    // Add click handlers
    container.querySelectorAll('.favorite-btn').forEach(btn => {
        btn.addEventListener('click', () => {
            const id = btn.dataset.id;
            const search = searches.find(s => s.id === id);
            if (search) {
                loadSavedSearch(search);
            }
        });
    });
}

function loadSavedSearch(search) {
    currentView = search.view;
    currentPage = 1;

    // Update UI to match saved search
    const viewType = document.getElementById('viewType');
    viewType.value = search.view.type;
    updateViewControls();

    if (search.view.type === 'table') {
        document.getElementById('tableName').value = search.view.params.table;
    } else if (search.view.type === 'search-work') {
        document.getElementById('workId').value = search.view.params.workId;
    } else if (search.view.type === 'search-path') {
        document.getElementById('pathPattern').value = search.view.params.pattern;
    }

    loadData();
}

function updateViewControls() {
    const viewType = document.getElementById('viewType').value;

    // Hide all optional groups
    document.getElementById('tableGroup').classList.add('hidden');
    document.getElementById('workIdGroup').classList.add('hidden');
    document.getElementById('pathPatternGroup').classList.add('hidden');

    // Show relevant group
    if (viewType === 'table') {
        document.getElementById('tableGroup').classList.remove('hidden');
    } else if (viewType === 'search-work') {
        document.getElementById('workIdGroup').classList.remove('hidden');
    } else if (viewType === 'search-path') {
        document.getElementById('pathPatternGroup').classList.remove('hidden');
    }
}

// API Functions
async function loadData() {
    const viewType = document.getElementById('viewType').value;

    // Build current view config
    currentView = { type: viewType, params: {} };

    let url;
    if (viewType === 'table') {
        const table = document.getElementById('tableName').value;
        currentView.params.table = table;
        url = `/api/table/${table}?page=${currentPage}`;
    } else if (viewType === 'filter-passages') {
        url = `/api/filters/passages-without-mbid?page=${currentPage}`;
    } else if (viewType === 'filter-files') {
        url = `/api/filters/files-without-passages?page=${currentPage}`;
    } else if (viewType === 'search-work') {
        const workId = document.getElementById('workId').value.trim();
        if (!workId) {
            showStatus('Please enter a Work ID', 'error');
            return;
        }
        currentView.params.workId = workId;
        url = `/api/search/by-work-id?work_id=${encodeURIComponent(workId)}&page=${currentPage}`;
    } else if (viewType === 'search-path') {
        const pattern = document.getElementById('pathPattern').value.trim();
        if (!pattern) {
            showStatus('Please enter a path pattern', 'error');
            return;
        }
        currentView.params.pattern = pattern;
        url = `/api/search/by-path?pattern=${encodeURIComponent(pattern)}&page=${currentPage}`;
    }

    try {
        const response = await authenticatedFetch(url);
        const data = await response.json();

        if (data.error) {
            showStatus(data.error, 'error');
            return;
        }

        currentData = data;
        renderTable(data);
        updatePagination(data);
    } catch (error) {
        showStatus('Failed to load data: ' + error.message, 'error');
    }
}

// REQ-F-001: SPEC017 tick-based timing constants and conversion
// Per SPEC017 SRC-LAYER-011: Developer UI displays both ticks AND seconds
const TICK_RATE = 28224000;  // Hz, LCM of 11 sample rates
const TIMING_COLUMNS = [
    'start_time_ticks',
    'end_time_ticks',
    'fade_in_start_ticks',
    'fade_out_start_ticks',
    'lead_in_start_ticks',
    'lead_out_start_ticks',
    'duration_ticks'
];

// SPEC024: Human-readable time display - cooldown columns (seconds in database)
const COOLDOWN_COLUMNS = {
    // Songs: 7-14 day cooldowns (604800-1209600 seconds) → Extended format
    'min_cooldown': 1209600,      // Typical max: 14 days
    'ramping_cooldown': 1209600,  // Typical max: 14 days
};

// Float columns (REAL type in database) - always display with at least 1 decimal place
const FLOAT_COLUMNS = [
    'base_probability',     // songs, artists, works tables
    'weight',               // related_songs table (and other tables)
    'progress_percentage',  // import_progress table
];

/**
 * Convert ticks to seconds with 3 decimal places (millisecond precision).
 * @param {number} ticks - Tick value (i64 from database)
 * @returns {string|null} Seconds formatted as "X.XXX" or null if input is null
 */
function ticksToSeconds(ticks) {
    if (ticks === null) return null;
    const seconds = ticks / TICK_RATE;
    return seconds.toFixed(3);
}

/**
 * Format seconds as human-readable time per SPEC024.
 * @param {number} seconds - Duration in seconds (INTEGER from database)
 * @param {number} typicalMax - Typical maximum value for this field (seconds)
 * @returns {string} Formatted time string
 */
function formatHumanTime(seconds, typicalMax) {
    if (seconds === null || seconds === undefined) return 'null';

    // [SPEC024-FMT-010] Select format by typical maximum
    if (typicalMax < 100) {
        // Short format: X.XXs (< 100 seconds)
        return `${seconds.toFixed(2)}s`;
    } else if (typicalMax < 6000) {  // 100 minutes
        // Medium format: M:SS.Xs (100s to 100m)
        const minutes = Math.floor(seconds / 60);
        const secs = seconds % 60;
        return `${minutes}:${secs.toFixed(1).padStart(4, '0')}s`;
    } else if (typicalMax < 90000) {  // 25 hours
        // Long format: H:MM:SS (100m to 25h)
        const hours = Math.floor(seconds / 3600);
        const mins = Math.floor((seconds % 3600) / 60);
        const secs = Math.floor(seconds % 60);
        return `${hours}:${mins.toString().padStart(2, '0')}:${secs.toString().padStart(2, '0')}`;
    } else {
        // Extended format: Dual sub-format based on actual value
        // [SPEC024-FMT-060] < 25h actual value: H:MM:SS, >= 25h: X.XXd
        if (seconds < 90000) {  // < 25 hours
            // Sub-format A: H:MM:SS (< 25 hours actual value)
            const hours = Math.floor(seconds / 3600);
            const mins = Math.floor((seconds % 3600) / 60);
            const secs = Math.floor(seconds % 60);
            return `${hours}:${mins.toString().padStart(2, '0')}:${secs.toString().padStart(2, '0')}`;
        } else {
            // Sub-format B: X.XXd (>= 25 hours actual value)
            const days = seconds / 86400.0;
            // Round to 2 decimal places first
            const rounded2dp = Math.round(days * 100) / 100;
            const rounded1dp = Math.round(days * 10) / 10;

            // Format with up to 2 decimal places, removing trailing zeros
            if (Math.abs(rounded2dp - Math.floor(rounded2dp)) < 0.001) {
                // Whole number
                return `${Math.floor(rounded2dp)}d`;
            } else if (Math.abs(rounded2dp * 10 - Math.floor(rounded2dp * 10)) < 0.001) {
                // One decimal place
                return `${rounded1dp.toFixed(1)}d`;
            } else {
                // Two decimal places
                return `${rounded2dp.toFixed(2)}d`;
            }
        }
    }
}

/**
 * Format relative time from now (e.g., "2.3d ago" or "0:01:23 in the future").
 * Uses SPEC024 extended format for display.
 * @param {number} timestampSeconds - Unix timestamp in seconds
 * @param {number} currentTimeSeconds - Current time as Unix timestamp in seconds
 * @returns {string} Formatted relative time string with "ago" or "in the future" suffix
 */
function formatRelativeTime(timestampSeconds, currentTimeSeconds) {
    const diff = currentTimeSeconds - timestampSeconds;

    if (diff >= 0) {
        // Past: timestamp is before current time
        let formatted;
        if (diff < 90000) {  // < 25 hours
            // H:MM:SS format
            const hours = Math.floor(diff / 3600);
            const mins = Math.floor((diff % 3600) / 60);
            const secs = Math.floor(diff % 60);
            formatted = `${hours}:${mins.toString().padStart(2, '0')}:${secs.toString().padStart(2, '0')}`;
        } else {
            // >= 25 hours: X.XXd format
            const days = diff / 86400.0;
            const rounded2dp = Math.round(days * 100) / 100;
            const rounded1dp = Math.round(days * 10) / 10;

            if (Math.abs(rounded2dp - Math.floor(rounded2dp)) < 0.001) {
                formatted = `${Math.floor(rounded2dp)}d`;
            } else if (Math.abs(rounded2dp * 10 - Math.floor(rounded2dp * 10)) < 0.001) {
                formatted = `${rounded1dp.toFixed(1)}d`;
            } else {
                formatted = `${rounded2dp.toFixed(2)}d`;
            }
        }
        return `${formatted} ago`;
    } else {
        // Future: timestamp is after current time
        const absDiff = Math.abs(diff);
        let formatted;
        if (absDiff < 90000) {  // < 25 hours
            // H:MM:SS format
            const hours = Math.floor(absDiff / 3600);
            const mins = Math.floor((absDiff % 3600) / 60);
            const secs = Math.floor(absDiff % 60);
            formatted = `${hours}:${mins.toString().padStart(2, '0')}:${secs.toString().padStart(2, '0')}`;
        } else {
            // >= 25 hours: X.XXd format
            const days = absDiff / 86400.0;
            const rounded2dp = Math.round(days * 100) / 100;
            const rounded1dp = Math.round(days * 10) / 10;

            if (Math.abs(rounded2dp - Math.floor(rounded2dp)) < 0.001) {
                formatted = `${Math.floor(rounded2dp)}d`;
            } else if (Math.abs(rounded2dp * 10 - Math.floor(rounded2dp * 10)) < 0.001) {
                formatted = `${rounded1dp.toFixed(1)}d`;
            } else {
                formatted = `${rounded2dp.toFixed(2)}d`;
            }
        }
        return `${formatted} in the future`;
    }
}

/**
 * Format float values with at least 1 decimal place and up to 6 decimal places.
 * Trailing zeros are removed, but at least 1 decimal place is always shown.
 *
 * Examples:
 * - 1.0 → "1.0"
 * - 1.035000 → "1.035"
 * - 1.1234567 → "1.123457" (rounded to 6 decimals)
 * - 0.5 → "0.5"
 *
 * @param {number} value - Float value to format
 * @returns {string} Formatted string
 */
function formatFloat(value) {
    // Round to 6 decimal places
    const rounded = Math.round(value * 1e6) / 1e6;

    // Convert to string with fixed 6 decimals, then remove trailing zeros
    // but keep at least 1 decimal place
    let formatted = rounded.toFixed(6);

    // Remove trailing zeros, but stop before the first decimal place
    formatted = formatted.replace(/(\.\d*?)0+$/, '$1');

    // Ensure at least 1 decimal place (if we removed all decimals, add .0)
    if (!formatted.includes('.')) {
        formatted += '.0';
    } else if (formatted.endsWith('.')) {
        formatted += '0';
    }

    return formatted;
}

function renderTable(data) {
    const container = document.getElementById('tableContainer');

    if (!data.rows || data.rows.length === 0) {
        container.innerHTML = '<p>No results found</p>';
        document.getElementById('resultInfo').textContent = 'No results';
        return;
    }

    // Update result info
    const resultType = data.table_name || data.filter_name || data.search_type || 'results';
    document.getElementById('resultInfo').textContent =
        `${data.total_results} total ${resultType} (page ${data.page} of ${data.total_pages})`;

    // Track de-referenced columns for styling
    const dereferencedCols = new Set(data.dereferenced_columns || []);
    console.log('De-referenced columns:', Array.from(dereferencedCols));

    // Render table
    let html = '<table><thead><tr>';
    data.columns.forEach(col => {
        const isDereferenced = dereferencedCols.has(col);
        const className = isDereferenced ? ' class="dereferenced"' : '';
        html += `<th${className}>${col}</th>`;
    });
    html += '</tr></thead><tbody>';

    data.rows.forEach(row => {
        html += '<tr>';
        row.forEach((cell, index) => {
            const colName = data.columns[index];
            const isDereferenced = dereferencedCols.has(colName);
            const className = isDereferenced ? ' class="dereferenced"' : '';

            // REQ-F-001: Display timing columns in dual format: {ticks} ({seconds}s)
            // Per SPEC017 SRC-LAYER-011: Developer UI shows both ticks AND seconds
            if (TIMING_COLUMNS.includes(colName)) {
                if (cell === null) {
                    html += `<td${className}><em>null</em></td>`;
                } else {
                    const ticks = parseInt(cell);
                    const seconds = ticksToSeconds(ticks);
                    html += `<td${className}>${ticks} (${seconds}s)</td>`;
                }
            }
            // SPEC024: Display cooldown columns in human-readable format
            else if (colName in COOLDOWN_COLUMNS) {
                if (cell === null) {
                    html += `<td${className}><em>null</em></td>`;
                } else {
                    const seconds = parseInt(cell);
                    const typicalMax = COOLDOWN_COLUMNS[colName];
                    const humanTime = formatHumanTime(seconds, typicalMax);
                    html += `<td${className}>${humanTime}</td>`;
                }
            }
            // Display modification_time with relative format (e.g., "2.3d ago")
            else if (colName === 'modification_time') {
                if (cell === null) {
                    html += `<td${className}><em>null</em></td>`;
                } else {
                    // modification_time is stored as RFC3339 string or Unix timestamp
                    let timestampSeconds;

                    // Try parsing as RFC3339 string first (e.g., "2025-11-02T12:34:56Z")
                    if (typeof cell === 'string' && cell.includes('T')) {
                        const date = new Date(cell);
                        timestampSeconds = Math.floor(date.getTime() / 1000);
                    } else {
                        // Fall back to integer parsing
                        timestampSeconds = parseInt(cell);

                        // If timestamp appears to be in milliseconds (> year 2100 in seconds), convert it
                        if (timestampSeconds > 4102444800) {
                            timestampSeconds = Math.floor(timestampSeconds / 1000);
                        }
                    }

                    const currentTimeSeconds = Math.floor(Date.now() / 1000);
                    const relative = formatRelativeTime(timestampSeconds, currentTimeSeconds);
                    html += `<td${className}>${cell} (${relative})</td>`;
                }
            }
            else {
                // Non-timing columns: format floats, otherwise use original value
                if (cell === null) {
                    html += `<td${className}><em>null</em></td>`;
                } else if (FLOAT_COLUMNS.includes(colName)) {
                    // Column is defined as REAL in database: always format as float
                    // (even if JavaScript sees it as integer, e.g., base_probability = 1)
                    const formatted = formatFloat(Number(cell));
                    html += `<td${className}>${formatted}</td>`;
                } else if (typeof cell === 'number' && !Number.isInteger(cell)) {
                    // Float value: format with at least 1 decimal place, up to 6
                    const formatted = formatFloat(cell);
                    html += `<td${className}>${formatted}</td>`;
                } else {
                    // Integer, string, or other type: use original value
                    html += `<td${className}>${String(cell)}</td>`;
                }
            }
        });
        html += '</tr>';
    });

    html += '</tbody></table>';
    container.innerHTML = html;
}

function updatePagination(data) {
    const pagination = document.getElementById('pagination');
    const pageInfo = document.getElementById('pageInfo');
    const prevBtn = document.getElementById('prevBtn');
    const nextBtn = document.getElementById('nextBtn');

    if (data.total_pages > 1) {
        pagination.classList.remove('hidden');
        pageInfo.textContent = `Page ${data.page} of ${data.total_pages}`;
        prevBtn.disabled = data.page <= 1;
        nextBtn.disabled = data.page >= data.total_pages;
    } else {
        pagination.classList.add('hidden');
    }
}

function saveCurrentSearch() {
    const name = prompt('Enter a name for this search:');
    if (name && name.trim()) {
        prefs.addSavedSearch(name.trim(), currentView);
    }
}

// Modal Functions
async function showTableSemantics() {
    // Get the currently selected table from the dropdown
    const tableName = document.getElementById('tableName').value;

    if (!tableName) {
        showStatus('No table selected', 'error');
        return;
    }

    try {
        const response = await authenticatedFetch(`/api/semantics/${tableName}`);
        const data = await response.json();

        if (data.error) {
            showStatus(data.error, 'error');
            return;
        }

        renderSemanticsModal(data);
    } catch (error) {
        showStatus('Failed to load table semantics: ' + error.message, 'error');
    }
}

function renderSemanticsModal(data) {
    const modal = document.getElementById('semanticsModal');
    const modalTitle = document.getElementById('modalTableName');
    const modalBody = document.getElementById('modalBody');

    modalTitle.textContent = `Table Semantics: ${data.table_name}`;

    let html = '';
    data.columns.forEach(col => {
        html += `
            <div class="column-description">
                <div class="column-name">${col.name}</div>
                <div class="column-desc-text">${col.description}</div>
            </div>
        `;
    });

    modalBody.innerHTML = html;
    modal.classList.remove('hidden');
}

function closeModal() {
    const modal = document.getElementById('semanticsModal');
    modal.classList.add('hidden');
}

// Event Listeners
document.getElementById('viewType').addEventListener('change', updateViewControls);
document.getElementById('loadBtn').addEventListener('click', () => {
    currentPage = 1;
    loadData();
});
document.getElementById('saveBtn').addEventListener('click', saveCurrentSearch);
document.getElementById('clearBtn').addEventListener('click', () => prefs.clearAll());
document.getElementById('prevBtn').addEventListener('click', () => {
    if (currentPage > 1) {
        currentPage--;
        loadData();
    }
});
document.getElementById('nextBtn').addEventListener('click', () => {
    if (currentData && currentPage < currentData.total_pages) {
        currentPage++;
        loadData();
    }
});
document.getElementById('semanticsBtn').addEventListener('click', showTableSemantics);
document.getElementById('modalClose').addEventListener('click', closeModal);

// Close modal when clicking outside (but not when clicking modal content)
document.getElementById('semanticsModal').addEventListener('click', (e) => {
    if (e.target.id === 'semanticsModal') {
        closeModal();
    }
});

// Prevent clicks on modal content from closing the modal
document.querySelector('.modal-content').addEventListener('click', (e) => {
    e.stopPropagation();
});

// Fetch and display build info
async function loadBuildInfo() {
    try {
        const response = await authenticatedFetch('/api/buildinfo');
        const data = await response.json();

        const buildInfoEl = document.getElementById('buildInfo');
        buildInfoEl.innerHTML = `
            <div class="build-info-line">wkmp-dr v${data.version}</div>
            <div class="build-info-line">${data.git_hash} (${data.build_profile})</div>
            <div class="build-info-line">${data.build_timestamp}</div>
        `;
    } catch (error) {
        console.error('Failed to load build info:', error);
        document.getElementById('buildInfo').innerHTML = '<div class="build-info-line">Build info unavailable</div>';
    }
}

// Connection status management using shared WKMP SSE utility
// Note: WkmpSSEConnection class is loaded from /static/wkmp-common/wkmp-sse.js
let sseConnection = null;

// Initialize
updateViewControls();
renderFavorites();
loadBuildInfo();

// Establish SSE connection
sseConnection = new WkmpSSEConnection('/api/events', 'connection-status');
sseConnection.connect();
