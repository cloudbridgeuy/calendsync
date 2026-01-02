# Dexie.js Reference

Dexie.js is a wrapper library for IndexedDB that provides a simple, promise-based API for client-side storage. It supports live queries for React, transactions, and offline-first patterns.

## Quick Start

### Declare a Database

```javascript
const db = new Dexie("MyDatabase");

db.version(1).stores({
  friends: "++id, name, age, *tags",
  gameSessions: "id, score",
});
```

Only declare properties you want to index (properties used in `where()` queries). Unlike SQL, you don't declare all columns.

### Schema Syntax Quick Reference

| Symbol  | Meaning                      | Example                |
| ------- | ---------------------------- | ---------------------- |
| `++`    | Auto-incremented primary key | `++id`                 |
| `&`     | Unique index                 | `&email`               |
| `*`     | Multi-entry index (arrays)   | `*tags`                |
| `[A+B]` | Compound index               | `[firstName+lastName]` |

## Schema Syntax

### Primary Key Options

| Syntax      | Description                                     |
| ----------- | ----------------------------------------------- |
| `++keyPath` | Auto-incremented primary key                    |
| `++`        | Hidden auto-incremented primary key             |
| `keyPath`   | Non-auto-incremented primary key (must provide) |
| (blank)     | Hidden primary key (not auto-incremented)       |

### Index Options

| Syntax                | Description                                        |
| --------------------- | -------------------------------------------------- |
| `keyPath`             | Standard index                                     |
| `&keyPath`            | Unique index                                       |
| `*keyPath`            | Multi-entry index (each array value becomes a key) |
| `[keyPath1+keyPath2]` | Compound index                                     |

Dotted paths work for nested properties (e.g., `address.city`).

### Complete Example

```javascript
const db = new Dexie("MyDatabase");

db.version(1).stores({
  friends: "++id,name,shoeSize", // Auto-incremented primary key
  pets: "id, name, kind", // Non-auto-incremented primary key
  cars: "++, name", // Auto-incremented but not inbound
  enemies: ",name,*weaknesses", // Neither inbound nor auto-incremented
  users: "meta.ssn, addr.city", // Dotted paths for nested properties
  people: "[name+ssn], &ssn", // Compound primary key, unique ssn index
});
```

### Database Versioning

```javascript
db.version(1).stores({
  friends: "++id,name,age,*tags",
  gameSessions: "id,score",
});

db.version(2)
  .stores({
    friends: "++id, [firstName+lastName], yearOfBirth, *tags",
    gameSessions: null, // Delete table
  })
  .upgrade((tx) => {
    return tx.table("friends").modify((friend) => {
      friend.firstName = friend.name.split(" ")[0];
      friend.lastName = friend.name.split(" ")[1];
      friend.birthDate = new Date(new Date().getFullYear() - friend.age, 0);
      delete friend.name;
      delete friend.age;
    });
  });
```

The `upgrade()` callback runs only when upgrading from a version below 2.

## CRUD Operations

### Adding Items

```javascript
// Single item
await db.friends.add({ name: "Josephine", age: 21 });

// Bulk add
await db.friends.bulkAdd([
  { name: "Foo", age: 31 },
  { name: "Bar", age: 32 },
]);
```

### Updating Items

```javascript
// Put (insert or replace by primary key)
await db.friends.put({ id: 4, name: "Foo", age: 33 });

// Bulk put
await db.friends.bulkPut([
  { id: 4, name: "Foo2", age: 34 },
  { id: 5, name: "Bar2", age: 44 },
]);

// Upsert (update if exists, insert if not)
await db.friends.upsert(4, { name: "Bar", age: 22 });

// Update specific fields
await db.friends.update(4, { name: "Bar" });

// Modify with query
await db.customers
  .where("age")
  .inAnyRange([
    [0, 18],
    [65, Infinity],
  ])
  .modify({ discount: 0.5 });
```

### Deleting Items

```javascript
// Single delete
await db.friends.delete(4);

// Bulk delete
await db.friends.bulkDelete([1, 2, 4]);

// Delete with query
const oneWeekAgo = new Date(Date.now() - 60 * 60 * 1000 * 24 * 7);
await db.logEntries.where("timestamp").below(oneWeekAgo).delete();
```

## Queries

### Basic Queries

```javascript
// Range query with pagination
const someFriends = await db.friends
  .where("age")
  .between(20, 25)
  .offset(150)
  .limit(25)
  .toArray();

// Case-insensitive equality
await db.friends
  .where("name")
  .equalsIgnoreCase("josephine")
  .each((friend) => {
    console.log("Found Josephine", friend);
  });

// Starts with (multiple options)
const abcFriends = await db.friends
  .where("name")
  .startsWithAnyOfIgnoreCase(["a", "b", "c"])
  .toArray();
```

### Compound Index Queries

```javascript
// Using compound index directly
const forbundsKansler = await db.friends
  .where("[firstName+lastName]")
  .equals(["Angela", "Merkel"])
  .first();

// Simplified object syntax (Dexie 2.0+)
const forbundsKansler = await db.friends
  .where({
    firstName: "Angela",
    lastName: "Merkel",
  })
  .first();

// Or with get()
const forbundsKansler = await db.friends.get({
  firstName: "Angela",
  lastName: "Merkel",
});

// Range query on compound index (sorted by lastName)
const angelasSortedByLastName = await db.friends
  .where("[firstName+lastName]")
  .between(["Angela", ""], ["Angela", "\uffff"])
  .toArray();
```

### OR Queries

```javascript
await db.friends
  .where("age")
  .above(25)
  .or("shoeSize")
  .below(8)
  .or("interests")
  .anyOf("sports", "pets", "cars")
  .modify((friend) => friend.tags.push("marketing-target"));
```

### Filtering

```javascript
// Filter with callback (less efficient than where())
const friendsContainingLetterA = await db.friends
  .filter((friend) => /a/i.test(friend.name))
  .toArray();
```

### Ordering and Limiting

```javascript
// Top 5 by score (descending)
const best5GameSession = await db.gameSessions
  .orderBy("score")
  .reverse()
  .limit(5)
  .toArray();
```

## Transactions

Transactions ensure atomicity: if any error occurs, all modifications roll back.

### Basic Transaction

```javascript
db.transaction("rw", db.friends, db.pets, function () {
  return db.pets.add({ name: "Bamse", kind: "cat" }).then((petId) => {
    return db.friends.add({ name: "Kate", age: 89, pets: [petId] });
  });
}).catch((error) => {
  console.error(error.stack || error);
});
```

Use `'rw'` for read-write, `'r'` for read-only transactions.

### Synchronous Operations in Transactions

Within a transaction, you don't need to await each operation:

```javascript
// Without transaction (requires chaining)
db.friends
  .add({ name: "Ulla Bella", age: 87, isCloseFriend: 0 })
  .then(() => db.friends.add({ name: "Elna", age: 99, isCloseFriend: 1 }))
  .then(() =>
    db.friends
      .where("age")
      .above(65)
      .each((friend) => console.log("Retired friend: " + friend.name)),
  )
  .catch(console.error);

// With transaction (synchronous style)
db.transaction("rw", db.friends, function () {
  db.friends.add({ name: "Ulla Bella", age: 87, isCloseFriend: 0 });
  db.friends.add({ name: "Elna", age: 99, isCloseFriend: 1 });
  db.friends
    .where("age")
    .above(65)
    .each((friend) => console.log("Retired friend: " + friend.name));
}).catch(console.error);
```

### Nested Transactions

Functions using transactions can be composed into larger transactions:

```javascript
function goodFriends() {
  return db.friends.where("tags").equals("close-friend");
}

async function addComment(friendId, comment) {
  await db.friends
    .where("id")
    .equals(friendId)
    .modify((friend) => friend.comments.push(comment));
}

// Compose into umbrella transaction
async function spreadYourLove() {
  await db.transaction("rw", db.friends, async () => {
    const goodFriendKeys = await goodFriends().primaryKeys();
    await Promise.all(
      goodFriendKeys.map((id) => addComment(id, "I like you!")),
    );
  });
}

// Further compose
db.transaction("rw", db.friends, db.diary, async () => {
  await spreadYourLove();
  await db.diary.log({
    date: Date.now(),
    text: "Today I successfully spread my love",
  });
}).catch((err) => {
  console.error("I failed to spread my love :( " + err.stack);
});
```

### Error Handling

Catching an error prevents transaction abort:

```javascript
db.transaction("rw", db.friends, function () {
  db.friends.add({ id: 1, name: "Fredrik" });
  db.friends.add({ id: 1, name: "Fredrik" }).catch((e) => {
    console.error("Failed to add Foo friend");
    throw e; // Re-throw to abort transaction
  });
});
```

If you catch without re-throwing, the transaction continues.

### Auto-Commit Behavior

IndexedDB commits transactions when not used within the same task. Don't await non-Dexie async APIs inside transactions, or you'll get `TransactionInactiveError`.

Use `Dexie.waitFor()` to wait for other async APIs while keeping the transaction active (Dexie 2.0.0-beta.6+).

## React Integration

### Installation

```bash
npm install dexie dexie-react-hooks
# or
yarn add dexie dexie-react-hooks
```

### Database Module

Create a singleton database instance:

```javascript
// db.js
import { Dexie } from "dexie";

export const db = new Dexie("myDatabase");

db.version(1).stores({
  friends: "++id, name, age",
});
```

### useLiveQuery Hook

The `useLiveQuery()` hook observes query results and re-renders when data changes.

```typescript
export function useLiveQuery<T, TDefault = undefined>(
  querier: () => Promise<T> | T,
  deps?: any[],
  defaultResult?: TDefault,
): T | TDefault;
```

**Basic Usage:**

```javascript
import { useLiveQuery } from "dexie-react-hooks";
import { db } from "./db";

export function FriendList() {
  const friends = useLiveQuery(() => db.friends.toArray());

  if (!friends) return null; // Loading

  return (
    <ul>
      {friends.map((friend) => (
        <li key={friend.id}>
          {friend.name}, {friend.age}
        </li>
      ))}
    </ul>
  );
}
```

**With Dependencies:**

```javascript
export function FriendList({ minAge, maxAge }) {
  const friends = useLiveQuery(
    async () => {
      return db.friends.where("age").between(minAge, maxAge).toArray();
    },
    [minAge, maxAge], // Re-run when these change
  );

  if (!friends) return null;

  return (
    <ul>
      {friends.map((friend) => (
        <li key={friend.id}>
          {friend.name}, {friend.age}
        </li>
      ))}
    </ul>
  );
}
```

**Decoupled Pattern:**

```javascript
export function FriendList({
  getFriendCount,
  getFriendsByAge,
  onBirthdayClick,
}) {
  const friendCount = useLiveQuery(getFriendCount);
  const friends = useLiveQuery(() => getFriendsByAge(maxAge), [maxAge]);

  // ...
}

// Parent provides callbacks
function App() {
  const getFriendCount = () => db.friends.count();
  const getFriendsByAge = (maxAge) =>
    db.friends.where("age").belowOrEqual(maxAge).sortBy("id");

  return (
    <FriendList
      getFriendCount={getFriendCount}
      getFriendsByAge={getFriendsByAge}
    />
  );
}
```

### Add Form Component

```javascript
export function AddFriendForm({ defaultAge } = { defaultAge: 21 }) {
  const [name, setName] = useState("");
  const [age, setAge] = useState(defaultAge);
  const [status, setStatus] = useState("");

  async function addFriend() {
    try {
      const id = await db.friends.add({ name, age });
      setStatus(`Friend ${name} successfully added. Got id ${id}`);
      setName("");
      setAge(defaultAge);
    } catch (error) {
      setStatus(`Failed to add ${name}: ${error}`);
    }
  }

  return (
    <>
      <p>{status}</p>
      Name:
      <input
        type="text"
        value={name}
        onChange={(ev) => setName(ev.target.value)}
      />
      Age:
      <input
        type="number"
        value={age}
        onChange={(ev) => setAge(Number(ev.target.value))}
      />
      <button onClick={addFriend}>Add</button>
    </>
  );
}
```

### Complete Example

```javascript
import React, { useState } from "react";
import { useLiveQuery } from "dexie-react-hooks";
import { db } from "../db";

export function FriendList() {
  const [maxAge, setMaxAge] = useState(21);

  const friends = useLiveQuery(
    () => db.friends.where("age").belowOrEqual(maxAge).sortBy("id"),
    [maxAge],
  );

  const friendCount = useLiveQuery(() => db.friends.count());

  if (!friends || friendCount === undefined) return null;

  return (
    <div>
      <p>
        Your have <b>{friendCount}</b> friends in total.
      </p>
      <label>
        Please enter max age to query:
        <input
          type="number"
          value={maxAge}
          onChange={(ev) => setMaxAge(parseInt(ev.target.value, 10))}
        />
      </label>
      <ul>
        {friends.map((friend) => (
          <li key={friend.id}>
            {friend.name}, {friend.age}
            <button
              onClick={() =>
                db.friends.where({ id: friend.id }).modify((f) => ++f.age)
              }
            >
              Birthday!
            </button>
          </li>
        ))}
      </ul>
    </div>
  );
}
```

### useLiveQuery Rules

1. **Only call Dexie APIs** from the querier function
2. **Wrap non-Dexie async calls** with `Promise.resolve()`:

```javascript
const friendWithMetaData = useLiveQuery(async () => {
  const friend = await db.friends.get(id);

  // Wrap non-Dexie API
  const friendMetaData = await Promise.resolve(
    fetch(friend.metaDataUrl).then((res) => res.json()),
  );

  friend.metaData = friendMetaData;
  return friend;
}, [id]);
```

3. **Use error boundaries** for error handling

### Observation Behavior

- Fine-grained: Only queries affected by changes re-render
- Works across tabs/windows (Dexie 3.1+)
- Changes must be made via Dexie (DevTools changes not observed)
- Same-origin only (IndexedDB limitation)

## TypeScript

### Type-Safe Database

```typescript
// db.ts
import { Dexie, type EntityTable } from "dexie";

interface Friend {
  id: number;
  name: string;
  age: number;
}

const db = new Dexie("FriendsDatabase") as Dexie & {
  friends: EntityTable<Friend, "id">;
};

db.version(1).stores({
  friends: "++id, name, age",
});

export type { Friend };
export { db };
```

The `EntityTable<Friend, 'id'>` type provides:

- Typed `add()`, `put()`, `update()` parameters
- Typed query results
- Primary key type inference

## Advanced Topics

### Class Binding

Map database objects to class instances:

```javascript
class Friend {
  save() {
    return db.friends.put(this);
  }

  get age() {
    return moment(Date.now()).diff(this.birthDate, "years");
  }
}

db.friends.mapToClass(Friend);
```

### Joining Data

```javascript
const db = new Dexie("music");

db.version(1).stores({
  genres: "++id,name",
  albums: "++id,name,year,*tracks",
  bands: "++id,name,*albumIds,genreId",
});

async function getBandsStartingWithA() {
  const bands = await db.bands.where("name").startsWith("A").toArray();

  await Promise.all(
    bands.map(async (band) => {
      [band.genre, band.albums] = await Promise.all([
        db.genres.get(band.genreId),
        db.albums.where("id").anyOf(band.albumIds).toArray(),
      ]);
    }),
  );

  return bands;
}
```

### Binary Data

**Storing Blobs:**

```javascript
const db = new Dexie("MyImgDb");

db.version(1).stores({
  friends: "name",
});

async function downloadAndStoreImage() {
  const res = await fetch("some-url-to-an-image.png");
  const blob = await res.blob();

  await db.friends.put({
    name: "David",
    image: blob,
  });
}
```

**Indexing Binary Data (IndexedDB 2.0):**

```javascript
const db = new Dexie("MyImgDb");

db.version(1).stores({
  friends: "id, name", // Binary UUID as id
});

async function playWithBinaryPrimKey() {
  await db.friends.put({
    id: new Uint8Array([1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16]),
    name: "David",
  });

  const friend = await db.friends.get(
    new Uint8Array([1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16]),
  );

  if (friend) {
    console.log(`Found friend: ${friend.name}`);
  }
}
```

Supported by Chrome, Safari, and Firefox (Firefox has bugs with binary primary keys).

### Promise-Specific Data (Zones)

Dexie implements thread-local-storage-like patterns for promises, enabling transaction context to flow through async operations without explicit passing.

### Indexable Types

Only certain types can be indexed:

- string
- number
- Date
- Array

**Not indexable:** boolean, null, undefined

Using `orderBy()` on non-indexable properties won't include those objects in results.
