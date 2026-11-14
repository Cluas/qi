import { signal, Text, token, View } from "nsh";
import {
  claimRecoveryDew,
  claimSeasonalReward,
  getDewSummary,
  getPetCount,
  getState,
  getVisualTheme,
  openFourSeasonsInsights,
  openMealsLog,
  openPetStudio,
  petCompanion,
  refreshLiveBriefing,
  setHardwareTier,
  setSealInk,
  setShowAtelierModal,
  setShowCoSanctuarySheet,
  setShowCompanionLetterModal,
  setShowDailyPoster,
  setShowQuickCaptureModal,
  setTab,
  setVisualTheme,
  setWeatherMood,
  setWidgetPrivacyMode,
  toggleEquippedKeepsake,
  toggleTodayAction,
} from "../store";
import { SeasonalPassPanel, WristWidgetPreviewCard } from "../ui/atelier";
import {
  AcwrSweetSpotCanvas,
  InteractiveRhythmPlot,
  SleepArchitectureBar,
} from "../ui/charts";
import {
  ActionButton,
  Button,
  ConsumerStatusPill,
  DrawerSheet,
  Hairline,
  HeaderActionPill,
  IosGroupedSection,
  IosLargeHeader,
  IosListRow,
  SectionHeader,
  SegmentedTabs,
  SelectChip,
  VitalBadge,
} from "../ui/components";
import { QiIcon, type QiIconName } from "../ui/icons";
import { tr } from "../ui/i18n";
import { runDrawerLayoutTransition } from "../ui/motion";
import {
  LiquidQiVesselCanvas,
  SanctuaryStage,
  type SealInkId,
  StampCheckinCard,
  ThemeSwitcherStrip,
  type WeatherMood,
} from "../ui/sanctuary";

type TodayDrawerTab = "briefing" | "curves" | "seasonal";

function formatMetricReading(value: number | null, unit: string): string {
  if (value === null || !Number.isFinite(value)) return "—";
  const rounded = Math.abs(value) >= 100 ? `${Math.round(value)}` : `${Number(value.toFixed(1))}`;
  return unit ? `${rounded} ${unit}` : rounded;
}

function toVitalTone(state: string): "prime_ready" | "moderate_steady" | "recovery_priority" {
  if (state === "prime_ready") return "prime_ready";
  if (state === "recovery_priority") return "recovery_priority";
  return "moderate_steady";
}

export function TodayPage() {
  const drawerOpen = signal(false);
  const drawerTab = signal<TodayDrawerTab>("briefing");
  const sceneSheetOpen = signal(false);
  const feelingOpen = signal(false);

  const state = () => getState();
  const snap = () => state().snapshot;
  const settings = () => state().settings;
  const briefing = () => state().briefing;
  const solar = () => state().solar_atelier;
  const entitlements = () => state().entitlements;

  const consumerMetricTitle = (key: string, fallback: string) => {
    switch (key) {
      case "hrv_rmssd_ms":
        return tr("today.metric_title_hrv_rmssd");
      case "hrv_sdnn_ms":
        return tr("today.metric_title_hrv_sdnn");
      case "resting_heart_rate_bpm":
        return tr("today.metric_title_rhr");
      case "sleep_minutes":
        return tr("today.metric_title_sleep_total");
      case "sleep_deep_minutes":
        return tr("today.metric_title_sleep_deep");
      case "sleep_rem_minutes":
        return tr("today.metric_title_sleep_rem");
      case "steps":
        return tr("today.metric_title_steps");
      case "active_zone_minutes":
        return tr("today.metric_title_active_min");
      case "calories_active_kcal":
        return tr("today.metric_title_active_kcal");
      default:
        return fallback;
    }
  };

  const friendlyLoadStatus = (acwr: number | null): string => {
    if (acwr === null || !Number.isFinite(acwr)) return tr("today.load_building");
    if (acwr < 0.8) return tr("today.load_recovery");
    if (acwr <= 1.3) return tr("today.load_sweet");
    return tr("today.load_overreach");
  };

  const friendlySleepDebt = (debt: number | null): string => {
    if (debt === null || !Number.isFinite(debt)) return tr("today.sleep_untracked");
    if (debt > 0.3) return tr("today.sleep_debt", { hours: debt.toFixed(1) });
    if (debt < -0.3) return tr("today.sleep_surplus", { hours: Math.abs(debt).toFixed(1) });
    return tr("today.sleep_balanced");
  };

  const feelingLevels = () => [
    { score: 1, label: tr("today.feeling_1") },
    { score: 2, label: tr("today.feeling_2") },
    { score: 3, label: tr("today.feeling_3") },
    { score: 4, label: tr("today.feeling_4") },
    { score: 5, label: tr("today.feeling_5") },
  ];

  const recoveryTile = () => {
    const m = snap().metrics;
    const hrv = m.hrv_rmssd_ms ?? m.hrv_sdnn_ms;
    if (hrv && hrv.value !== null) {
      return {
        value: `${Math.round(hrv.value)} ms`,
        sublabel:
          hrv.baseline_30d_mean !== null
            ? tr("today.metric_baseline_avg", { val: `${Math.round(hrv.baseline_30d_mean)} ms` })
            : tr("today.metric_title_hrv_rmssd"),
      };
    }
    const rhr = m.resting_heart_rate_bpm;
    if (rhr && rhr.value !== null) {
      return {
        value: `${Math.round(rhr.value)} bpm`,
        sublabel:
          rhr.baseline_30d_mean !== null
            ? tr("today.metric_baseline_avg", { val: `${Math.round(rhr.baseline_30d_mean)} bpm` })
            : tr("today.metric_title_rhr"),
      };
    }
    return {
      value: `${settings().morning_feeling} / 5`,
      sublabel: tr("today.metric_morning_mood"),
    };
  };

  const sleepTile = () => {
    const h = snap().sleep_last_night_hours;
    if (h === null || !Number.isFinite(h)) {
      return {
        value: tr("today.metric_unmeasured"),
        sublabel: tr("today.metric_phone_steps"),
      };
    }
    return {
      value: tr("common.hours_unit", { value: h.toFixed(1) }),
      sublabel: friendlySleepDebt(snap().sleep_debt_7d_hours),
    };
  };

  const loadTile = () => {
    const m = snap().metrics;
    const steps = m.steps;
    const acwr = snap().acwr;
    if (steps && steps.value !== null) {
      return {
        value: tr("today.steps_count", { count: Math.round(steps.value) }),
        sublabel: friendlyLoadStatus(acwr),
      };
    }
    return {
      value: friendlyLoadStatus(acwr),
      sublabel: tr("today.activity_7d"),
    };
  };

  const baselineMetrics = () => {
    const m = snap().metrics;
    const keys = [
      "hrv_rmssd_ms",
      "hrv_sdnn_ms",
      "resting_heart_rate_bpm",
      "sleep_minutes",
      "sleep_deep_minutes",
      "sleep_rem_minutes",
      "steps",
      "active_zone_minutes",
    ];
    return keys.filter((k) => Boolean(m[k])).map((k) => m[k]);
  };

  const drawerTabs = (): { id: TodayDrawerTab; label: string }[] => [
    { id: "briefing", label: tr("today.drawer_tab_briefing") },
    { id: "curves", label: tr("today.drawer_tab_curves") },
    { id: "seasonal", label: tr("today.drawer_tab_seasonal") },
  ];

  const unreadLetterCount = () =>
    solar().pet_weekly_letters.length + solar().weekly_letters.filter((w) => w.unlocked).length;

  const currentFeelingLabel = () =>
    feelingLevels().find((item) => item.score === settings().morning_feeling)?.label ??
    tr("today.feeling_3");

  const vitalsGlanceTiles = (): {
    id: string;
    icon: QiIconName;
    badgeBgClass: string;
    iconColor: string;
    label: string;
    value: string;
    sublabel: string;
  }[] => [
    {
      id: "recovery",
      icon: "heart_pulse",
      badgeBgClass: "bg-vital-prime-soft",
      iconColor: token("vital-prime"),
      label: tr("today.metric_recovery"),
      value: recoveryTile().value,
      sublabel: recoveryTile().sublabel,
    },
    {
      id: "sleep",
      icon: "moon",
      badgeBgClass: "bg-bento-lavender",
      iconColor: token("sleep-rem"),
      label: tr("today.metric_sleep"),
      value: sleepTile().value,
      sublabel: sleepTile().sublabel,
    },
    {
      id: "activity",
      icon: "flame",
      badgeBgClass: "bg-vital-steady-soft",
      iconColor: token("vital-steady"),
      label: tr("today.metric_activity"),
      value: loadTile().value,
      sublabel: loadTile().sublabel,
    },
  ];

  return (
    <View class="gap-3.5">
      <IosLargeHeader
        kicker={tr("today.kicker")}
        kickerMeta={snap().target_date}
        title={tr("app.name")}
        trailing={
          <View class="flex-row items-center gap-1.5">
            <HeaderActionPill
              label={tr("today.scene_picker_btn")}
              icon="sparkles"
              onPress={() => sceneSheetOpen.set(true)}
            />
            <HeaderActionPill
              label={tr("nav.quick_short")}
              icon="plus"
              tone="primary"
              accessibilityLabel={tr("nav.quick_capture")}
              onPress={() => setShowQuickCaptureModal(true)}
            />
          </View>
        }
      />

      <SanctuaryStage
        themeId={getVisualTheme()}
        score={snap().qi_score}
        state={snap().state}
        stateBadge={snap().state_badge}
        headline={snap().headline}
        dewSummary={getDewSummary()}
        petCount={getPetCount()}
        checkedAction={settings().checked_action_today}
        solarTerm={solar().solar_term.term_name}
        lunarPhase={solar().solar_term.lunar_phase}
        weatherMood={solar().solar_term.weather_mood as WeatherMood}
        equippedKeepsakes={solar().keepsake_catalog.equipped_ids}
        partnerMoodBadge={`${solar().co_sanctuary.partner_name} · ${solar().co_sanctuary.partner_mood_label}`}
        atelierUnlocked={entitlements().is_atelier_unlocked}
        customPet={solar().active_custom_pet}
        unreadLettersCount={unreadLetterCount()}
        onPet={() => petCompanion()}
        onOpenPoster={() => setShowDailyPoster(true)}
        onOpenMeals={() => openMealsLog()}
        onOpenLetters={() => setShowCompanionLetterModal(true)}
        onOpenCoWindow={() => setShowCoSanctuarySheet(true)}
        onOpenPetStudio={() => openPetStudio()}
      />

      <View class="gap-3.5 rounded-3xl border border-border/45 bg-card p-4 shadow-2xs">
        <View class="flex-row items-center gap-4">
          <LiquidQiVesselCanvas score={snap().qi_score} state={snap().state} />
          <View class="flex-1 gap-2">
            <View class="flex-row flex-wrap items-center gap-2">
              <VitalBadge tone={toVitalTone(snap().state)} label={snap().state_badge} />
            </View>
            <Text class="text-[15px] leading-relaxed text-foreground">{snap().headline}</Text>
          </View>
        </View>

        <Hairline />

        <StampCheckinCard
          embedded={true}
          themeId={getVisualTheme()}
          microAction={snap().micro_action}
          checked={settings().checked_action_today}
          onToggle={() => toggleTodayAction()}
          onGoLog={() => setTab("log")}
          onOpenPoster={() => setShowDailyPoster(true)}
        />

        <Hairline />

        <View class="gap-2.5">
          <View class="min-h-11 flex-row items-center justify-between gap-2">
            <View class="flex-1 flex-row items-center gap-2">
              <Text class="text-xs font-medium text-muted-foreground">
                {tr("today.feeling_compact_label")}
              </Text>
              <Text class="text-xs font-semibold text-foreground">
                {currentFeelingLabel()}
              </Text>
            </View>
            <Button
              hitSlop={8}
              accessibilityLabel={tr("today.feeling_adjust")}
              onPress={() => {
                runDrawerLayoutTransition("easeInEaseOut");
                feelingOpen.set(!feelingOpen());
              }}
              class="min-h-9 flex-row items-center gap-1 rounded-full bg-surface-sunken px-3 py-1.5 transition-colors duration-150 hover:bg-secondary active:scale-[0.97] motion-reduce:transition-none"
            >
              <Text class="text-xs font-medium text-primary">{tr("today.feeling_adjust")}</Text>
              <View class={`transition-transform duration-150 ${feelingOpen() ? "rotate-180" : ""}`}>
                <QiIcon name="chevron_down" size={12} color={token("primary")} />
              </View>
            </Button>
          </View>

          <If when={feelingOpen()}>
            <View class="flex-row flex-wrap gap-1.5">
              <Each of={feelingLevels()} key={(item) => item.score}>
                {(item) => (
                  <SelectChip
                    label={item().label}
                    selected={settings().morning_feeling === item().score}
                    onPress={() => {
                      setHardwareTier(settings().hardware_tier, item().score);
                      runDrawerLayoutTransition("easeInEaseOut");
                      feelingOpen.set(false);
                    }}
                  />
                )}
              </Each>
            </View>
          </If>
        </View>
      </View>

      <View class="flex-row items-stretch gap-2.5">
        <Each of={vitalsGlanceTiles()} key={(tile) => tile.id}>
          {(tile) => (
            <Button
              hitSlop={4}
              accessibilityLabel={`${tile().label} ${tile().value}`}
              onPress={() => {
                drawerTab.set("curves");
                drawerOpen.set(true);
              }}
              class="min-h-11 flex-1 justify-between gap-2 rounded-2xl border border-border/45 bg-card p-3 shadow-2xs transition-colors duration-150 hover:bg-surface-sunken/60 active:scale-[0.97] focus-visible:ring-2 focus-visible:ring-ring motion-reduce:transition-none"
            >
              <View class="flex-row items-center gap-1.5">
                <View
                  class={`size-[22px] shrink-0 items-center justify-center rounded-[6px] ${tile().badgeBgClass}`}
                >
                  <QiIcon name={tile().icon} size={12} color={tile().iconColor} />
                </View>
                <Text class="line-clamp-1 flex-1 text-[11px] font-medium text-muted-foreground">
                  {tile().label}
                </Text>
              </View>
              <View class="gap-0.5">
                <Text
                  class="line-clamp-1 text-base font-bold tracking-tight text-foreground"
                  style={{ fontVariant: ["tabular-nums"] }}
                >
                  {tile().value}
                </Text>
                <Text
                  class="line-clamp-1 text-[11px] text-muted-foreground"
                  style={{ fontVariant: ["tabular-nums"] }}
                >
                  {tile().sublabel}
                </Text>
              </View>
            </Button>
          )}
        </Each>
      </View>

      <View class="flex-row items-center justify-between gap-3 rounded-3xl border border-border/45 bg-card px-4 py-3 shadow-2xs">
        <View class="flex-1 flex-row items-center gap-2.5">
          <View class="size-7 items-center justify-center rounded-[8px] bg-accent">
            <QiIcon name="record" size={15} color={token("primary")} />
          </View>
          <View class="flex-1 gap-0.5">
            <Text role="header" class="text-sm font-semibold text-foreground">
              {tr("today.drawer_trigger_title")}
            </Text>
            <Text class="text-xs text-muted-foreground">{tr("today.drawer_trigger_sub")}</Text>
          </View>
        </View>
        <ActionButton
          title={tr("common.expand")}
          icon="chevron_down"
          variant="secondary"
          size="sm"
          shape="pill"
          onPress={() => drawerOpen.set(true)}
        />
      </View>

      <DrawerSheet
        open={drawerOpen()}
        title={tr("today.drawer_trigger_title")}
        subtitle={`${solar().solar_term.term_name} · ${solar().solar_term.lunar_phase}`}
        onClose={() => drawerOpen.set(false)}
      >
        <View class="gap-4">
          <SegmentedTabs
            items={drawerTabs()}
            activeId={drawerTab()}
            onSelect={(id) => drawerTab.set(id)}
          />

          <If when={drawerTab() === "briefing"}>
            <View class="gap-3.5">
              <SectionHeader
                title={tr("today.briefing_title")}
                subtitle={briefing().provider_label}
                right={
                  <VitalBadge
                    tone={briefing().degraded ? "moderate_steady" : "prime_ready"}
                    label={
                      briefing().degraded ? tr("today.briefing_lite") : tr("today.briefing_ready")
                    }
                  />
                }
              />

              <View class="rounded-2xl bg-surface-sunken p-4">
                <Text class="text-sm leading-relaxed text-foreground">{briefing().text}</Text>
              </View>

              <If when={snap().state === "recovery_priority"}>
                <View class="flex-row flex-wrap items-center justify-between gap-3 rounded-2xl bg-surface-sunken p-4">
                  <View class="flex-1 flex-row items-center gap-2.5">
                    <View class="size-7 items-center justify-center rounded-[8px] bg-dew-soft">
                      <QiIcon name="sparkles" size={15} color={token("primary")} />
                    </View>
                    <View class="flex-1 gap-0.5">
                      <Text class="text-xs font-semibold text-foreground">
                        {tr("today.recovery_dew_title")}
                      </Text>
                      <Text class="text-[11px] leading-relaxed text-muted-foreground">
                        {tr("today.recovery_dew_desc")}
                      </Text>
                    </View>
                  </View>
                  <ActionButton
                    title={tr("today.recovery_dew_btn")}
                    icon="sparkles"
                    size="sm"
                    shape="pill"
                    onPress={() => claimRecoveryDew()}
                  />
                </View>
              </If>

              <View class="flex-row flex-wrap gap-2">
                <ActionButton
                  title={tr("today.briefing_refresh")}
                  icon="sparkles"
                  variant="outline"
                  onPress={() => refreshLiveBriefing()}
                />
                <ActionButton
                  title={tr("today.briefing_ai_prefs")}
                  icon="sliders"
                  variant="secondary"
                  onPress={() => {
                    drawerOpen.set(false);
                    setTab("me");
                  }}
                />
              </View>

              <Text class="text-[11px] leading-relaxed text-muted-foreground">
                {briefing().disclaimer}
              </Text>
            </View>
          </If>

          <If when={drawerTab() === "curves"}>
            <View class="gap-4">
              <If when={Boolean(snap().sleep_stages_last_night)}>
                <View class="gap-2.5 rounded-2xl bg-surface-sunken p-4">
                  <SectionHeader title={tr("insights.sleep_arch_title")} />
                  <SleepArchitectureBar
                    deepMin={snap().sleep_stages_last_night!.deep_min}
                    coreMin={snap().sleep_stages_last_night!.core_min}
                    remMin={snap().sleep_stages_last_night!.rem_min}
                    awakeMin={snap().sleep_stages_last_night!.awake_min}
                  />
                </View>
              </If>

              <If when={snap().acwr !== null}>
                <View class="gap-2 rounded-2xl bg-surface-sunken p-4">
                  <View class="flex-row items-center justify-between">
                    <Text class="text-xs font-semibold text-foreground">{tr("insights.acwr_title")}</Text>
                    <Text class="text-xs font-medium text-primary">
                      {friendlyLoadStatus(snap().acwr)}
                    </Text>
                  </View>
                  <AcwrSweetSpotCanvas acwr={snap().acwr} />
                </View>
              </If>

              <View class="gap-2.5">
                <SectionHeader title={tr("insights.rhythm_trend_title")} />
                <InteractiveRhythmPlot points={snap().trend_14d} />
              </View>

              <IosGroupedSection title={tr("today.baseline_title")}>
                <If
                  when={baselineMetrics().length > 0}
                  else={
                    <View class="px-4 py-3.5">
                      <Text class="text-xs text-muted-foreground">
                        {tr("today.baseline_empty")}
                      </Text>
                    </View>
                  }
                >
                  <Each of={baselineMetrics()} key={(item) => item.key}>
                    {(item, idx) => (
                      <IosListRow
                        title={consumerMetricTitle(item().key, item().label)}
                        subtitle={
                          item().baseline_30d_mean === null
                            ? tr("today.baseline_accumulating")
                            : tr("today.baseline_compare", {
                                mean: formatMetricReading(item().baseline_30d_mean, item().unit),
                                avg7: formatMetricReading(item().avg_7d, item().unit),
                              })
                        }
                        value={formatMetricReading(item().value, item().unit)}
                        trailing={<ConsumerStatusPill status={item().status} />}
                        showChevron={false}
                        divider={idx() < baselineMetrics().length - 1}
                      />
                    )}
                  </Each>
                </If>
              </IosGroupedSection>

              <WristWidgetPreviewCard
                score={snap().qi_score}
                state={snap().state}
                stateBadge={solar().widget_preview.badge_text}
                poeticLine={solar().widget_preview.headline}
                numericLine={
                  solar().widget_preview.metrics_line ??
                  `${tr("today.rhythm_score", { score: snap().qi_score })} · ${solar().widget_preview.subline}`
                }
                solarTerm={solar().widget_preview.solar_term}
                mode={solar().widget_preview.privacy_mode === "metrics" ? "numeric" : "poetic"}
                onChangeMode={(m) => setWidgetPrivacyMode(m === "numeric" ? "metrics" : "metaphor")}
              />
            </View>
          </If>

          <If when={drawerTab() === "seasonal"}>
            <View class="gap-3.5">
              <SeasonalPassPanel
                solarTerm={solar().solar_term.term_name}
                solarSubtitle={solar().solar_term.whisper}
                dewBalance={solar().dew_balance}
                atelierUnlocked={entitlements().is_atelier_unlocked}
                nodes={solar().seasonal_pass.map((n) => ({
                  id: n.id,
                  title: n.title,
                  track: n.track === "atelier" ? "atelier" : "free",
                  dewCost: n.cost_dew,
                  claimed: n.claimed,
                  description: n.evergreen_term
                    ? tr("today.seasonal_evergreen_tag", { term: n.evergreen_term })
                    : undefined,
                }))}
                archiveItems={solar()
                  .seasonal_pass.filter((n) => n.evergreen_term)
                  .map((n) => ({
                    solarTerm: n.evergreen_term,
                    title: n.title,
                    badge:
                      n.track === "free"
                        ? tr("today.seasonal_free_badge")
                        : tr("today.seasonal_pro_badge"),
                    unlocked: n.unlocked_by_tier || n.claimed,
                  }))}
                outfits={solar().keepsake_catalog.outfits.map((o) => ({
                  id: o.id,
                  label: o.name,
                  atelierOnly: o.track === "atelier",
                }))}
                activeSealInk={solar().keepsake_catalog.active_seal_ink as SealInkId}
                onSelectSealInk={(ink) => setSealInk(ink)}
                weatherMood={solar().solar_term.weather_mood as WeatherMood}
                onSelectWeatherMood={(mood) => setWeatherMood(mood)}
                equippedKeepsakes={solar().keepsake_catalog.equipped_ids}
                availableKeepsakes={solar().keepsake_catalog.items.map((k) => ({
                  id: k.id,
                  name: k.name,
                  unlocked: k.unlocked,
                }))}
                onToggleEquipKeepsake={(id) => toggleEquippedKeepsake(id)}
                onClaim={(id) => claimSeasonalReward(id)}
                onOpenAtelier={() => {
                  drawerOpen.set(false);
                  setShowAtelierModal(true);
                }}
              />

              <ActionButton
                title={tr("today.open_scroll")}
                icon="handscroll"
                variant="outline"
                onPress={() => {
                  drawerOpen.set(false);
                  openFourSeasonsInsights();
                }}
              />
            </View>
          </If>
        </View>
      </DrawerSheet>

      <DrawerSheet
        open={sceneSheetOpen()}
        title={tr("today.scene_picker_title")}
        subtitle={tr("today.scene_picker_sub")}
        onClose={() => sceneSheetOpen.set(false)}
      >
        <View class="gap-3.5">
          <ThemeSwitcherStrip
            activeId={getVisualTheme()}
            onSelect={(id) => setVisualTheme(id)}
            detailed={true}
          />
        </View>
      </DrawerSheet>
    </View>
  );
}
