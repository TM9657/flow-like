import { type IProfile } from "./lib/schema/profile/profile";

export interface ISystemInfo {
  cores: number,
  vram: number,
  ram: number,
}

interface IExecutionSettings {
  gpu_mode: boolean;
  max_context_size: number;
}
interface IFlowSettings {
  connection_mode: "straight" | "step" | "simpleBezier"
}

interface IHubConfig {
  cache_dir?: string,
  artifacts_dir?: string,
  models_dir?: string,
  flow_dir?: string,
  vault_dir?: string,
}

export interface ISettingsProfile {
  hub_profile: IProfile,
  config: IHubConfig,
  execution_settings: IExecutionSettings,
  vaults: string[],
  apps: string[],
  flow_settings: IFlowSettings,
  updated: string,
  created: string,
}

export interface IDate {
  secs_since_epoch: number,
  nanos_since_epoch: number,
}

