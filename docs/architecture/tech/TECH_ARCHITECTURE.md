# Payment Technical Architecture
Specs: ARCHITECTURE_DECISION_SPEC.md, DOCUMENTATION_SPEC.md, API_SPEC.md, WEB_FRAMEWORK_SPEC.md, WEB_BACKEND_SPEC.md, SECURITY_SPEC.md

Status: active
Owner: SDKWork maintainers
Updated: 2026-06-27

## 1. Architecture Overview

`sdkwork-payment` owns the full **payment** capability for the SDKWork commerce domain.

## Capability stack

| Layer | Path |
| --- | --- |
| Domain (Rust) | `crates/sdkwork-commerce-payment-service/` |
| SQL | `crates/sdkwork-commerce-payment-repository-sqlx/` |
| HTTP routers | `crates/sdkwork-routes-payment-*-api/` |
| API server | `crates/sdkwork-payment-standalone-gateway/` |
| PC client | `apps/sdkwork-payment-pc/` |
| Client facade | `packages/common/payment/sdkwork-payment-service/` |

## PC surface

```text
apps/sdkwork-payment-pc/
  packages/sdkwork-payment-pc-core/
  packages/sdkwork-payment-pc-shell/
  packages/sdkwork-payment-pc-payment/    ← migrated from sdkwork-commerce-pc
```

Composition apps (`sdkwork-mall`, etc.) consume `@sdkwork/payment-pc-payment` via workspace paths — not a central commerce PC repo.

## API ownership

- App API prefix: `/app/v3/api/payments`
- Backend API prefix: `/backend/v3/api/payments`
- Table prefix: `commerce_` (commerce domain)

## HTTP contract layer

### Route manifest (C17)

每个 route crate 通过 `sdkwork-web-core` 的 `HttpRoute` / `HttpRouteManifest` 声明
静态路由清单，满足 `API_SPEC.md` §4.2.1 与 `WEB_BACKEND_SPEC.md` §4.2/§4.3 的要求：

- `sdkwork-routes-payment-app-api/src/http_route_manifest.rs` → `app_route_manifest()`
  + `gateway_route_manifest()`（22 条路由，全部 `RouteAuth::DualToken`）
- `sdkwork-routes-payment-backend-api/src/http_route_manifest.rs` → `backend_route_manifest()`
  + `gateway_route_manifest()`（19 条路由，全部 `RouteAuth::DualToken`）

manifest 在 `web_bootstrap.rs` 中通过 `WebFrameworkLayer::with_route_manifest` 注入
框架中间件层，启用运行时 operationId / rate-limit tier / 公共路径解析。

### Contract fallback

`sdkwork-payment-gateway-assembly` 导出 `gateway_contract_fallback_config()`，合并
app-api 与 backend-api 的所有 manifest 路由。`sdkwork-payment-standalone-gateway` 将其传入
`ServiceRouterConfig::with_contract_fallback`，使框架自动挂载 axum `fallback`：

- manifest 内声明但运行时未挂载 handler 的路径 → **501 Not Implemented** Problem+json
- 完全未知路径 → **404 Not Found** Problem+json

### RFC 9457 Problem+json 错误响应（C16）

所有 app-api / backend-api handler 的错误响应统一使用 `problem_error_response()`
（`problem_details.rs`），输出 `application/problem+json` content-type，包含
`type` / `title` / `status` / `detail` / `code` / `traceId` 字段。成功响应保留
`{code, msg, data}` envelope。

### IAM 边界（C11/C12）

backend-api 所有 handler 通过 `backend_runtime_subject_from_extension` 强制执行：

1. `IamAppContext` 必须存在（否则 401）
2. `LoginScope::Organization`（拒绝个人租户会话）
3. `can_access_backend_api()` 权限检查
4. `organization_id` 非空（租户范围强约束）

URL 查询参数仅暴露过滤/分页字段，`tenant_scope` 由 IAM context 注入，杜绝跨租户越权。

### Webhook 重放限制（C15）

backend-api webhook replay 路由通过 `COALESCE(retries, 0) < 5` 原子守卫防止无限重放，
超限返回 409 Conflict Problem+json，不存在返回 404。

## Production hardening

- **CORS**（C13）：`PAYMENT_API_CORS_ORIGINS` 环境变量驱动白名单，严禁 wildcard
- **Graceful shutdown**（H1）：SIGINT/SIGTERM 后停止接受新连接，等待在途请求
- **请求超时**（H1）：30s
- **请求体限制**（H1）：1 MiB
- **结构化追踪**（C23）：TraceLayer span / URI / 状态码 / 耗时

## Verification

```powershell
cd E:\sdkwork-space\sdkwork-payment
cargo test --workspace
pnpm verify
```

## Related docs

- Commerce repository dissolution: `../../sdkwork-specs/MIGRATION_SPEC.md` §8
