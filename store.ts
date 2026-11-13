import { Clipboard, Notifications, Picker, setColorScheme, signal, Storage } from "nsh";
import { QiEngine, QiMeta } from "qi/native";
import {
  applyVisualTheme,
  getVisualThemeMeta,
  type KeepsakeSpec,
  VISUAL_THEMES,
  type VisualThemeId,
} from "./ui/themes";

export type { VisualThemeId };
export { VISUAL_THEMES, getVisualThemeMeta };

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
  hrv_ms?: number | null;
  sleep_h?: number | null;
}

export interface SleepStagesBreakdown {
  deep_min: number;
  core_min: number;
  rem_min: number;
  awake_min: number;
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
  sleep_stages_last_night?: SleepStagesBreakdown | null;
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
  profile_id: string;
  test_date: string;
  marker_name: string;
  canonical_key: string;
  value: number;
  unit: string;
  ref_low: number | null;
  ref_high: number | null;
  reference_high: number | null;
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
  washout_days: number;
  washout_skipped_days: number;
  outcome_verdict: string;
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

export interface PersonaMeta {
  id: string;
  name: string;
  badge: string;
  hardware_tier: string;
  primary_goal: string;
  days_count: number;
  summary: string;
  highlight_metrics: string[];
}

export interface AuditCheckItem {
  name: string;
  passed: boolean;
  detail: string;
}

export interface DataAuditReport {
  active_persona: string;
  metric_rows: number;
  distinct_days: number;
  intake_rows: number;
  lab_rows: number;
  experiment_rows: number;
  qi_score_in_bounds: boolean;
  rmssd_sdnn_isolated: boolean;
  tier_hrv_contract_held: boolean;
  lab_trends_consistent: boolean;
  pii_guard_verified: boolean;
  vault_non_lockout_verified: boolean;
  passed: boolean;
  checks: AuditCheckItem[];
}

export interface AppSettings {
  active_persona: string;
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
  entitlement_token: string;
  max_seen_epoch: number;
  guardian_sponsor: boolean;
  companion_loyalty_days: number;
  sandbox_tier: string;
  sandbox_state: string;
  weather_mood: string;
  widget_privacy_mode: string;
  equipped_keepsakes_json: string;
  pair_code: string;
  partner_name: string;
  partner_mood: number;
  bonsai_dew: number;
  active_seal_ink: string;
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
  month?: string;
  year_month?: string;
  lab_parses_used: number;
  lab_parses_remaining: number;
  local_ollama_unlocked: boolean;
  webdav_unlocked?: boolean;
  cross_platform_webdav_unlocked?: boolean;
  entitlement_token?: string;
  entitlement_expires_epoch?: number;
  guardian_sponsor?: boolean;
  companion_loyalty_days?: number;
  seats?: number;
  tier_display?: string;
  pair_code?: string;
}

export interface EntitlementVerificationResult {
  valid_signature: boolean;
  tier: string;
  tier_display: string;
  state: "free_local" | "active_atelier" | "offline_grace" | "expired_read_only_vault";
  is_atelier_unlocked: boolean;
  seats: number;
  guardian_sponsor: boolean;
  companion_loyalty_eligible: boolean;
  exp_epoch: number;
  grace_until: number;
  grace_days_remaining: number;
  clock_rollback_detected: boolean;
  effective_now_epoch: number;
  new_max_seen_epoch: number;
  features: string[];
  can_read_history_vault: boolean;
  can_export_vault: boolean;
  status_banner: string;
  reason: string;
}

export interface PricingPlanDescriptor {
  tier: string;
  title: string;
  subtitle: string;
  price_cny: number;
  original_price_cny: number | null;
  billing_cycle: string;
  seats: number;
  highlights: string[];
  badge: string;
  loyalty_applied: boolean;
}

export interface TrialTimelineStep {
  day_label: string;
  title: string;
  detail: string;
}

export interface EntitlementCatalog {
  plans: PricingPlanDescriptor[];
  guardian_addon_title: string;
  guardian_addon_price_cny: number;
  guardian_addon_desc: string;
  transparent_timeline: TrialTimelineStep[];
  ethical_promises: string[];
}

export interface SolarTermState {
  date: string;
  term_name: string;
  term_index: number;
  season: string;
  season_cn: string;
  lunar_phase: string;
  lunar_day_approx: number;
  weather_mood: string;
  active_world: string;
  whisper: string;
}

export interface SeasonalPassNodePayload {
  id: string;
  title: string;
  track: string;
  reward_kind: string;
  reward_id: string;
  cost_dew: number;
  claimed: boolean;
  unlocked_by_tier: boolean;
  evergreen_term: string;
}

export interface OutfitSpec {
  id: string;
  name: string;
  desc: string;
  track: string;
  unlocked: boolean;
}

export interface SealInkSpec {
  id: string;
  name: string;
  hex_hint: string;
  unlocked: boolean;
}

export interface KeepsakeCatalogItem {
  id: string;
  name: string;
  hint: string;
  track: string;
  unlocked: boolean;
  equipped: boolean;
}

export interface KeepsakeCatalog {
  max_equipped_slots: number;
  allowed_slots_for_tier: number;
  equipped_ids: string[];
  items: KeepsakeCatalogItem[];
  outfits: OutfitSpec[];
  seal_inks: SealInkSpec[];
  active_seal_ink: string;
}

export interface DewLedgerEntry {
  id: number;
  created_at: string;
  delta: number;
  source: string;
  note: string;
  reward_id: string;
}

export interface SanctuaryPreset {
  id: number;
  name: string;
  world_id: string;
  weather_mood: string;
  seal_ink: string;
  keepsakes_json: string;
  created_at: string;
}

export interface FoodSticker {
  id: number;
  date: string;
  title: string;
  filter: string;
  note: string;
  nutrition_hint: string;
  image_ref: string;
  is_favorite: boolean;
}

export interface StickerMonthlyQuota {
  year_month: string;
  used_count: number;
  limit: number | null;
  remaining: number | null;
  unlimited: boolean;
  can_create: boolean;
}

export interface CompanionWeeklyLetter {
  week_number: number;
  title: string;
  companion_id: string;
  companion_name: string;
  solar_term: string;
  ex_libris_title: string;
  ex_libris_seal: string;
  body: string;
  unlocked: boolean;
  lock_reason: string | null;
}

export interface CoSanctuaryActionSpec {
  id: string;
  label: string;
  dew_reward: number;
}

export interface CoSanctuaryState {
  pair_code: string;
  partner_name: string;
  self_mood_byte: number;
  self_mood_label: string;
  partner_mood_byte: number;
  partner_mood_label: string;
  bonsai_name: string;
  bonsai_dew: number;
  bonsai_stage: string;
  next_stage_dew: number | null;
  available_actions: CoSanctuaryActionSpec[];
  privacy_note: string;
}

export interface WidgetStudioPayload {
  privacy_mode: string;
  solar_term: string;
  lunar_phase: string;
  headline: string;
  subline: string;
  badge_text: string;
  companion_line: string;
  qi_score_visible: boolean;
  qi_score: number | null;
  metrics_line: string | null;
}

export interface FourSeasonsScrollNode {
  season: string;
  season_cn: string;
  solar_term: string;
  title: string;
  summary: string;
  highlight_value: string;
  sealed: boolean;
}

export interface CeramicBadge {
  id: string;
  title: string;
  glaze_style: string;
  earned: boolean;
  reason: string;
}

export interface SolarAtelierPayload {
  solar_term: SolarTermState;
  dew_balance: number;
  dew_ledger: DewLedgerEntry[];
  seasonal_pass: SeasonalPassNodePayload[];
  keepsake_catalog: KeepsakeCatalog;
  presets: SanctuaryPreset[];
  preset_limit: number | null;
  stickers: FoodSticker[];
  sticker_quota: StickerMonthlyQuota;
  weekly_letters: CompanionWeeklyLetter[];
  co_sanctuary: CoSanctuaryState;
  widget_preview: WidgetStudioPayload;
  four_seasons_scroll: FourSeasonsScrollNode[];
  ceramic_badges: CeramicBadge[];
}

export interface MultiLagImpactItem {
  tag: string;
  lag_days: number;
  lag_label: string;
  outcome_key: string;
  outcome_label: string;
  sample_n: number;
  control_n: number;
  delta_pct: number;
  cohen_d: number;
  confounder_note: string;
  summary: string;
}

export interface MultiProfileUlnPoint {
  profile_id: string;
  profile_label: string;
  canonical_key: string;
  marker_name: string;
  test_date: string;
  value: number;
  unit: string;
  reference_high: number;
  uln_pct: number;
  flag: string;
}

export interface MultiProfileRecheckReminder {
  profile_id: string;
  profile_label: string;
  canonical_key: string;
  marker_name: string;
  last_test_date: string;
  days_since_last: number;
  flag: string;
  reminder_text: string;
}

export interface ParentsPeaceLetter {
  recipient_label: string;
  title: string;
  greeting: string;
  body: string;
  highlights: string[];
  recheck_notes: string[];
  closing: string;
  large_print_markdown: string;
  e2ee_note: string;
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
  entitlements: EntitlementVerificationResult;
  entitlement_catalog: EntitlementCatalog;
  solar_atelier: SolarAtelierPayload;
  multi_lag_radar: MultiLagImpactItem[];
  multi_profile_uln_trends: MultiProfileUlnPoint[];
  recheck_reminders: MultiProfileRecheckReminder[];
  parents_peace_letter_preview: ParentsPeaceLetter;
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
const showQaStudio = signal<boolean>(false);
const lastAuditReport = signal<DataAuditReport | null>(null);
const visualTheme = signal<VisualThemeId>("cat_window");
const petCount = signal<number>(0);
const showDailyPoster = signal<boolean>(false);
const showAtelierModal = signal<boolean>(false);
const showCompanionLetterModal = signal<boolean>(false);
const showCoSanctuarySheet = signal<boolean>(false);
const showFourSeasonsScrollModal = signal<boolean>(false);
const showFamilyPeaceLetterModal = signal<boolean>(false);
const parentsPeaceLetterResult = signal<ParentsPeaceLetter | null>(null);
const hidePosterNumbers = signal<boolean>(false);
let pendingZdrCallback: (() => void) | null = null;

void Storage.getItem("qi.visual_theme")
  .then((saved) => {
    if (saved === "cat_window" || saved === "zen_garden" || saved === "playful_bento") {
      visualTheme.set(saved);
      applyVisualTheme(saved);
    } else {
      applyVisualTheme("cat_window");
    }
  })
  .catch(() => {
    applyVisualTheme("cat_window");
  });

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
export const getShowQaStudio = () => showQaStudio();
export const setShowQaStudio = (open: boolean) => showQaStudio.set(open);
export const getLastAuditReport = () => lastAuditReport();
export const getVisualTheme = () => visualTheme();
export const getPetCount = () => petCount();
export const getShowDailyPoster = () => showDailyPoster();
export const setShowDailyPoster = (open: boolean) => showDailyPoster.set(open);
export const getShowAtelierModal = () => showAtelierModal();
export const setShowAtelierModal = (open: boolean) => showAtelierModal.set(open);
export const getShowCompanionLetterModal = () => showCompanionLetterModal();
export const setShowCompanionLetterModal = (open: boolean) => showCompanionLetterModal.set(open);
export const getShowCoSanctuarySheet = () => showCoSanctuarySheet();
export const setShowCoSanctuarySheet = (open: boolean) => showCoSanctuarySheet.set(open);
export const getShowFourSeasonsScrollModal = () => showFourSeasonsScrollModal();
export const setShowFourSeasonsScrollModal = (open: boolean) => showFourSeasonsScrollModal.set(open);
export const getShowFamilyPeaceLetterModal = () => showFamilyPeaceLetterModal();
export const setShowFamilyPeaceLetterModal = (open: boolean) => showFamilyPeaceLetterModal.set(open);
export const getParentsPeaceLetterResult = () => parentsPeaceLetterResult();
export const getHidePosterNumbers = () => hidePosterNumbers();
export const setHidePosterNumbers = (hide: boolean) => hidePosterNumbers.set(hide);
export const getNoticeText = () => QiMeta.noticeText();
export const getDisclaimerText = () => QiMeta.disclaimer();

export interface DewItemStatus extends KeepsakeSpec {
  unlocked: boolean;
}

export interface DewSummary {
  points: number;
  unlockedCount: number;
  keepsakes: DewItemStatus[];
  nextTarget: DewItemStatus | null;
}

export function setVisualTheme(id: VisualThemeId, silent = false) {
  visualTheme.set(id);
  applyVisualTheme(id);
  void Storage.setItem("qi.visual_theme", id).catch(() => undefined);
  if (!silent) {
    const meta = getVisualThemeMeta(id);
    notify(`灵境画风已切换为：${meta.title}`);
  }
}

export function cycleVisualTheme() {
  const order: VisualThemeId[] = ["cat_window", "zen_garden", "playful_bento"];
  const idx = order.indexOf(visualTheme());
  const next = order[(idx + 1) % order.length];
  setVisualTheme(next);
}

export function petCompanion() {
  petCount.update((n) => n + 1);
}

export function getDewSummary(): DewSummary {
  const s = uiState();
  const actionDew = s.settings.checked_action_today ? 2 : 0;
  const intakeDew = Math.min(4, s.intakes.length);
  const expDays = s.experiments.reduce((acc, e) => acc + Math.max(0, e.completed_days), 0);
  const expDew = Math.min(4, expDays);
  const petDew = petCount() > 0 ? 1 : 0;
  const points = 1 + actionDew + intakeDew + expDew + petDew;
  const meta = getVisualThemeMeta(visualTheme());
  const keepsakes: DewItemStatus[] = meta.keepsakes.map((k) => ({
    ...k,
    unlocked: points >= k.threshold,
  }));
  const unlockedCount = keepsakes.filter((k) => k.unlocked).length;
  const nextTarget = keepsakes.find((k) => !k.unlocked) ?? null;
  return { points, unlockedCount, keepsakes, nextTarget };
}

function notify(msg: string) {
  toastMessage.set(msg);
}

function errorMessage(err: unknown): string {
  if (err instanceof Error) return err.message;
  return String(err);
}

export function getPersonas(): PersonaMeta[] {
  try {
    return JSON.parse(QiMeta.personasJson()) as PersonaMeta[];
  } catch {
    return [];
  }
}

export function loadPersona(personaId: string) {
  try {
    applyJson(engine.loadPersona(personaId));
    lastAuditReport.set(null);
    const matched = getPersonas().find((p) => p.id === personaId);
    notify(`已切换画像：${matched ? matched.name : personaId}`);
  } catch (err) {
    notify(`画像切换失败：${errorMessage(err)}`);
  }
}

export function runDataIntegrityAudit(): DataAuditReport | null {
  try {
    const report = JSON.parse(engine.runDataIntegrityAudit()) as DataAuditReport;
    lastAuditReport.set(report);
    notify(
      report.passed
        ? "所有 6 项不变量校验通过"
        : "数据自检发现异常项，请查看校验明细",
    );
    return report;
  } catch (err) {
    notify(`数据完整性自检失败：${errorMessage(err)}`);
    return null;
  }
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
  notify("已启用端侧健康计算，个人数据仅存本机");
}

export function setHardwareTier(tier: string, feeling?: number) {
  applyJson(engine.setHardwareTier(tier, feeling ?? null));
}

export function toggleTodayAction() {
  const wasChecked = uiState().settings.checked_action_today;
  applyJson(engine.toggleTodayAction());
  if (!wasChecked && uiState().settings.checked_action_today) {
    notify("已盖下今日印章 · 收集 +2 💧 晨露");
  }
}

export function logQuickIntake(
  category: string,
  itemName: string,
  tag: string,
  notes: string,
  caloriesKcal?: number | null,
) {
  applyJson(engine.logQuickIntake(category, itemName, tag, notes, caloriesKcal ?? null));
  notify(`已记入「${itemName}」并更新节律关联`);
}

export function logNaturalIntake(sentence: string) {
  const text = sentence.trim();
  if (!text) return;
  try {
    applyJson(engine.logNaturalIntake(text));
    notify(`已完成一句话解析并记入「${text}」`);
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
    notify(`相册化验单离线识别完成：提取 ${last?.imported_count ?? 0} 项指标`);
  } catch (err) {
    notify(`相册识图失败：${errorMessage(err)}`);
  }
}

export function importExternalHealthData(format: string, rawContent: string) {
  try {
    applyJson(engine.importExternalHealthData(format, rawContent));
    notify("历史穿戴健康数据已标准化入库并更新个人节律");
  } catch (err) {
    notify(`外部数据导入失败：${errorMessage(err)}`);
  }
}

export function startExperiment(title: string, hypothesis: string, targetDays: number) {
  try {
    applyJson(engine.startExperiment(title, hypothesis, targetDays));
    notify(`已开启 ${targetDays} 天微习惯实验「${title}」`);
  } catch (err) {
    notify(`开启实验失败：${errorMessage(err)}`);
  }
}

export function toggleExperimentToday(experimentId: number) {
  try {
    applyJson(engine.toggleExperimentToday(experimentId));
    notify("已更新今日实验打卡并重算前后变化");
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
      notify(`账号会话连接返回 HTTP ${resp.status}`);
      return;
    }
    const data = (await resp.json()) as {
      token: string;
      handle: string;
      entitlements: CloudEntitlementsView;
    };
    applyJson(engine.updateCloudSession(cleanUrl, data.handle, data.token));
    if (data.entitlements?.entitlement_token) {
      applyJson(engine.applyEntitlementToken(data.entitlements.entitlement_token));
    }
    cloudEntitlements.set(data.entitlements);
    notify(`已连接知气服务（${data.entitlements.plan_tier.toUpperCase()}）`);
  } catch (err) {
    notify(`云端服务当前离线，本机核心功能不受影响 (${errorMessage(err)})`);
  }
}

export async function activateCloudPro(tier = "pro_annual", guardianSponsor = false) {
  const s = uiState().settings;
  const cleanUrl = s.cloud_url.trim().replace(/\/+$/, "");
  if (!s.cloud_token) {
    // 离线或未登录云账号时，直接以本机 Ed25519 沙盒签发体验通行证
    setEntitlementSandbox(tier, "active", guardianSponsor, s.companion_loyalty_days);
    return;
  }
  try {
    const resp = await fetch(`${cleanUrl}/v1/billing/mock-upgrade`, {
      method: "POST",
      headers: {
        "Content-Type": "application/json",
        Authorization: `Bearer ${s.cloud_token}`,
      },
      body: JSON.stringify({
        tier,
        guardian_sponsor: guardianSponsor,
        companion_loyalty_days: s.companion_loyalty_days,
      }),
    });
    if (!resp.ok) {
      notify(`升级雅集返回 HTTP ${resp.status}`);
      return;
    }
    const ent = (await resp.json()) as CloudEntitlementsView;
    cloudEntitlements.set(ent);
    if (ent.entitlement_token) {
      applyJson(engine.applyEntitlementToken(ent.entitlement_token));
    }
    notify(`雅集权益已生效（${ent.tier_display ?? ent.plan_tier}）`);
  } catch (err) {
    notify(`云端服务离线，暂无法切换雅集权益 (${errorMessage(err)})`);
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
    notify(uiState().settings.last_sync_summary || "个人云加密同步完成");
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
      notify(`系统通知当前不可用 (${perm})：打包为已签名应用后即可接收 08:30 推送`);
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
    notify(`本地通知提示：${errorMessage(err)}`);
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
  lastAuditReport.set(null);
  showQaStudio.set(false);
  showDailyPoster.set(false);
  petCount.set(0);
  notify("已物理清空本机健康数据库");
}

export function reseedDemoData() {
  applyJson(engine.reseedDemo());
  lastAuditReport.set(null);
  notify("已重新载入 35 天节律演示数据");
}

// ---------- 知气雅集 v4.0 动作方法 ----------

export function hasFeature(feature: string): boolean {
  const ent = uiState().entitlements;
  return ent.features.includes(feature);
}

export function verifyEntitlements() {
  try {
    applyJson(engine.verifyEntitlements());
    notify("已完成 Ed25519 离线通行证与单调时钟校验");
  } catch (err) {
    notify(`验签提示：${errorMessage(err)}`);
  }
}

export function setEntitlementSandbox(
  tier: string,
  simulatedState = "active",
  guardianSponsor = false,
  loyaltyDays = 32,
) {
  try {
    applyJson(engine.setEntitlementSandbox(tier, simulatedState, guardianSponsor, loyaltyDays));
    notify(`已签发 ${uiState().entitlements.tier_display} 凭证（${uiState().entitlements.state}）`);
  } catch (err) {
    notify(`切换权益沙盒失败：${errorMessage(err)}`);
  }
}

export function applyEntitlementToken(token: string) {
  try {
    applyJson(engine.applyEntitlementToken(token));
    notify("Ed25519 雅集通行证已生效");
  } catch (err) {
    notify(errorMessage(err));
  }
}

export function clearEntitlementToken() {
  applyJson(engine.clearEntitlementToken());
  notify("已回归「素心清欢」永久免费版（历史档案仍永久可读可导出）");
}

export function claimSeasonalReward(rewardId: string) {
  try {
    applyJson(engine.claimSeasonalReward(rewardId));
    notify("已收下该岁时风物");
  } catch (err) {
    notify(errorMessage(err));
  }
}

export function claimRecoveryDew() {
  try {
    applyJson(engine.claimRecoveryDew());
    notify("已领取「安心静养 · 睡饱双倍露 +3 💧」");
  } catch (err) {
    notify(errorMessage(err));
  }
}

export function setWeatherMood(mood: string) {
  applyJson(engine.setWeatherMood(mood));
  notify("已切换窗外天色心境");
}

export function toggleEquippedKeepsake(keepsakeId: string) {
  try {
    applyJson(engine.toggleEquippedKeepsake(keepsakeId));
  } catch (err) {
    notify(errorMessage(err));
  }
}

export function setSealInk(ink: string) {
  try {
    applyJson(engine.setSealInk(ink));
    notify("已更换日签印泥");
  } catch (err) {
    notify(errorMessage(err));
  }
}

export function saveSanctuaryPreset(name: string) {
  try {
    applyJson(engine.saveSanctuaryPreset(name));
    notify(`已保存书房预设「${name.trim()}」`);
  } catch (err) {
    notify(errorMessage(err));
  }
}

export function applySanctuaryPreset(presetId: number) {
  try {
    applyJson(engine.applySanctuaryPreset(presetId));
    notify("已套用书房预设");
  } catch (err) {
    notify(errorMessage(err));
  }
}

export function addFoodSticker(
  date: string,
  title: string,
  filter: string,
  note = "",
  imageRef = "",
) {
  try {
    applyJson(engine.addFoodSticker(date, title, filter, note, imageRef));
    notify(`已定格食光贴纸「${title.trim()}」(+1 💧)`);
  } catch (err) {
    notify(errorMessage(err));
  }
}

export function toggleStickerFavorite(stickerId: number) {
  applyJson(engine.toggleStickerFavorite(stickerId));
}

export function deleteFoodSticker(stickerId: number) {
  applyJson(engine.deleteFoodSticker(stickerId));
  notify("已移除该贴纸");
}

export function coSanctuaryAction(action: string, partnerName?: string, pairCode?: string) {
  applyJson(engine.cosanctuaryAction(action, partnerName ?? null, pairCode ?? null));
  if (action === "warm_tea" || action === "tuck_blanket" || action === "paw_stamp") {
    notify("已向同窗好友送去问候并浇灌窗台金桂 (+1 💧)");
  }
}

export function setWidgetPrivacyMode(mode: "metaphor" | "metrics") {
  applyJson(engine.setWidgetPrivacyMode(mode));
}

export function startCustomExperiment(
  title: string,
  hypothesis: string,
  targetDays: number,
  washoutDays: number,
) {
  try {
    applyJson(engine.startCustomExperiment(title, hypothesis, targetDays, washoutDays));
    notify(`已开启自定义实验（洗脱期 ${washoutDays} 天）`);
  } catch (err) {
    notify(errorMessage(err));
  }
}

export function generateParentsPeaceLetter(recipientLabel = "爸妈") {
  try {
    const raw = engine.generateParentsPeaceLetter(recipientLabel);
    parentsPeaceLetterResult.set(JSON.parse(raw) as ParentsPeaceLetter);
    showFamilyPeaceLetterModal.set(true);
  } catch (err) {
    notify(errorMessage(err));
  }
}

export function addProfileLabMarker(
  profileId: "self" | "partner" | "parents",
  testDate: string,
  markerName: string,
  value: number,
  unit: string,
  refLow: number | null,
  refHigh: number | null,
) {
  try {
    applyJson(
      engine.addProfileLabMarker(profileId, testDate, markerName, value, unit, refLow, refHigh),
    );
    notify(`已记录${profileId === "partner" ? "伴侣" : profileId === "parents" ? "父母长辈" : "本人"}指标：${markerName}`);
  } catch (err) {
    notify(errorMessage(err));
  }
}

export function pullWebdavSync() {
  try {
    applyJson(engine.pullWebdavSync());
    notify(uiState().settings.last_sync_summary);
  } catch (err) {
    notify(`WebDAV 拉取提示：${errorMessage(err)}`);
  }
}
