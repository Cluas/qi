import { signal } from "nsh";
import { QiEngine, QiMeta } from "qi/native";

export interface MetricSummary {
  key: string;
  label: string;
  unit: string;
  value: number | null;
  avg_7d: number | null;
  baseline_30d_mean: number | null;
  baseline_30d_std: number | null;
  baseline_days: number;
  z_score: number | null;
  status: string;
  source: string | null;
}

export interface TrendPoint {
  date: string;
  short_label: string;
  qi_score: number;
  state: string;
}

export interface LaggedImpact {
  tag: string;
  outcome_label: string;
  event_days: number;
  delta_value: number;
  unit: string;
  summary: string;
}

export interface AnalyticsSnapshot {
  target_date: string;
  hardware_tier: string;
  hardware_tier_label: string;
  qi_score: number;
  state: string;
  state_badge: string;
  headline: string;
  micro_action: string;
  signals: string[];
  sleep_debt_7d_hours: number | null;
  sleep_last_night_hours: number | null;
  acwr: number | null;
  metrics: Record<string, MetricSummary>;
  trend_14d: TrendPoint[];
  lagged_impacts: LaggedImpact[];
}

export interface IntakeLogRow {
  id: number;
  date: string;
  category: string;
  item_name: string;
  calories_kcal: number | null;
  notes: string;
  tags: string[];
}

export interface LabBiomarkerRow {
  id: number;
  test_date: string;
  marker_name: string;
  canonical_key: string;
  value: number;
  unit: string;
  ref_low: number | null;
  ref_high: number | null;
  flag: string;
  plain_note: string;
}

export interface AppSettings {
  onboarding_done: boolean;
  primary_goal: string;
  hardware_tier: string;
  morning_feeling: number;
  sync_mode: string;
  webdav_url: string;
  byo_server_url: string;
  ai_mode: string;
  zdr_consented: boolean;
  lan_ai_url: string;
  lan_ai_model: string;
  morning_push_enabled: boolean;
  morning_push_time: string;
  thermal_throttled: boolean;
  simulated_ram_gb: number;
  recovery_phrase: string;
  checked_action_today: boolean;
}

export interface BriefingResult {
  provider_label: string;
  degraded: boolean;
  text: string;
  disclaimer: string;
}

export interface LastLabImportFeedback {
  redacted_fields: string[];
  scrubbed_preview: string;
  imported_count: number;
}

export interface UiStatePayload {
  settings: AppSettings;
  snapshot: AnalyticsSnapshot;
  briefing: BriefingResult;
  intakes: IntakeLogRow[];
  labs: LabBiomarkerRow[];
  last_lab_import: LastLabImportFeedback | null;
}

export type ActiveTab = "today" | "log" | "me";

// 单一可信状态源：所有数据变更在 Rust QiEngine 落库计算，TSX 仅持只读信号视图
let engine = new QiEngine(null);
const uiState = signal<UiStatePayload>(JSON.parse(engine.stateJson()) as UiStatePayload);
const activeTab = signal<ActiveTab>("today");
const toastMessage = signal<string | null>(null);
const encryptedBatchPreview = signal<string | null>(null);
const jsonlExportPreview = signal<string | null>(null);
const showZdrConsentModal = signal<boolean>(false);
let pendingZdrCallback: (() => void) | null = null;

function applyJson(raw: string) {
  uiState.set(JSON.parse(raw) as UiStatePayload);
}

export const getState = () => uiState();
export const getTab = () => activeTab();
export const setTab = (tab: ActiveTab) => activeTab.set(tab);
export const getToast = () => toastMessage();
export const clearToast = () => toastMessage.set(null);
export const getEncryptedBatchPreview = () => encryptedBatchPreview();
export const getJsonlExportPreview = () => jsonlExportPreview();
export const getShowZdrConsentModal = () => showZdrConsentModal();
export const getNoticeText = () => QiMeta.noticeText();
export const getDisclaimerText = () => QiMeta.disclaimer();

function notify(msg: string) {
  toastMessage.set(msg);
}

export function completeOnboarding(goal: string, tier: string) {
  applyJson(engine.completeOnboarding(goal, tier));
  notify("已启用端侧健康计算，个人数据零上传");
}

export function setHardwareTier(tier: string, feeling?: number) {
  applyJson(engine.setHardwareTier(tier, feeling ?? null));
}

export function toggleTodayAction() {
  applyJson(engine.toggleTodayAction());
}

export function logQuickIntake(category: string, itemName: string, tag: string, notes: string) {
  applyJson(engine.logQuickIntake(category, itemName, tag, notes));
  notify(`已记入「${itemName}」并更新 D+1 归因`);
}

/** 合规懒加载：若使用官方 ZDR 云端且尚未授权，先弹出 5.1.2(i) 显式同意窗 */
export function requestAiAction(fn: () => void) {
  const s = uiState().settings;
  if (s.ai_mode === "official_zdr_cloud" && !s.zdr_consented) {
    pendingZdrCallback = fn;
    showZdrConsentModal.set(true);
    return;
  }
  fn();
}

export function confirmZdrConsent() {
  const s = uiState().settings;
  applyJson(
    engine.updateAiConfig(
      s.ai_mode,
      true,
      s.lan_ai_url,
      s.lan_ai_model,
      s.thermal_throttled,
      s.simulated_ram_gb,
    ),
  );
  showZdrConsentModal.set(false);
  const action = pendingZdrCallback;
  pendingZdrCallback = null;
  if (action) action();
}

export function cancelZdrConsent() {
  showZdrConsentModal.set(false);
  pendingZdrCallback = null;
}

export function importLabReport(testDate: string, rawText: string) {
  applyJson(engine.importLabReport(testDate, rawText));
  const last = uiState().last_lab_import;
  if (last) {
    notify(`已完成端侧脱敏（遮盖 ${last.redacted_fields.length} 项隐私）并提取 ${last.imported_count} 项指标`);
  }
}

export function updateSyncConfig(syncMode: string, webdavUrl: string, byoUrl: string) {
  applyJson(engine.updateSyncConfig(syncMode, webdavUrl, byoUrl));
  notify("个人云同步策略已更新");
}

export function updateAiConfig(
  aiMode: string,
  zdrConsented: boolean,
  lanUrl: string,
  lanModel: string,
  thermalThrottled: boolean,
  ramGb: number,
) {
  applyJson(engine.updateAiConfig(aiMode, zdrConsented, lanUrl, lanModel, thermalThrottled, ramGb));
}

export function toggleMorningPush(enabled: boolean) {
  applyJson(engine.toggleMorningPush(enabled));
}

export function generateEncryptedPreview() {
  encryptedBatchPreview.set(engine.previewEncryptedBatch(7));
}

export function generateJsonlExport() {
  jsonlExportPreview.set(engine.exportJsonl());
}

export function wipeVaultAndReset() {
  applyJson(engine.wipeVaultAndReset());
  encryptedBatchPreview.set(null);
  jsonlExportPreview.set(null);
  notify("已物理清空本机健康数据库");
}

export function reseedDemoData() {
  applyJson(engine.reseedDemo());
  notify("已重新载入 35 天节律演示数据");
}
