import { useState } from "react";
import { Download, PackageCheck, RefreshCw, ShieldCheck } from "lucide-react";
import { updaterApi, type UpdateInfo } from "./api";
import { Button, Card } from "./components";

export function UpdatePanel() {
  const [info, setInfo] = useState<UpdateInfo | null>(null);
  const [checking, setChecking] = useState(false);
  const [message, setMessage] = useState("");

  const check = async () => {
    setChecking(true);
    setMessage("");
    try {
      setInfo(await updaterApi.check());
    } catch (error) {
      setMessage(String(error));
    } finally {
      setChecking(false);
    }
  };

  const stagePatch = async () => {
    if (!info?.patch) return;
    setChecking(true);
    try {
      const staged = await updaterApi.stagePatch(info.patch);
      setMessage(`Patch verified and staged: ${staged.to_version}`);
    } catch (error) {
      setMessage(String(error));
    } finally {
      setChecking(false);
    }
  };

  return (
    <Card>
      <div className="inline" style={{ justifyContent: "space-between", width: "100%" }}>
        <div>
          <h2 className="card-title">Updates</h2>
          <div className="card-copy">GitHub Releases with optional SHA-256 verified small patch packages.</div>
        </div>
        <Button disabled={checking} onClick={() => void check()}><RefreshCw size={14} /> Check</Button>
      </div>

      <div className="divider" />
      {!info && <div className="card-copy">No update check performed yet.</div>}
      {info && (
        <div>
          <div className="inline" style={{ justifyContent: "space-between", width: "100%" }}>
            <span className="card-copy">Installed</span><span className="kbd">v{info.current_version}</span>
          </div>
          <div className="inline" style={{ justifyContent: "space-between", width: "100%", marginTop: 8 }}>
            <span className="card-copy">Latest</span><span className="kbd">v{info.latest_version}</span>
          </div>
          <div className="quick-actions">
            {info.available ? <span className="status-pill status-recording">Update available</span> : <span className="status-pill status-running">Up to date</span>}
            {info.patch && <Button variant="primary" disabled={checking} onClick={() => void stagePatch()}><Download size={14} /> Stage small patch</Button>}
          </div>
          {info.patch && <div className="card-copy section-gap"><ShieldCheck size={12} style={{ display: "inline", marginRight: 5 }} />Patch {info.patch.from_version} → {info.patch.to_version} · SHA-256 verification required.</div>}
          {info.full_release && <div className="card-copy section-gap"><PackageCheck size={12} style={{ display: "inline", marginRight: 5 }} />Full package: {info.full_release.name}</div>}
        </div>
      )}
      {message && <div className="card-copy section-gap">{message}</div>}
    </Card>
  );
}
