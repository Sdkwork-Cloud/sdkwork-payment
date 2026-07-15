/**
 * Shared defensive parsing helpers for admin controllers.
 *
 * These helpers coerce `unknown` wire payloads (from the backend SDK
 * boundary) into typed views. They are camelCase + snake_case tolerant to
 * accommodate both wire forms. Extracted to admin-core to eliminate 4
 * duplicate definitions across the admin capability packages.
 */

export function asString(value: unknown): string | undefined {
  if (typeof value !== "string") {
    return undefined;
  }
  const trimmed = value.trim();
  return trimmed || undefined;
}

export function asRequiredString(value: unknown, fallback = ""): string {
  return asString(value) ?? fallback;
}

/**
 * Generic status coercion: returns `value` if it is a string contained in
 * `allowed`, otherwise returns `fallback`. The generic `<T extends string>`
 * preserves the literal union type without requiring `as` casts at call
 * sites.
 */
export function asStatus<T extends string>(value: unknown, allowed: readonly T[], fallback: T): T {
  return typeof value === "string" && (allowed as readonly string[]).includes(value)
    ? (value as T)
    : fallback;
}

export function asNumber(value: unknown): number | undefined {
  return typeof value === "number" && Number.isFinite(value) ? value : undefined;
}

export function asRecord(value: unknown): Record<string, unknown> {
  if (value && typeof value === "object" && !Array.isArray(value)) {
    return value as Record<string, unknown>;
  }
  return {};
}
