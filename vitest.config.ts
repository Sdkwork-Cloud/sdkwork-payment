import { existsSync, readFileSync } from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";
import react from "@vitejs/plugin-react";
import { defineConfig } from "vitest/config";

const root = path.dirname(fileURLToPath(import.meta.url));

function forceLocalReactPlugin() {
  const reactRoot = path.join(root, "node_modules/react");
  const reactDomRoot = path.join(root, "node_modules/react-dom");
  return {
    name: "force-local-react",
    enforce: "pre" as const,
    resolveId(source: string) {
      if (source === "react") {
        return path.join(reactRoot, "index.js");
      }
      if (source.startsWith("react/")) {
        return path.join(reactRoot, `${source.slice("react/".length)}.js`);
      }
      if (source === "react-dom") {
        return path.join(reactDomRoot, "index.js");
      }
      if (source.startsWith("react-dom/")) {
        return path.join(reactDomRoot, `${source.slice("react-dom/".length)}.js`);
      }
      return null;
    },
  };
}

function loadLocalReactAliases() {
  const reactRoot = path.join(root, "node_modules/react");
  const reactDomRoot = path.join(root, "node_modules/react-dom");
  const uiReactRoot = path.resolve(root, "../sdkwork-ui/sdkwork-ui-pc-react");
  const radixRoot = path.join(uiReactRoot, "node_modules/@radix-ui");
  return [
    { find: "react", replacement: reactRoot },
    { find: "react-dom", replacement: reactDomRoot },
    { find: "react/jsx-runtime", replacement: path.join(reactRoot, "jsx-runtime.js") },
    { find: "react/jsx-dev-runtime", replacement: path.join(reactRoot, "jsx-dev-runtime.js") },
    { find: /^@radix-ui\/(.*)$/, replacement: `${radixRoot}/$1` },
  ];
}

function loadTsconfigAliases() {
  const tsconfig = JSON.parse(readFileSync(path.join(root, "tsconfig.base.json"), "utf8"));
  const paths = tsconfig?.compilerOptions?.paths ?? {};
  return Object.entries(paths)
    .map(([find, replacements]) => {
      const replacement = Array.isArray(replacements) ? replacements[0] : undefined;
      if (typeof replacement !== "string") {
        return null;
      }
      return {
        find,
        replacement: path.resolve(root, replacement),
      };
    })
    .filter(Boolean)
    .sort((left, right) => right!.find.length - left!.find.length) as Array<{
    find: string;
    replacement: string;
  }>;
}

export default defineConfig({
  plugins: [forceLocalReactPlugin(), react()],
  resolve: {
    dedupe: ["react", "react-dom"],
    alias: [
      ...loadLocalReactAliases(),
      ...loadTsconfigAliases(),
      {
        find: "lucide-react",
        replacement: path.join(root, "node_modules/lucide-react"),
      },
    ].filter((entry) => entry.find !== "@sdkwork/ui-pc-react" || existsSync(entry.replacement)),
  },
  test: {
    environment: "jsdom",
    setupFiles: [path.join(root, "vitest.setup.ts")],
    include: [
      "apps/sdkwork-payment-pc/packages/**/*.test.ts",
      "apps/sdkwork-payment-pc/packages/**/*.test.tsx",
    ],
  },
});
