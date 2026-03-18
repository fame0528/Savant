# Savant Coding System — Python Supplement

> **Version:** 0.0.1  
> **Requires:** `dev/SAVANT-CODING-SYSTEM.md` (foundation, loaded first)  
> **Scope:** Python standards for Savant agents.

---

## Verification Commands

Always run these before marking a task complete:

```bash
python -m py_compile file.py     # Syntax check
python -m mypy . --strict        # Type check (0 errors)
python -m pytest                 # All tests pass
python -m ruff check .           # Linting (0 warnings)
python -m ruff format --check .  # Formatting
python -m bandit -r .            # Security scan
```

## Type System Rules

- **Type hints everywhere.** Every function signature must have types.
- **Use `typing` module** for complex types (`Optional`, `Union`, `Generic`, `Protocol`).
- **Use `from __future__ import annotations`** for forward references.
- **No `Any`** unless interfacing with untyped third-party code. Document why.
- **Use `Protocol`** for structural subtyping (duck typing with safety).
- **Use `Literal`** for constrained string/int values.
- **Use `TypedDict`** for dictionary shapes.
- **Use `dataclass` or `pydantic`** for data models.

```python
from __future__ import annotations
from typing import Protocol, TypedDict, Literal, TypeVar, Generic
from dataclasses import dataclass
from enum import Enum

# Branded types via NewType
from typing import NewType
UserId = NewType('UserId', str)
SessionId = NewType('SessionId', str)

# Protocol for structural typing
class Renderable(Protocol):
    def render(self) -> str: ...

# TypedDict for structured dicts
class ConfigDict(TypedDict):
    host: str
    port: int
    debug: bool

# Dataclass for models
@dataclass(frozen=True, slots=True)
class Agent:
    id: UserId
    name: str
    status: Literal['active', 'inactive']

# Generic result type
T = TypeVar('T')
E = TypeVar('E')

@dataclass(frozen=True, slots=True)
class Ok(Generic[T]):
    value: T

@dataclass(frozen=True, slots=True)
class Err(Generic[E]):
    error: E

type Result[T, E] = Ok[T] | Err[E]
```

## Error Handling Patterns

```python
# Use custom exception hierarchy
class SavantError(Exception):
    """Base error for all Savant operations."""
    pass

class ConfigError(SavantError):
    """Configuration-related errors."""
    pass

class NetworkError(SavantError):
    """Network-related errors."""
    pass

# Use Result pattern for explicit error handling (when not using exceptions)
def parse_config(raw: str) -> Result[Config, str]:
    try:
        data = json.loads(raw)
        return Ok(Config(**data))
    except (json.JSONDecodeError, TypeError) as e:
        return Err(f"Parse failed: {e}")

# Use context managers for resource cleanup
from contextlib import contextmanager

@contextmanager
def temporary_file(suffix: str = '.tmp'):
    path = Path(tempfile.mktemp(suffix=suffix))
    try:
        yield path
    finally:
        path.unlink(missing_ok=True)
```

## Async Patterns

```python
import asyncio
from typing import AsyncIterator

# Use asyncio for I/O-bound concurrency
async def fetch_data(url: str) -> Result[bytes, str]:
    try:
        async with aiohttp.ClientSession() as session:
            async with session.get(url) as resp:
                if resp.status != 200:
                    return Err(f"HTTP {resp.status}")
                return Ok(await resp.read())
    except aiohttp.ClientError as e:
        return Err(f"Network error: {e}")

# Use asyncio.gather for parallel operations
async def fetch_all(urls: list[str]) -> list[Result[bytes, str]]:
    return await asyncio.gather(*[fetch_data(url) for url in urls])

# Use async generators for streaming
async def stream_events(ws: WebSocket) -> AsyncIterator[Event]:
    async for message in ws:
        yield parse_event(message)

# Never use blocking I/O in async code
# BAD: time.sleep(1)  # blocks event loop
# GOOD: await asyncio.sleep(1)  # yields to event loop
```

## Naming Conventions

| Element | Convention | Example |
|---------|-----------|---------|
| Files | snake_case | `user_service.py` |
| Classes | PascalCase | `UserService` |
| Functions | snake_case | `get_user_by_id` |
| Constants | SCREAMING_SNAKE | `MAX_RETRIES` |
| Private | _single_leading_underscore | `_internal_method` |
| Type aliases | PascalCase | `UserId`, `ConfigDict` |
| Booleans | `is_`/`has_`/`should_` prefix | `is_active`, `has_permission` |
| Enums | PascalCase class, UPPER members | `Status.ACTIVE` |

## Project Structure

```
project/
├── src/
│   ├── __init__.py
│   ├── config.py         # Configuration
│   ├── models.py         # Data models
│   ├── services.py       # Business logic
│   ├── api/              # API routes
│   │   ├── __init__.py
│   │   ├── routes.py
│   │   └── middleware.py
│   └── utils/            # Utilities
│       ├── __init__.py
│       └── helpers.py
├── tests/
│   ├── __init__.py
│   ├── test_services.py
│   └── test_api.py
├── pyproject.toml         # Project config
└── README.md
```

## Package Management

- Use `pyproject.toml` for project configuration.
- Use `uv` or `pip` with `requirements.txt` for dependencies.
- Pin exact versions for security-critical deps.
- Use virtual environments. Never install globally.
- Audit with `pip-audit` or `safety` regularly.

## Testing Patterns

```python
import pytest
from unittest.mock import AsyncMock, patch

class TestUserService:
    async def test_get_user_by_id(self):
        result = await get_user_by_id(UserId('valid-id'))
        assert isinstance(result, Ok)
        assert result.value.id == 'valid-id'

    async def test_get_user_not_found(self):
        result = await get_user_by_id(UserId('nonexistent'))
        assert isinstance(result, Err)

    async def test_network_failure(self):
        with patch('aiohttp.ClientSession.get', side_effect=aiohttp.ClientError):
            result = await get_user_by_id(UserId('any-id'))
            assert isinstance(result, Err)

# Use fixtures for shared setup
@pytest.fixture
def mock_db():
    return AsyncMock(spec=Database)

# Use parametrize for multiple test cases
@pytest.mark.parametrize('input,expected', [
    ('hello', 'HELLO'),
    ('world', 'WORLD'),
    ('', ''),
])
def test_upper(input: str, expected: str):
    assert input.upper() == expected
```

## Performance Rules

- **Use `__slots__`** on dataclasses for memory efficiency.
- **Use `frozen=True`** on dataclasses when immutability is desired.
- **Use generators** for large sequences. Don't materialize lists unnecessarily.
- **Use `asyncio.gather`** for parallel I/O operations.
- **Profile with `cProfile`** before optimizing.
- **Use `lru_cache`** for memoization of pure functions.
- **Avoid global mutable state.** Use dependency injection.

## Security Rules

- **Use `secrets` module** for cryptographic randomness. Never `random`.
- **Hash passwords with `bcrypt` or `argon2`.** Never plain text.
- **Validate all input.** Use `pydantic` for structured validation.
- **Use parameterized queries.** Never string-interpolate SQL.
- **Store secrets in environment variables.** Never commit `.env`.
- **Use `bandit`** for security scanning.

## Documentation

```python
def get_user_by_id(user_id: UserId) -> Result[User, str]:
    """Retrieve a user by their unique identifier.

    Args:
        user_id: The unique user identifier.

    Returns:
        Ok(User) if found, Err("not found") if not found,
        Err("network error") if the request fails.

    Examples:
        >>> result = get_user_by_id(UserId('abc-123'))
        >>> assert isinstance(result, Ok)
    """
```

- Use Google-style docstrings.
- Document all public functions and classes.
- Include `Raises`, `Returns`, `Examples` sections.

---

*Load this supplement after the foundation. Together they form the complete Python coding standard.*
