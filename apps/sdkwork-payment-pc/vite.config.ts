import react from "@vitejs/plugin-react";
import { defineConfig } from "vite";

// SECURITY: 不再把 SDKWORK_ACCESS_TOKEN 内联进客户端 bundle。
// 认证 token 必须由运行时通过 httpOnly cookie / SDK 授权流程获取，
// 严禁以 `process.env.*` 静态 define 的方式注入到浏览器可见的代码中。
export default defineConfig({
  plugins: [react()],
  server: {
    port: 5182,
    host: "127.0.0.1",
  },
});
