export type CompareMode = "ast" | "idl";
export type SeverityLevel = "SAFE" | "MINOR" | "MAJOR" | "CRITICAL";
export type OverrideAction = "allow" | "downgrade";

export interface RawOverrideConfig {
  account: string;
  finding: string;
  field?: string;
  action: OverrideAction;
  severity?: SeverityLevel;
  note: string;
}

export interface RawProgramConfig {
  path: string;
  id: string;
  idl_path?: string;
  overrides?: RawOverrideConfig[];
}

export interface RawWorkspaceConfig {
  compare_mode?: CompareMode;
  fail_on_severity?: SeverityLevel;
  rpc_url?: string;
  exclude_paths?: string[];
  enforce_padding?: boolean;
}

export interface RawEpicConfig {
  workspace?: RawWorkspaceConfig;
  programs?: Record<string, RawProgramConfig>;
  ignore?: string[];
}

export interface ResolvedOverride {
  account: string;
  finding: string;
  field?: string;
  action: OverrideAction;
  severity?: SeverityLevel;
  note: string;
  used: boolean;
}

export interface ResolvedProgram {
  name: string;
  absolutePath: string;
  programId: string;
  idlPath?: string;
  overrides: ResolvedOverride[];
}

export interface ResolvedEpicConfig {
  compareMode: CompareMode;
  failOnSeverity: SeverityLevel;
  rpcUrl?: string;
  excludePaths: string[];
  enforcePadding: boolean;
  programs: Map<string, ResolvedProgram>;
  ignore: string[];
}
