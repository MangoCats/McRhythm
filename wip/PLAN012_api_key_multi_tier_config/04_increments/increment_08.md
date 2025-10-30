# Increment 8: Web UI Settings Page

**Estimated Effort:** 2-3 hours
**Dependencies:** Increment 7 (API endpoint)
**Risk:** LOW

---

## Objectives

Create /settings page with API key input, save button, and security (no key display).

---

## Requirements Addressed

- [APIK-UI-040], [APIK-UI-050], [APIK-UI-060] - Settings page UI
- [APIK-SEC-070], [APIK-SEC-080] - Security documentation

---

## Deliverables

### Code Changes

**File: wkmp-ai/static/settings.html** (new)

```html
<!DOCTYPE html>
<html>
<head>
    <title>WKMP Settings</title>
    <link rel="stylesheet" href="settings.css">
</head>
<body>
    <h1>WKMP Settings</h1>

    <section>
        <h2>AcoustID API Key</h2>
        <p>Configure your AcoustID API key for audio fingerprinting.</p>
        <p><a href="https://acoustid.org/api-key" target="_blank">Get API key</a></p>

        <form id="api-key-form">
            <label for="api-key">API Key:</label>
            <input type="password" id="api-key" placeholder="Enter API key">
            <button type="submit">Save</button>
        </form>

        <div id="message"></div>
    </section>

    <script src="settings.js"></script>
</body>
</html>
```

**File: wkmp-ai/static/settings.js** (new)

```javascript
document.getElementById('api-key-form').addEventListener('submit', async (e) => {
    e.preventDefault();

    const apiKey = document.getElementById('api-key').value;
    const messageDiv = document.getElementById('message');

    try {
        const response = await fetch('/api/settings/acoustid_api_key', {
            method: 'POST',
            headers: {'Content-Type': 'application/json'},
            body: JSON.stringify({api_key: apiKey})
        });

        const result = await response.json();

        if (result.success) {
            messageDiv.textContent = result.message;
            messageDiv.className = 'success';
            document.getElementById('api-key').value = ''; // Clear input
        } else {
            messageDiv.textContent = result.message;
            messageDiv.className = 'error';
        }
    } catch (error) {
        messageDiv.textContent = 'Network error: ' + error.message;
        messageDiv.className = 'error';
    }
});
```

**File: wkmp-ai/static/settings.css** (new)

```css
body {
    font-family: Arial, sans-serif;
    max-width: 600px;
    margin: 50px auto;
}

form {
    margin: 20px 0;
}

label {
    display: block;
    margin-bottom: 5px;
}

input {
    width: 100%;
    padding: 8px;
    margin-bottom: 10px;
}

button {
    padding: 10px 20px;
    background: #007bff;
    color: white;
    border: none;
    cursor: pointer;
}

button:hover {
    background: #0056b3;
}

.success {
    color: green;
    padding: 10px;
    margin-top: 10px;
}

.error {
    color: red;
    padding: 10px;
    margin-top: 10px;
}
```

---

### System Tests

**File: Manual testing** (tc_s_workflow_001)

Test user workflow: Open /settings → Enter key → Save → Verify success

---

## Acceptance Criteria

- [ ] /settings page accessible at http://localhost:5723/settings
- [ ] API key input field (type=password for security)
- [ ] Save button triggers POST to endpoint
- [ ] Success message displayed on save
- [ ] Error message displayed on validation failure
- [ ] Link to acoustid.org for key registration
- [ ] No key display (security requirement)

---

## Test Traceability

- tc_s_workflow_001: New user configures key via web UI

---

## Rollback Plan

Remove static files. No impact on existing functionality (endpoint still works via curl/API).
