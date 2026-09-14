import { useState } from "react";
import { Activity, Gauge, Play, Timer } from "lucide-react";
import { benchmarkApi, type BenchmarkReport } from "./api";
import { Button, Card, Field, MetricCard } from "./components";

export function BenchmarkPanel() {
  const [cps, setCps] = useState(500);
  const [durationMs, setDurationMs] = useState(3000);
  const [button, setButton] = useState("left");
  const [running, setRunning] = useState(false);
  const [result, setResult] = useState<BenchmarkReport | null>(null);
  const [error, setError] = useState("");

  const run = async () => {
    setRunning(true);
    setError("");
    try {
      setResult(await benchmarkApi.run(cps, durationMs, button));
    } catch (nextError) {
      setError(String(nextError));
    } finally {
      setRunning(false);
    }
  };

  return (
    <Card>
      <div className="inline" style={{ justifyContent: "space-between", width: "100%" }}>
        <div>
          <h2 className="card-title">Precision benchmark</h2>
          <div className="card-copy">Measure generated CPS, interval accuracy, jitter and missed deadlines on this PC.</div>
        </div>
        <Button variant="primary" disabled={running} onClick={() => void run()}>
          <Play size={14} /> {running ? "Running..." : "Run benchmark"}
        </Button>
      </div>

      <div className="form-row section-gap">
        <Field label="Target CPS">
          <input className="input" type="number" min="1" max="20000" value={cps} onChange={(event) => setCps(Math.max(1, Math.min(20000, Number(event.target.value))))} />
        </Field>
        <Field label="Duration (ms)">
          <input className="input" type="number" min="100" max="30000" value={durationMs} onChange={(event) => setDurationMs(Math.max(100, Math.min(30000, Number(event.target.value))))} />
        </Field>
        <Field label="Button">
          <select className="select" value={button} onChange={(event) => setButton(event.target.value)}>
            <option value="left">Left</option>
            <option value="right">Right</option>
            <option value="middle">Middle</option>
            <option value="x1">X1 / Mouse 4</option>
            <option value="x2">X2 / Mouse 5</option>
          </select>
        </Field>
      </div>

      {error && <div className="card-copy section-gap" style={{ color: "var(--danger)" }}>{error}</div>}

      {result && (
        <>
          <div className="grid grid-4 section-gap">
            <MetricCard label="Actual CPS" value={result.actual_cps.toFixed(2)} accent icon={<Gauge size={16} color="var(--cyan)" />} />
            <MetricCard label="Deviation" value={`${result.deviation_percent >= 0 ? "+" : ""}${result.deviation_percent.toFixed(3)}%`} icon={<Activity size={16} color="var(--accent)" />} />
            <MetricCard label="P99 jitter" value={`${result.jitter.p99_us.toFixed(2)} µs`} icon={<Timer size={16} color="var(--muted)" />} />
            <MetricCard label="Missed deadlines" value={result.missed_deadlines.toLocaleString()} icon={<Activity size={16} color="var(--muted)" />} />
          </div>
          <div className="divider" />
          <div className="card-copy">
            Interval mean {result.interval.mean_us.toFixed(2)} µs · p95 {result.interval.p95_us.toFixed(2)} µs · p99 {result.interval.p99_us.toFixed(2)} µs · worst jitter {result.jitter.worst_us.toFixed(2)} µs · {result.clicks.toLocaleString()} generated clicks in {result.elapsed_seconds.toFixed(3)} s.
          </div>
        </>
      )}
    </Card>
  );
}
