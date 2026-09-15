import { useEffect, useState } from "react";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import { CircleStop, Radio } from "lucide-react";
import { type StoredMacro } from "./api";
import { Card } from "./components";
import { MacroStudio } from "./MacroStudio";

export function MacroStudioPage() {
  const [recording, setRecording] = useState(false);
  const [revision, setRevision] = useState(0);
  const [lastRecorded, setLastRecorded] = useState<StoredMacro | null>(null);

  useEffect(() => {
    let disposed = false;
    const unlisteners: UnlistenFn[] = [];

    void (async () => {
      try {
        const stateUnlisten = await listen<boolean>("macro-recording-state", (event) => {
          if (disposed) return;
          setRecording(Boolean(event.payload));
          setRevision((value) => value + 1);
        });
        if (disposed) stateUnlisten();
        else unlisteners.push(stateUnlisten);

        const recordedUnlisten = await listen<StoredMacro>("macro-recorded", (event) => {
          if (disposed) return;
          setRecording(false);
          setLastRecorded(event.payload);
          setRevision((value) => value + 1);
        });
        if (disposed) recordedUnlisten();
        else unlisteners.push(recordedUnlisten);
      } catch {
        // Browser preview has no Tauri event bridge.
      }
    })();

    return () => {
      disposed = true;
      for (const unlisten of unlisteners) unlisten();
    };
  }, []);

  return (
    <>
      <div className="page" style={{ marginBottom: 14 }}>
        <Card>
          <div className="inline" style={{ justifyContent: "space-between", width: "100%", flexWrap: "wrap" }}>
            <div className="inline">
              {recording ? <Radio size={16} color="var(--warning)" /> : <CircleStop size={16} color="var(--muted)" />}
              <div>
                <h2 className="card-title">Global macro recorder</h2>
                <div className="card-copy">
                  {recording
                    ? "Recording globally. Press F2 to stop and save the recording."
                    : "Press F1 anywhere to start recording. Press F2 to stop and save it as an unassigned macro draft."}
                </div>
              </div>
            </div>
            <div className={`status-pill ${recording ? "status-recording" : "status-ready"}`}>
              {recording ? "Recording · F2 stops" : "F1 start · F2 stop"}
            </div>
          </div>
          {lastRecorded && (
            <div className="card-copy section-gap">
              Saved <strong>{lastRecorded.name}</strong> with {lastRecorded.events.filter((event) => event.type !== "delay").length} captured actions. It is available in the Saved macro list below.
            </div>
          )}
        </Card>
      </div>
      <MacroStudio key={revision} />
    </>
  );
}
