# User Profile Settings

The settings menu displays the logged-in user's profile (name, email) with a logout button.

## Data Flow

### SSR → Client Hydration

```
Rust Handler                    React SSR                    Client Hydration
     │                              │                              │
     │  SsrUserInfo {name, email}   │                              │
     ├─────────────────────────────>│                              │
     │                              │   <App user={...} />         │
     │                              ├─────────────────────────────>│
     │                              │                              │
                                    └── HTML with embedded props ──┘
```

User data flows from the Rust handler through SSR props to the client. No client-side fetch is needed.

### Security Projection

`SsrUserInfo` (Rust) and `UserInfo` (TypeScript) are intentional security projections:

```rust
// Only expose safe fields to client
struct SsrUserInfo {
    name: String,
    email: String,
}

// Excluded: id, provider, provider_subject, timestamps
```

## RequestContext Extractor

Handlers use `RequestContext` to access request-scoped data:

```rust
pub async fn calendar_react_ssr(
    State(state): State<AppState>,
    ctx: RequestContext,  // Contains user + request_id
    Path(calendar_id): Path<Uuid>,
) -> Response {
    let user = ctx.user;      // Option<User>
    let rid = ctx.request_id; // For tracing
}
```

This separates concerns:
- `AppState`: Application-scoped (repositories, SSR pool, channels)
- `RequestContext`: Request-scoped (auth user, request ID)

## Components

### Profile (Standalone)

```tsx
// components/Profile.tsx
function Profile({ user, onLogout }: ProfileProps) {
  if (!user) return null
  return (
    <div className="settings-profile">
      <span>{user.name}</span>
      <span>{user.email}</span>
      <button onClick={onLogout}>Log out</button>
    </div>
  )
}
```

### SettingsMenu.Profile (Compound)

```tsx
// Accesses context for user and logout action
function Profile() {
  const { state, actions } = useSettingsMenuContext()
  return <ProfileComponent user={state.user} onLogout={actions.logout} />
}
```

### Usage

```tsx
<SettingsMenu.Panel>
  <SettingsMenu.Profile />   {/* Profile at top */}
  <SettingsMenu.ViewToggle />
  <SettingsMenu.StyleToggle />
  <SettingsMenu.TaskToggle />
</SettingsMenu.Panel>
```

## Logout Flow

1. User clicks "Log out"
2. `transport.logout()` called (platform-specific)
3. Web: `POST /auth/logout` clears session cookie
4. Tauri: `invoke('logout')` clears stored session
5. Redirect to `/login` via `window.location.href`

Full page navigation ensures clean state.

## Files

| File | Purpose |
|------|---------|
| `crates/calendsync/src/context/` | RequestContext extractor |
| `crates/calendsync/src/handlers/calendar_react.rs` | SsrUserInfo, passes to SSR |
| `crates/frontend/src/calendsync/types.ts` | UserInfo interface |
| `crates/frontend/src/calendsync/components/Profile.tsx` | Profile display component |
| `crates/frontend/src/calendsync/components/SettingsMenuWrapper.tsx` | Bridges context to menu |
