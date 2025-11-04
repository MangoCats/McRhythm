/**
 * WKMP SSE Connection Utility
 *
 * Manages Server-Sent Events (SSE) connections with automatic status updates.
 * Shared across all WKMP microservices with web UIs.
 *
 * Usage:
 *   const sse = new WkmpSSEConnection('/events', 'connection-status');
 *   sse.connect();
 *   sse.addEventListener('CustomEvent', (e) => { ... });
 */
class WkmpSSEConnection {
    /**
     * Create a new SSE connection manager
     *
     * @param {string} endpoint - SSE endpoint path (e.g., '/events', '/api/events')
     * @param {string} statusElementId - ID of connection status badge element
     */
    constructor(endpoint, statusElementId = 'connection-status') {
        this.endpoint = endpoint;
        this.statusElementId = statusElementId;
        this.eventSource = null;
    }

    /**
     * Establish SSE connection and set up event handlers
     *
     * @returns {EventSource} The EventSource instance
     */
    connect() {
        this.updateStatus('connecting');
        this.eventSource = new EventSource(this.endpoint);

        this.eventSource.onopen = () => {
            console.log(`SSE connection opened to ${this.endpoint}`);
            this.updateStatus('connected');
        };

        this.eventSource.onerror = (err) => {
            console.error(`SSE connection error for ${this.endpoint}:`, err);
            this.updateStatus('disconnected');
            // EventSource automatically attempts to reconnect
        };

        return this.eventSource;
    }

    /**
     * Update connection status badge
     *
     * @param {string} status - Status: 'connected', 'connecting', or 'disconnected'
     */
    updateStatus(status) {
        const statusEl = document.getElementById(this.statusElementId);
        if (statusEl) {
            statusEl.className = 'connection-status status-' + status;
            statusEl.textContent = status === 'connected' ? 'Connected' :
                                  status === 'connecting' ? 'Connecting...' : 'Disconnected';
        }
    }

    /**
     * Add event listener for specific SSE event type
     *
     * @param {string} eventType - Event type name
     * @param {Function} handler - Event handler function
     */
    addEventListener(eventType, handler) {
        if (this.eventSource) {
            this.eventSource.addEventListener(eventType, handler);
        }
    }

    /**
     * Close the SSE connection
     */
    close() {
        if (this.eventSource) {
            this.eventSource.close();
            this.updateStatus('disconnected');
        }
    }
}
