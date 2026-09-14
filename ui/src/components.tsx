import type { ButtonHTMLAttributes, PropsWithChildren, ReactNode } from "react";

export function Card({ children, className = "" }: PropsWithChildren<{ className?: string }>) {
  return <section className={`card ${className}`}>{children}</section>;
}

export function Button({
  variant = "secondary",
  className = "",
  children,
  ...props
}: ButtonHTMLAttributes<HTMLButtonElement> & {
  variant?: "primary" | "secondary" | "danger";
}) {
  return (
    <button className={`btn btn-${variant} ${className}`} {...props}>
      {children}
    </button>
  );
}

export function Toggle({ value, onChange }: { value: boolean; onChange: (next: boolean) => void }) {
  return (
    <button
      type="button"
      aria-pressed={value}
      aria-label={value ? "Enabled" : "Disabled"}
      className={`toggle ${value ? "on" : ""}`}
      onClick={() => onChange(!value)}
    >
      <div className="toggle-knob" />
    </button>
  );
}

export function StatusPill({
  status,
}: {
  status: "Ready" | "Running" | "Stopped" | "Recording" | "Error";
}) {
  const slug = status.toLowerCase();
  return (
    <div className={`status-pill status-${slug}`}>
      <span className="status-dot" style={{ backgroundColor: "currentColor" }} />
      <span>{status}</span>
    </div>
  );
}

export function MetricCard({ label, value, accent = false, icon }: { label: string; value: string; accent?: boolean; icon?: ReactNode }) {
  return (
    <Card>
      <div className="inline" style={{ justifyContent: "space-between", width: "100%" }}>
        <span className="metric-label">{label}</span>
        {icon}
      </div>
      <div className={`metric-value ${accent ? "metric-accent" : ""}`}>{value}</div>
    </Card>
  );
}

export function Field({ label, children }: PropsWithChildren<{ label: string }>) {
  return (
    <label>
      <span className="field-label">{label}</span>
      {children}
    </label>
  );
}
