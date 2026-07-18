# payment database module

Reference contract for payment capability tables under commerce platform bootstrap.

## Initialization state

This module is in **initialization state** for greenfield deployments:

1. **Baseline** — `database/ddl/baseline/{engine}/0001_payment_baseline.sql` contains the full DDL snapshot.
2. **Migrations** — `database/migrations/{engine}/` is reserved for post-GA incremental schema changes only. It is intentionally empty at initialization.
3. **Drift** — run `pnpm db:drift:check` before release.

## Commands

```bash
pnpm run db:validate
pnpm run db:materialize:contract
pnpm run db:plan
pnpm run db:init
pnpm run db:migrate
pnpm run db:seed
pnpm run db:status
pnpm run db:drift:check
```

## Payment bootstrap profiles

The initial payment catalog is selected by
`SDKWORK_PAYMENT_DATABASE_SEED_PROFILE` and is applied only when
`SDKWORK_PAYMENT_DATABASE_SEED_ON_BOOT=true` or when `pnpm db:seed` is run with
the desired profile. `seedOnBoot` remains `false` in the module manifest, so a
production service never writes bootstrap data unless deployment configuration
explicitly opts in.

| Runtime | Profile | Initial state |
| --- | --- | --- |
| Development | `development` | Complete catalog and an active local sandbox method/channel; external PSP templates remain inactive. |
| Test/CI | `test` | Complete catalog and an active isolated test sandbox method/channel; external PSP templates remain inactive. |
| Production | `production` or `standard` | Complete catalog, provider-account templates, and channels are present but inactive. |

For example, a development service can use:

```text
SDKWORK_PAYMENT_DATABASE_SEED_ON_BOOT=true
SDKWORK_PAYMENT_DATABASE_SEED_PROFILE=development
SDKWORK_PAYMENT_DATABASE_SEED_LOCALE=zh-CN
```

Production deployment should run the `production` seed explicitly during its
controlled database bootstrap, then configure secret references and activate
only the reviewed provider accounts, methods, and channels in the payment admin
workspace. Seed SQL contains references to environment variable names only; it
never persists credential values.

## Federated host integration

An application that embeds payment into a shared database pool must register
payment's owned module with the framework registry rather than copying payment
DDL or seed files into its own database directory. The registry runs each
module's lifecycle exactly through its manifest and `SDKWORK_<SERVICE>_DATABASE_*`
overrides.

```rust
use sdkwork_database_lifecycle::RegistryLifecycleOrchestrator;
use sdkwork_database_spi::DatabaseModuleRegistry;

let registry = DatabaseModuleRegistry::builder()
    .register(sdkwork_payment_database_host::database_module()?)?
    .build();

RegistryLifecycleOrchestrator::new(shared_pool, registry)
    .with_applied_by("your-application")
    .bootstrap_all_from_env()
    .await?;
```

Set `SDKWORK_PAYMENT_DATABASE_SEED_ON_BOOT=true` and choose `development`,
`test`, or `production` through `SDKWORK_PAYMENT_DATABASE_SEED_PROFILE` in the
integrating application's selected runtime profile. Do not rely on the host's
unprefixed configuration: payment owns its own lifecycle options. Production
continues to default to no automatic seed write unless an operator explicitly
enables it.
