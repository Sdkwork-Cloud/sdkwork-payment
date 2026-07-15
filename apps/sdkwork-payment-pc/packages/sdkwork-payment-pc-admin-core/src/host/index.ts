/**
 * Host runtime integration subpath.
 *
 * Reserved contract surface for host apps to inject runtime context (auth,
 * feature flags, telemetry) into the admin composition. Currently empty —
 * host integration is handled by the parent app's bootstrap. Aligns with
 * `CORE_EXPORT_SUBPATHS` in `sdkwork-specs/tools/lib/app-composition.mjs`.
 */

export {};
