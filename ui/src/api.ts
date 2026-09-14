import { invoke } from "@tauri-apps/api/core";

export type ClickerProfileSettings = {
  cps: number;
  button: "left" | "right" | "middle" | "x1" | "x2";
  mode: "hold" | "toggle" | "once";
  randomize: boolean;
  burst: boolean;
  start_hotkey: string;
  emergency_stop_hotkey: string;
};

export type Profile = {
  id: string;
  name: string;
  description: string;
  process_names: string[];
  auto_switch: boolean;
  clicker: ClickerProfileSettings;
};

export type ProfileDocument = {
  schema_version: number;
  active_profile_id: string;
  auto_switch_enabled: boolean;
  profiles: Profile[];
};

export type ProfilesSnapshot = {
  document: ProfileDocument;
  foreground_process: string | null;
};

export type PatchAsset = {
  from_version: string;
  to_version: string;
  url: string;
  sha256: string;
  size: number;
  format: string;
};

export type FullReleaseAsset = {
  name: string;
  url: string;
  size: number;
};

export type UpdateInfo = {
  repository: string;
  current_version: string;
  latest_version: string;
  available: boolean;
  release_url: string;
  published_at: string | null;
  full_release: FullReleaseAsset | null;
  patch: PatchAsset | null;
};

export type StagedPatch = {
  path: string;
  sha256: string;
  size: number;
  from_version: string;
  to_version: string;
};

export type DiagnosticsSnapshot = {
  directory: string;
  session_log: string;
  crash_log: string;
  performance_log: string;
  session_id: string;
  session_log_bytes: number;
  crash_log_bytes: number;
  performance_log_bytes: number;
};

export const profilesApi = {
  snapshot: () => invoke<ProfilesSnapshot>("profiles_snapshot"),
  create: (name: string, processName?: string) =>
    invoke<Profile>("create_profile", { name, processName: processName || null }),
  save: (profile: Profile) => invoke<Profile>("save_profile", { profile }),
  activate: (id: string) => invoke<void>("activate_profile", { id }),
  remove: (id: string) => invoke<void>("delete_profile", { id }),
  setAutoSwitch: (enabled: boolean) => invoke<void>("set_profile_auto_switch", { enabled }),
  foregroundProcess: () => invoke<string | null>("foreground_process"),
};

export const updaterApi = {
  check: () => invoke<UpdateInfo>("check_for_updates"),
  stagePatch: (patch: PatchAsset) => invoke<StagedPatch>("stage_patch", { patch }),
};

export const diagnosticsApi = {
  snapshot: () => invoke<DiagnosticsSnapshot>("diagnostics_snapshot"),
  clear: () => invoke<void>("clear_diagnostics"),
  log: (level: "debug" | "info" | "warn" | "error", target: string, message: string) =>
    invoke<void>("diagnostics_client_log", { level, target, message }),
};
