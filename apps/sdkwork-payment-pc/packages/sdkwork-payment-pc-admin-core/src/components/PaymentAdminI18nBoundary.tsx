import { useEffect, useMemo, useRef, type PropsWithChildren } from "react";
import {
  PAYMENT_ADMIN_I18N_CATALOG,
  usePaymentAdminMessages,
  type PaymentAdminMessages,
} from "../i18n";

const LOCALIZED_ATTRIBUTES = ["aria-label", "placeholder", "title"] as const;
/** Radix renders Dialog content, Select popovers, and popper overlays into
 *  `document.body` via portals, which sit outside the boundary root. These
 *  scope selectors identify portal content owned by admin workspaces. */
const PORTAL_SCOPE_SELECTOR =
  '[role="dialog"], [role="listbox"], [data-radix-popper-content-wrapper]';

function escapeRegularExpression(value: string) {
  return value.replace(/[.*+?^${}()|[\]\\]/gu, "\\$&");
}

function replaceCopy(value: string, phrases: Record<string, string>, tokens: Record<string, string>) {
  const exact = phrases[value];
  if (exact !== undefined) return exact;
  return Object.entries(tokens).sort(([left], [right]) => right.length - left.length).reduce(
    (translated, [source, target]) => translated.replace(
      new RegExp(
        /^[A-Za-z0-9_]+$/u.test(source)
          ? `\\b${escapeRegularExpression(source)}\\b`
          : escapeRegularExpression(source),
        "gu",
      ),
      target,
    ),
    value,
  );
}

/** Localizes legacy workspace controls, lazy dialogs, and accessible text from the registered catalog. */
export function PaymentAdminI18nBoundary({ children }: PropsWithChildren) {
  const rootRef = useRef<HTMLDivElement>(null);
  const messages = usePaymentAdminMessages().legacy;
  const catalogCopy = useMemo(() => {
    const canonical = new Map<string, string>();
    const reversePhrases: Record<string, string> = {};
    const reverseTokens: Record<string, string> = {};
    const locales = Object.values(PAYMENT_ADMIN_I18N_CATALOG.locales) as PaymentAdminMessages[];
    for (const locale of locales) {
      for (const [source, localized] of Object.entries(locale.legacy.phrases)) {
        canonical.set(source, source); canonical.set(localized, source);
        reversePhrases[localized] = source;
      }
      for (const [source, localized] of Object.entries(locale.legacy.tokens)) {
        canonical.set(source, source); canonical.set(localized, source);
        reverseTokens[localized] = source;
      }
    }
    return { canonical, reversePhrases, reverseTokens };
  }, []);

  useEffect(() => {
    const root = rootRef.current;
    if (!root) return undefined;
    const localizeValue = (value: string) => {
      const canonical = catalogCopy.canonical.get(value)
        ?? replaceCopy(value, catalogCopy.reversePhrases, catalogCopy.reverseTokens);
      return replaceCopy(canonical, messages.phrases, messages.tokens);
    };
    const localizeTree = (node: Node) => {
      const walker = document.createTreeWalker(node, NodeFilter.SHOW_TEXT);
      let textNode = walker.nextNode();
      while (textNode) {
        const source = textNode.textContent ?? "";
        const translated = localizeValue(source);
        if (translated !== source) textNode.textContent = translated;
        textNode = walker.nextNode();
      }
      const elements = node instanceof Element ? [node, ...Array.from(node.querySelectorAll("*"))] : Array.from(root.querySelectorAll("*"));
      for (const element of elements) for (const attribute of LOCALIZED_ATTRIBUTES) {
        const source = element.getAttribute(attribute);
        if (!source) continue;
        const translated = localizeValue(source);
        if (translated !== source) element.setAttribute(attribute, translated);
      }
    };
    // Portal content (Radix Dialog / Select / popper) renders into document.body,
    // outside the boundary root, so it is never observed by the root-scoped
    // observer below. Localize any portal scope whose text matches the catalog —
    // the catalog itself is the allow-list, so unrelated surfaces are untouched.
    const localizeAddedNode = (node: Node) => {
      if (root.contains(node)) {
        localizeTree(node);
        return;
      }
      const host = node instanceof Element ? node : node.parentElement;
      const scope = host?.closest(PORTAL_SCOPE_SELECTOR);
      if (scope) localizeTree(scope);
    };
    localizeTree(root);
    // Catch portal content already open on first render (e.g. after HMR).
    for (const scope of document.querySelectorAll(PORTAL_SCOPE_SELECTOR)) {
      if (!root.contains(scope)) localizeTree(scope);
    }
    const observer = new MutationObserver((records) => records.forEach((record) => record.addedNodes.forEach(localizeAddedNode)));
    observer.observe(root, { childList: true, subtree: true });
    observer.observe(document.body, { childList: true, subtree: true });
    return () => observer.disconnect();
  }, [catalogCopy, messages]);
  return <div ref={rootRef}>{children}</div>;
}
