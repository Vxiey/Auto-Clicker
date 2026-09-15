import { useEffect, useRef, useState } from "react";
import { KeyRound } from "lucide-react";
import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import { Button } from "./components";

type HotkeyCaptureResult = {
  request_id: string;
  binding: string | null;
};

type HotkeyCaptureInputProps = {
  value: string;
  onChange: (value: string) => void;
  disabled?: boolean;
  placeholder?: string;
  reserved?: readonly string[];
  list?: string;
};

let captureSequence = 0;

function canonical(value: string) {
  return value.trim().toLowerCase().replace(/[\s_-]+/g, "");
}

export function HotkeyCaptureInput({
  value,
  onChange,
  disabled = false,
  placeholder = "Hotkey",
  reserved = [],
  list,
}: HotkeyCaptureInputProps) {
  const [capturing, setCapturing] = useState(false);
  const [captureError, setCaptureError] = useState("");
  const requestIdRef = useRef<string | null>(null);
  const unlistenRef = useRef<UnlistenFn | null>(null);

  const cleanupListener = () => {
    unlistenRef.current?.();
    unlistenRef.current = null;
  };

  useEffect(() => () => {
    cleanupListener();
    const requestId = requestIdRef.current;
    requestIdRef.current = null;
    if (requestId) void invoke("cancel_hotkey_capture", { requestId }).catch(() => undefined);
  }, []);

  const beginCapture = async () => {
    if (disabled || capturing) return;
    const requestId = `hotkey-${Date.now()}-${captureSequence++}`;
    requestIdRef.current = requestId;
    setCaptureError("");
    setCapturing(true);

    try {
      cleanupListener();
      unlistenRef.current = await listen<HotkeyCaptureResult>("hotkey-captured", (event) => {
        if (event.payload.request_id !== requestIdRef.current) return;
        const binding = event.payload.binding;
        cleanupListener();
        requestIdRef.current = null;
        setCapturing(false);
        if (!binding) return;
        if (reserved.some((item) => canonical(item) === canonical(binding))) {
          setCaptureError(`${binding} is reserved by VxClick.`);
          return;
        }
        setCaptureError("");
        onChange(binding);
      });
      await invoke<void>("start_hotkey_capture", { requestId });
    } catch (error) {
      cleanupListener();
      requestIdRef.current = null;
      setCapturing(false);
      setCaptureError(`Could not start key capture: ${String(error)}`);
    }
  };

  return (
    <div>
      <div style={{ display: "grid", gridTemplateColumns: "minmax(0, 1fr) auto", gap: 7 }}>
        <input
          className="input"
          list={list}
          value={capturing ? "Press a key, combo or mouse button…" : value}
          placeholder={placeholder}
          disabled={disabled || capturing}
          onChange={(event) => onChange(event.target.value)}
        />
        <Button disabled={disabled || capturing} onClick={() => void beginCapture()}>
          <KeyRound size={13} /> {capturing ? "Listening…" : "Bind"}
        </Button>
      </div>
      {capturing && <div className="card-copy" style={{ marginTop: 4 }}>Press the desired hotkey. Esc cancels without changing the current binding.</div>}
      {captureError && <div className="card-copy" style={{ marginTop: 4, color: "var(--warning)" }}>{captureError}</div>}
    </div>
  );
}
