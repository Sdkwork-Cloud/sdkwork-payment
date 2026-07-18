import * as React from "react";
import {
  TabsContent,
  TabsList,
  TabsTrigger,
  type TabsContentProps,
  type TabsListProps,
  type TabsTriggerProps,
} from "@sdkwork/ui-pc-react";

export interface PaymentAdminWorkspaceProps
  extends Omit<React.HTMLAttributes<HTMLElement>, "title"> {
  description?: React.ReactNode;
  error?: React.ReactNode;
  title: React.ReactNode;
}

/** Compact frame shared by payment backend-admin workspaces. */
export function PaymentAdminWorkspace({
  children,
  className,
  description,
  error,
  title,
  ...props
}: PaymentAdminWorkspaceProps) {
  const titleId = React.useId();
  const classes = ["min-w-0 space-y-4", className].filter(Boolean).join(" ");

  return (
    <section aria-labelledby={titleId} className={classes} {...props}>
      <header className="flex min-h-9 min-w-0 flex-col justify-center gap-1 border-b border-[var(--sdk-color-border-subtle)] pb-3 sm:flex-row sm:items-center sm:justify-between">
        <h1
          className="min-w-0 break-words text-base font-semibold text-[var(--sdk-color-text-primary)]"
          id={titleId}
        >
          {title}
        </h1>
        {description ? (
          <p className="max-w-2xl text-xs text-[var(--sdk-color-text-secondary)]">
            {description}
          </p>
        ) : null}
      </header>

      {error ? (
        <div
          className="border-l-2 border-[var(--sdk-color-border-error)] bg-[var(--sdk-color-bg-error-subtle)] px-3 py-2 text-sm text-[var(--sdk-color-text-error)]"
          role="alert"
        >
          {error}
        </div>
      ) : null}

      {children}
    </section>
  );
}

/** Scrollable, underline-style navigation for dense admin workspaces. */
export function PaymentAdminTabsList({ className, ...props }: TabsListProps) {
  const classes = [
    "flex h-9 w-full justify-start gap-0 overflow-x-auto rounded-none border-b border-[var(--sdk-color-border-subtle)] bg-transparent p-0",
    className,
  ].filter(Boolean).join(" ");

  return <TabsList className={classes} {...props} />;
}

export function PaymentAdminTabsTrigger({ className, ...props }: TabsTriggerProps) {
  const classes = [
    "h-9 min-w-fit shrink-0 rounded-none border-b-2 border-transparent px-3 py-2 text-xs shadow-none data-[state=active]:border-[var(--sdk-color-brand-primary)] data-[state=active]:!bg-transparent data-[state=active]:shadow-none",
    className,
  ].filter(Boolean).join(" ");

  return <TabsTrigger className={classes} {...props} />;
}

export function PaymentAdminTabsContent({ className, ...props }: TabsContentProps) {
  const classes = [
    "mt-4 rounded-none border-0 bg-transparent p-0 shadow-none",
    className,
  ].filter(Boolean).join(" ");

  return <TabsContent className={classes} {...props} />;
}
