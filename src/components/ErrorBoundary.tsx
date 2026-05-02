import { Component, type ReactNode } from "react";

interface Props {
  children: ReactNode;
  fallback?: ReactNode;
}

interface State {
  hasError: boolean;
  error: Error | null;
  errorInfo: React.ErrorInfo | null;
}

export class ErrorBoundary extends Component<Props, State> {
  constructor(props: Props) {
    super(props);
    this.state = { hasError: false, error: null, errorInfo: null };
  }

  static getDerivedStateFromError(error: Error): Partial<State> {
    return { hasError: true, error };
  }

  componentDidCatch(error: Error, errorInfo: React.ErrorInfo) {
    console.error("ErrorBoundary caught an error:", error, errorInfo);
    this.setState({ errorInfo });
  }

  render() {
    if (this.state.hasError) {
      if (this.props.fallback) {
        return this.props.fallback;
      }
      const stack =
        this.state.error?.stack ?? "无堆栈信息";
      const componentStack =
        this.state.errorInfo?.componentStack ?? "无组件堆栈";
      return (
        <div className="error-boundary-fallback">
          <h2>页面出现错误</h2>
          <p>抱歉，发生了意外错误。请尝试刷新页面。</p>
          <details open>
            <summary>错误信息</summary>
            <pre>{this.state.error?.message}</pre>
          </details>
          <details>
            <summary>堆栈跟踪</summary>
            <pre style={{ whiteSpace: "pre-wrap", wordBreak: "break-all" }}>
              {stack}
            </pre>
          </details>
          <details>
            <summary>组件堆栈</summary>
            <pre style={{ whiteSpace: "pre-wrap", wordBreak: "break-all" }}>
              {componentStack}
            </pre>
          </details>
          <button onClick={() => window.location.reload()} type="button">
            刷新页面
          </button>
        </div>
      );
    }

    return this.props.children;
  }
}

export function initGlobalErrorListeners() {
  window.addEventListener("error", (event) => {
    console.error("Global error:", event.error);
  });
  window.addEventListener("unhandledrejection", (event) => {
    console.error("Unhandled promise rejection:", event.reason);
  });
}
