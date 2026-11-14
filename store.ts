import { Clipboard, Notifications, Picker, setColorScheme, signal, Storage } from "nsh";
import { QiEngine, QiMeta } from "qi/native";
import type { AtelierTierId } from "./ui/atelier";
import { applyLocale, type AppLocale, getLocale, normalizeLocale, tr } from "./ui/i18n";
import {
  applyVisualTheme,
  getVisualThemeMeta,
  type KeepsakeSpec,
  VISUAL_THEMES,
  type VisualThemeId,
} from "./ui/themes";

export type { AppLocale, VisualThemeId };
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
  locale: string;
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

export interface CustomPetCompanion {
  id: string;
  name: string;
  species: "cat" | "dog" | "rabbit" | "fox" | "other" | string;
  persona: string;
  photo_data_url: string;
  die_cut_frame:
    | "celadon_oval"
    | "lacquer_arch"
    | "paper_cut"
    | "starlight_halo"
    | "warm_stamp"
    | string;
  starlight_memorial: boolean;
  paw_stamp_title: string;
  is_active: boolean;
  created_at: string;
  updated_at: string;
}

export interface CustomPetQuota {
  used_count: number;
  allowed_count: number;
  remaining_add_slots: number;
  can_add_more: boolean;
  is_pro_ex_libris_unlocked: boolean;
  can_generate_weekly_letter: boolean;
  retained_above_free_quota: boolean;
}

export interface PetWeeklyLetter {
  id: number;
  pet_id: string;
  pet_name: string;
  species: string;
  week_number: number;
  solar_term: string;
  title: string;
  body: string;
  ex_libris_seal: string;
  starlight_memorial: boolean;
  locale: string;
  created_at: string;
}

export interface AppleCatalogProduct {
  product_id: string;
  tier?: string;
  kind?: string;
}

export interface AppleCatalogIds {
  subscriptions: AppleCatalogProduct[];
  non_consumables: AppleCatalogProduct[];
  consumables: AppleCatalogProduct[];
}

export interface StoreKitTransactionInput {
  product_id: string;
  transaction_id: string;
  original_transaction_id: string;
  environment: string;
  jws_representation?: string | null;
  expires_date_epoch?: number | null;
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
  custom_pets: CustomPetCompanion[];
  active_custom_pet: CustomPetCompanion | null;
  custom_pet_quota: CustomPetQuota;
  pet_weekly_letters: PetWeeklyLetter[];
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
export type LogSubTab = "quick" | "labs" | "vault";
export type InsightsSubTab = "curves" | "correlations" | "handscroll";
export type MeSubTab = "pet_studio" | "atelier_billing" | "vault_export";
export type AppearancePreference = "system" | "light" | "dark";

const HEARTH_GIFT_PASS_PRODUCT_ID = "com.qi.hearth_gift_pass";

const DEFAULT_APPLE_PRODUCT_MAP: Record<string, string> = {
  pro_monthly: "com.qi.atelier.pro_monthly",
  seasonal: "com.qi.atelier.seasonal",
  pro_annual: "com.qi.atelier.pro_annual",
  duo: "com.qi.atelier.duo",
  family: "com.qi.atelier.family",
  lifetime_local: "com.qi.atelier.lifetime_local",
  hearth_gift_pass: HEARTH_GIFT_PASS_PRODUCT_ID,
};

// Single source of truth: mutations persist in Rust QiEngine; TSX holds reactive signals.
const engine = new QiEngine(null);
const initialPayload = JSON.parse(engine.stateJson()) as UiStatePayload;
applyLocale(normalizeLocale(initialPayload.settings?.locale || "zh-CN"));

const uiState = signal<UiStatePayload>(initialPayload);
const activeTab = signal<ActiveTab>("today");
const logSubTab = signal<LogSubTab>("quick");
const insightsSubTab = signal<InsightsSubTab>("curves");
const meSubTab = signal<MeSubTab>("pet_studio");
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
const showTermsDrawer = signal<boolean>(false);
const showPrivacyDrawer = signal<boolean>(false);
const restoringPurchases = signal<boolean>(false);
const restoreStatusMessage = signal<string>("");
const hearthGiftCount = signal<number>(initialPayload.settings?.guardian_sponsor ? 1 : 0);
const parentsPeaceLetterResult = signal<ParentsPeaceLetter | null>(null);
const hidePosterNumbers = signal<boolean>(false);
const recentAppleTransactions: StoreKitTransactionInput[] = [];
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
  const parsed = JSON.parse(raw) as UiStatePayload;
  uiState.set(parsed);
  const nextLocale = normalizeLocale(parsed.settings?.locale || "zh-CN");
  if (nextLocale !== getLocale()) {
    applyLocale(nextLocale);
  }
  if (parsed.entitlements?.guardian_sponsor) {
    hearthGiftCount.update((n) => Math.max(1, n));
  }
}

export const getState = () => uiState();
export const getTab = () => activeTab();
export const setTab = (tab: ActiveTab) => activeTab.set(tab);
export const getLogSubTab = () => logSubTab();
export const setLogSubTab = (sub: LogSubTab) => logSubTab.set(sub);
export const getInsightsSubTab = () => insightsSubTab();
export const setInsightsSubTab = (sub: InsightsSubTab) => insightsSubTab.set(sub);
export const getMeSubTab = () => meSubTab();
export const setMeSubTab = (sub: MeSubTab) => meSubTab.set(sub);
export function openPetStudio() {
  meSubTab.set("pet_studio");
  activeTab.set("me");
}
export function openMealsLog() {
  logSubTab.set("quick");
  activeTab.set("log");
}
export function openFourSeasonsInsights() {
  insightsSubTab.set("handscroll");
  activeTab.set("insights");
}
export const getAppLocale = () => getLocale();
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
export const isQaStudioEnabled = (): boolean => QiMeta.qaStudioEnabled();
export const setShowQaStudio = (open: boolean) => {
  if (open && !isQaStudioEnabled()) {
    return;
  }
  showQaStudio.set(open);
};
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
export const getShowTermsDrawer = () => showTermsDrawer();
export const setShowTermsDrawer = (open: boolean) => showTermsDrawer.set(open);
export const getShowPrivacyDrawer = () => showPrivacyDrawer();
export const setShowPrivacyDrawer = (open: boolean) => showPrivacyDrawer.set(open);
export const getRestoringPurchases = () => restoringPurchases();
export const getRestoreStatusMessage = () => restoreStatusMessage();
export const getHearthGiftCount = () => hearthGiftCount();
export const getParentsPeaceLetterResult = () => parentsPeaceLetterResult();
export const getHidePosterNumbers = () => hidePosterNumbers();
export const setHidePosterNumbers = (hide: boolean) => hidePosterNumbers.set(hide);
export const getNoticeText = () => QiMeta.noticeText();
export const getDisclaimerText = () => QiMeta.disclaimerForLocale(getLocale());

export interface DewItemStatus extends KeepsakeSpec {
  unlocked: boolean;
}

export interface DewSummary {
  points: number;
  unlockedCount: number;
  keepsakes: DewItemStatus[];
  nextTarget: DewItemStatus | null;
}

export function setAppLocale(nextLocale: AppLocale, silent = false) {
  const normalized = normalizeLocale(nextLocale);
  try {
    const raw = engine.setLocale(normalized);
    applyLocale(normalized);
    applyJson(raw);
  } catch {
    applyLocale(normalized);
  }
  if (!silent) {
    notify(tr("toast.locale_changed"));
  }
}

export function toggleAppLocale() {
  const next: AppLocale = getLocale() === "en" ? "zh-CN" : "en";
  setAppLocale(next);
}

export function setVisualTheme(id: VisualThemeId, silent = false) {
  visualTheme.set(id);
  applyVisualTheme(id);
  void Storage.setItem("qi.visual_theme", id).catch(() => undefined);
  if (!silent) {
    const meta = getVisualThemeMeta(id);
    notify(tr("toast.theme_changed", { name: meta.title }));
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
  if (!isQaStudioEnabled()) {
    notify(tr("toast.persona_failed", { error: "QA Studio is disabled in release builds." }));
    return;
  }
  try {
    applyJson(engine.loadPersona(personaId));
    lastAuditReport.set(null);
    const matched = getPersonas().find((p) => p.id === personaId);
    notify(tr("toast.persona_loaded", { name: matched ? matched.name : personaId }));
  } catch (err) {
    notify(tr("toast.persona_failed", { error: errorMessage(err) }));
  }
}

export function runDataIntegrityAudit(): DataAuditReport | null {
  try {
    const report = JSON.parse(engine.runDataIntegrityAudit()) as DataAuditReport;
    lastAuditReport.set(report);
    notify(report.passed ? tr("toast.audit_passed") : tr("toast.audit_failed"));
    return report;
  } catch (err) {
    notify(tr("toast.audit_error", { error: errorMessage(err) }));
    return null;
  }
}

export function setAppearancePref(next: AppearancePreference, silent = false) {
  appearancePref.set(next);
  try {
    setColorScheme(next);
  } catch {
    // Headless test hosts may ignore system color scheme hints.
  }
  if (!silent) {
    const label =
      next === "light"
        ? tr("common.appearance_paper")
        : next === "dark"
          ? tr("common.appearance_ink")
          : tr("common.appearance_auto");
    notify(tr("toast.appearance_changed", { label }));
  }
}

export function cycleAppearance() {
  const current = appearancePref();
  const next: AppearancePreference =
    current === "system" ? "light" : current === "light" ? "dark" : "system";
  setAppearancePref(next);
}

export function completeOnboarding(goal: string, tier: string) {
  applyJson(engine.completeOnboarding(goal, tier));
  notify(tr("toast.onboarding_ready"));
}

export function setHardwareTier(tier: string, feeling?: number) {
  applyJson(engine.setHardwareTier(tier, feeling ?? null));
}

export function toggleTodayAction() {
  const wasChecked = uiState().settings.checked_action_today;
  applyJson(engine.toggleTodayAction());
  if (!wasChecked && uiState().settings.checked_action_today) {
    notify(tr("toast.stamp_done"));
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
  notify(tr("toast.intake_logged", { item: itemName }));
}

export function logNaturalIntake(sentence: string) {
  const text = sentence.trim();
  if (!text) return;
  try {
    applyJson(engine.logNaturalIntake(text));
    notify(tr("toast.natural_intake_logged", { text }));
  } catch (err) {
    notify(tr("toast.natural_intake_failed", { error: errorMessage(err) }));
  }
}

export function searchFoods(query: string, limit = 8): FoodSearchHit[] {
  try {
    return JSON.parse(QiMeta.searchFoodsJson(query, limit)) as FoodSearchHit[];
  } catch {
    return [];
  }
}

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
      notify(
        tr("toast.lab_imported", {
          redacted: last.redacted_fields.length,
          count: last.imported_count,
        }),
      );
    }
  } catch (err) {
    notify(tr("toast.lab_import_failed", { error: errorMessage(err) }));
  }
}

/** Offline Vision OCR on built-in sample lab image (0 bytes uploaded). */
export function ocrSampleFixtureLab() {
  try {
    applyJson(engine.ocrAndImportLabImage("2026-11-13", "tests/fixtures/sample_lab_report.png"));
    const last = uiState().last_lab_import;
    notify(
      last
        ? tr("toast.ocr_ready", {
            count: last.imported_count,
            redacted: last.redacted_fields.length,
          })
        : tr("toast.ocr_done_simple"),
    );
  } catch (err) {
    notify(tr("toast.ocr_not_ready", { error: errorMessage(err) }));
  }
}

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
    notify(tr("toast.photo_ocr_done", { count: last?.imported_count ?? 0 }));
  } catch (err) {
    notify(tr("toast.photo_ocr_failed", { error: errorMessage(err) }));
  }
}

export function importExternalHealthData(format: string, rawContent: string) {
  try {
    applyJson(engine.importExternalHealthData(format, rawContent));
    notify(tr("toast.wearable_imported"));
  } catch (err) {
    notify(tr("toast.wearable_failed", { error: errorMessage(err) }));
  }
}

export function startExperiment(title: string, hypothesis: string, targetDays: number) {
  try {
    applyJson(engine.startExperiment(title, hypothesis, targetDays));
    notify(tr("toast.experiment_started", { days: targetDays, title }));
  } catch (err) {
    notify(tr("toast.experiment_failed", { error: errorMessage(err) }));
  }
}

export function toggleExperimentToday(experimentId: number) {
  try {
    applyJson(engine.toggleExperimentToday(experimentId));
    notify(tr("toast.experiment_toggled"));
  } catch (err) {
    notify(tr("toast.experiment_toggle_failed", { error: errorMessage(err) }));
  }
}

export function refreshLiveBriefing() {
  requestAiAction(() => {
    try {
      applyJson(engine.refreshLiveBriefing());
      notify(tr("toast.briefing_refreshed", { provider: uiState().briefing.provider_label }));
    } catch (err) {
      notify(tr("toast.briefing_failed", { error: errorMessage(err) }));
    }
  });
}

export function probeCapabilities() {
  try {
    applyJson(engine.probeCapabilities());
    const probe = uiState().capability_probe;
    notify(probe ? probe.summary : tr("toast.capabilities_probed"));
  } catch (err) {
    notify(tr("toast.capabilities_failed", { error: errorMessage(err) }));
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
  notify(tr("toast.sync_updated"));
}

export function updateCloudUrlAndHandle(cloudUrl: string, cloudHandle: string) {
  const s = uiState().settings;
  applyJson(engine.updateCloudSession(cloudUrl, cloudHandle, s.cloud_token));
}

export async function connectCloudSession(cloudUrl: string, handle: string) {
  const cleanUrl = cloudUrl.trim().replace(/\/+$/, "");
  const cleanHandle = handle.trim() || "qi-member@example.com";
  try {
    const s = uiState().settings;
    const authProofHex = QiMeta.deriveCloudAuthProof(cleanHandle, s.recovery_phrase);
    const headers: Record<string, string> = { "Content-Type": "application/json" };
    if (s.cloud_token?.trim()) {
      headers.Authorization = `Bearer ${s.cloud_token.trim()}`;
    }
    const resp = await fetch(`${cleanUrl}/v1/auth/session`, {
      method: "POST",
      headers,
      body: JSON.stringify({ handle: cleanHandle, auth_proof_hex: authProofHex }),
    });
    if (!resp.ok) {
      notify(tr("toast.cloud_http_error", { status: resp.status }));
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
    notify(tr("toast.cloud_connected", { tier: data.entitlements.plan_tier.toUpperCase() }));
  } catch (err) {
    notify(tr("toast.cloud_offline", { error: errorMessage(err) }));
  }
}

export async function activateCloudPro(tier = "pro_annual", guardianSponsor = false) {
  if (!isQaStudioEnabled()) {
    notify(tr("errors.iap_unavailable"));
    return;
  }
  const s = uiState().settings;
  const cleanUrl = s.cloud_url.trim().replace(/\/+$/, "");
  if (!s.cloud_token) {
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
      notify(tr("toast.upgrade_http_error", { status: resp.status }));
      return;
    }
    const ent = (await resp.json()) as CloudEntitlementsView;
    cloudEntitlements.set(ent);
    if (ent.entitlement_token) {
      applyJson(engine.applyEntitlementToken(ent.entitlement_token));
    }
    notify(tr("toast.atelier_activated", { tier: ent.tier_display ?? ent.plan_tier }));
  } catch (err) {
    notify(tr("toast.atelier_offline", { error: errorMessage(err) }));
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
    notify(tr("toast.preview_failed", { error: errorMessage(err) }));
  }
}

export function runE2eeSyncNow() {
  try {
    applyJson(engine.runE2eeSyncNow());
    notify(uiState().settings.last_sync_summary || tr("toast.sync_done_default"));
  } catch (err) {
    notify(tr("toast.sync_failed", { error: errorMessage(err) }));
  }
}

export function restoreFromRecoveryPhrase(phrase: string, sourceMode: string, archiveOrUrl: string) {
  try {
    applyJson(engine.restoreFromRecoveryPhrase(phrase, sourceMode, archiveOrUrl));
    notify(uiState().settings.last_sync_summary || tr("toast.restore_phrase_done"));
    return true;
  } catch (err) {
    notify(tr("toast.restore_phrase_failed", { error: errorMessage(err) }));
    return false;
  }
}

export function generateEncryptedArchive() {
  try {
    encryptedArchivePreview.set(engine.exportEncryptedArchive());
    notify(tr("toast.backup_exported"));
  } catch (err) {
    notify(tr("toast.archive_failed", { error: errorMessage(err) }));
  }
}

export function generateJsonlExport() {
  jsonlExportPreview.set(engine.exportJsonl());
  notify(tr("toast.jsonl_ready"));
}

export async function copyToClipboard(text: string, label: string) {
  try {
    await Clipboard.writeText(text);
    notify(tr("toast.copied", { label }));
  } catch (err) {
    notify(tr("toast.clipboard_unavailable", { error: errorMessage(err) }));
  }
}

export async function sendMorningTestNotification() {
  try {
    const perm = await Notifications.requestPermission();
    if (perm === "denied" || perm === "unavailable") {
      notify(tr("toast.push_unavailable", { perm }));
      return;
    }
    const snap = uiState().snapshot;
    const shown = await Notifications.show({
      id: "qi-morning-brief",
      title: tr("toast.push_title", { score: snap.qi_score, badge: snap.state_badge }),
      body: snap.micro_action,
    });
    notify(shown ? tr("toast.push_sent") : tr("toast.push_silent"));
  } catch (err) {
    notify(tr("toast.push_error", { error: errorMessage(err) }));
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
  hearthGiftCount.set(0);
  restoreStatusMessage.set("");
  restoringPurchases.set(false);
  recentAppleTransactions.length = 0;
  notify(tr("toast.vault_wiped"));
}

export function reseedDemoData() {
  if (!isQaStudioEnabled()) {
    notify(tr("toast.sandbox_failed", { error: "QA Studio is disabled in release builds." }));
    return;
  }
  applyJson(engine.reseedDemo());
  lastAuditReport.set(null);
  notify(tr("toast.demo_reseeded"));
}

// ---------- Qi Atelier v5.0 & Apple IAP Actions ----------

export function getAppleCatalogProductIds(): AppleCatalogIds {
  try {
    return JSON.parse(QiMeta.appleCatalogProductIdsJson()) as AppleCatalogIds;
  } catch {
    return {
      subscriptions: [
        { product_id: DEFAULT_APPLE_PRODUCT_MAP.pro_monthly, tier: "pro_monthly" },
        { product_id: DEFAULT_APPLE_PRODUCT_MAP.seasonal, tier: "seasonal" },
        { product_id: DEFAULT_APPLE_PRODUCT_MAP.pro_annual, tier: "pro_annual" },
        { product_id: DEFAULT_APPLE_PRODUCT_MAP.duo, tier: "duo" },
        { product_id: DEFAULT_APPLE_PRODUCT_MAP.family, tier: "family" },
      ],
      non_consumables: [
        { product_id: DEFAULT_APPLE_PRODUCT_MAP.lifetime_local, tier: "lifetime_local" },
      ],
      consumables: [
        { product_id: HEARTH_GIFT_PASS_PRODUCT_ID, kind: "consumable_gift_pass" },
      ],
    };
  }
}

export function resolveAppleProductIdForTier(tier: string): string | null {
  if (tier === "free") return null;
  const catalog = getAppleCatalogProductIds();
  const sub = catalog.subscriptions.find((item) => item.tier === tier);
  if (sub) return sub.product_id;
  const nc = catalog.non_consumables.find((item) => item.tier === tier);
  if (nc) return nc.product_id;
  return DEFAULT_APPLE_PRODUCT_MAP[tier] ?? null;
}

export function hasFeature(feature: string): boolean {
  const ent = uiState().entitlements;
  return ent.features.includes(feature);
}

export function verifyEntitlements() {
  try {
    applyJson(engine.verifyEntitlements());
    notify(tr("toast.entitlement_verified"));
  } catch (err) {
    notify(tr("toast.verify_hint", { error: errorMessage(err) }));
  }
}

export function setEntitlementSandbox(
  tier: string,
  simulatedState = "active",
  guardianSponsor = false,
  loyaltyDays = 32,
) {
  if (!isQaStudioEnabled()) {
    notify(tr("toast.sandbox_failed", { error: "QA Studio is disabled in release builds." }));
    return;
  }
  try {
    applyJson(engine.setEntitlementSandbox(tier, simulatedState, guardianSponsor, loyaltyDays));
    if (guardianSponsor) {
      hearthGiftCount.update((n) => Math.max(1, n));
    } else if (tier === "free") {
      hearthGiftCount.set(0);
    }
    notify(
      tr("toast.sandbox_applied", {
        tier: uiState().entitlements.tier_display,
        state: uiState().entitlements.state,
      }),
    );
  } catch (err) {
    notify(tr("toast.sandbox_failed", { error: errorMessage(err) }));
  }
}

export function applyEntitlementToken(token: string) {
  try {
    applyJson(engine.applyEntitlementToken(token));
    notify(tr("toast.token_applied"));
  } catch (err) {
    notify(errorMessage(err));
  }
}

export function clearEntitlementToken() {
  applyJson(engine.clearEntitlementToken());
  notify(tr("toast.free_restored"));
}

function rememberAppleTransaction(tx: StoreKitTransactionInput) {
  const idx = recentAppleTransactions.findIndex((item) => item.product_id === tx.product_id);
  if (idx >= 0) {
    recentAppleTransactions[idx] = tx;
  } else {
    recentAppleTransactions.push(tx);
  }
}

export function verifyAppleIapTransaction(input: {
  productId: string;
  transactionId?: string;
  originalTransactionId?: string;
  environment?: string;
  jwsRepresentation?: string | null;
  expiresDateEpoch?: number | null;
  silent?: boolean;
}): boolean {
  const nowSec = Math.floor(Date.now() / 1000);
  const txId =
    input.transactionId?.trim() || (isQaStudioEnabled() ? `tx_${input.productId}_${nowSec}` : "");
  const origTxId = input.originalTransactionId?.trim() || txId;
  const env = input.environment?.trim() || "Sandbox";
  const prevGiftCount = hearthGiftCount();
  try {
    applyJson(
      engine.verifyAppleIapTransaction(
        input.productId,
        txId,
        origTxId,
        env,
        input.jwsRepresentation ?? null,
        input.expiresDateEpoch ?? null,
      ),
    );
    rememberAppleTransaction({
      product_id: input.productId,
      transaction_id: txId,
      original_transaction_id: origTxId,
      environment: env,
      jws_representation: input.jwsRepresentation ?? null,
      expires_date_epoch: input.expiresDateEpoch ?? null,
    });
    if (input.productId === HEARTH_GIFT_PASS_PRODUCT_ID) {
      hearthGiftCount.set(prevGiftCount + 1);
      if (!input.silent) notify(tr("toast.gift_pass_done"));
    } else if (!input.silent) {
      notify(tr("toast.apple_verified", { tier: uiState().entitlements.tier_display }));
    }
    return true;
  } catch (err) {
    if (!input.silent) {
      notify(tr("toast.apple_verify_failed", { error: errorMessage(err) }));
    }
    return false;
  }
}

export interface HostStoreKitPurchaseResult extends Partial<StoreKitTransactionInput> {
  cancelled?: boolean;
  pending?: boolean;
  error?: string;
  productId?: string;
  transactionId?: string;
  originalTransactionId?: string;
  jwsRepresentation?: string | null;
  jws?: string | null;
  expiresDateEpoch?: number | null;
}

interface HostStoreKitBridge {
  purchaseProduct?: (productId: string) => Promise<string | HostStoreKitPurchaseResult>;
  restorePurchases?: () => Promise<string | StoreKitTransactionInput[]>;
  fetchProducts?: (productIds: string[]) => Promise<string | unknown[]>;
}

function getHostStoreKitBridge(): HostStoreKitBridge | undefined {
  return (globalThis as { __qiStoreKitBridge?: HostStoreKitBridge }).__qiStoreKitBridge;
}

async function executeNativeStoreKitPurchase(
  bridge: HostStoreKitBridge,
  productId: string,
  silent = false,
): Promise<boolean> {
  if (!bridge.purchaseProduct) {
    if (!silent) notify(tr("errors.iap_unavailable"));
    return false;
  }
  try {
    const raw = await bridge.purchaseProduct(productId);
    const parsed: HostStoreKitPurchaseResult =
      typeof raw === "string" ? (JSON.parse(raw) as HostStoreKitPurchaseResult) : raw;
    if (parsed.cancelled) {
      return false;
    }
    if (parsed.pending) {
      if (!silent) {
        notify(tr("toast.apple_verify_failed", { error: "StoreKit purchase pending approval" }));
      }
      return false;
    }
    if (parsed.error) {
      if (!silent) notify(tr("toast.apple_verify_failed", { error: parsed.error }));
      return false;
    }
    const resolvedProductId = (parsed.product_id ?? parsed.productId ?? productId).trim();
    const resolvedTxId = (parsed.transaction_id ?? parsed.transactionId ?? "").trim();
    const resolvedOrigTxId = (
      parsed.original_transaction_id ??
      parsed.originalTransactionId ??
      resolvedTxId
    ).trim();
    const resolvedEnv = (parsed.environment ?? "Production").trim();
    const resolvedJws = parsed.jws_representation ?? parsed.jwsRepresentation ?? parsed.jws ?? null;
    const resolvedExp = parsed.expires_date_epoch ?? parsed.expiresDateEpoch ?? null;
    return verifyAppleIapTransaction({
      productId: resolvedProductId,
      transactionId: resolvedTxId,
      originalTransactionId: resolvedOrigTxId,
      environment: resolvedEnv,
      jwsRepresentation: resolvedJws,
      expiresDateEpoch: resolvedExp,
      silent,
    });
  } catch (err) {
    if (!silent) {
      notify(tr("toast.apple_verify_failed", { error: errorMessage(err) }));
    }
    return false;
  }
}

export function purchaseHearthGiftPass(silent = false): boolean | Promise<boolean> {
  const bridge = getHostStoreKitBridge();
  if (bridge?.purchaseProduct) {
    return executeNativeStoreKitPurchase(bridge, HEARTH_GIFT_PASS_PRODUCT_ID, silent);
  }
  if (!isQaStudioEnabled()) {
    if (!silent) notify(tr("errors.iap_unavailable"));
    return false;
  }
  return verifyAppleIapTransaction({
    productId: HEARTH_GIFT_PASS_PRODUCT_ID,
    environment: "Sandbox",
    silent,
  });
}

export function confirmAtelierApplePurchase(
  tier: AtelierTierId,
  includeHearthGiftPass: boolean,
) {
  if (tier === "free") {
    clearEntitlementToken();
    if (includeHearthGiftPass) {
      void purchaseHearthGiftPass();
    }
    return;
  }

  const productId = resolveAppleProductIdForTier(tier);
  const bridge = getHostStoreKitBridge();
  if (bridge?.purchaseProduct && productId) {
    void (async () => {
      const ok = await executeNativeStoreKitPurchase(bridge, productId, false);
      if (ok && includeHearthGiftPass) {
        await executeNativeStoreKitPurchase(bridge, HEARTH_GIFT_PASS_PRODUCT_ID, true);
        notify(tr("toast.gift_pass_done"));
      }
    })();
    return;
  }

  if (!isQaStudioEnabled()) {
    notify(tr("errors.iap_unavailable"));
    return;
  }

  if (productId) {
    const ok = verifyAppleIapTransaction({
      productId,
      environment: "Sandbox",
      silent: false,
    });
    if (ok && includeHearthGiftPass) {
      purchaseHearthGiftPass(true);
      notify(tr("toast.gift_pass_done"));
    }
    return;
  }

  void activateCloudPro(tier, includeHearthGiftPass);
}

export async function restoreApplePurchases(explicitTransactionsJson?: string): Promise<boolean> {
  restoringPurchases.set(true);
  try {
    let payloadJson = explicitTransactionsJson;
    if (!payloadJson) {
      const hostBridge = getHostStoreKitBridge();
      if (hostBridge?.restorePurchases) {
        const res = await hostBridge.restorePurchases();
        payloadJson = typeof res === "string" ? res : JSON.stringify(res);
      } else if (isQaStudioEnabled()) {
        const fallbackList: StoreKitTransactionInput[] = [...recentAppleTransactions];
        const currentTier = uiState().entitlements.tier;
        const activeProductId = resolveAppleProductIdForTier(currentTier);
        if (
          activeProductId &&
          !fallbackList.some((tx) => tx.product_id === activeProductId)
        ) {
          const ts = Math.floor(Date.now() / 1000);
          fallbackList.push({
            product_id: activeProductId,
            transaction_id: `restore_${activeProductId}_${ts}`,
            original_transaction_id: `restore_${activeProductId}_${ts}`,
            environment: "Sandbox",
          });
        }
        payloadJson = JSON.stringify(fallbackList);
      } else {
        const msg = tr("errors.iap_unavailable");
        restoreStatusMessage.set(msg);
        notify(msg);
        return false;
      }
    }

    const parsedArr = JSON.parse(payloadJson) as unknown[];
    applyJson(engine.restoreAppleIapPurchases(payloadJson));
    if (Array.isArray(parsedArr) && parsedArr.length === 0) {
      const msg = tr("toast.restore_empty");
      restoreStatusMessage.set(msg);
      notify(msg);
      return false;
    }
    const msg = tr("toast.restore_done");
    restoreStatusMessage.set(msg);
    notify(msg);
    return true;
  } catch (err) {
    const msg = tr("toast.apple_verify_failed", { error: errorMessage(err) });
    restoreStatusMessage.set(msg);
    notify(msg);
    return false;
  } finally {
    restoringPurchases.set(false);
  }
}

export function claimSeasonalReward(rewardId: string) {
  try {
    applyJson(engine.claimSeasonalReward(rewardId));
    notify(tr("toast.reward_claimed"));
  } catch (err) {
    notify(errorMessage(err));
  }
}

export function claimRecoveryDew() {
  try {
    applyJson(engine.claimRecoveryDew());
    notify(tr("toast.recovery_dew_claimed"));
  } catch (err) {
    notify(errorMessage(err));
  }
}

export function setWeatherMood(mood: string) {
  applyJson(engine.setWeatherMood(mood));
  notify(tr("toast.weather_changed"));
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
    notify(tr("toast.seal_changed"));
  } catch (err) {
    notify(errorMessage(err));
  }
}

export function saveSanctuaryPreset(name: string) {
  try {
    applyJson(engine.saveSanctuaryPreset(name));
    notify(tr("toast.preset_saved", { name: name.trim() }));
  } catch (err) {
    notify(errorMessage(err));
  }
}

export function applySanctuaryPreset(presetId: number) {
  try {
    applyJson(engine.applySanctuaryPreset(presetId));
    notify(tr("toast.preset_applied"));
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
    notify(tr("toast.sticker_added", { title: title.trim() }));
  } catch (err) {
    notify(errorMessage(err));
  }
}

export function toggleStickerFavorite(stickerId: number) {
  applyJson(engine.toggleStickerFavorite(stickerId));
}

export function deleteFoodSticker(stickerId: number) {
  applyJson(engine.deleteFoodSticker(stickerId));
  notify(tr("toast.sticker_removed"));
}

export function coSanctuaryAction(action: string, partnerName?: string, pairCode?: string) {
  applyJson(engine.cosanctuaryAction(action, partnerName ?? null, pairCode ?? null));
  if (action === "warm_tea" || action === "tuck_blanket" || action === "paw_stamp") {
    notify(tr("toast.cosanctuary_sent"));
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
    notify(tr("toast.custom_experiment_started", { days: washoutDays }));
  } catch (err) {
    notify(errorMessage(err));
  }
}

export function generateParentsPeaceLetter(recipientLabel?: string) {
  const cleanLabel = recipientLabel?.trim() || tr("log.profile_parents_default");
  try {
    const raw = engine.generateParentsPeaceLetter(cleanLabel);
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
    const profileLabel =
      profileId === "partner"
        ? tr("log.profile_partner")
        : profileId === "parents"
          ? tr("log.profile_parents_default")
          : tr("log.profile_self");
    notify(tr("toast.marker_recorded", { profile: profileLabel, marker: markerName }));
  } catch (err) {
    notify(errorMessage(err));
  }
}

export function pullWebdavSync() {
  try {
    applyJson(engine.pullWebdavSync());
    notify(uiState().settings.last_sync_summary);
  } catch (err) {
    notify(tr("toast.webdav_hint", { error: errorMessage(err) }));
  }
}

// ---------- Custom Pet Companion v5.0 ----------

export function listCustomPets(): {
  pets: CustomPetCompanion[];
  active_pet: CustomPetCompanion | null;
  quota: CustomPetQuota;
  letters: PetWeeklyLetter[];
} | null {
  try {
    return JSON.parse(engine.listCustomPets()) as {
      pets: CustomPetCompanion[];
      active_pet: CustomPetCompanion | null;
      quota: CustomPetQuota;
      letters: PetWeeklyLetter[];
    };
  } catch {
    return null;
  }
}

export function upsertCustomPet(input: {
  name: string;
  species: string;
  persona: string;
  photoDataUrl: string;
  dieCutFrame: string;
  starlightMemorial: boolean;
  pawStampTitle: string;
  isActive: boolean;
  petId?: string | null;
}): boolean {
  const trimmedName = input.name.trim();
  if (!trimmedName) {
    notify(tr("errors.pet_name_required"));
    return false;
  }
  try {
    applyJson(
      engine.upsertCustomPet(
        trimmedName,
        input.species,
        input.persona.trim(),
        input.photoDataUrl.trim(),
        input.dieCutFrame,
        input.starlightMemorial,
        input.pawStampTitle.trim(),
        input.isActive,
        input.petId ?? null,
      ),
    );
    notify(tr("toast.pet_saved", { name: trimmedName }));
    return true;
  } catch (err) {
    notify(errorMessage(err));
    return false;
  }
}

export function setActiveCustomPet(petId: string, petName?: string): boolean {
  try {
    applyJson(engine.setActiveCustomPet(petId));
    const displayName = petName?.trim() || tr("pet.preset_xiaoman");
    notify(tr("toast.pet_activated", { name: displayName }));
    return true;
  } catch (err) {
    notify(errorMessage(err));
    return false;
  }
}

export function deleteCustomPet(petId: string, petName?: string): boolean {
  try {
    applyJson(engine.deleteCustomPet(petId));
    notify(tr("toast.pet_removed", { name: petName?.trim() || petId }));
    return true;
  } catch (err) {
    notify(errorMessage(err));
    return false;
  }
}

export function generatePetWeeklyLetter(
  petId: string,
  petName?: string,
  weekNumber?: number | null,
): boolean {
  try {
    applyJson(engine.generatePetWeeklyLetter(petId, weekNumber ?? null));
    notify(tr("toast.pet_letter_ready", { name: petName?.trim() || petId }));
    return true;
  } catch (err) {
    notify(errorMessage(err));
    return false;
  }
}

function uint8ArrayToBase64(bytes: Uint8Array): string {
  const alphabet = "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
  let out = "";
  for (let i = 0; i < bytes.length; i += 3) {
    const a = bytes[i] ?? 0;
    const b = bytes[i + 1] ?? 0;
    const c = bytes[i + 2] ?? 0;
    const triple = (a << 16) | (b << 8) | c;
    out += alphabet[(triple >> 18) & 63];
    out += alphabet[(triple >> 12) & 63];
    out += i + 1 < bytes.length ? alphabet[(triple >> 6) & 63] : "=";
    out += i + 2 < bytes.length ? alphabet[triple & 63] : "=";
  }
  return out;
}

export async function pickCustomPetPhotoDataUrl(): Promise<string | null> {
  try {
    const picked = await Picker.pickImages({ multiple: false, maxDimension: 512 });
    if (picked.length === 0) return null;
    const file = picked[0].file;
    const buf = await file.arrayBuffer();
    const bytes = new Uint8Array(buf);
    if (bytes.length > 360 * 1024) {
      notify(tr("toast.pet_photo_too_large"));
      return null;
    }
    const lowerName = file.name.toLowerCase();
    const mime =
      file.type && file.type.startsWith("image/") && file.type !== "image/svg+xml"
        ? file.type
        : lowerName.endsWith(".png")
          ? "image/png"
          : lowerName.endsWith(".webp")
            ? "image/webp"
            : lowerName.endsWith(".gif")
              ? "image/gif"
              : "image/jpeg";
    const dataUrl = `data:${mime};base64,${uint8ArrayToBase64(bytes)}`;
    notify(tr("toast.pet_photo_ready"));
    return dataUrl;
  } catch (err) {
    notify(errorMessage(err));
    return null;
  }
}
