export type SdkworkPaymentStatus =
  | "closed"
  | "failed"
  | "pending"
  | "success"
  | "timeout"
  | "unknown";

// === 分页契约（对齐 PAGINATION_SPEC.md / API_SPEC.md §16） ===
// 全部分页辅助函数与类型从 ./list-page.ts 统一导出，避免重复定义。
export * from "./list-page";
