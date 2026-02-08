// File Classification Report - Client-side JavaScript
// **[AIA-CLASSIFY-UI-040]** PLAN027

let classificationData = null;
let currentCategory = 'audio';

// Initialize page on load
document.addEventListener('DOMContentLoaded', async () => {
    try {
        await loadClassificationData();
        renderTabs();
        renderFileList(currentCategory);
    } catch (error) {
        showError('Failed to load classification data: ' + error.message);
    }
});

/**
 * Fetch classification data from API
 */
async function loadClassificationData() {
    const response = await fetch('/api/import/file-classification');

    if (!response.ok) {
        if (response.status === 404) {
            throw new Error('No import session found');
        } else if (response.status === 409) {
            throw new Error('Scan still in progress, please wait');
        } else {
            throw new Error(`HTTP ${response.status}: ${response.statusText}`);
        }
    }

    classificationData = await response.json();

    // Display scanned folder
    document.getElementById('scanned-folder').textContent =
        `Scanned: ${classificationData.scanned_folder}`;
}

/**
 * Render category tabs
 */
function renderTabs() {
    const tabsContainer = document.getElementById('categoryTabs');

    const categories = [
        {
            id: 'audio',
            label: 'Audio Files',
            count: classificationData.audio_files.count,
            size: formatBytes(classificationData.audio_files.total_size_bytes)
        },
        {
            id: 'image',
            label: 'Image Files',
            count: classificationData.image_files.count,
            size: formatBytes(classificationData.image_files.total_size_bytes)
        },
        {
            id: 'other',
            label: 'Other Files',
            count: classificationData.other_files.count,
            size: formatBytes(classificationData.other_files.total_size_bytes)
        }
    ];

    tabsContainer.innerHTML = categories.map(cat => `
        <div class="tab ${cat.id === currentCategory ? 'active' : ''}"
             onclick="switchCategory('${cat.id}')">
            <div class="tab-label">${cat.label}</div>
            <div class="tab-count">${cat.count} files</div>
            <div class="tab-size">${cat.size}</div>
        </div>
    `).join('');
}

/**
 * Switch active category
 */
function switchCategory(category) {
    currentCategory = category;
    renderTabs();
    renderFileList(category);
}

/**
 * Render file list for current category
 */
function renderFileList(category) {
    const contentDiv = document.getElementById('fileListContent');

    const categoryData = classificationData[`${category}_files`];
    const files = categoryData.files;

    if (files.length === 0) {
        contentDiv.innerHTML = '<div class="empty-state">No files in this category</div>';
        return;
    }

    contentDiv.innerHTML = `
        <div class="file-list-header">
            <div>Path</div>
            <div style="text-align: right;">Size</div>
            <div style="text-align: right;">Modified</div>
        </div>
        <div class="file-list">
            ${files.map(file => renderFileItem(file)).join('')}
        </div>
        ${categoryData.count > files.length ? renderPaginationInfo(categoryData) : ''}
    `;
}

/**
 * Render individual file item
 */
function renderFileItem(file) {
    return `
        <div class="file-item">
            <div class="file-path" title="${file.path}">${file.path}</div>
            <div class="file-size">${formatBytes(file.size_bytes)}</div>
            <div class="file-modified">${formatDate(file.modified_at)}</div>
        </div>
    `;
}

/**
 * Render pagination info if not all files are shown
 */
function renderPaginationInfo(categoryData) {
    const shown = categoryData.files.length;
    const total = categoryData.count;

    if (shown >= total) return '';

    return `
        <div style="text-align: center; padding: 20px; color: #888;">
            Showing first ${shown} of ${total} files
            <button class="btn btn-secondary"
                    style="margin-left: 15px; padding: 8px 20px;"
                    onclick="loadMoreFiles()">
                Load More
            </button>
        </div>
    `;
}

/**
 * Load more files for current category (pagination)
 */
async function loadMoreFiles() {
    try {
        const currentFiles = classificationData[`${currentCategory}_files`].files;
        const offset = currentFiles.length;

        const response = await fetch(
            `/api/import/file-classification?category=${currentCategory}&offset=${offset}&limit=500`
        );

        if (!response.ok) {
            throw new Error(`HTTP ${response.status}`);
        }

        const data = await response.json();

        // Append new files to existing list
        classificationData[`${currentCategory}_files`].files.push(...data.files);

        // Re-render the list
        renderFileList(currentCategory);
    } catch (error) {
        showError('Failed to load more files: ' + error.message);
    }
}

/**
 * Format bytes to human-readable size
 * **[AIA-CLASSIFY-UI-030]** Per SPEC032 v2.2
 */
function formatBytes(bytes) {
    const KB = 1024;
    const MB = KB * 1024;
    const GB = MB * 1024;

    if (bytes < KB) {
        return `${bytes.toLocaleString()} B`;
    } else if (bytes < MB) {
        return `${(bytes / KB).toFixed(1)} KB`;
    } else if (bytes < GB) {
        return `${(bytes / MB).toFixed(1)} MB`;
    } else {
        return `${(bytes / GB).toFixed(1)} GB`;
    }
}

/**
 * Format ISO 8601 timestamp to readable date
 */
function formatDate(isoString) {
    try {
        const date = new Date(isoString);
        const now = new Date();
        const oneYearAgo = new Date(now.getTime() - 365 * 24 * 60 * 60 * 1000);

        if (date > oneYearAgo) {
            // Recent file: show date only
            return date.toISOString().split('T')[0];
        } else {
            // Old file: show full timestamp
            return date.toLocaleString();
        }
    } catch (e) {
        return isoString;
    }
}

/**
 * Show error message
 */
function showError(message) {
    const contentDiv = document.getElementById('fileListContent');
    contentDiv.innerHTML = `<div class="error">${message}</div>`;
}

/**
 * Cancel import (navigate to cancel endpoint)
 */
function cancelImport() {
    if (confirm('Are you sure you want to cancel this import?')) {
        fetch('/import/cancel', { method: 'POST' })
            .then(() => {
                window.location.href = '/';
            })
            .catch(error => {
                alert('Failed to cancel import: ' + error.message);
            });
    }
}
