import { Canvas, colorScheme, createMotion, Image, Scroll, signal, Text, token, View } from "nsh";
import { Button } from "nsh/base";
import catNightImg from "../assets/scenes/cat_night.jpg";
import fourSeasonsHandscrollImg from "../assets/scenes/four_seasons_handscroll.jpg";
import zenDayImg from "../assets/scenes/zen_day.jpg";
import { hapticSelection } from "./haptics";
import {
  ActionButton,
  Card,
  FieldInput,
  Hairline,
  MeterBar,
  ModalSurface,
  SectionHeader,
  SegmentedTabs,
  SelectChip,
  SheetDialog,
  SunkenBox,
  SwitchRow,
} from "./components";
import { isEnglishLocale, tr } from "./i18n";
import {
  BadgeCeramicIcon,
  CoSanctuaryIcon,
  CustomPetIcon,
  EnvelopeIcon,
  FamilyHeartIcon,
  HandscrollIcon,
  HearthEmberIcon,
  QiIcon,
  SolarTermIcon,
  WaxSealIcon,
  WidgetIcon,
} from "./icons";
import type { SealInkId, WeatherMood } from "./sanctuary";
import { getVisualThemeMeta, type VisualThemeId } from "./themes";

export type AtelierTierId =
  | "free"
  | "pro_monthly"
  | "seasonal"
  | "pro_annual"
  | "duo"
  | "family"
  | "lifetime_local"
  | string;

export interface AtelierPlanOption {
  tier: AtelierTierId;
  title: string;
  subtitle: string;
  priceCny: number;
  displayPrice?: string;
  originalPriceCny?: number | null;
  originalDisplayPrice?: string | null;
  billingCycle: string;
  seats: number;
  highlights: string[];
  badge: string;
  loyaltyApplied?: boolean;
}

export interface TransparentTrialStep {
  dayLabel: string;
  title: string;
  detail: string;
}

function getDefaultAtelierPlans(): AtelierPlanOption[] {
  return [
    {
      tier: "free",
      title: tr("atelier.plans.free.title"),
      subtitle: tr("atelier.plans.free.subtitle"),
      priceCny: 0,
      billingCycle: tr("atelier.plans.free.cadence"),
      seats: 1,
      highlights: [
        tr("atelier.plans.free.highlight_1"),
        tr("atelier.plans.free.highlight_2"),
        tr("atelier.plans.free.highlight_3"),
      ],
      badge: tr("atelier.plans.free.badge"),
    },
    {
      tier: "pro_monthly",
      title: tr("atelier.plans.pro_monthly.title"),
      subtitle: tr("atelier.plans.pro_monthly.subtitle"),
      priceCny: 18,
      billingCycle: tr("atelier.plans.pro_monthly.cadence"),
      seats: 1,
      highlights: [
        tr("atelier.plans.pro_monthly.highlight_1"),
        tr("atelier.plans.pro_monthly.highlight_2"),
        tr("atelier.plans.pro_monthly.highlight_3"),
      ],
      badge: tr("atelier.plans.pro_monthly.badge"),
    },
    {
      tier: "seasonal",
      title: tr("atelier.plans.seasonal.title"),
      subtitle: tr("atelier.plans.seasonal.subtitle"),
      priceCny: 38,
      originalPriceCny: 54,
      billingCycle: tr("atelier.plans.seasonal.cadence"),
      seats: 1,
      highlights: [
        tr("atelier.plans.seasonal.highlight_1"),
        tr("atelier.plans.seasonal.highlight_2"),
      ],
      badge: tr("atelier.plans.seasonal.badge"),
    },
    {
      tier: "pro_annual",
      title: tr("atelier.plans.pro_annual.title"),
      subtitle: tr("atelier.plans.pro_annual.subtitle"),
      priceCny: 98,
      originalPriceCny: 148,
      billingCycle: tr("atelier.plans.pro_annual.cadence"),
      seats: 1,
      highlights: [
        tr("atelier.plans.pro_annual.highlight_1"),
        tr("atelier.plans.pro_annual.highlight_2"),
      ],
      badge: tr("atelier.plans.pro_annual.badge"),
      loyaltyApplied: true,
    },
    {
      tier: "duo",
      title: tr("atelier.plans.duo.title"),
      subtitle: tr("atelier.plans.duo.subtitle"),
      priceCny: 168,
      originalPriceCny: 236,
      billingCycle: tr("atelier.plans.duo.cadence"),
      seats: 2,
      highlights: [
        tr("atelier.plans.duo.highlight_1"),
        tr("atelier.plans.duo.highlight_2"),
      ],
      badge: tr("atelier.plans.duo.badge"),
    },
    {
      tier: "family",
      title: tr("atelier.plans.family.title"),
      subtitle: tr("atelier.plans.family.subtitle"),
      priceCny: 228,
      originalPriceCny: 472,
      billingCycle: tr("atelier.plans.family.cadence"),
      seats: 4,
      highlights: [
        tr("atelier.plans.family.highlight_1"),
        tr("atelier.plans.family.highlight_2"),
      ],
      badge: tr("atelier.plans.family.badge"),
    },
    {
      tier: "lifetime_local",
      title: tr("atelier.plans.lifetime_local.title"),
      subtitle: tr("atelier.plans.lifetime_local.subtitle"),
      priceCny: 368,
      billingCycle: tr("atelier.plans.lifetime_local.cadence"),
      seats: 1,
      highlights: [
        tr("atelier.plans.lifetime_local.highlight_1"),
        tr("atelier.plans.lifetime_local.highlight_2"),
      ],
      badge: tr("atelier.plans.lifetime_local.badge"),
    },
  ];
}

function getDefaultTrialTimeline(): TransparentTrialStep[] {
  return [
    {
      dayLabel: tr("atelier.trial_day_1_label"),
      title: tr("atelier.trial_day_1"),
      detail: tr("atelier.trial_day_1_detail"),
    },
    {
      dayLabel: tr("atelier.trial_day_5_label"),
      title: tr("atelier.trial_day_5"),
      detail: tr("atelier.trial_day_5_detail"),
    },
    {
      dayLabel: tr("atelier.trial_day_7_label"),
      title: tr("atelier.trial_day_7"),
      detail: tr("atelier.trial_day_7_detail"),
    },
  ];
}

function getDefaultEthicalPromises(): string[] {
  return [
    tr("atelier.promise_no_dark"),
    tr("atelier.promise_calm_mode"),
    tr("atelier.promise_read_only"),
    tr("atelier.promise_local_first"),
  ];
}

/** 1. 温柔雅集订阅弹窗：零压力沟通 + 自定陪伴梯度 + 礼赠漂流席位券 + Apple 合规三联底栏 */
export function QiAtelierModal(props: {
  open: boolean;
  companionThemeId?: VisualThemeId;
  currentTier: AtelierTierId;
  selectedTier?: AtelierTierId;
  onSelectTier?: (tier: AtelierTierId) => void;
  guardianSponsor: boolean;
  onToggleGuardian: (enabled: boolean) => void;
  loyaltyDays?: number;
  offlineStateBanner?: string;
  plans?: AtelierPlanOption[];
  timelineSteps?: TransparentTrialStep[];
  ethicalPromises?: string[];
  onConfirm: (tier: AtelierTierId, guardianSponsor: boolean) => void;
  onClose: () => void;
  onRestorePurchases?: () => void | Promise<void>;
  restoringPurchases?: boolean;
  restoreStatusMessage?: string;
  hearthGiftCount?: number;
  onOpenTerms?: () => void;
  onOpenPrivacy?: () => void;
}) {
  const localSelected = signal<AtelierTierId>(
    props.selectedTier ?? (props.currentTier === "free" ? "pro_annual" : props.currentTier),
  );
  const showAllPlans = signal(false);
  const legalDrawer = signal<"terms" | "privacy" | null>(null);
  const localRestoreHint = signal<string>("");

  const activeTier = () => props.selectedTier ?? localSelected();
  const selectTier = (tier: AtelierTierId) => {
    localSelected.set(tier);
    props.onSelectTier?.(tier);
  };

  const themeMeta = () => getVisualThemeMeta(props.companionThemeId ?? "cat_window");
  const loyaltyDays = () => props.loyaltyDays ?? 30;
  const loyaltyEligible = () => loyaltyDays() >= 30;
  const plans = () => props.plans ?? getDefaultAtelierPlans();
  const timeline = () => props.timelineSteps ?? getDefaultTrialTimeline();
  const promises = () => props.ethicalPromises ?? getDefaultEthicalPromises();

  const displayedPlans = () => {
    const all = plans();
    if (showAllPlans() || all.length <= 3) return all;
    const featuredIds = new Set<AtelierTierId>(["pro_annual", "seasonal", "lifetime_local"]);
    if (props.currentTier !== "free") featuredIds.add(props.currentTier);
    const active = activeTier();
    if (active !== "free") featuredIds.add(active);
    return all.filter((p) => featuredIds.has(p.tier));
  };

  const selectedPlan = () => plans().find((p) => p.tier === activeTier()) ?? plans()[0];

  const handleRestorePress = async () => {
    localRestoreHint.set("");
    if (props.onRestorePurchases) {
      await props.onRestorePurchases();
      return;
    }
    localRestoreHint.set(tr("atelier.compliance.restore_empty"));
  };

  const toggleLegalDrawer = (section: "terms" | "privacy") => {
    legalDrawer.update((curr) => (curr === section ? null : section));
    if (section === "terms") {
      props.onOpenTerms?.();
    } else {
      props.onOpenPrivacy?.();
    }
  };

  const restoreFeedback = () => props.restoreStatusMessage ?? localRestoreHint();

  return (
    <ModalSurface open={props.open} presentation="pageSheet" onClose={props.onClose}>
      <Scroll
        class="flex-1"
        keyboardShouldPersistTaps="handled"
        contentStyle={{ paddingHorizontal: 18, paddingBottom: 32, gap: 14 }}
      >
        <View class="mx-auto w-full max-w-2xl gap-3.5">
          {/* 顶部标题与礼遇胶囊 */}
          <View class="flex-row items-center justify-between gap-2">
            <View class="flex-row items-center gap-2">
              <View class="size-9 items-center justify-center rounded-xl bg-atelier-soft">
                <SolarTermIcon size={18} color={token("atelier-accent")} />
              </View>
              <View class="gap-0.5">
                <Text role="header" class="text-lg font-bold text-foreground">
                  {tr("atelier.brand_title")}
                </Text>
                <Text class="text-xs text-muted-foreground">{tr("atelier.modal_subtitle")}</Text>
              </View>
            </View>
            <ActionButton
              title={tr("atelier.browse_btn")}
              size="sm"
              variant="secondary"
              shape="pill"
              onPress={props.onClose}
            />
          </View>

          {/* 伙伴轻声耳语 + 30天陪伴礼遇 */}
          <View class="gap-2.5 rounded-2xl border border-border/45 bg-card p-4 shadow-2xs">
            <View class="flex-row flex-wrap items-center justify-between gap-2">
              <View class="flex-row items-center gap-1.5">
                <QiIcon name="sparkles" size={14} color={token("atelier-accent")} />
                <Text class="text-xs font-bold text-foreground">
                  {tr("atelier.companion_note_title", { companionName: themeMeta().companionName })}
                </Text>
              </View>
              <View class="rounded-full bg-secondary px-2.5 py-0.5">
                <Text
                  class="text-[11px] font-semibold text-primary"
                  style={{ fontVariant: ["tabular-nums"] }}
                >
                  {loyaltyEligible()
                    ? tr("atelier.loyalty_eligible", { days: loyaltyDays() })
                    : tr("atelier.loyalty_progress", { days: loyaltyDays() })}
                </Text>
              </View>
            </View>
            <Text class="text-xs leading-relaxed text-muted-foreground">
              {tr("atelier.companion_note_body")}
            </Text>
            <If when={Boolean(props.offlineStateBanner)}>
              <View class="rounded-xl bg-surface-sunken px-3 py-1.5">
                <Text class="text-[11px] font-medium text-muted-foreground">{props.offlineStateBanner}</Text>
              </View>
            </If>
          </View>

          {/* 自定陪伴权益说明：1 位永久免费 vs Pro 3 位 + 爪印藏书票 + 每周来信 */}
          <View class="gap-2 rounded-2xl border border-border/45 bg-card p-4">
            <View class="flex-row items-center gap-2">
              <CustomPetIcon size={16} color={token("atelier-accent")} />
              <Text class="flex-1 text-xs font-bold text-foreground">
                {tr("atelier.pet_perk_banner_title")}
              </Text>
            </View>
            <Text class="text-[11px] leading-relaxed text-muted-foreground">
              {tr("atelier.pet_perk_banner_desc")}
            </Text>
            <View class="flex-row flex-wrap gap-1.5 pt-0.5">
              <View class="rounded-full bg-vital-prime-soft px-2.5 py-0.5">
                <Text class="text-[10px] font-semibold text-vital-prime">
                  {tr("atelier.free_forever_pill")}
                </Text>
              </View>
              <View class="rounded-full bg-secondary px-2.5 py-0.5">
                <Text class="text-[10px] font-semibold text-foreground">
                  {tr("atelier.pro_pets_pill")}
                </Text>
              </View>
            </View>
          </View>

          {/* 方案卡片（默认呈现 3 档核心方案，可按需展开全部） */}
          <View class="gap-2">
            <View class="flex-row flex-wrap items-center justify-between gap-2">
              <Text class="text-xs font-semibold text-muted-foreground">
                {showAllPlans()
                  ? tr("atelier.plans_section_label", { count: plans().length })
                  : isEnglishLocale()
                    ? `Featured ${displayedPlans().length} plans (${plans().length} total)`
                    : `精选 ${displayedPlans().length} 档方案（共 ${plans().length} 档）`}
              </Text>
              <If when={props.currentTier === "free" && !showAllPlans()}>
                <View class="rounded-full bg-vital-prime-soft px-2.5 py-0.5">
                  <Text class="text-[10px] font-semibold text-vital-prime">
                    {isEnglishLocale() ? "Current: Calm Core · Free forever" : "当前：素心清欢 · 永久免费"}
                  </Text>
                </View>
              </If>
            </View>
            <Each of={displayedPlans()} key={(p) => p.tier}>
              {(plan) => {
                const isSelected = () => activeTier() === plan().tier;
                const isCurrent = () => props.currentTier === plan().tier;
                return (
                  <Button
                    onPress={() => selectTier(plan().tier)}
                    class={`gap-2 rounded-2xl border p-3.5 transition-colors duration-150 focus-visible:ring-2 focus-visible:ring-ring motion-reduce:transition-none ${
                      isSelected()
                        ? "border-primary bg-card ring-1 ring-primary/20 shadow-2xs"
                        : "border-border/45 bg-card hover:border-border"
                    }`}
                  >
                    <View class="flex-row items-start justify-between gap-2">
                      <View class="flex-1 gap-1">
                        <View class="flex-row flex-wrap items-center gap-1.5">
                          <Text class="text-sm font-bold text-foreground">{plan().title}</Text>
                          <View class="rounded-full bg-secondary px-2 py-0.5">
                            <Text class="text-[10px] font-semibold text-primary">{plan().badge}</Text>
                          </View>
                          <If when={isCurrent()}>
                            <View class="rounded-full bg-vital-prime-soft px-2 py-0.5">
                              <Text class="text-[10px] font-semibold text-vital-prime">
                                {tr("atelier.current_plan_badge")}
                              </Text>
                            </View>
                          </If>
                        </View>
                        <Text class="text-xs leading-relaxed text-muted-foreground">{plan().subtitle}</Text>
                      </View>

                      <View class="items-end">
                        <View class="flex-row items-baseline gap-1">
                          <Text
                            class="text-lg font-bold text-foreground"
                            style={{ fontVariant: ["tabular-nums"] }}
                          >
                            {plan().displayPrice ?? `¥${plan().priceCny}`}
                          </Text>
                          <If when={Boolean(plan().originalDisplayPrice ?? plan().originalPriceCny)}>
                            <Text
                              class="text-[11px] text-muted-foreground line-through"
                              style={{ fontVariant: ["tabular-nums"] }}
                            >
                              {plan().originalDisplayPrice ?? `¥${plan().originalPriceCny}`}
                            </Text>
                          </If>
                        </View>
                        <Text class="text-[10px] text-muted-foreground">{plan().billingCycle}</Text>
                      </View>
                    </View>

                    <View class="gap-1 pt-0.5">
                      <Each of={plan().highlights} key={(h) => `${plan().tier}-${h}`}>
                        {(line) => (
                          <View class="flex-row items-center gap-1.5">
                            <QiIcon name="check" size={12} color={token("vital-prime")} />
                            <Text class="flex-1 text-[11px] text-foreground">{line()}</Text>
                          </View>
                        )}
                      </Each>
                    </View>
                  </Button>
                );
              }}
            </Each>
            <If when={plans().length > 3}>
              <Button
                hitSlop={4}
                onPress={() => {
                  hapticSelection();
                  showAllPlans.update((v) => !v);
                }}
                class="min-h-10 flex-row items-center justify-center gap-1.5 rounded-2xl border border-border/55 bg-surface-sunken px-3 py-2 active:opacity-80 focus-visible:ring-2 focus-visible:ring-ring"
              >
                <Text class="text-xs font-semibold text-foreground">
                  {showAllPlans()
                    ? isEnglishLocale()
                      ? "Show fewer plans"
                      : "收起其余方案"
                    : isEnglishLocale()
                      ? `Show all ${plans().length} plans (Monthly / Duo / Family)`
                      : `展开全部 ${plans().length} 档方案（含月度 / 双人 / 家庭）`}
                </Text>
                <QiIcon
                  name={showAllPlans() ? "chevron_down" : "chevron_right"}
                  size={13}
                  color={token("muted-foreground")}
                />
              </Button>
            </If>
          </View>

          {/* 商业消耗型内购：窗台添柴礼赠券 · 赠予社区漂流席位 (+¥12) */}
          <Card>
            <View class="flex-row flex-wrap items-center justify-between gap-2">
              <View class="flex-row items-center gap-2">
                <HearthEmberIcon size={16} color={token("wax-seal")} />
                <Text class="text-xs font-bold text-foreground">{tr("atelier.hearth_gift_title")}</Text>
              </View>
              <View class="rounded-full bg-secondary px-2 py-0.5">
                <Text class="text-[10px] font-semibold text-wax-seal">
                  {tr("atelier.hearth_gift_badge")}
                </Text>
              </View>
            </View>
            <SwitchRow
              title={tr("atelier.hearth_gift_switch_title")}
              subtitle={tr("atelier.hearth_gift_desc")}
              checked={props.guardianSponsor}
              onChange={props.onToggleGuardian}
            />
            <If when={typeof props.hearthGiftCount === "number" && props.hearthGiftCount > 0}>
              <Text class="text-[11px] font-medium text-muted-foreground" style={{ fontVariant: ["tabular-nums"] }}>
                {tr("atelier.hearth_gift_count", { count: props.hearthGiftCount ?? 0 })}
              </Text>
            </If>
          </Card>

          {/* 7 天透明体验时间轴 */}
          <Card>
            <SectionHeader
              title={tr("atelier.trial_section_title")}
              subtitle={tr("atelier.trial_section_subtitle")}
            />
            <View class="gap-2.5">
              <Each of={timeline()} key={(step) => step.dayLabel}>
                {(step, idx) => (
                  <View class="flex-row items-start gap-2.5">
                    <View class="size-6 items-center justify-center rounded-full bg-secondary">
                      <Text
                        class="text-[11px] font-bold text-primary"
                        style={{ fontVariant: ["tabular-nums"] }}
                      >
                        {`${idx() + 1}`}
                      </Text>
                    </View>
                    <View class="flex-1 gap-0.5">
                      <View class="flex-row items-center gap-2">
                        <Text class="text-xs font-bold text-foreground">{step().dayLabel}</Text>
                        <Text class="text-xs font-semibold text-primary">{step().title}</Text>
                      </View>
                      <Text class="text-[11px] leading-relaxed text-muted-foreground">{step().detail}</Text>
                    </View>
                  </View>
                )}
              </Each>
            </View>
          </Card>

          {/* 温和商业承诺 */}
          <SunkenBox>
            <View class="flex-row items-center gap-1.5">
              <QiIcon name="user_shield" size={14} color={token("vital-prime")} />
              <Text class="text-xs font-bold text-vital-prime">{tr("atelier.promises_section_title")}</Text>
            </View>
            <Each of={promises()} key={(p) => p}>
              {(item) => (
                <View class="flex-row items-start gap-1.5">
                  <QiIcon name="check" size={12} color={token("vital-prime")} />
                  <Text class="flex-1 text-[11px] leading-relaxed text-muted-foreground">{item()}</Text>
                </View>
              )}
            </Each>
          </SunkenBox>

          {/* 确认 CTA */}
          <View class="gap-2 pt-1">
            <ActionButton
              title={
                activeTier() === "free"
                  ? tr("atelier.confirm_cta_free")
                  : props.guardianSponsor
                    ? tr("atelier.confirm_cta_with_gift", { title: selectedPlan().title })
                    : tr("atelier.confirm_cta_subscribe", { title: selectedPlan().title })
              }
              icon={activeTier() === "free" ? "tea_cup" : "wax_seal"}
              size="lg"
              onPress={() => props.onConfirm(activeTier(), props.guardianSponsor)}
            />
            <ActionButton
              title={tr("atelier.dismiss_btn")}
              variant="ghost"
              size="sm"
              onPress={props.onClose}
            />
          </View>

          <Hairline />

          {/* Apple App Store 合规三联底栏：恢复购买 + 24小时自动续订声明 + 可展开 EULA/隐私政策 */}
          <View class="gap-2.5 rounded-2xl bg-surface-sunken p-4">
            {/* 1. Restore purchases */}
            <View class="flex-row flex-wrap items-center justify-between gap-2">
              <Text class="flex-1 text-[11px] text-muted-foreground">
                {tr("atelier.compliance.restore_prompt")}
              </Text>
              <ActionButton
                title={
                  props.restoringPurchases
                    ? tr("atelier.compliance.restoring")
                    : tr("atelier.compliance.restore_purchases")
                }
                icon="restore_arrow"
                size="sm"
                variant="outline"
                disabled={Boolean(props.restoringPurchases)}
                onPress={handleRestorePress}
              />
            </View>

            <If when={Boolean(restoreFeedback())}>
              <View class="rounded-xl bg-card px-3 py-2">
                <Text class="text-[11px] font-medium text-foreground">{restoreFeedback()}</Text>
              </View>
            </If>

            {/* 2. 24-hour auto-renewal disclosure */}
            <Text class="text-[10px] leading-relaxed text-muted-foreground">
              {tr("atelier.compliance.auto_renewal_disclosure")}
            </Text>

            {/* 3. Terms of Service (EULA) & Privacy Policy */}
            <View class="flex-row flex-wrap items-center justify-center gap-3 pt-0.5">
              <Button
                onPress={() => toggleLegalDrawer("terms")}
                class="rounded-lg px-2 py-1 focus-visible:ring-2 focus-visible:ring-ring"
              >
                <Text class="text-[11px] font-semibold text-primary underline">
                  {tr("atelier.compliance.terms_link")}
                </Text>
              </Button>
              <Text class="text-[11px] text-muted-foreground">·</Text>
              <Button
                onPress={() => toggleLegalDrawer("privacy")}
                class="rounded-lg px-2 py-1 focus-visible:ring-2 focus-visible:ring-ring"
              >
                <Text class="text-[11px] font-semibold text-primary underline">
                  {tr("atelier.compliance.privacy_link")}
                </Text>
              </Button>
            </View>

            {/* Inline readable Terms / Privacy drawer inside the modal */}
            <If when={legalDrawer() !== null}>
              <View class="gap-2 rounded-xl bg-card p-3.5 shadow-2xs">
                <View class="flex-row items-center justify-between gap-2">
                  <Text class="flex-1 text-xs font-bold text-foreground">
                    {legalDrawer() === "terms"
                      ? tr("atelier.compliance.terms_title")
                      : tr("atelier.compliance.privacy_title")}
                  </Text>
                  <ActionButton
                    title={tr("atelier.compliance.hide_drawer")}
                    size="sm"
                    variant="ghost"
                    onPress={() => legalDrawer.set(null)}
                  />
                </View>
                <Text class="text-[11px] leading-relaxed text-muted-foreground">
                  {legalDrawer() === "terms"
                    ? tr("atelier.compliance.terms_body")
                    : tr("atelier.compliance.privacy_body")}
                </Text>
              </View>
            </If>
          </View>
        </View>
      </Scroll>
    </ModalSurface>
  );
}

export interface SeasonalPassNode {
  id: string;
  title: string;
  track: "free" | "atelier";
  dewCost: number;
  claimed: boolean;
  description?: string;
}

export interface SeasonalArchiveItem {
  solarTerm: string;
  title: string;
  badge: string;
  unlocked: boolean;
}

/** 2. 岁时风物季票面板：清欢/雅集双轨 + 晨露余额 + 往期节气博物架 + 服饰/印泥/天象/3格陈列 */
export function SeasonalPassPanel(props: {
  solarTerm: string;
  solarSubtitle?: string;
  dewBalance: number;
  atelierUnlocked: boolean;
  nodes: SeasonalPassNode[];
  archiveItems?: SeasonalArchiveItem[];
  activeOutfit?: string;
  outfits?: { id: string; label: string; atelierOnly?: boolean }[];
  onSelectOutfit?: (id: string) => void;
  activeSealInk?: SealInkId;
  onSelectSealInk?: (ink: SealInkId) => void;
  weatherMood?: WeatherMood;
  onSelectWeatherMood?: (mood: WeatherMood) => void;
  equippedKeepsakes?: string[];
  availableKeepsakes?: { id: string; name: string; unlocked: boolean }[];
  onToggleEquipKeepsake?: (id: string) => void;
  onClaim: (nodeId: string) => void;
  onOpenAtelier?: () => void;
}) {
  const showArchiveAndDecor = signal<boolean>(false);

  const weatherOptions = (): { id: WeatherMood; label: string }[] => [
    { id: "sunny_crisp", label: tr("atelier.seasonal.weather_sunny_crisp") },
    { id: "misty_rain", label: tr("atelier.seasonal.weather_misty_rain") },
    { id: "first_snow", label: tr("atelier.seasonal.weather_first_snow") },
    { id: "clear_moon", label: tr("atelier.seasonal.weather_clear_moon") },
  ];

  const sealOptions = (): { id: SealInkId; label: string }[] => [
    { id: "cinnabar", label: tr("atelier.seasonal.seal_cinnabar") },
    { id: "gold", label: tr("atelier.seasonal.seal_gold") },
    { id: "lapis", label: tr("atelier.seasonal.seal_lapis") },
  ];

  const freeNodes = () => props.nodes.filter((n) => n.track === "free");
  const atelierNodes = () => props.nodes.filter((n) => n.track === "atelier");

  return (
    <Card>
      <SectionHeader
        title={tr("atelier.seasonal.panel_title", { solarTerm: props.solarTerm })}
        subtitle={props.solarSubtitle ?? tr("atelier.seasonal.panel_subtitle")}
        right={
          <View class="flex-row items-center gap-1 rounded-full bg-dew-soft px-3 py-1">
            <QiIcon name="droplet" size={11} color={token("dew-accent")} />
            <Text
              class="text-xs font-bold text-dew-accent"
              style={{ fontVariant: ["tabular-nums"] }}
            >
              {`${props.dewBalance}`}
            </Text>
          </View>
        }
      />

      {/* 双轨奖励：清欢轨 vs 雅集轨 */}
      <View class="gap-3">
        <View class="gap-2">
          <Text class="text-xs font-semibold text-vital-prime">{tr("atelier.seasonal.free_track")}</Text>
          <View class="flex-row flex-wrap gap-2">
            <Each of={freeNodes()} key={(n) => n.id}>
              {(node) => (
                <View class="min-w-40 flex-1 gap-1.5 rounded-xl border border-border/45 bg-card p-3">
                  <View class="flex-row items-center justify-between gap-1">
                    <Text class="text-xs font-bold text-foreground">{node().title}</Text>
                    <View class="flex-row items-center gap-1">
                      <QiIcon name="droplet" size={11} color={token("dew-accent")} />
                      <Text
                        class="text-[11px] font-semibold text-dew-accent"
                        style={{ fontVariant: ["tabular-nums"] }}
                      >
                        {`${node().dewCost}`}
                      </Text>
                    </View>
                  </View>
                  <If when={Boolean(node().description)}>
                    <Text class="text-[11px] text-muted-foreground">{node().description}</Text>
                  </If>
                  <ActionButton
                    title={
                      node().claimed
                        ? tr("atelier.seasonal.claimed_free")
                        : tr("atelier.seasonal.claim_free")
                    }
                    size="sm"
                    variant={node().claimed ? "secondary" : "primary"}
                    disabled={node().claimed || props.dewBalance < node().dewCost}
                    onPress={() => props.onClaim(node().id)}
                  />
                </View>
              )}
            </Each>
          </View>
        </View>

        <View class="gap-2">
          <View class="flex-row items-center justify-between">
            <Text class="text-xs font-semibold text-primary">{tr("atelier.seasonal.pro_track")}</Text>
            <If when={!props.atelierUnlocked && Boolean(props.onOpenAtelier)}>
              <ActionButton
                title={tr("atelier.seasonal.learn_atelier")}
                size="sm"
                variant="outline"
                shape="pill"
                onPress={() => props.onOpenAtelier?.()}
              />
            </If>
          </View>
          <View class="flex-row flex-wrap gap-2">
            <Each of={atelierNodes()} key={(n) => n.id}>
              {(node) => (
                <View
                  class={`min-w-40 flex-1 gap-1.5 rounded-xl p-3 ${
                    props.atelierUnlocked
                      ? "border border-border/45 bg-card"
                      : "bg-surface-sunken opacity-85"
                  }`}
                >
                  <View class="flex-row items-center justify-between gap-1">
                    <Text class="text-xs font-bold text-foreground">{node().title}</Text>
                    <View class="flex-row items-center gap-1">
                      <QiIcon name="droplet" size={11} color={token("dew-accent")} />
                      <Text
                        class="text-[11px] font-semibold text-atelier-accent"
                        style={{ fontVariant: ["tabular-nums"] }}
                      >
                        {`${node().dewCost}`}
                      </Text>
                    </View>
                  </View>
                  <If when={Boolean(node().description)}>
                    <Text class="text-[11px] text-muted-foreground">{node().description}</Text>
                  </If>
                  <ActionButton
                    title={
                      !props.atelierUnlocked
                        ? tr("atelier.seasonal.unlock_pro")
                        : node().claimed
                          ? tr("atelier.seasonal.claimed_pro")
                          : tr("atelier.seasonal.claim_pro")
                    }
                    size="sm"
                    variant={node().claimed ? "secondary" : "outline"}
                    disabled={
                      props.atelierUnlocked && (node().claimed || props.dewBalance < node().dewCost)
                    }
                    onPress={() => {
                      if (!props.atelierUnlocked) props.onOpenAtelier?.();
                      else props.onClaim(node().id);
                    }}
                  />
                </View>
              )}
            </Each>
          </View>
        </View>
      </View>

      {/* 往期节气博物架与风物装扮折叠切换 */}
      <If
        when={
          Boolean(props.onSelectWeatherMood) ||
          Boolean(props.outfits && props.outfits.length > 0 && props.onSelectOutfit) ||
          Boolean(props.onSelectSealInk) ||
          Boolean(props.availableKeepsakes && props.onToggleEquipKeepsake) ||
          Boolean(props.archiveItems && props.archiveItems.length > 0)
        }
      >
        <View class="gap-3 pt-1">
          <ActionButton
            title={
              showArchiveAndDecor()
                ? tr("atelier.seasonal.archive_collapse")
                : isEnglishLocale()
                  ? "Past seasonal keepsakes & decor"
                  : tr("atelier.seasonal.archive_expand")
            }
            icon="handscroll"
            size="sm"
            variant="secondary"
            onPress={() => showArchiveAndDecor.update((v) => !v)}
          />

          <If when={showArchiveAndDecor()}>
            <View class="gap-3">
              {/* 窗台天象、服饰、印泥与 3 格摆件槽位 */}
              <If when={Boolean(props.onSelectWeatherMood)}>
                <View class="gap-1.5">
                  <Text class="text-xs font-semibold text-foreground">{tr("atelier.seasonal.weather_label")}</Text>
                  <View class="flex-row flex-wrap gap-1.5">
                    <Each of={weatherOptions()} key={(w) => w.id}>
                      {(w) => (
                        <SelectChip
                          label={w().label}
                          selected={(props.weatherMood ?? "sunny_crisp") === w().id}
                          onPress={() => props.onSelectWeatherMood?.(w().id)}
                        />
                      )}
                    </Each>
                  </View>
                </View>
              </If>

              <If when={Boolean(props.outfits && props.outfits.length > 0 && props.onSelectOutfit)}>
                <View class="gap-1.5">
                  <Text class="text-xs font-semibold text-foreground">{tr("atelier.seasonal.outfit_label")}</Text>
                  <View class="flex-row flex-wrap gap-1.5">
                    <Each of={props.outfits ?? []} key={(o) => o.id}>
                      {(o) => (
                        <SelectChip
                          label={
                            o().atelierOnly && !props.atelierUnlocked
                              ? tr("atelier.seasonal.outfit_atelier_suffix", { label: o().label })
                              : o().label
                          }
                          selected={props.activeOutfit === o().id}
                          onPress={() => props.onSelectOutfit?.(o().id)}
                        />
                      )}
                    </Each>
                  </View>
                </View>
              </If>

              <If when={Boolean(props.onSelectSealInk)}>
                <View class="gap-1.5">
                  <Text class="text-xs font-semibold text-foreground">{tr("atelier.seasonal.seal_ink_label")}</Text>
                  <View class="flex-row flex-wrap gap-1.5">
                    <Each of={sealOptions()} key={(s) => s.id}>
                      {(s) => (
                        <SelectChip
                          label={s().label}
                          selected={(props.activeSealInk ?? "cinnabar") === s().id}
                          onPress={() => props.onSelectSealInk?.(s().id)}
                        />
                      )}
                    </Each>
                  </View>
                </View>
              </If>

              <If when={Boolean(props.availableKeepsakes && props.onToggleEquipKeepsake)}>
                <View class="gap-1.5">
                  <View class="flex-row items-center justify-between">
                    <Text class="text-xs font-semibold text-foreground">
                      {tr("atelier.seasonal.keepsakes_display_label")}
                    </Text>
                    <Text
                      class="text-[11px] text-muted-foreground"
                      style={{ fontVariant: ["tabular-nums"] }}
                    >
                      {tr("atelier.seasonal.keepsakes_equipped_count", {
                        count: (props.equippedKeepsakes ?? []).length,
                      })}
                    </Text>
                  </View>
                  <View class="flex-row flex-wrap gap-1.5">
                    <Each of={props.availableKeepsakes ?? []} key={(k) => k.id}>
                      {(k) => (
                        <SelectChip
                          label={k().name}
                          selected={(props.equippedKeepsakes ?? []).includes(k().id)}
                          disabled={!k().unlocked}
                          onPress={() => props.onToggleEquipKeepsake?.(k().id)}
                        />
                      )}
                    </Each>
                  </View>
                </View>
              </If>

              <If when={Boolean(props.archiveItems && props.archiveItems.length > 0)}>
                <View class="flex-row flex-wrap gap-2 rounded-xl bg-surface-sunken p-3">
                  <Each of={props.archiveItems ?? []} key={(a) => `${a.solarTerm}-${a.title}`}>
                    {(item) => (
                      <View class="min-w-36 flex-1 gap-1 rounded-xl bg-card p-2.5 shadow-2xs">
                        <View class="flex-row items-center justify-between">
                          <Text class="text-xs font-bold text-foreground">{item().solarTerm}</Text>
                          <Text class="text-[10px] font-semibold text-primary">
                            {item().unlocked ? item().badge : tr("atelier.seasonal.archive_claimable")}
                          </Text>
                        </View>
                        <Text class="text-[11px] text-muted-foreground">{item().title}</Text>
                      </View>
                    )}
                  </Each>
                </View>
              </If>
            </View>
          </If>
        </View>
      </If>
    </Card>
  );
}

export type StickerFrameStyle = "watercolor" | "polaroid" | "celadon";

export interface FoodStickerItem {
  id: string;
  date: string;
  dayNumber: number;
  title: string;
  frameStyle: StickerFrameStyle;
  purineHint?: string;
  sodiumHint?: string;
  kcal?: number;
}

/** 3. 模切食光贴纸造物机 & 人间烟火月历垫 */
export function StickerCameraMatPanel(props: {
  stickers: FoodStickerItem[];
  monthlyQuotaUsed: number;
  monthlyQuotaLimit?: number;
  atelierUnlocked: boolean;
  onCreateSticker: (input: {
    title: string;
    frameStyle: StickerFrameStyle;
    purineHint?: string;
  }) => void;
  onOpenAtelier?: () => void;
}) {
  const draftTitle = signal<string>("");
  const draftFrame = signal<StickerFrameStyle>("watercolor");
  const draftHint = signal<string>("");
  const limit = () => props.monthlyQuotaLimit ?? 5;
  const quotaReached = () => !props.atelierUnlocked && props.monthlyQuotaUsed >= limit();

  const resolvedTitle = () => draftTitle().trim() || tr("atelier.camera_mat.default_title");
  const resolvedHint = () => draftHint().trim() || tr("atelier.camera_mat.default_hint");

  const frameStyles = (): { id: StickerFrameStyle; label: string }[] => [
    { id: "watercolor", label: tr("atelier.camera_mat.tpl_watercolor") },
    { id: "polaroid", label: tr("atelier.camera_mat.tpl_polaroid") },
    { id: "celadon", label: tr("atelier.camera_mat.tpl_celadon") },
  ];

  const calendarRows = [0, 7, 14, 21];
  const calendarCols = [0, 1, 2, 3, 4, 5, 6];

  const submitSticker = () => {
    if (quotaReached()) {
      props.onOpenAtelier?.();
      return;
    }
    const trimmed = draftTitle().trim() || tr("atelier.camera_mat.create_fallback");
    props.onCreateSticker({
      title: trimmed,
      frameStyle: draftFrame(),
      purineHint: resolvedHint(),
    });
  };

  return (
    <Card>
      <SectionHeader
        title={tr("atelier.camera_mat.title")}
        right={
          <View class="rounded-full bg-secondary px-2.5 py-1">
            <Text
              class={`text-[11px] font-semibold ${
                props.atelierUnlocked ? "text-primary" : "text-foreground"
              }`}
              style={{ fontVariant: ["tabular-nums"] }}
            >
              {props.atelierUnlocked
                ? tr("atelier.camera_mat.unlimited")
                : tr("atelier.camera_mat.monthly_free", {
                    used: props.monthlyQuotaUsed,
                    limit: limit(),
                  })}
            </Text>
          </View>
        }
      />

      {/* Canvas 实时模切贴纸工作台：96x96 预览 + 标题输入/贴士，独立成行的 3 模切样式 */}
      <View class="gap-2.5 rounded-2xl bg-surface-sunken p-3">
        <View class="flex-row items-center gap-3">
          <Canvas
            style={{ width: 96, height: 96 }}
            draw={(cx, size) => {
              const w = size.width;
              const h = size.height;
              const style = draftFrame();
              const isEn = isEnglishLocale();
              cx.clearRect(0, 0, w, h);

              // 外层模切白边
              cx.save();
              cx.fillStyle = token("card");
              cx.strokeStyle =
                style === "celadon"
                  ? token("primary")
                  : style === "watercolor"
                    ? token("atelier-accent")
                    : token("border");
              cx.lineWidth = 2;
              cx.beginPath();
              cx.roundRect(4, 4, w - 8, h - 8, style === "polaroid" ? 8 : 16);
              cx.fill();
              cx.stroke();
              cx.restore();

              // 内框美食画布区
              cx.save();
              cx.fillStyle =
                style === "celadon"
                  ? token("bento-sage")
                  : style === "watercolor"
                    ? token("bento-peach")
                    : token("bento-amber");
              const innerH = style === "polaroid" ? h - 32 : h - 22;
              cx.beginPath();
              cx.roundRect(10, 10, w - 20, innerH, style === "polaroid" ? 4 : 10);
              cx.fill();
              cx.restore();

              // 中心食物器皿矢量简绘
              const plateY = style === "polaroid" ? h * 0.39 : h * 0.45;
              cx.save();
              cx.strokeStyle = token("primary");
              cx.lineWidth = 1.8;
              cx.beginPath();
              cx.arc(w / 2, plateY, 14, 0, Math.PI * 2);
              cx.stroke();
              cx.beginPath();
              cx.arc(w / 2, plateY, 8.5, 0.2, Math.PI * 1.4);
              cx.stroke();
              cx.restore();

              // 底部手写标签
              cx.save();
              cx.fillStyle = token("foreground");
              cx.font = isEn ? "600 9px system-ui, sans-serif" : '600 9px "PingFang SC", sans-serif';
              cx.textAlign = "center";
              cx.textBaseline = "middle";
              const maxChars = isEn ? 11 : 6;
              const rawLabel =
                (resolvedTitle() || tr("atelier.camera_mat.canvas_fallback")).split("·")[0].trim() ||
                tr("atelier.camera_mat.canvas_fallback");
              const shortTitle = rawLabel.slice(0, maxChars).replace(/[·\s,，、]+$/u, "");
              cx.fillText(shortTitle, w / 2, h - 13);
              cx.restore();
            }}
          />

          <View class="min-w-0 flex-1 gap-1.5">
            <FieldInput
              value={draftTitle()}
              placeholder={tr("atelier.camera_mat.input_placeholder")}
              onChange={(val) => draftTitle.set(val)}
              onSubmit={submitSticker}
            />
            <Text class="line-clamp-1 text-[11px] text-muted-foreground">{resolvedHint()}</Text>
          </View>
        </View>

        <SegmentedTabs
          items={frameStyles()}
          activeId={draftFrame()}
          onSelect={(id) => draftFrame.set(id)}
          size="sm"
        />

        <ActionButton
          title={
            quotaReached()
              ? tr("atelier.camera_mat.cta_quota_reached", { limit: limit() })
              : tr("atelier.camera_mat.cta_create")
          }
          icon="camera"
          size="sm"
          variant={quotaReached() ? "outline" : "primary"}
          onPress={submitSticker}
        />
      </View>

      {/* 人间烟火月历垫网格（7 列 × 4 周严格对齐） */}
      <View class="gap-1.5 rounded-2xl bg-surface-sunken p-2.5">
        <View class="flex-row items-center justify-between">
          <Text class="text-xs font-bold text-foreground">{tr("atelier.camera_mat.calendar_title")}</Text>
          <Text class="text-[11px] text-muted-foreground" style={{ fontVariant: ["tabular-nums"] }}>
            {tr("atelier.camera_mat.calendar_count", { count: props.stickers.length })}
          </Text>
        </View>
        <View class="gap-1.5">
          <Each of={calendarRows} key={(rowStart) => `cal-row-${rowStart}`}>
            {(rowStart) => (
              <View class="flex-row gap-1.5">
                <Each of={calendarCols} key={(colIdx) => `cal-cell-${rowStart() + colIdx + 1}`}>
                  {(colIdx) => {
                    const day = () => rowStart() + colIdx() + 1;
                    const matched = () => props.stickers.filter((s) => s.dayNumber === day());
                    return (
                      <View
                        class={`h-10 flex-1 items-center justify-center rounded-xl ${
                          matched().length > 0 ? "bg-card shadow-2xs" : "bg-card/55"
                        }`}
                      >
                        <Text
                          class="text-[10px] font-semibold text-muted-foreground"
                          style={{ fontVariant: ["tabular-nums"] }}
                        >
                          {`${day()}`}
                        </Text>
                        <If when={matched().length > 0}>
                          <View class="mt-0.5 rounded-full bg-secondary px-1.5">
                            <Text class="text-[9px] font-bold text-primary">
                              {tr("atelier.camera_mat.day_sticker_badge", {
                                count: matched().length,
                              })}
                            </Text>
                          </View>
                        </If>
                      </View>
                    );
                  }}
                </Each>
              </View>
            )}
          </Each>
        </View>
      </View>
    </Card>
  );
}

export interface CompanionLetterItem {
  weekIndex: number;
  title: string;
  solarTerm: string;
  dateLabel: string;
  body: string;
  exLibrisTitle: string;
  exLibrisQuote: string;
  unlocked: boolean;
}

/** 4. 《周日夜话慢递》火漆信封与藏书票读信弹窗 */
export function CompanionLetterModal(props: {
  open: boolean;
  letters: CompanionLetterItem[];
  selectedWeek?: number;
  onSelectWeek?: (weekIndex: number) => void;
  atelierUnlocked: boolean;
  onOpenAtelier?: () => void;
  onClose: () => void;
}) {
  const localWeek = signal<number>(props.selectedWeek ?? 1);
  const sealMotion = createMotion();
  const isUnsealed = signal<boolean>(true);

  const activeWeek = () => props.selectedWeek ?? localWeek();
  const currentLetter = () =>
    props.letters.find((l) => l.weekIndex === activeWeek()) ?? props.letters[0] ?? null;

  const chooseWeek = (week: number) => {
    localWeek.set(week);
    props.onSelectWeek?.(week);
  };

  return (
    <ModalSurface open={props.open} presentation="pageSheet" onClose={props.onClose}>
      <Scroll
        class="flex-1"
        keyboardShouldPersistTaps="handled"
        contentStyle={{ paddingHorizontal: 18, paddingBottom: 28, gap: 14 }}
      >
        <View class="mx-auto w-full max-w-xl gap-4">
          <View class="flex-row items-center justify-between">
            <View class="flex-row items-center gap-2">
              <EnvelopeIcon size={18} color={token("wax-seal")} />
              <Text role="header" class="text-lg font-bold text-foreground">
                {tr("atelier.letter.title")}
              </Text>
            </View>
            <ActionButton
              title={tr("atelier.letter.keep_btn")}
              size="sm"
              variant="secondary"
              shape="pill"
              onPress={props.onClose}
            />
          </View>

          {/* 周次选择器 */}
          <View class="flex-row flex-wrap gap-1.5">
            <Each of={props.letters} key={(l) => `week-${l.weekIndex}`}>
              {(letter) => (
                <SelectChip
                  label={tr("atelier.letter.week_chip", {
                    week: letter().weekIndex,
                    solarTerm: letter().solarTerm,
                  })}
                  selected={activeWeek() === letter().weekIndex}
                  onPress={() => chooseWeek(letter().weekIndex)}
                />
              )}
            </Each>
          </View>

          <If when={currentLetter()}>
            {(letter) => (
              <View class="gap-3.5">
                {/* 火漆信封封面 */}
                <View class="items-center gap-2.5 rounded-2xl bg-surface-sunken p-5">
                  <View style={sealMotion.style}>
                    <Button
                      hitSlop={4}
                      onPress={() => {
                        void sealMotion.play("popIn");
                        isUnsealed.update((v) => !v);
                      }}
                      class="size-14 items-center justify-center rounded-full border border-border/45 bg-card shadow-xs"
                    >
                      <WaxSealIcon size={26} color={token("wax-seal")} />
                    </Button>
                  </View>
                  <Text class="text-sm font-bold text-foreground">{letter().title}</Text>
                  <Text class="text-xs text-muted-foreground">
                    {tr("atelier.letter.date_solar", {
                      date: letter().dateLabel,
                      solarTerm: letter().solarTerm,
                    })}
                  </Text>
                </View>

                <If
                  when={letter().unlocked || letter().weekIndex === 1 || props.atelierUnlocked}
                  else={
                    <Card class="items-center gap-3 py-6">
                      <Text class="text-sm font-semibold text-foreground">
                        {tr("atelier.letter.locked_title")}
                      </Text>
                      <Text class="text-xs text-muted-foreground">{tr("atelier.letter.locked_desc")}</Text>
                      <If when={Boolean(props.onOpenAtelier)}>
                        <ActionButton
                          title={tr("atelier.letter.unlock_btn")}
                          icon="wax_seal"
                          onPress={() => props.onOpenAtelier?.()}
                        />
                      </If>
                    </Card>
                  }
                >
                  <If when={isUnsealed()}>
                    {/* 暖米色信纸正文 */}
                    <View class="gap-3 rounded-2xl border border-border/45 bg-card p-5 shadow-2xs">
                      <Text class="text-sm leading-loose text-foreground">{letter().body}</Text>
                    </View>

                    {/* 底部节气藏书票 */}
                    <View class="flex-row items-center gap-3 rounded-2xl border border-border/45 bg-card p-3.5">
                      <View class="size-11 items-center justify-center rounded-xl bg-surface-sunken">
                        <BadgeCeramicIcon size={22} color={token("ink-gold")} />
                      </View>
                      <View class="flex-1 gap-0.5">
                        <Text class="text-xs font-bold text-foreground">
                          {tr("atelier.letter.ex_libris_prefix", { title: letter().exLibrisTitle })}
                        </Text>
                        <Text class="text-xs italic text-muted-foreground">{letter().exLibrisQuote}</Text>
                      </View>
                    </View>
                  </If>
                </If>
              </View>
            )}
          </If>
        </View>
      </Scroll>
    </ModalSurface>
  );
}

/** 5. 隐私优先《同窗共养》抽屉：1字节心情 + 互助动作 + 金桂盆栽 + 绝不共享声明 */
export function CoSanctuarySheet(props: {
  open: boolean;
  myMood: "sunny_full" | "breeze_steady" | "rest_recharge" | string;
  onSelectMood: (mood: "sunny_full" | "breeze_steady" | "rest_recharge") => void;
  partnerName?: string;
  partnerMood?: string;
  osmanthusLevel: number;
  osmanthusProgressPct: number;
  careHistory?: { id: string; label: string; timeLabel: string }[];
  onSendCare: (action: "warm_tea" | "blanket" | "paw_print") => void;
  onClose: () => void;
}) {
  const moodOptions = (): { id: "sunny_full" | "breeze_steady" | "rest_recharge"; label: string }[] => [
    { id: "sunny_full", label: tr("atelier.co_sanctuary.mood_sunny_full") },
    { id: "breeze_steady", label: tr("atelier.co_sanctuary.mood_breeze_steady") },
    { id: "rest_recharge", label: tr("atelier.co_sanctuary.mood_rest_recharge") },
  ];

  const careActions = (): { id: "warm_tea" | "blanket" | "paw_print"; label: string }[] => [
    { id: "warm_tea", label: tr("atelier.co_sanctuary.care_warm_tea") },
    { id: "blanket", label: tr("atelier.co_sanctuary.care_blanket") },
    { id: "paw_print", label: tr("atelier.co_sanctuary.care_paw_print") },
  ];

  return (
    <SheetDialog open={props.open} title={tr("atelier.co_sanctuary.title")} onClose={props.onClose}>
      {/* 隐私信任盾牌 */}
      <View class="flex-row items-center gap-2 rounded-2xl bg-vital-prime-soft px-3.5 py-2.5">
        <QiIcon name="user_shield" size={16} color={token("vital-prime")} />
        <Text class="flex-1 text-xs font-semibold text-vital-prime">
          {tr("atelier.co_sanctuary.privacy_banner")}
        </Text>
      </View>

      {/* 我的 1 字节心情状态 */}
      <Card>
        <SectionHeader
          title={tr("atelier.co_sanctuary.my_mood_title")}
          subtitle={tr("atelier.co_sanctuary.my_mood_subtitle")}
        />
        <View class="flex-row flex-wrap gap-2">
          <Each of={moodOptions()} key={(m) => m.id}>
            {(m) => (
              <SelectChip
                label={m().label}
                selected={props.myMood === m().id}
                onPress={() => props.onSelectMood(m().id)}
              />
            )}
          </Each>
        </View>
      </Card>

      {/* 窗台岁时金桂盆栽 */}
      <Card>
        <View class="flex-row items-center justify-between gap-2">
          <View class="flex-row items-center gap-2">
            <CoSanctuaryIcon size={18} color={token("duo-accent")} />
            <View class="gap-0.5">
              <Text class="text-sm font-bold text-foreground">
                {tr("atelier.co_sanctuary.osmanthus_title")}
              </Text>
              <Text class="text-xs text-muted-foreground">
                {tr("atelier.co_sanctuary.partner_status", {
                  partner: props.partnerName ?? tr("atelier.co_sanctuary.default_partner"),
                  mood: props.partnerMood ?? tr("atelier.co_sanctuary.mood_breeze_steady"),
                })}
              </Text>
            </View>
          </View>
          <View class="rounded-full bg-secondary px-2.5 py-1">
            <Text class="text-xs font-bold text-duo-accent" style={{ fontVariant: ["tabular-nums"] }}>
              {`Lv.${props.osmanthusLevel}`}
            </Text>
          </View>
        </View>

        <MeterBar
          value={props.osmanthusProgressPct}
          accessibilityLabel={tr("atelier.co_sanctuary.meter_label")}
        />

        <View class="flex-row flex-wrap gap-2 pt-1">
          <Each of={careActions()} key={(c) => c.id}>
            {(act) => (
              <ActionButton
                title={act().label}
                size="sm"
                variant="outline"
                icon={act().id === "warm_tea" ? "tea_cup" : act().id === "blanket" ? "moon" : "paw"}
                onPress={() => props.onSendCare(act().id)}
                class="flex-1"
              />
            )}
          </Each>
        </View>
      </Card>

      <If when={Boolean(props.careHistory && props.careHistory.length > 0)}>
        <SunkenBox>
          <Text class="text-xs font-semibold text-foreground">
            {tr("atelier.co_sanctuary.history_title")}
          </Text>
          <Each of={props.careHistory ?? []} key={(h) => h.id}>
            {(row) => (
              <View class="flex-row flex-wrap items-center justify-between gap-1.5">
                <Text class="flex-1 text-xs text-foreground">{row().label}</Text>
                <Text class="text-[11px] text-muted-foreground">{row().timeLabel}</Text>
              </View>
            )}
          </Each>
        </SunkenBox>
      </If>
    </SheetDialog>
  );
}

export interface SeasonScrollSummary {
  season: string;
  solarRange: string;
  highlight: string;
  avgQiScore: number;
  dewCollected: number;
}

export interface CeramicBadgeItem {
  id: string;
  title: string;
  subtitle: string;
  unlocked: boolean;
}

/** 6. 《我的四时身体风物长卷》横向长卷弹窗 + 宋瓷勋章架 */
export function FourSeasonsScrollModal(props: {
  open: boolean;
  seasons: SeasonScrollSummary[];
  badges: CeramicBadgeItem[];
  onClose: () => void;
}) {
  return (
    <ModalSurface open={props.open} presentation="pageSheet" onClose={props.onClose}>
      <Scroll
        class="flex-1"
        keyboardShouldPersistTaps="handled"
        contentStyle={{ paddingHorizontal: 18, paddingBottom: 28, gap: 16 }}
      >
        <View class="mx-auto w-full max-w-3xl gap-4">
          <View class="flex-row items-center justify-between">
            <View class="flex-row items-center gap-2">
              <HandscrollIcon size={18} color={token("ink-lapis")} />
              <Text role="header" class="text-lg font-bold text-foreground">
                {tr("atelier.scroll.title")}
              </Text>
            </View>
            <ActionButton
              title={tr("atelier.scroll.close_btn")}
              size="sm"
              variant="secondary"
              shape="pill"
              onPress={props.onClose}
            />
          </View>

          {/* 横向卷轴插画 */}
          <Scroll horizontal={true} class="w-full rounded-2xl border border-border bg-card">
            <View class="flex-row items-center gap-3 p-2.5">
              <Image
                src={fourSeasonsHandscrollImg}
                class="h-44 w-160 rounded-xl"
                style={{ objectFit: "cover", objectPosition: "center" }}
              />
            </View>
          </Scroll>

          {/* 四季身体摘要卡 */}
          <View class="flex-row flex-wrap gap-2.5">
            <Each of={props.seasons} key={(s) => s.season}>
              {(item) => (
                <View class="min-w-44 flex-1 gap-1.5 rounded-2xl border border-border/75 bg-card p-3.5">
                  <View class="flex-row items-center justify-between">
                    <Text class="text-sm font-bold text-foreground">{item().season}</Text>
                    <Text class="text-[11px] font-semibold text-atelier-accent">{item().solarRange}</Text>
                  </View>
                  <Text class="text-xs font-semibold text-primary" style={{ fontVariant: ["tabular-nums"] }}>
                    {tr("atelier.scroll.season_stats", {
                      score: Math.round(item().avgQiScore),
                      dew: item().dewCollected,
                    })}
                  </Text>
                  <Text class="text-xs leading-relaxed text-muted-foreground">{item().highlight}</Text>
                </View>
              )}
            </Each>
          </View>

          {/* 宋瓷成就徽章 */}
          <Card>
            <SectionHeader
              title={tr("atelier.scroll.badges_title")}
              subtitle={tr("atelier.scroll.badges_subtitle")}
            />
            <View class="flex-row flex-wrap gap-2">
              <Each of={props.badges} key={(b) => b.id}>
                {(badge) => (
                  <View
                    class={`min-w-40 flex-1 flex-row items-center gap-2.5 rounded-xl p-3 ${
                      badge().unlocked
                        ? "border border-border/45 bg-card"
                        : "bg-surface-sunken opacity-75"
                    }`}
                  >
                    <BadgeCeramicIcon
                      size={22}
                      color={badge().unlocked ? token("ink-gold") : token("muted-foreground")}
                    />
                    <View class="flex-1 gap-0.5">
                      <Text class="text-xs font-bold text-foreground">{badge().title}</Text>
                      <Text class="text-[11px] text-muted-foreground">{badge().subtitle}</Text>
                    </View>
                  </View>
                )}
              </Each>
            </View>
          </Card>
        </View>
      </Scroll>
    </ModalSurface>
  );
}

/** 7. 《抬腕嘴替》Apple Watch / iOS WidgetKit 三景分化微缩艺术小组件展台（Waterllama 标杆分化设计） */
export function WristWidgetPreviewCard(props: {
  score: number;
  state: string;
  stateBadge: string;
  poeticLine: string;
  numericLine?: string;
  solarTerm?: string;
  mode?: "poetic" | "numeric";
  onChangeMode?: (mode: "poetic" | "numeric") => void;
  hideModeSelector?: boolean;
  embedded?: boolean;
}) {
  const localMode = signal<"poetic" | "numeric">(props.mode ?? "poetic");
  const activeMode = () => props.mode ?? localMode();
  const setMode = (m: "poetic" | "numeric") => {
    hapticSelection();
    localMode.set(m);
    props.onChangeMode?.(m);
  };
  const toggleMode = () => setMode(activeMode() === "poetic" ? "numeric" : "poetic");

  const isNightScene = () =>
    colorScheme() === "dark" || props.state === "recovery_priority";

  const lockHeadlineShort = () =>
    activeMode() === "poetic"
      ? props.poeticLine
      : tr("atelier.widget.score_out_of_100", { score: Math.round(props.score) });

  const clampedScore = () =>
    Math.max(0, Math.min(100, Number.isFinite(props.score) ? Math.round(props.score) : 0));

  return (
    <View
      class={
        props.embedded
          ? "gap-3.5"
          : "gap-3.5 rounded-2xl border border-border/45 bg-card p-4"
      }
    >
      <If
        when={!props.embedded}
        else={
          <Text class="text-xs text-muted-foreground">{tr("atelier.widget.subtitle")}</Text>
        }
      >
        <SectionHeader
          title={tr("atelier.widget.title")}
          subtitle={tr("atelier.widget.subtitle")}
          right={<WidgetIcon size={18} color={token("primary")} />}
        />
      </If>

      <If when={!props.hideModeSelector}>
        <View class="gap-2">
          <Text class="text-xs font-semibold text-muted-foreground">
            {tr("atelier.widget.ios_gallery_title")}
          </Text>
          <SegmentedTabs
            items={[
              { id: "poetic", label: tr("atelier.widget.mode_poetic") },
              { id: "numeric", label: tr("atelier.widget.mode_numeric") },
            ]}
            activeId={activeMode()}
            onSelect={(id) => setMode(id)}
            size="sm"
          />
        </View>
      </If>

      {/* WidgetKit 三态彻底分化微缩艺术场景矩阵 */}
      <View class="gap-3">
        {/* 第一行：小卡 1（深玻璃双环刻度锁屏圆环） + 小卡 2（磨砂和纸手绘猫影锁屏矩形条） */}
        <View class="flex-row items-stretch gap-2.5">
          {/* 小卡 1：锁屏圆环 — 深玻璃半透明圆角方形底座 + 高对比双环元气刻度弧与中心大字数字 */}
          <Button
            hitSlop={4}
            accessibilityLabel={tr("atelier.widget.lock_circular_label")}
            onPress={toggleMode}
            class="w-28 items-center justify-between gap-1.5 rounded-3xl border border-stage-ink/20 bg-stage-scrim p-3 shadow-xs transition active:scale-[0.98] focus-visible:ring-2 focus-visible:ring-ring motion-reduce:transition-none"
          >
            <Canvas
              style={{ width: 68, height: 68 }}
              draw={(cx, size) => {
                const w = size.width;
                const h = size.height;
                const midX = w / 2;
                const midY = h / 2;
                const score = clampedScore();
                const outerR = Math.min(w, h) / 2 - 4;
                const innerR = outerR - 7.5;
                const startA = Math.PI * 0.78;
                const sweepA = Math.PI * 1.44;
                const endA = startA + sweepA;
                const progA = startA + (sweepA * score) / 100;

                const inkBright = token("stage-ink");
                const arcPrimary =
                  props.state === "recovery_priority"
                    ? token("ink-gold")
                    : token("dew-accent");
                const arcSecondary = token("vital-prime");

                cx.clearRect(0, 0, w, h);

                // 外圈 19 枚锁屏微刻度短线
                const ticks = 19;
                for (let i = 0; i < ticks; i++) {
                  const r = i / (ticks - 1);
                  const a = startA + sweepA * r;
                  const lit = r * 100 <= score && score > 0;
                  cx.save();
                  cx.strokeStyle = lit ? arcPrimary : inkBright;
                  cx.globalAlpha = lit ? 0.92 : 0.22;
                  cx.lineWidth = lit ? 1.6 : 1.1;
                  cx.lineCap = "round";
                  cx.beginPath();
                  cx.moveTo(midX + Math.cos(a) * (outerR - 2.2), midY + Math.sin(a) * (outerR - 2.2));
                  cx.lineTo(midX + Math.cos(a) * outerR, midY + Math.sin(a) * outerR);
                  cx.stroke();
                  cx.restore();
                }

                // 内环双层轨道底弧
                cx.save();
                cx.strokeStyle = inkBright;
                cx.globalAlpha = 0.18;
                cx.lineWidth = 4.6;
                cx.lineCap = "round";
                cx.beginPath();
                cx.arc(midX, midY, innerR, startA, endA);
                cx.stroke();
                cx.restore();

                // 内环高对比元气活跃圆弧 + 次环金青细轨
                if (score > 0) {
                  cx.save();
                  cx.strokeStyle = arcPrimary;
                  cx.globalAlpha = 0.96;
                  cx.lineWidth = 4.6;
                  cx.lineCap = "round";
                  cx.beginPath();
                  cx.arc(midX, midY, innerR, startA, progA);
                  cx.stroke();

                  cx.strokeStyle = arcSecondary;
                  cx.globalAlpha = 0.72;
                  cx.lineWidth = 1.6;
                  cx.beginPath();
                  cx.arc(midX, midY, innerR - 4.8, startA, progA);
                  cx.stroke();

                  // 弧端高光圆珠
                  const tipX = midX + Math.cos(progA) * innerR;
                  const tipY = midY + Math.sin(progA) * innerR;
                  cx.fillStyle = inkBright;
                  cx.globalAlpha = 1;
                  cx.beginPath();
                  cx.arc(tipX, tipY, 2.3, 0, Math.PI * 2);
                  cx.fill();
                  cx.restore();
                }

                // 中心大字数字与副标
                cx.save();
                cx.textAlign = "center";
                cx.textBaseline = "middle";
                cx.fillStyle = inkBright;
                cx.font = '700 20px "PingFang SC", -apple-system, sans-serif';
                cx.fontVariantNumeric = "tabular-nums";
                cx.fillText(`${score}`, midX, midY - 2);

                cx.globalAlpha = 0.72;
                cx.font = '600 8px "Menlo", monospace';
                cx.fillText(activeMode() === "poetic" ? "QI" : "/100", midX, midY + 13);
                cx.restore();
              }}
            />
            <Text class="text-center text-[10px] font-semibold leading-tight text-stage-ink/90">
              {tr("atelier.widget.lock_circular_label")}
            </Text>
          </Button>

          {/* 小卡 2：锁屏矩形条 — 细长磨砂微件材质 + 左侧手绘线条小猫侧影 + 右侧迷你双行节律波纹与轻提示 */}
          <Button
            hitSlop={4}
            accessibilityLabel={tr("atelier.widget.lock_rect_label")}
            onPress={toggleMode}
            class="flex-1 justify-between gap-1.5 rounded-3xl border border-border/45 bg-secondary/75 p-3 shadow-xs transition active:scale-[0.99] focus-visible:ring-2 focus-visible:ring-ring motion-reduce:transition-none"
          >
            <View class="w-full flex-row items-center justify-between gap-2">
              <Text class="text-[10px] font-bold text-primary">
                {props.solarTerm ? `${tr("app.name")} · ${props.solarTerm}` : tr("app.name")}
              </Text>
              <Text class="line-clamp-1 text-[10px] font-medium text-muted-foreground">
                {tr("atelier.widget.lock_rect_label")}
              </Text>
            </View>

            <View class="w-full flex-row items-center gap-2.5">
              {/* 左侧手绘线条小猫侧影插画 */}
              <Canvas
                style={{ width: 46, height: 50 }}
                draw={(cx, size) => {
                  const w = size.width;
                  const h = size.height;
                  const ink = token("foreground");
                  const scarf = token("stamp-ink");
                  const cushion = token("card");
                  const accent = token("atelier-accent");

                  cx.clearRect(0, 0, w, h);

                  // 暖团蒲草垫圆斑
                  cx.save();
                  cx.fillStyle = cushion;
                  cx.globalAlpha = 0.82;
                  cx.beginPath();
                  cx.ellipse(w * 0.48, h - 7, 17, 5, 0, 0, Math.PI * 2);
                  cx.fill();
                  cx.restore();

                  // 手绘线条小猫蹲坐侧影（脊背 + 圆头 + 尖耳 + 翘尾）
                  cx.save();
                  cx.strokeStyle = ink;
                  cx.fillStyle = cushion;
                  cx.lineWidth = 1.75;
                  cx.lineCap = "round";
                  cx.lineJoin = "round";

                  // 身躯圆弧
                  cx.beginPath();
                  cx.moveTo(13, 41);
                  cx.quadraticCurveTo(9, 29, 17, 23);
                  cx.quadraticCurveTo(28, 22, 30, 33);
                  cx.quadraticCurveTo(31, 40, 26, 41);
                  cx.closePath();
                  cx.fill();
                  cx.stroke();

                  // 猫咪圆脸与双尖耳
                  cx.beginPath();
                  cx.moveTo(14, 21);
                  cx.lineTo(13, 10);
                  cx.lineTo(19, 15);
                  cx.quadraticCurveTo(22, 14, 25, 15);
                  cx.lineTo(30, 9);
                  cx.lineTo(29, 21);
                  cx.quadraticCurveTo(29, 27, 21.5, 27);
                  cx.quadraticCurveTo(14, 27, 14, 21);
                  cx.closePath();
                  cx.fill();
                  cx.stroke();

                  // 翘起的小尾巴
                  cx.beginPath();
                  cx.moveTo(28, 39);
                  cx.bezierCurveTo(37, 38, 41, 28, 37, 22);
                  cx.stroke();

                  // 温柔闭目笑眼与胡须
                  cx.lineWidth = 1.3;
                  cx.beginPath();
                  cx.arc(18.5, 20.5, 1.8, 0.15 * Math.PI, 0.85 * Math.PI);
                  cx.stroke();
                  cx.beginPath();
                  cx.arc(24.5, 20.5, 1.8, 0.15 * Math.PI, 0.85 * Math.PI);
                  cx.stroke();

                  // 脖间朱砂小领巾点缀
                  cx.strokeStyle = scarf;
                  cx.lineWidth = 2.2;
                  cx.beginPath();
                  cx.moveTo(15.5, 26.5);
                  cx.quadraticCurveTo(21.5, 29, 27.5, 26.5);
                  cx.stroke();

                  // 耳尖橘花斑点
                  cx.fillStyle = accent;
                  cx.globalAlpha = 0.55;
                  cx.beginPath();
                  cx.arc(26.5, 16.5, 2.2, 0, Math.PI * 2);
                  cx.fill();
                  cx.restore();
                }}
              />

              {/* 右侧迷你双行节律波纹路径 + 一句轻提示 */}
              <View class="min-w-0 flex-1 gap-1">
                <Canvas
                  class="w-full"
                  style={{ height: 20 }}
                  draw={(cx, size) => {
                    const w = Math.max(80, size.width);
                    const h = size.height;
                    const padR = 6;
                    const span = w - padR;
                    const phase = (clampedScore() * 0.09) % (Math.PI * 2);

                    cx.clearRect(0, 0, w, h);

                    // 第二行暖金虚线呼吸伴波
                    cx.save();
                    cx.strokeStyle = token("atelier-accent");
                    cx.globalAlpha = 0.55;
                    cx.lineWidth = 1.3;
                    cx.setLineDash([2.5, 2.5]);
                    cx.beginPath();
                    for (let x = 0; x <= span; x += 3) {
                      const t = x / span;
                      const y = h * 0.68 + Math.sin(t * Math.PI * 3.2 + phase + 1.1) * 3.2;
                      if (x === 0) cx.moveTo(x, y);
                      else cx.lineTo(x, y);
                    }
                    cx.stroke();
                    cx.restore();

                    // 第一行青瓷实线主节律波 + 右端白芯圆点
                    const endY = h * 0.38 - Math.sin(Math.PI * 3.0 + phase) * 3.8;
                    cx.save();
                    cx.strokeStyle = token("vital-prime");
                    cx.lineWidth = 2;
                    cx.lineCap = "round";
                    cx.beginPath();
                    for (let x = 0; x <= span; x += 2.5) {
                      const t = x / span;
                      const y = h * 0.38 - Math.sin(t * Math.PI * 3.0 + phase) * 3.8;
                      if (x === 0) cx.moveTo(x, y);
                      else cx.lineTo(x, y);
                    }
                    cx.stroke();

                    cx.fillStyle = token("card");
                    cx.strokeStyle = token("vital-prime");
                    cx.lineWidth = 1.8;
                    cx.beginPath();
                    cx.arc(span, endY, 2.8, 0, Math.PI * 2);
                    cx.fill();
                    cx.stroke();
                    cx.restore();
                  }}
                />

                <Text class="line-clamp-2 text-[11px] font-bold leading-snug text-foreground">
                  {lockHeadlineShort()}
                </Text>
                <Text class="line-clamp-1 text-[10px] text-muted-foreground" style={{ fontVariant: ["tabular-nums"] }}>
                  {props.numericLine ?? tr("atelier.widget.rhythm_status", { status: props.stateBadge })}
                </Text>
              </View>
            </View>
          </Button>
        </View>

        {/* 第二行：小卡 3（桌面 2×2 全出血场景卡 — 暖竹绿/夜雨全景微缩窗景小画 + 底部角标宠物状态丝带与大字状态签） */}
        <Button
          hitSlop={4}
          accessibilityLabel={tr("atelier.widget.home_small_label")}
          onPress={toggleMode}
          class="relative h-44 w-full overflow-hidden rounded-3xl border border-border/45 shadow-xs transition active:scale-[0.99] focus-visible:ring-2 focus-visible:ring-ring motion-reduce:transition-none"
        >
          <Image
            src={isNightScene() ? catNightImg : zenDayImg}
            class="absolute inset-0 h-full w-full object-cover"
            accessibilityLabel={tr("atelier.widget.home_small_label")}
          />

          {/* 微缩窗景光斑粒子与底部丝绒暗角渐晕层 */}
          <Canvas
            class="pointer-events-none absolute inset-0"
            style={{ pointerEvents: "none" }}
            draw={(cx, size) => {
              const w = Math.max(180, size.width);
              const h = Math.max(120, size.height);
              const night = isNightScene();

              cx.clearRect(0, 0, w, h);

              // 底部柔和暗角渐晕保证白色状态大字清晰可读
              const grad = cx.createLinearGradient(0, h * 0.18, 0, h);
              grad.addColorStop(0, "transparent");
              grad.addColorStop(0.58, token("stage-scrim"));
              grad.addColorStop(1, token("stage-scrim"));
              cx.save();
              cx.fillStyle = grad;
              cx.globalAlpha = night ? 0.88 : 0.82;
              cx.fillRect(0, 0, w, h);
              cx.restore();

              // 窗景右上柔光竹影/夜雨萤火微粒
              const motes = [
                { rx: 0.76, ry: 0.24, r: 2.2 },
                { rx: 0.84, ry: 0.34, r: 1.5 },
                { rx: 0.68, ry: 0.31, r: 1.7 },
                { rx: 0.22, ry: 0.28, r: 1.4 },
              ];
              cx.save();
              cx.fillStyle = night ? token("ink-gold") : token("stage-ink");
              for (let i = 0; i < motes.length; i++) {
                cx.globalAlpha = night ? 0.55 - i * 0.08 : 0.42 - i * 0.06;
                cx.beginPath();
                cx.arc(w * motes[i].rx, h * motes[i].ry, motes[i].r, 0, Math.PI * 2);
                cx.fill();
              }
              cx.restore();
            }}
          />

          {/* 前景全出血桌面小组件信息层 */}
          <View class="relative h-full w-full justify-between p-3.5">
            <View class="flex-row items-center justify-between gap-2">
              <View class="flex-row items-center gap-1.5 rounded-full border border-stage-ink/20 bg-stage-scrim/80 px-2.5 py-1">
                <QiIcon name="sparkles" size={11} color={token("ink-gold")} />
                <Text class="text-[11px] font-semibold text-stage-ink">
                  {props.solarTerm ? `${tr("app.name")} · ${props.solarTerm}` : tr("app.name")}
                </Text>
              </View>
            </View>

            <View class="gap-2">
              <Text
                class="line-clamp-2 text-base font-bold leading-snug text-stage-ink sm:text-lg"
                style={{ fontVariant: ["tabular-nums"] }}
              >
                {activeMode() === "poetic"
                  ? props.poeticLine
                  : tr("atelier.widget.score_out_of_100", { score: clampedScore() })}
              </Text>

              {/* 底部角标宠物状态丝带 */}
              <View class="flex-row items-center justify-between gap-1.5 rounded-2xl border border-stage-ink/20 bg-stage-scrim/85 px-2.5 py-1.5">
                <View class="min-w-0 flex-1 flex-row items-center gap-1">
                  <QiIcon name="paw" size={11} color={token("ink-gold")} />
                  <Text
                    class="line-clamp-1 flex-1 text-[10px] font-medium text-stage-ink/90"
                    style={{ fontVariant: ["tabular-nums"] }}
                  >
                    {activeMode() === "poetic"
                      ? tr("atelier.widget.poetic_caption")
                      : (props.numericLine ?? tr("atelier.widget.rhythm_status", { status: props.stateBadge }))}
                  </Text>
                </View>
                <Text class="shrink-0 text-[10px] font-semibold text-ink-gold">
                  {tr("atelier.widget.home_small_label")}
                </Text>
              </View>
            </View>
          </View>
        </Button>
      </View>
    </View>
  );
}

/** 8. 父母大字端到端加密《气韵平安家书》阅读与一键发送弹窗 */
export function FamilyPeaceLetterModal(props: {
  open: boolean;
  recipientLabel?: string;
  dateLabel: string;
  peaceHeadline: string;
  bodyParagraphs: string[];
  signatureLine?: string;
  onCopyLetter: (fullText: string) => void;
  onClose: () => void;
}) {
  const recipient = () => props.recipientLabel ?? tr("atelier.family_letter.default_recipient");

  const fullLetterText = () =>
    [
      tr("atelier.family_letter.export_header", { date: props.dateLabel }),
      tr("atelier.family_letter.salutation", { recipient: recipient() }),
      props.peaceHeadline,
      ...props.bodyParagraphs,
      props.signatureLine ?? tr("atelier.family_letter.default_signature_export"),
    ].join("\n\n");

  return (
    <ModalSurface open={props.open} presentation="pageSheet" onClose={props.onClose}>
      <Scroll
        class="flex-1"
        keyboardShouldPersistTaps="handled"
        contentStyle={{ paddingHorizontal: 20, paddingBottom: 28, gap: 16 }}
      >
        <View class="mx-auto w-full max-w-lg gap-4">
          <View class="flex-row items-center justify-between">
            <View class="flex-row items-center gap-2">
              <FamilyHeartIcon size={20} color={token("wax-seal")} />
              <Text role="header" class="text-lg font-bold text-foreground">
                {tr("atelier.family_letter.title")}
              </Text>
            </View>
            <ActionButton
              title={tr("atelier.family_letter.close_btn")}
              size="sm"
              variant="secondary"
              shape="pill"
              onPress={props.onClose}
            />
          </View>

          <View class="flex-row items-center gap-2 rounded-2xl bg-vital-prime-soft px-3.5 py-2">
            <QiIcon name="cloud_lock" size={14} color={token("vital-prime")} />
            <Text class="flex-1 text-xs font-semibold text-vital-prime">
              {tr("atelier.family_letter.privacy_banner")}
            </Text>
          </View>

          <View class="gap-4 rounded-2xl border border-border/45 bg-card p-5 shadow-2xs">
            <View class="flex-row items-center justify-between">
              <Text class="text-base font-bold text-wax-seal">
                {tr("atelier.family_letter.salutation", { recipient: recipient() })}
              </Text>
              <Text class="text-xs text-muted-foreground">{props.dateLabel}</Text>
            </View>

            <Text class="text-xl font-bold leading-relaxed text-foreground">{props.peaceHeadline}</Text>

            <View class="gap-3">
              <Each of={props.bodyParagraphs} key={(p, idx) => `peace-${idx}`}>
                {(para) => <Text class="text-base font-medium leading-loose text-foreground">{para()}</Text>}
              </Each>
            </View>

            <Hairline />
            <Text class="text-right text-sm font-semibold text-muted-foreground">
              {props.signatureLine ?? tr("atelier.family_letter.default_signature_card")}
            </Text>
          </View>

          <View class="flex-row gap-2">
            <ActionButton
              title={tr("atelier.family_letter.copy_btn")}
              icon="family_heart"
              size="lg"
              onPress={() => props.onCopyLetter(fullLetterText())}
              class="flex-1"
            />
            <ActionButton
              title={tr("atelier.family_letter.back_btn")}
              variant="secondary"
              size="lg"
              onPress={props.onClose}
            />
          </View>
        </View>
      </Scroll>
    </ModalSurface>
  );
}
