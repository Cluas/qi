import { Clipboard, Notifications, Picker, setColorScheme, signal } from "nsh";
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

export interface FoodSearchHit {
  id: string;
  name: string;
  category: string;
  kcal: number;
  protein_g: number;
  carbs_g: number;
  fat_g: number;
  rhythm_tag: string;
  tip: string;
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
  prev_value: number | null;
  prev_date: string | null;
  delta: number | null;
  trend: string | null;
}

export interface ExperimentEvaluation {
  id: number;
  title: string;
  hypothesis: string;
  target_days: number;
  start_date: string;
  status: string;
  completed_days: number;
  checked_today: boolean;
  hrv_delta_ms: number | null;
  rhr_delta_bpm: number | null;
  deep_sleep_delta_min: number | null;
  verdict_summary: string;
}

export interface ExperimentTemplate {
  id: string;
  title: string;
  hypothesis: string;
  target_days: number;
}

export interface PeriodComparison {
  current_window: string;
  previous_window: string;
  hrv_current: number | null;
  hrv_prev: number | null;
  rhr_current: number | null;
  rhr_prev: number | null;
  sleep_current_h: number | null;
  sleep_prev_h: number | null;
  summary: string;
}

export interface LocalAiCapabilityProbe {
  platform: string;
  vision_ocr_available: boolean;
  vision_revision: number;
  foundation_models_supported: boolean;
  foundation_models_status: string;
  ollama_reachable: boolean;
  ollama_models: string[];
  summary: string;
}

export interface AppSettings {
  onboarding_done: boolean;
  primary_goal: string;
  hardware_tier: string;
  morning_feeling: number;
  sync_mode: string;
  webdav_url: string;
  webdav_username: string;
  webdav_password: string;
  byo_server_url: string;
  cloud_url: string;
  cloud_handle: string;
  cloud_token: string;
  ai_mode: string;
  zdr_consented: boolean;
  lan_ai_url: string;
  lan_ai_model: string;
  morning_push_enabled: boolean;
  morning_push_time: string;
  thermal_throttled: boolean;
  simulated_ram_gb: number;
  recovery_phrase: string;
  device_id: string;
  sync_seq: number;
  pull_cursor_id: number;
  last_sync_summary: string;
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

export interface CloudEntitlementsView {
  plan_tier: string;
  month: string;
  lab_parses_used: number;
  lab_parses_remaining: number;
  local_ollama_unlocked: boolean;
  webdav_unlocked: boolean;
}

export interface UiStatePayload {
  settings: AppSettings;
  snapshot: AnalyticsSnapshot;
  briefing: BriefingResult;
  intakes: IntakeLogRow[];
  labs: LabBiomarkerRow[];
  experiments: ExperimentEvaluation[];
  experiment_templates: ExperimentTemplate[];
  period_comparison: PeriodComparison;
  last_lab_import: LastLabImportFeedback | null;
  capability_probe: LocalAiCapabilityProbe | null;
}

export type ActiveTab = "today" | "log" | "insights" | "me";
export type AppearancePreference = "system" | "light" | "dark";

// 单一可信状态源：所有数据变更在 Rust QiEngine 落库计算，TSX 仅持只读信号视图
const engine = new QiEngine(null);
const uiState = signal<UiStatePayload>(JSON.parse(engine.stateJson()) as UiStatePayload);
const activeTab = signal<ActiveTab>("today");
const toastMessage = signal<string | null>(null);
const encryptedBatchPreview = signal<string | null>(null);
const encryptedArchivePreview = signal<string | null>(null);
const jsonlExportPreview = signal<string | null>(null);
const cloudEntitlements = signal<CloudEntitlementsView | null>(null);
const appearancePref = signal<AppearancePreference>("system");
const showZdrConsentModal = signal<boolean>(false);
const showQuickCaptureModal = signal<boolean>(false);
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
export const getEncryptedArchivePreview = () => encryptedArchivePreview();
export const getJsonlExportPreview = () => jsonlExportPreview();
export const getCloudEntitlements = () => cloudEntitlements();
export const getAppearancePref = () => appearancePref();
export const getShowZdrConsentModal = () => showZdrConsentModal();
export const getShowQuickCaptureModal = () => showQuickCaptureModal();
export const setShowQuickCaptureModal = (open: boolean) => showQuickCaptureModal.set(open);
export const getNoticeText = () => QiMeta.noticeText();
export const getDisclaimerText = () => QiMeta.disclaimer();

function notify(msg: string) {
  toastMessage.set(msg);
}

function errorMessage(err: unknown): string {
  if (err instanceof Error) return err.message;
  return String(err);
}

export function cycleAppearance() {
  const current = appearancePref();
  const next: AppearancePreference =
    current === "system" ? "light" : current === "light" ? "dark" : "system";
  appearancePref.set(next);
  try {
    setColorScheme(next);
  } catch {
    // 某些无头测试宿主的视觉宿主可忽略
  }
  const label = next === "light" ? "宣纸浅色" : next === "dark" ? "松烟深色" : "跟随系统";
  notify(`主题外观已切换为：${label}`);
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

export function logQuickIntake(
  category: string,
  itemName: string,
  tag: string,
  notes: string,
  caloriesKcal?: number | null,
) {
  applyJson(engine.logQuickIntake(category, itemName, tag, notes, caloriesKcal ?? null));
  notify(`已记入「${itemName}」并更新 D+1 归因`);
}

export function logNaturalIntake(sentence: string) {
  const text = sentence.trim();
  if (!text) return;
  try {
    applyJson(engine.logNaturalIntake(text));
    notify(`已完成一句话智能解析并记入「${text}」`);
  } catch (err) {
    notify(`速记解析失败：${errorMessage(err)}`);
  }
}

export function searchFoods(query: string, limit = 8): FoodSearchHit[] {
  try {
    return JSON.parse(QiMeta.searchFoodsJson(query, limit)) as FoodSearchHit[];
  } catch {
    return [];
  }
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
  try {
    applyJson(engine.importLabReport(testDate, rawText));
    const last = uiState().last_lab_import;
    if (last) {
      notify(`已完成端侧脱敏（遮盖 ${last.redacted_fields.length} 项隐私）并提取 ${last.imported_count} 项指标`);
    }
  } catch (err) {
    notify(`体检导入失败：${errorMessage(err)}`);
  }
}

/** 纯本机 Apple Vision 离线识别预置体检单图片（零字节上云） */
export function ocrSampleFixtureLab() {
  try {
    applyJson(engine.ocrAndImportLabImage("2026-11-13", "tests/fixtures/sample_lab_report.png"));
    const last = uiState().last_lab_import;
    notify(
      last
        ? `离线 Vision OCR 建档成功：提取 ${last.imported_count} 项指标，已脱敏 ${last.redacted_fields.length} 项`
        : "离线 Vision OCR 建档完成",
    );
  } catch (err) {
    notify(`离线 OCR 识别未就绪：${errorMessage(err)}`);
  }
}

/** 调用系统相册选取化验单图片并在端侧离线 OCR 识别 */
export async function pickAndOcrLabPhoto() {
  try {
    const picked = await Picker.pickImages({ multiple: false, maxDimension: 2048 });
    if (picked.length === 0) return;
    const file = picked[0].file;
    const buf = await file.arrayBuffer();
    const bytes = new Uint8Array(buf);
    const ext = file.name.toLowerCase().endsWith(".png") ? "png" : "jpg";
    applyJson(engine.ocrAndImportLabBytes("2026-11-13", ext, bytes));
    const last = uiState().last_lab_import;
    notify(`相册化验单离线 OCR 完成：提取 ${last?.imported_count ?? 0} 项指标`);
  } catch (err) {
    notify(`相册 OCR 失败：${errorMessage(err)}`);
  }
}

export function importExternalHealthData(format: string, rawContent: string) {
  try {
    applyJson(engine.importExternalHealthData(format, rawContent));
    notify("外部穿戴健康数据已标准化入库并重算个人基线");
  } catch (err) {
    notify(`外部数据导入失败：${errorMessage(err)}`);
  }
}

export function startExperiment(title: string, hypothesis: string, targetDays: number) {
  try {
    applyJson(engine.startExperiment(title, hypothesis, targetDays));
    notify(`已开启 ${targetDays} 天 N-of-1 实验「${title}」`);
  } catch (err) {
    notify(`开启实验失败：${errorMessage(err)}`);
  }
}

export function toggleExperimentToday(experimentId: number) {
  try {
    applyJson(engine.toggleExperimentToday(experimentId));
    notify("已更新今日实验打卡并重算前后指标对照");
  } catch (err) {
    notify(`实验打卡失败：${errorMessage(err)}`);
  }
}

export function refreshLiveBriefing() {
  requestAiAction(() => {
    try {
      applyJson(engine.refreshLiveBriefing());
      notify(`已完成早报实时润色（${uiState().briefing.provider_label}）`);
    } catch (err) {
      notify(`实时润色失败：${errorMessage(err)}`);
    }
  });
}

export function probeCapabilities() {
  try {
    applyJson(engine.probeCapabilities());
    const probe = uiState().capability_probe;
    notify(probe ? probe.summary : "已完成本地与局域网算力探测");
  } catch (err) {
    notify(`算力探测失败：${errorMessage(err)}`);
  }
}

export function updateSyncConfig(
  syncMode: string,
  webdavUrl: string,
  byoUrl: string,
  webdavUsername = uiState().settings.webdav_username,
  webdavPassword = uiState().settings.webdav_password,
) {
  applyJson(engine.updateSyncConfig(syncMode, webdavUrl, byoUrl, webdavUsername, webdavPassword));
  notify("个人云同步策略已更新");
}

export function updateCloudUrlAndHandle(cloudUrl: string, cloudHandle: string) {
  const s = uiState().settings;
  applyJson(engine.updateCloudSession(cloudUrl, cloudHandle, s.cloud_token));
}

export async function connectCloudSession(cloudUrl: string, handle: string) {
  const cleanUrl = cloudUrl.trim().replace(/\/+$/, "");
  const cleanHandle = handle.trim() || "qi-member@example.com";
  try {
    const resp = await fetch(`${cleanUrl}/v1/auth/session`, {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({ handle: cleanHandle }),
    });
    if (!resp.ok) {
      notify(`控制面登录返回 HTTP ${resp.status}`);
      return;
    }
    const data = (await resp.json()) as {
      token: string;
      handle: string;
      entitlements: CloudEntitlementsView;
    };
    applyJson(engine.updateCloudSession(cleanUrl, data.handle, data.token));
    cloudEntitlements.set(data.entitlements);
    notify(`已连接知气控制面（${data.entitlements.plan_tier.toUpperCase()}）`);
  } catch (err) {
    notify(`控制面未启动或离线，本机核心功能不受影响 (${errorMessage(err)})`);
  }
}

export async function activateCloudPro() {
  const s = uiState().settings;
  const cleanUrl = s.cloud_url.trim().replace(/\/+$/, "");
  if (!s.cloud_token) {
    notify("请先点击「连接控制面会话」获取 Bearer Token");
    return;
  }
  try {
    const resp = await fetch(`${cleanUrl}/v1/billing/activate-pro`, {
      method: "POST",
      headers: {
        "Content-Type": "application/json",
        Authorization: `Bearer ${s.cloud_token}`,
      },
      body: JSON.stringify({ tier: "pro" }),
    });
    if (!resp.ok) {
      notify(`激活 Pro 返回 HTTP ${resp.status}`);
      return;
    }
    const ent = (await resp.json()) as CloudEntitlementsView;
    cloudEntitlements.set(ent);
    notify("Pro 订阅演示权益已生效（ZDR 无限次 + WebDAV/Ollama 解锁）");
  } catch (err) {
    notify(`控制面离线，无法激活 Pro (${errorMessage(err)})`);
  }
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
  try {
    encryptedBatchPreview.set(engine.previewEncryptedBatch(7));
  } catch (err) {
    notify(`加密分片生成失败：${errorMessage(err)}`);
  }
}

export function runE2eeSyncNow() {
  try {
    applyJson(engine.runE2eeSyncNow());
    notify(uiState().settings.last_sync_summary || "个人云 E2EE 同步完成");
  } catch (err) {
    notify(`个人云同步失败：${errorMessage(err)}`);
  }
}

export function restoreFromRecoveryPhrase(phrase: string, sourceMode: string, archiveOrUrl: string) {
  try {
    applyJson(engine.restoreFromRecoveryPhrase(phrase, sourceMode, archiveOrUrl));
    notify(uiState().settings.last_sync_summary || "12 词恢复完成");
    return true;
  } catch (err) {
    notify(`恢复失败：${errorMessage(err)}`);
    return false;
  }
}

export function generateEncryptedArchive() {
  try {
    encryptedArchivePreview.set(engine.exportEncryptedArchive());
    notify("已生成 .qivault 整库加密归档");
  } catch (err) {
    notify(`归档导出失败：${errorMessage(err)}`);
  }
}

export function generateJsonlExport() {
  jsonlExportPreview.set(engine.exportJsonl());
}

export async function copyToClipboard(text: string, label: string) {
  try {
    await Clipboard.writeText(text);
    notify(`已复制${label}到系统剪贴板`);
  } catch (err) {
    notify(`剪贴板不可用：${errorMessage(err)}`);
  }
}

export async function sendMorningTestNotification() {
  try {
    const perm = await Notifications.requestPermission();
    if (perm === "denied" || perm === "unavailable") {
      notify(`系统通知当前不可用 (${perm})：打包为已签名 .app 或手机端即可接收 08:30 推送`);
      return;
    }
    const snap = uiState().snapshot;
    const shown = await Notifications.show({
      id: "qi-morning-brief",
      title: `知气 · 今日元气 ${snap.qi_score}/100 (${snap.state_badge})`,
      body: snap.micro_action,
    });
    notify(shown ? "已发送 08:30 早安本地通知预览" : "系统暂未展示通知（无窗口或免打扰）");
  } catch (err) {
    notify(`本地通知调试提示：${errorMessage(err)}`);
  }
}

export function wipeVaultAndReset() {
  const s = uiState().settings;
  if (s.cloud_token && s.cloud_url) {
    const cleanUrl = s.cloud_url.trim().replace(/\/+$/, "");
    void fetch(`${cleanUrl}/v1/account`, {
      method: "DELETE",
      headers: { Authorization: `Bearer ${s.cloud_token}` },
    }).catch(() => undefined);
  }
  applyJson(engine.wipeVaultAndReset());
  encryptedBatchPreview.set(null);
  encryptedArchivePreview.set(null);
  jsonlExportPreview.set(null);
  cloudEntitlements.set(null);
  notify("已物理清空本机健康数据库");
}

export function reseedDemoData() {
  applyJson(engine.reseedDemo());
  notify("已重新载入 35 天节律演示数据");
}
