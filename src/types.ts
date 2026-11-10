export interface AdbStatus {
  found: boolean;
  path: string | null;
  version: string | null;
  source: string | null;
  error?: string | null;
}

export type DeviceConnectionState =
  | 'Ready'
  | 'Unauthorized'
  | 'Offline'
  | 'NoPermissions'
  | 'Recovery'
  | 'Sideload'
  | { Unknown: string };

export interface DeviceInfo {
  serial: string;
  state: DeviceConnectionState;
  connection_type: string; // 'USB' | 'Wireless'
  model: string;
  brand: string;
  manufacturer: string;
  android_version: string;
  android_sdk: number;
  build_id: string;
  security_patch: string;
}

export interface AndroidUser {
  id: number;
  name: string;
  flags: number;
  is_running: boolean;
  is_primary: boolean;
  profile_type: string;
  display_label: string;
}

export type PackageState = 'Enabled' | 'Disabled' | 'Uninstalled';

export type AndroidPackageType = 'System' | 'UpdatedSystem' | 'User';

export type RemovalEligibility = 'Eligible' | 'ReviewOnly' | 'Protected' | 'AlreadyRemoved';

export interface DevicePackage {
  package_id: string;
  state: PackageState;
  is_system: boolean;
  package_type: AndroidPackageType;
  user_id: number;
  list_category: string;
  removal_recommendation: string;
  removal_eligibility: RemovalEligibility;
  description: string;
  dependencies: string[];
  needed_by: string[];
  is_protected: boolean;
  protection_reason: string | null;
}

export type ActionType = 'Disable' | 'Enable' | 'UninstallUser' | 'Restore';

export type ShortcutIntent = 'ReviewBloatware' | 'ReviewTrackers' | 'UninstallPromos';

export type ClassifiedCause =
  | 'MissingPrivileges'
  | 'OemRestriction'
  | 'DevicePolicyRestricted'
  | 'UnsupportedOperation'
  | 'InvalidProfileTarget'
  | 'ApplicationPolicyBlock'
  | 'PackageNotFound'
  | 'DeviceOffline'
  | 'Unclassified';

export interface OperationOutcome {
  package_id: string;
  action: ActionType;
  success: boolean;
  exit_code: number | null;
  previous_state: PackageState | null;
  resulting_state: PackageState | null;
  message: string;
  verified: boolean;
  classified_cause?: ClassifiedCause | null;
  execution_backend?: string;
  command_args?: string[];
  target_device?: string | null;
  target_user_id?: number | null;
  raw_stdout?: string | null;
  raw_stderr?: string | null;
}

export interface BatchItemValidation {
  package_id: string;
  package_type: AndroidPackageType;
  current_state: PackageState;
  action: ActionType;
  target_user: number;
  eligible: boolean;
  exclusion_reason: string | null;
}

export interface BatchPreflightResult {
  items: BatchItemValidation[];
  total_requested: number;
  eligible_count: number;
  excluded_count: number;
}

export interface ProposedAction {
  package_id: string;
  package_name: string | null;
  proposed_action: ActionType;
  reason: string;
  evidence: string;
  expected_side_effects: string | null;
  uncertainty: string;
  removal_category: string;
  is_protected: boolean;
  protection_warning: string | null;
  selected: boolean;
  is_executable: boolean;
}

export interface ActionPlan {
  id: string;
  revision: number;
  target_device_serial: string;
  target_user_id: number;
  explanation: string;
  preserved_functionality: string[];
  actions: ProposedAction[];
  created_at: number;
  approved_by_user: boolean;
  is_valid: boolean;
  invalidation_reason: string | null;
}

export interface PlanApprovalSession {
  approval_token: string;
  plan_id: string;
  revision: number;
  expires_at: number;
  target_device_serial: string;
  target_user_id: number;
  inventory_hash: string;
  approved_actions: ProposedAction[];
  created_at: number;
}

export interface PlanValidationResult {
  is_valid: boolean;
  errors: string[];
  warnings: string[];
  executable_count: number;
  protected_blocked_count: number;
  missing_package_count?: number;
  unsafe_package_count?: number;
}

export interface ChatMessage {
  id: string;
  role: 'user' | 'assistant' | 'system';
  content: string;
  timestamp: number;
  plan_id?: string | null;
  plan?: ActionPlan | null;
  served_model?: string | null;
}

export type ConnectionMode = 'DirectApi' | 'Gateway';

export type ConnectionPreset =
  | 'GoogleGemini'
  | 'OpenAi'
  | 'Anthropic'
  | 'FreeLlmApi'
  | 'CustomGateway';

export type ApiProtocol =
  | 'GeminiNative'
  | 'OpenAiChatCompletions'
  | 'OpenAiResponses'
  | 'AnthropicMessages';

export interface ConnectionProfile {
  id: string;
  name: string;
  mode: ConnectionMode;
  preset: ConnectionPreset;
  base_url: string;
  protocol: ApiProtocol;
  credential_id: string;
  selected_model: string;
  manual_model_id: string | null;
  last_test_status: string | null;
  last_verified_at: string | null;
  supported_capabilities: string[];
  tested_for_generation: boolean;
  has_credential: boolean;
}

export interface DiscoveredModel {
  id: string;
  display_name: string;
  description: string | null;
  supported_generation_methods: string[];
  is_router_alias: boolean;
  tested_for_generation: boolean;
}

export interface ModelDiscoveryResult {
  success: boolean;
  connection_id: string;
  models: DiscoveredModel[];
  next_page_token: string | null;
  error: ClassifiedError | null;
}

export interface GenerationTestResult {
  success: boolean;
  connection_id: string;
  model_id: string;
  latency_ms: number;
  message: string;
  error: ClassifiedError | null;
}

export interface GatewayReadinessInfo {
  is_ready: boolean;
  gateway_available: boolean;
  upstream_available: boolean;
  message: string;
  served_model: string | null;
}

export type ErrorType =
  | 'Authentication'
  | 'ModelUnavailable'
  | 'PermissionAccess'
  | 'QuotaRateLimit'
  | 'InvalidEndpointOrProtocol'
  | 'NetworkTimeout'
  | 'UnsupportedRequestParameter'
  | 'InvalidTruncatedAiResponse'
  | 'Unknown';

export interface ClassifiedError {
  error_type: ErrorType;
  message: string;
  suggested_action: string;
  diagnostic_details: string | null;
  http_status: number | null;
}

export interface StreamChunkPayload {
  request_id: string;
  text: string;
  is_thought: boolean;
}

export interface StreamDonePayload {
  request_id: string;
  finish_reason: string;
  served_model: string | null;
  plan_id: string | null;
}

export interface StreamErrorPayload {
  request_id: string;
  error: ClassifiedError;
}

export interface PlanProgressEvent {
  plan_id: string;
  package_id: string;
  action: string;
  current: number;
  total: number;
  status: string;
}

export interface PlanExecutionCompletePayload {
  plan_id: string;
  total: number;
  successful: number;
  failed: number;
  attempted_count: number;
  succeeded_count: number;
  failed_count: number;
  skipped_count: number;
  unknown_count: number;
}

export interface DeviceSnapshot {
  snapshot_id: string;
  device_serial: string;
  device_model: string;
  user_id: number;
  timestamp: number;
  note: string;
  package_states: Array<{
    package_id: string;
    state: PackageState;
    is_system: boolean;
  }>;
}

export interface JournalEntry {
  entry_id: string;
  timestamp: number;
  device_serial: string;
  user_id: number;
  package_id: string;
  action: ActionType;
  previous_state: PackageState | null;
  resulting_state: PackageState | null;
  success: boolean;
  verified: boolean;
  message: string;
  recovery_steps: ActionType[];
}

export interface RecoveryProposal {
  entry_id: string;
  package_id: string;
  device_serial: string;
  user_id: number;
  target_state: PackageState;
  steps: ActionType[];
  explanation: string;
  can_recover: boolean;
  blockage_reason: string | null;
}

export interface ProfileItem {
  package_id: string;
  desired_action: ActionType;
}

export interface DebloatProfile {
  profile_name: string;
  description: string;
  created_at: number;
  actions: ProfileItem[];
}

export interface AppConfig {
  custom_adb_path: string | null;
  active_connection_id: string;
  connections: ConnectionProfile[];
  ai_consent_given: boolean;
  approved_destinations: string[];
  auto_refresh_on_connect: boolean;
  confirm_each_action: boolean;
}

export interface DatasetStats {
  total_packages: number;
  recommended_count: number;
  advanced_count: number;
  expert_count: number;
  unsafe_count: number;
  oem_count: number;
  google_count: number;
  aosp_count: number;
  carrier_count: number;
  misc_count: number;
}
