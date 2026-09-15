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

function keyboardMainToken(event: KeyboardEvent): string | null {
  const { code } = event;

  if (/^Key[A-Z]$/.test(code)) return code.slice(3);
  if (/^Digit[0-9]$/.test(code)) return code.slice(5);
  if (/^F(?:[1-9]|1[0-9]|2[0-4])$/.test(code)) return code;
  if (/^Numpad[0-9]$/.test(code)) return code;

  switch (code) {
    case "Backspace": return "Backspace";
    case "Tab": return "Tab";
    case "Enter":
    case "NumpadEnter": return "Enter";
    case "Pause": return "Pause";
    case "CapsLock": return "CapsLock";
    case "Space": return "Space";
    case "PageUp": return "PageUp";
    case "PageDown": return "PageDown";
    case "End": return "End";
    case "Home": return "Home";
    case "ArrowLeft": return "Left";
    case "ArrowUp": return "Up";
    case "ArrowRight": return "Right";
    case "ArrowDown": return "Down";
    case "PrintScreen": return "PrintScreen";
    case "Insert": return "Insert";
    case "Delete": return "Delete";
    case "NumLock": return "NumLock";
    case "ScrollLock": return "ScrollLock";
    default: return null;
  }
}

function modifierToken(code: string): string | null {
  switch (code) {
    case "ControlLeft":
    case "ControlRight": return "Ctrl";
    case "AltLeft":
    case "AltRight": return "Alt";
    case "ShiftLeft":
    case "ShiftRight": return "Shift";
    case "MetaLeft":
    case "MetaRight": return "Win";
    default: return null;
  }
}

function keyboardBinding(event: KeyboardEvent): string | null {
  const main = keyboardMainToken(event);
  if (!main) return null;

  const parts: string[] = [];
  if (event.ctrlKey) parts.push("Ctrl");
  if (event.altKey) parts.push("Alt");
  if (event.shiftKey) parts.push("Shift");
  if (event.metaKey) parts.push("Win");
  parts.push(main);
  return parts.join(" + ");
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

  const finishCapture = (binding: string | null) => {
    const requestId = requestIdRef.current;
    if (!requestId) return;

    cleanupListener();
    requestIdRef.current = null;
    setCapturing(false);
    void invoke("cancel_hotkey_capture", { requestId }).catch(() => undefined);

    if (!binding) return;
    if (reserved.some((item) => canonical(item) === canonical(binding))) {
      setCaptureError(`${binding} is reserved by VxClick.`);
      return;
    }
    setCaptureError("");
    onChange(binding);
  };

  useEffect(() => () => {
    cleanupListener();
    const requestId = requestIdRef.current;
    requestIdRef.current = null;
    if (requestId) void invoke("cancel_hotkey_capture", { requestId }).catch(() => undefined);
  }, []);

  useEffect(() => {
    if (!capturing) return;

    const onKeyDown = (event: KeyboardEvent) => {
      if (!requestIdRef.current) return;
      if (event.code === "Escape") {
        event.preventDefault();
        event.stopPropagation();
        finishCapture(null);
        return;
      }
      if (modifierToken(event.code)) {
        event.preventDefault();
        event.stopPropagation();
        return;
      }

      const binding = keyboardBinding(event);
      if (!binding) return;
      event.preventDefault();
      event.stopPropagation();
      finishCapture(binding);
    };

    const onKeyUp = (event: KeyboardEvent) => {
      if (!requestIdRef.current) return;
      const modifier = modifierToken(event.code);
      if (!modifier) return;
      event.preventDefault();
      event.stopPropagation();
      finishCapture(modifier);
    };

    window.addEventListener("keydown", onKeyDown, true);
    window.addEventListener("keyup", onKeyUp, true);
    return () => {
      window.removeEventListener("keydown", onKeyDown, true);
      window.removeEventListener("keyup", onKeyUp, true);
    };
  }, [capturing, reserved, onChange]);

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
        finishCapture(event.payload.binding);
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
          disabled={disabled}
          readOnly={capturing}
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
