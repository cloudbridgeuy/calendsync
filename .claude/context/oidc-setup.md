# OIDC Provider Setup Guide

## Google OAuth Setup

1. Go to [Google Cloud Console](https://console.cloud.google.com/)
2. Create a new project or select existing
3. Navigate to APIs & Services > Credentials
4. Click "Create Credentials" > "OAuth client ID"
5. Select "Web application"
6. Add authorized redirect URIs:
   - Production: `https://calendsync.app/auth/google/callback`
   - Development (with ngrok): `https://<your-ngrok>.ngrok.io/auth/google/callback`
   - Local development: `http://localhost:3000/auth/google/callback`
7. Copy Client ID and Client Secret

Environment variables:
```bash
GOOGLE_CLIENT_ID=your-client-id
GOOGLE_CLIENT_SECRET=your-client-secret
```

## Apple Sign In Setup

1. Go to [Apple Developer Portal](https://developer.apple.com/)
2. Navigate to Certificates, Identifiers & Profiles
3. Create an App ID with Sign In with Apple capability
4. Create a Services ID for web authentication
5. Configure the Services ID:
   - Domains: `calendsync.app`
   - Return URLs: `https://calendsync.app/auth/apple/callback`
6. Create a Sign In with Apple key (ES256)
7. Download the .p8 key file

Environment variables:
```bash
APPLE_CLIENT_ID=your-services-id
APPLE_TEAM_ID=your-team-id
APPLE_KEY_ID=your-key-id
APPLE_PRIVATE_KEY="-----BEGIN PRIVATE KEY-----\n...\n-----END PRIVATE KEY-----"
```

Note: Apple always sends the callback as a POST request with form data, which the server handles via the `POST /auth/apple/callback` endpoint.

## Environment Variables Reference

| Variable | Required | Default | Description |
|----------|----------|---------|-------------|
| `AUTH_BASE_URL` | No | `http://localhost:3000` | Base URL for callback redirects |
| `GOOGLE_CLIENT_ID` | No | - | Google OAuth client ID (enables Google auth) |
| `GOOGLE_CLIENT_SECRET` | If Google enabled | - | Google OAuth client secret |
| `APPLE_CLIENT_ID` | No | - | Apple OAuth client ID (enables Apple auth) |
| `APPLE_TEAM_ID` | If Apple enabled | - | Apple developer team ID |
| `APPLE_KEY_ID` | If Apple enabled | - | Apple key ID |
| `APPLE_PRIVATE_KEY` | If Apple enabled | - | Apple ES256 private key (PEM format) |
| `SESSION_TTL_DAYS` | No | `7` | Session TTL in days |
| `COOKIE_SECURE` | No | `true` | Whether to set secure flag on cookies |

## Development with Mock IdP

For local development without real OIDC providers:

```bash
# Run with mock auth (feature must be enabled)
cargo run -p calendsync --features mock

# Mock IdP runs on port 3001
# Login redirects to http://localhost:3001/google/authorize
# Enter any email to authenticate
```

The Mock IdP server provides:
- `GET /google/authorize` - Google login page
- `GET /apple/authorize` - Apple login page
- `POST /authorize/submit` - Form submission handler

It generates a mock authorization code encoding the user info (email, name, provider), which the mock provider decodes during token exchange.

## Auth Routes

The authentication system provides these endpoints:

| Endpoint | Method | Description |
|----------|--------|-------------|
| `/auth/google/login` | GET | Initiate Google OIDC flow |
| `/auth/google/callback` | GET | Handle Google OIDC callback |
| `/auth/apple/login` | GET | Initiate Apple OIDC flow |
| `/auth/apple/callback` | POST | Handle Apple OIDC callback (form POST) |
| `/auth/logout` | POST | End current session |
| `/auth/logout-all` | POST | End all sessions for current user |
| `/auth/me` | GET | Get current authenticated user |

## Production Checklist

- [ ] Set `COOKIE_SECURE=true` (default)
- [ ] Set `AUTH_BASE_URL=https://calendsync.app`
- [ ] Configure real Google OAuth credentials
- [ ] Configure real Apple Sign In credentials
- [ ] Use Redis for session storage (`--features redis`)
- [ ] Set appropriate `SESSION_TTL_DAYS` (default: 7)
- [ ] Ensure HTTPS is enforced (cookies are secure by default)

## Troubleshooting

### "Session not found" errors
- Check that the session cookie is being sent with requests
- Verify `COOKIE_SECURE` matches your environment (use `false` for HTTP in dev)
- Ensure Redis/SQLite session storage is properly configured

### "Provider not configured" errors
- Ensure all required environment variables are set for the provider
- For Google: both `GOOGLE_CLIENT_ID` and `GOOGLE_CLIENT_SECRET` are required
- For Apple: all four Apple variables are required

### "Invalid state" errors
- The PKCE/state has expired (5 minute TTL) or was already consumed
- User may have refreshed the callback page
- Clear cookies and try the login flow again
