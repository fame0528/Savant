# Savant Coding System — TypeScript Supplement

> **Version:** 0.0.1  
> **Requires:** `dev/SAVANT-CODING-SYSTEM.md` (foundation, loaded first)  
> **Scope:** TypeScript/JavaScript standards for Savant agents.

---

## Verification Commands

Always run these before marking a task complete:

```bash
npx tsc --noEmit            # 0 errors
npm run lint                 # 0 warnings
npm test                     # All tests pass
npm run build                # Builds successfully
```

## Type System Rules

- **Strict mode always.** `"strict": true` in tsconfig. No exceptions.
- **No `any`.** Use `unknown` + narrowing instead. Every `any` is a lie to the compiler.
- **No `as` casts** unless narrowing from `unknown`. Use type guards.
- **No `!` non-null assertions.** Use proper narrowing or `if (x !== null)`.
- **No `@ts-ignore` or `@ts-expect-error`.** Fix the type, don't suppress it.
- **Prefer `interface` for object shapes, `type` for unions/intersections/computed types.**
- **Use `satisfies`** to validate type conformance without widening.
- **Use `const` assertions** (`as const`) for literal types.
- **Branded types** for domain-specific primitives (IDs, emails, etc.).

```typescript
// Branded types for domain safety
type UserId = string & { readonly __brand: 'UserId' };
type SessionId = string & { readonly __brand: 'SessionId' };

function createUserId(id: string): UserId {
  if (!id.match(/^[a-f0-9-]{36}$/)) throw new Error('Invalid user ID');
  return id as UserId;
}

// Type guards for narrowing
function isError(value: unknown): value is Error {
  return value instanceof Error;
}

// satisfies for validation without widening
const routes = {
  home: '/',
  api: '/api/v1',
} satisfies Record<string, string>;
// routes.home is still '/', not string
```

## Error Handling Patterns

```typescript
// Use Result pattern for explicit error handling
type Result<T, E = Error> = { ok: true; value: T } | { ok: false; error: E };

function ok<T>(value: T): Result<T, never> {
  return { ok: true, value };
}

function err<E>(error: E): Result<never, E> {
  return { ok: false, error };
}

// Usage
function parseConfig(raw: string): Result<Config, string> {
  try {
    const parsed = JSON.parse(raw);
    if (!isConfig(parsed)) return err('Invalid config shape');
    return ok(parsed);
  } catch (e) {
    return err(`Parse failed: ${e}`);
  }
}

// Never throw in library code. Return Result.
// Only throw in startup/initialization where recovery is impossible.
```

## Async Patterns

```typescript
// Always handle promise rejections
async function fetchData(url: string): Promise<Result<Response, string>> {
  try {
    const res = await fetch(url);
    if (!res.ok) return err(`HTTP ${res.status}`);
    return ok(res);
  } catch (e) {
    return err(`Network error: ${e}`);
  }
}

// Never use floating promises
// BAD: doSomething();  // fire and forget
// GOOD: await doSomething();  // intentional
// GOOD: doSomething().catch(logError);  // explicitly handled

// Use AbortController for cancellation
const controller = new AbortController();
const timeout = setTimeout(() => controller.abort(), 30000);
try {
  const res = await fetch(url, { signal: controller.signal });
} finally {
  clearTimeout(timeout);
}
```

## Naming Conventions

| Element | Convention | Example |
|---------|-----------|---------|
| Files (components) | PascalCase | `UserProfile.tsx` |
| Files (utilities) | camelCase | `formatDate.ts` |
| Files (types) | camelCase | `userTypes.ts` |
| Interfaces | PascalCase, no `I` prefix | `UserConfig` |
| Types | PascalCase | `UserId`, `Result<T>` |
| Functions | camelCase | `getUserById` |
| Constants | SCREAMING_SNAKE | `MAX_RETRIES` |
| Enums | PascalCase, values PascalCase | `Status.Active` |
| Booleans | `is`/`has`/`should` prefix | `isActive`, `hasPermission` |

## Package Management

- Use `npm` or `pnpm`. Lock file must be committed.
- Pin exact versions for security-critical deps. Use `^` for others.
- Audit with `npm audit` regularly.
- Keep dependencies minimal. Every dep is an attack surface.
- Use `devDependencies` for build/test tools only.

## React Patterns (Dashboard)

```typescript
// Prefer function components
// Use hooks for state and side effects

// Custom hooks for reusable logic
function useWebSocket(url: string) {
  const [state, setState] = useState<WsState>('disconnected');
  const wsRef = useRef<WebSocket | null>(null);

  useEffect(() => {
    const ws = new WebSocket(url);
    ws.onopen = () => setState('connected');
    ws.onclose = () => setState('disconnected');
    ws.onerror = () => setState('error');
    wsRef.current = ws;
    return () => ws.close();
  }, [url]);

  return { state, send: wsRef.current?.send.bind(wsRef.current) };
}

// Memoization for expensive computations
const ExpensiveComponent = React.memo(({ data }: Props) => {
  const processed = useMemo(() => heavyComputation(data), [data]);
  return <div>{processed}</div>;
});

// Avoid inline objects/functions in JSX (causes re-renders)
// BAD: <Component style={{ color: 'red' }} />
// GOOD: const styles = { color: 'red' } as const;
```

## Testing Patterns

```typescript
import { describe, it, expect, vi } from 'vitest';

describe('UserService', () => {
  describe('getUserById', () => {
    it('should return user when found', async () => {
      const result = await getUserById('valid-id');
      expect(result.ok).toBe(true);
      if (result.ok) {
        expect(result.value.id).toBe('valid-id');
      }
    });

    it('should return error when not found', async () => {
      const result = await getUserById('nonexistent');
      expect(result.ok).toBe(false);
    });

    it('should handle network failure', async () => {
      vi.spyOn(global, 'fetch').mockRejectedValue(new Error('Network'));
      const result = await getUserById('any-id');
      expect(result.ok).toBe(false);
    });
  });
});
```

## Performance Rules

- **Avoid unnecessary re-renders.** Use `React.memo`, `useMemo`, `useCallback`.
- **Lazy load routes.** Use `React.lazy` + `Suspense` for code splitting.
- **Debounce expensive operations.** Search, resize handlers, etc.
- **Virtualize long lists.** Use `react-window` or similar for 100+ items.
- **Optimize images.** WebP format, lazy loading, responsive sizes.
- **Bundle analysis.** Run `npm run build` and check bundle size regularly.

## Security Rules

- **Sanitize all user input.** Never trust data from the client.
- **Use Content Security Policy headers.** Prevent XSS.
- **Store tokens in httpOnly cookies.** Never in localStorage for sensitive tokens.
- **Validate on server AND client.** Client validation is UX, server validation is security.
- **Use environment variables for secrets.** Never commit `.env` files.
- **Avoid `dangerouslySetInnerHTML`.** Use DOMPurify if absolutely necessary.

---

*Load this supplement after the foundation. Together they form the complete TypeScript coding standard.*
