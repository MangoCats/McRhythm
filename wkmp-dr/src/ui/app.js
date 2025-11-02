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

    // Render table
    let html = '<table><thead><tr>';
    data.columns.forEach(col => {
        html += `<th>${col}</th>`;
    });
    html += '</tr></thead><tbody>';

    data.rows.forEach(row => {
        html += '<tr>';
        row.forEach(cell => {
            const value = cell === null ? '<em>null</em>' : String(cell);
            html += `<td>${value}</td>`;
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
            <div class="build-info-line">v${data.version} [${data.git_hash}]</div>
            <div class="build-info-line">${data.build_timestamp}</div>
            <div class="build-info-line">(${data.build_profile})</div>
        `;
    } catch (error) {
        console.error('Failed to load build info:', error);
        document.getElementById('buildInfo').innerHTML = '<div class="build-info-line">Build info unavailable</div>';
    }
}

// Initialize
updateViewControls();
renderFavorites();
loadBuildInfo();
