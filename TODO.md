# CalendSync - Roadmap to Production

This document tracks the remaining work to evolve the `/calendar/{calendar_id}` endpoint from a PoC to a production-ready application.

## Context

The original analysis identified these priorities for moving from PoC to production:
- **Use case**: Shared calendars (multiple users viewing/editing)
- **Authentication**: Email/password + OAuth
- **Database**: DynamoDB
- **Key priorities**: Real SSE events, Error handling, Accessibility, Testing

The SSR worker pool infrastructure (`calendsync_ssr` and `calendsync_ssr_core` crates) has been extracted as the architectural foundation. The items below represent the remaining work.

---

## 1. Authentication System

### Current State
- No authentication exists
- All endpoints are publicly accessible
- User model exists but is not used for auth (`crates/calendsync/src/models/user.rs`)

### Rationale
Shared calendars require user identity to:
- Control who can view/edit calendars
- Track who made changes (audit trail)
- Send notifications to the right users
- Support calendar sharing/permissions

### Accepted Plan

**Phase 1: Email/Password Authentication**
1. Add password hashing with `argon2` crate
2. Create `/auth/register` and `/auth/login` endpoints
3. Implement JWT tokens for session management
4. Add `Authorization` header middleware extractor
5. Protect calendar endpoints with auth middleware

**Phase 2: OAuth Integration**
1. Add `oauth2` crate dependency
2. Implement Google OAuth flow (`/auth/google`, `/auth/google/callback`)
3. Implement GitHub OAuth flow (optional)
4. Link OAuth accounts to existing users
5. Store OAuth tokens for calendar sync (Google Calendar API)

**Key Files to Modify**
- `crates/calendsync/src/handlers/` - New `auth.rs` module
- `crates/calendsync/src/state.rs` - Add session/token storage
- `crates/calendsync/src/app.rs` - Auth middleware layer
- `crates/core/src/` - Auth validation logic (Functional Core)

---

## 2. Database Integration (DynamoDB)

### Current State
- In-memory `HashMap` storage (`crates/calendsync/src/state.rs`)
- Data lost on server restart
- No persistence layer abstraction

### Rationale
Production requires:
- Data persistence across restarts
- Scalable storage for multiple users
- Efficient queries (by user, by date range, by calendar)
- DynamoDB chosen for serverless scalability and AWS integration

### Accepted Plan

**Phase 1: Repository Pattern**
1. Define `CalendarRepository` and `EntryRepository` traits in `calendsync_core`
2. Create in-memory implementations (current behavior, for tests)
3. Keep business logic independent of storage

**Phase 2: DynamoDB Implementation**
1. Add `aws-sdk-dynamodb` dependency
2. Design table schema:
   - PK: `USER#{user_id}`, SK: `CALENDAR#{calendar_id}`
   - PK: `CALENDAR#{calendar_id}`, SK: `ENTRY#{date}#{entry_id}`
   - GSI for date-range queries
3. Implement repository traits with DynamoDB client
4. Add connection pooling and retry logic

**Phase 3: Migration**
1. Create table provisioning script (CloudFormation/CDK)
2. Seed script for demo data
3. Remove in-memory storage from production path

**Key Files to Create**
- `crates/core/src/repository/` - Repository traits
- `crates/calendsync/src/db/` - DynamoDB implementations
- `infrastructure/` - AWS CDK/CloudFormation templates

---

## 3. Real SSE Events

### Current State
- SSE endpoint exists (`/api/events`)
- Events are **simulated** with random data every 3-5 seconds
- No connection to actual calendar mutations
- Location: `crates/calendsync/src/handlers/events.rs`

### Rationale
Real-time updates are essential for shared calendars:
- Users see changes immediately without refresh
- Prevents stale data conflicts
- Enables collaborative editing experience

### Accepted Plan

**Phase 1: Event Bus**
1. Create `EventBus` in `AppState` using `tokio::sync::broadcast`
2. Entry CRUD handlers publish events to the bus
3. SSE handler subscribes to the bus (filtered by calendar_id)

**Phase 2: Replace Simulated Events**
1. Remove random event generation from `events.rs`
2. Connect SSE stream to `EventBus` subscription
3. Add event deduplication (last_event_id already exists)

**Phase 3: Presence (Optional)**
1. Track connected users per calendar
2. Broadcast presence changes (user joined/left)
3. Show "X users viewing" in UI

**Key Files to Modify**
- `crates/calendsync/src/state.rs` - Add `EventBus`
- `crates/calendsync/src/handlers/entries.rs` - Publish on mutations
- `crates/calendsync/src/handlers/events.rs` - Subscribe to bus

---

## 4. Accessibility Improvements

### Current State
- Basic semantic HTML from React components
- No ARIA labels or roles
- No keyboard navigation support
- No screen reader testing
- Location: `crates/frontend/src/calendsync/components/`

### Rationale
Accessibility is:
- Legal requirement (ADA, WCAG compliance)
- Ethical responsibility
- Better UX for all users (keyboard users, screen readers)

### Accepted Plan

**Phase 1: Audit**
1. Run axe-core accessibility audit
2. Test with VoiceOver/NVDA screen readers
3. Document violations and priorities

**Phase 2: Semantic Improvements**
1. Add ARIA labels to interactive elements
2. Implement proper heading hierarchy
3. Add `role` attributes to calendar grid
4. Ensure focus indicators are visible

**Phase 3: Keyboard Navigation**
1. Arrow keys to navigate calendar days
2. Enter/Space to open entry details
3. Escape to close modals
4. Tab order follows logical flow

**Phase 4: Screen Reader Experience**
1. Announce day changes
2. Announce entry counts per day
3. Live regions for SSE updates

**Key Files to Modify**
- `crates/frontend/src/calendsync/components/*.tsx` - Add ARIA
- `crates/frontend/src/calendsync/hooks/` - Keyboard handlers
- `crates/frontend/src/calendsync/styles.css` - Focus styles

---

## 5. Comprehensive Testing Strategy

### Current State
- Unit tests for `calendsync_core` (22 tests)
- Unit tests for `calendsync_ssr_core` (14 tests)
- Integration tests for API endpoints (7 tests in `app.rs`)
- TypeScript tests for frontend core (89 tests)
- No E2E tests
- No SSR integration tests
- No load testing

### Rationale
Production confidence requires:
- Regression prevention
- Refactoring safety
- Performance baselines
- Cross-browser verification

### Accepted Plan

**Phase 1: Expand Unit Tests**
1. Add tests for `calendsync_ssr` (worker pool, error handling)
2. Add tests for all handlers (auth, entries, calendars)
3. Target 80% code coverage

**Phase 2: Integration Tests**
1. Test SSR pool with actual React rendering
2. Test SSE event flow end-to-end
3. Test auth flows with mocked OAuth

**Phase 3: E2E Tests**
1. Add Playwright for browser automation
2. Test calendar navigation
3. Test entry CRUD operations
4. Test real-time updates (two browser windows)

**Phase 4: Performance Testing**
1. Benchmark SSR render times
2. Load test SSE connections (1000+ concurrent)
3. Profile memory usage under load
4. Set performance budgets in CI

**Key Files to Create**
- `crates/calendsync/tests/` - Integration tests
- `crates/frontend/e2e/` - Playwright tests
- `benchmarks/` - Performance tests

---

## Priority Order

1. **Real SSE Events** - Highest impact, builds on existing infrastructure
2. **Database Integration** - Required for persistence, blocks auth
3. **Authentication System** - Required for multi-user, depends on database
4. **Comprehensive Testing** - Parallel with above, de-risks changes
5. **Accessibility** - Important but can be incremental

---

## Related Resources

- Plan file: `.claude/plans/moonlit-finding-glacier.md`
- SSR architecture: `crates/ssr/` and `crates/ssr_core/`
- React calendar docs: `.claude/context/react-calendar.md`
- Axum reference: `.claude/context/AXUM.md`
