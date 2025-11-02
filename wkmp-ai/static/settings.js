/**
 * WKMP Settings - API Key Configuration
 *
 * Traceability: [APIK-UI-040], [APIK-UI-050]
 */

document.addEventListener('DOMContentLoaded', () => {
    const form = document.getElementById('api-key-form');
    const apiKeyInput = document.getElementById('api-key');
    const messageDiv = document.getElementById('message');

    form.addEventListener('submit', async (e) => {
        e.preventDefault();

        const apiKey = apiKeyInput.value.trim();

        // Clear previous message
        messageDiv.textContent = '';
        messageDiv.className = '';

        // Client-side validation
        if (!apiKey) {
            showMessage('API key cannot be empty', 'error');
            return;
        }

        // Show loading state
        const submitButton = form.querySelector('button[type="submit"]');
        const originalButtonText = submitButton.textContent;
        submitButton.textContent = 'Saving...';
        submitButton.disabled = true;

        try {
            const response = await fetch('/api/settings/acoustid_api_key', {
                method: 'POST',
                headers: {
                    'Content-Type': 'application/json',
                },
                body: JSON.stringify({ api_key: apiKey }),
            });

            const result = await response.json();

            if (response.ok && result.success) {
                showMessage(result.message, 'success');
                apiKeyInput.value = ''; // Clear input for security
            } else {
                // Handle error response
                const errorMessage = result.message || 'Failed to save API key';
                showMessage(errorMessage, 'error');
            }
        } catch (error) {
            console.error('Settings API error:', error);
            showMessage(`Network error: ${error.message}`, 'error');
        } finally {
            // Restore button state
            submitButton.textContent = originalButtonText;
            submitButton.disabled = false;
        }
    });

    /**
     * Display message to user
     * @param {string} text - Message text
     * @param {string} type - 'success' or 'error'
     */
    function showMessage(text, type) {
        messageDiv.textContent = text;
        messageDiv.className = type;

        // Auto-clear success messages after 5 seconds
        if (type === 'success') {
            setTimeout(() => {
                messageDiv.textContent = '';
                messageDiv.className = '';
            }, 5000);
        }
    }
});
