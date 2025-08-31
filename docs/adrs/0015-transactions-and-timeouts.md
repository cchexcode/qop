# ADR-0015: Transactions and Timeout Semantics per Backend

## Status

Accepted

## Date

2025-01-31T00:18:00Z

## Context

Migrations must be executed safely and atomically. Each backend has different knobs for timeouts and transactional behavior.

## Decision

- Each migration apply/revert MUST execute within its own database transaction.
- Timeouts MUST be applied per-transaction using backend-specific mechanisms when configured.

### Backend Semantics
- PostgreSQL:
  - Use `SET LOCAL statement_timeout = <ms>` within the transaction to scope the timeout to that transaction.
  - Default pool size SHOULD be modest for a CLI (e.g., 10 connections).
- SQLite:
  - Use `PRAGMA busy_timeout = <ms>` within the transaction.
  - Pool size SHOULD be 1 to avoid concurrency pitfalls in local workflows.

## Consequences

### Positive
- Atomic migration operations
- Clear timeout scoping per operation
- Predictable resource usage for CLI workloads

### Negative
- Some long-running migrations may require explicit timeout tuning
- Per-transaction timeout setting adds a small overhead

## Implementation

- Postgres:
  - `set_timeout_if_needed(tx, Option<u64>)` with ms conversion and `SET LOCAL statement_timeout`.
  - Execute SQL via `sqlx::raw_sql(sql)`.
- SQLite:
  - `set_timeout_if_needed(tx, Option<u64>)` with `PRAGMA busy_timeout`.
  - Execute SQL via `sqlx::raw_sql(sql)`.

- Each migration is applied in its own transaction; dry-run executes and then `rollback()`; otherwise `commit()`.

## References

- `subsystem::postgres::migration::{set_timeout_if_needed, execute_sql_statements}`
- `subsystem::sqlite::migration::{set_timeout_if_needed, execute_sql_statements}`
