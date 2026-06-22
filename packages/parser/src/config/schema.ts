import { z } from "zod";

const SeverityLevelSchema = z.enum(["SAFE", "MINOR", "MAJOR", "CRITICAL"]);
const CompareModeSchema = z.enum(["ast", "idl"]);
const OverrideActionSchema = z.enum(["allow", "downgrade"]);

export const OverrideSchema = z.object({
  account: z.string().min(1, "Account name cannot be empty"),
  finding: z.string().min(1, "Finding kind cannot be empty"),
  field: z.string().optional(),
  action: OverrideActionSchema,
  severity: SeverityLevelSchema.optional(),
  note: z.string().min(10, "Override note must be at least 10 characters long to maintain audit logs")
}).refine(data => {
  if (data.action === "downgrade" && !data.severity) {
    return false;
  }
  return true;
}, {
  message: "Field 'severity' is required when action is 'downgrade'",
  path: ["severity"]
});

export const ProgramConfigSchema = z.object({
  path: z.string().min(1, "Program path must be defined"),
  id: z.string().min(1, "Program ID must be defined"),
  idl_path: z.string().optional(),
  overrides: z.array(OverrideSchema).default([])
});

export const EpicConfigSchema = z.object({
  workspace: z.object({
    compare_mode: CompareModeSchema.default("ast"),
    fail_on_severity: SeverityLevelSchema.default("CRITICAL"),
    rpc_url: z.string().url("rpc_url must be a valid http/https endpoint").optional(),
    exclude_paths: z.array(z.string()).default([]),
    enforce_padding: z.boolean().default(false)
  }).default({}),
  programs: z.record(z.string(), ProgramConfigSchema).default({}),
  ignore: z.array(z.string()).default([])
});
