# RESTful Analysis: Timestamp + Hash Authentication

## Question
Does the timestamp + hash authentication approach fit with the RESTful architectural model?

## TL;DR
**Yes, with caveats.** The approach is **reasonably RESTful** (more so than session-based auth, less than pure bearer tokens), and is a **pragmatic trade-off** suitable for WKMP's use case (local music player with security requirements).

---

## REST Principles Applied

### 1. **Stateless** ✅ PASSES
**Principle:** Each request contains all information needed to process it. Server maintains no session state.

**Our Approach:**
- ✅ No session storage on server
- ✅ No session cookies
- ✅ Each request self-contained (timestamp + hash + data)
- ✅ Server can process requests independently

**Verdict:** Fully stateless in the traditional sense. Server doesn't maintain any per-client state.

### 2. **Uniform Interface** ✅ PASSES
**Principle:** Standard HTTP methods, URIs, status codes, media types.

**Our Approach:**
- ✅ Standard HTTP methods (GET, POST, PUT, DELETE)
- ✅ Standard URIs (`/playback/play`, `/audio/volume`)
- ✅ Standard HTTP status codes (200, 401, 400)
- ✅ Standard media types (JSON, query parameters)
- ✅ Auth info in standard places (query params for GET, JSON body for POST)

**Verdict:** Fully compliant. Uses HTTP as designed.

### 3. **Client-Server Separation** ✅ PASSES
**Principle:** Client and server are independent, communicate only through interface.

**Our Approach:**
- ✅ Client (browser) and server (wkmp-ap) are separate processes
- ✅ Communication only via HTTP API
- ✅ Shared secret pre-configured out-of-band (via database/HTML embedding)

**Verdict:** Fully compliant. Clear separation of concerns.

### 4. **Cacheable** ⚠️ PARTIAL PASS
**Principle:** Responses should indicate if they can be cached.

**Our Approach:**
- ⚠️ Timestamp changes on every request → different hash → no caching possible
- ⚠️ Even identical operations (e.g., GET /health) can't be cached
- ❌ Could add `Cache-Control: no-store` headers to be explicit

**Trade-off Analysis:**
- **Security benefit:** Prevents replay attacks within cache lifetime
- **Performance cost:** No HTTP caching (but WKMP is local network, so minimal impact)
- **Mitigation:** WKMP responses are fast (local), caching less critical

**Verdict:** Fails pure cacheability, but justified by security requirements.

### 5. **Layered System** ✅ PASSES
**Principle:** Client can't tell if connected directly to server or through intermediaries.

**Our Approach:**
- ✅ Standard HTTP → works through proxies, load balancers, etc.
- ✅ No special connection requirements
- ✅ Authentication in standard HTTP fields

**Verdict:** Fully compliant. Could add nginx/reverse proxy without changes.

---

## Time-Dependent State Analysis

**Key Question:** Is the ±1000ms time window a violation of statelessness?

### Arguments FOR Statelessness
1. **No explicit state storage:** Server doesn't store "valid tokens" or "sessions"
2. **Deterministic validation:** Any server with correct time can validate
3. **No coordination needed:** Multiple servers can validate independently
4. **Time is universal:** Clock is external state, not application state

### Arguments AGAINST Statelessness
1. **Temporal state:** Requests are only valid for ~2 seconds
2. **Time-dependent:** Same request (same hash) invalid 3 seconds later
3. **Implicit state:** Server's clock is effectively shared state

### Verdict: **Effectively Stateless**
While time-dependent, this doesn't violate REST's intent:
- REST's statelessness goal: Server doesn't maintain per-client state
- Time is universal, external state (not application state)
- Multiple servers can validate without coordination (just need synchronized clocks)
- Common pattern in distributed systems (AWS API, webhooks, OAuth2 timestamps)

---

## Comparison to Alternatives

| Auth Method | Stateless | Cacheable | Security | Complexity | RESTful Score |
|-------------|-----------|-----------|----------|------------|---------------|
| **None**             | ✅ Yes  | ✅ Yes         | ❌ None                      | ✅ Low        | ✅ 100% |
| **Basic Auth**       | ✅ Yes  | ✅ Yes         | ⚠️ Credentials every request | ✅ Low        | ✅  95% |
| **API Key (static)** | ✅ Yes  | ✅ Yes         | ⚠️ Key can be stolen         | ✅ Low        | ✅  95% |
| **Timestamp + Hash** | ✅ Yes* | ❌ No          | ✅ Replay-resistant          | ✅ Low-Medium | ⚠️  85% |
| **JWT Bearer Token** | ✅ Yes  | ⚠️ Conditional | ✅ Good                      | ⚠️ Medium     | ✅  90% |
| **OAuth2**           | ✅ Yes  | ⚠️ Conditional | ✅ Excellent                 | ❌ High       | ✅  90% |
| **Session Cookies**  | ❌ No   | ❌ No          | ⚠️ CSRF risk                 | ⚠️ Medium     | ❌  40% |

\* Time-dependent statelessness

**Why Timestamp + Hash for WKMP:**
1. **Local use case:** Not exposed to public internet, lower threat model
2. **Simplicity:** No token server, no complex OAuth flow
3. **Replay protection:** Time window prevents replay attacks
4. **No credential exposure:** Hash changes every request (unlike static API keys)
5. **Fits use case:** Interactive music player (2-second window is fine)

---

## Security Properties (Bonus Analysis)

### Strengths
- ✅ **Replay attack mitigation:** ±1000ms window limits replay window
- ✅ **No credential transmission:** Shared secret never sent over network
- ✅ **Request integrity:** Hash proves request wasn't tampered with
- ✅ **Time-bound validity:** Old requests automatically invalid

### Weaknesses
- ⚠️ **Replay window:** Within ±1000ms, requests can be replayed
- ⚠️ **Requires time sync:** Server and client clocks must be reasonably synchronized
- ⚠️ **Shared secret:** If secret stolen, attacker can generate valid requests
- ⚠️ **No revocation:** Can't invalidate compromised secret without database change

### Mitigations in WKMP
- Local network reduces eavesdropping risk
- Interactive use (not batch/automated) reduces replay attack value
- Short time window (2 seconds) limits replay usefulness
- Bypass mode (secret=0) available for trusted environments

---

## Conclusion

### RESTful Score: **85/100** ⚠️ "Mostly RESTful"

**Passes:**
- ✅ Statelessness (with time-dependent caveat)
- ✅ Uniform Interface
- ✅ Client-Server Separation
- ✅ Layered System

**Fails:**
- ❌ Cacheability (acceptable trade-off for security)

### Recommendation: **APPROVED for WKMP**

The timestamp + hash approach is **appropriate for WKMP** because:

1. **Context matters:** Local music player ≠ public web API
   - Threat model: Prevent unauthorized control on local network
   - Not protecting: Nuclear launch codes or financial transactions

2. **Pragmatic trade-off:** Balances security with simplicity
   - More secure than static API keys
   - Simpler than OAuth2
   - No caching needed (local, fast responses)

3. **Industry precedent:** Common pattern for API authentication
   - AWS API requests use similar HMAC + timestamp
   - Webhook signatures use similar patterns
   - Considered acceptable in distributed systems

4. **REST philosophy alignment:**
   - REST is about architectural constraints, not dogma
   - Statelessness achieved (no server-side sessions)
   - Uniform interface preserved
   - Time-dependency is pragmatic, not problematic

### Alternative if Concerns Persist

If pure RESTfulness is critical, consider:
- **JWT Bearer Tokens:** More cacheable, still stateless
  - Trade-off: Longer-lived tokens = larger replay window
  - Added complexity: Token issuance, rotation, revocation

---

## References

- **REST Dissertation:** Fielding, Roy Thomas. "Architectural Styles and the Design of Network-based Software Architectures." (2000)
- **AWS API Authentication:** Uses HMAC-SHA256 with timestamp (similar approach)
- **HTTP Authentication Schemes:** RFC 7235 (Basic, Bearer, etc.)
- **Time-based Security:** Common in distributed systems (Kerberos, TOTP, etc.)

---

**Date:** 2025-10-27
**Analysis by:** Claude Code (with human oversight)
**Document Status:** For review and discussion
