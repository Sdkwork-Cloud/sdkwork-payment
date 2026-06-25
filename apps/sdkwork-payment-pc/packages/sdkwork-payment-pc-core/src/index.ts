export type SdkworkPaymentPcRouteSurface = "app" | "backend-admin";

export interface SdkworkPaymentPcRouteContribution {
  readonly auth: "public" | "required";
  readonly capability: string;
  readonly domain: "commerce";
  readonly id: string;
  readonly packageName: string;
  readonly path: string;
  readonly permissionHint?: string;
  readonly screen: string;
  readonly surface: SdkworkPaymentPcRouteSurface;
  readonly title: string;
  readonly titleKey: string;
}

export const sdkworkPaymentPcRuntimeIdentity = {
  appKey: "sdkwork-payment-pc",
  architecture: "pc-react",
  domain: "commerce",
  capability: "payment",
  runtimeFamily: "web",
} as const;

export function createSdkworkPaymentPcRouteRegistry(
  ...routeGroups: readonly (readonly SdkworkPaymentPcRouteContribution[])[]
): readonly SdkworkPaymentPcRouteContribution[] {
  return routeGroups.flat();
}
