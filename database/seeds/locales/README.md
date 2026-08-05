# Locale seed directories

Locale seed directories:

- `zh-CN/` — default active locale
- `en-US/`, `ja-JP/`, `de-DE/`, `fr-FR/`, `ru-RU/`, `ko-KR/` — reserved placeholders

Each active locale directory contains ordered SQL seed files referenced by `seeds/seed.manifest.json`.

Localized reference-data names (payment methods, channels, providers) follow
`DATABASE_SPEC.md` §6.4.1: stable base rows plus locale-specific translation
files. Each locale file only manages its own keys through `jsonb_set` on the
`display_name_i18n` / `channel_name_i18n` maps, so repeated seeds are
idempotent and locales never overwrite each other.
