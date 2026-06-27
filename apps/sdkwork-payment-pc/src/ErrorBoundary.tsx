import { Component, type ErrorInfo, type ReactNode } from "react";

interface ErrorBoundaryProps {
  children: ReactNode;
  fallback?: (error: Error, reset: () => void) => ReactNode;
}

interface ErrorBoundaryState {
  error: Error | null;
}

/**
 * C19 修复：React ErrorBoundary，捕获子树渲染异常，避免白屏。
 *
 * 符合 `FRONTEND_SPEC.md` §5 与 `APP_PC_REACT_UI_SPEC.md` §5 的状态覆盖要求：
 * 渲染异常 MUST 展示用户可理解的错误界面，不得暴露堆栈/SQL/token 等内部细节。
 */
export class ErrorBoundary extends Component<ErrorBoundaryProps, ErrorBoundaryState> {
  state: ErrorBoundaryState = { error: null };

  static getDerivedStateFromError(error: Error): ErrorBoundaryState {
    return { error };
  }

  componentDidCatch(error: Error, info: ErrorInfo): void {
    console.error("[PaymentApp] uncaught render error:", error, info);
  }

  reset = (): void => {
    this.setState({ error: null });
  };

  render(): ReactNode {
    if (this.state.error) {
      if (this.props.fallback) {
        return this.props.fallback(this.state.error, this.reset);
      }
      return (
        <main className="payment-error-boundary" role="alert">
          <section className="payment-error-boundary__card">
            <h1>Payment surface unavailable</h1>
            <p>
              The payment surface encountered an unexpected error. Please refresh
              the page or try again later.
            </p>
            <button type="button" onClick={this.reset}>
              Retry
            </button>
          </section>
        </main>
      );
    }
    return this.props.children;
  }
}
