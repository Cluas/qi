import { signal, Text, token, View } from "nsh";
import {
  getDewSummary,
  getInsightsSubTab,
  getState,
  getVisualTheme,
  hasFeature,
  type InsightsSubTab,
  setInsightsSubTab,
  setShowAtelierModal,
  setShowFourSeasonsScrollModal,
  startCustomExperiment,
  startExperiment,
  toggleExperimentToday,
} from "../store";
import { InteractiveRhythmPlot, MultiLagRadarBars, SleepArchitectureBar } from "../ui/charts";
import {
  ActionButton,
  Card,
  CollapsibleCard,
  FieldInput,
  MeterBar,
  SectionHeader,
  SegmentedTabs,
  SelectChip,
  SunkenBox,
  VitalBadge,
} from "../ui/components";
import { BadgeCeramicIcon, HandscrollIcon } from "../ui/icons";
import { tr } from "../ui/i18n";
import { BentoMetricCard, KeepsakeShelfCard } from "../ui/sanctuary";

function progressPercent(completed: number, target: number): number {
  if (!Number.isFinite(target) || target <= 0) return 0;
  return Math.max(0, Math.min(100, Math.round((completed / target) * 100)));
}

export function InsightsPage() {
  const customTitle = signal("");
  const customHypothesis = signal("");
  const customDays = signal(7);
  const customWashout = signal(2);
  const customOpen = signal(false);

  const state = () => getState();
  const snap = () => state().snapshot;
  const period = () => state().period_comparison;
  const experiments = () => state().experiments;
  const templates = () => state().experiment_templates;

  const subTabs = (): { id: InsightsSubTab; label: string }[] => [
    { id: "curves", label: tr("insights.subtab_curves") },
    { id: "correlations", label: tr("insights.subtab_correlations") },
    { id: "handscroll", label: tr("insights.subtab_handscroll") },
  ];

  const formatDelta = (val: number | null, unit: string, lowerIsBetter = false): string => {
    if (val === null || !Number.isFinite(val)) return tr("insights.delta_accumulating");
    const sign = val > 0 ? "+" : "";
    const formatted = `${sign}${val.toFixed(1)} ${unit}`;
    if (Math.abs(val) < 0.2) return `${formatted} · ${tr("insights.delta_flat")}`;
    const improved = lowerIsBetter ? val < 0 : val > 0;
    return `${formatted} · ${improved ? tr("insights.delta_improved") : tr("insights.delta_watch")}`;
  };

  const submitCustomExperiment = () => {
    const title = customTitle().trim();
    if (!title) return;
    const hypothesis = customHypothesis().trim() || tr("insights.custom_exp_default_hypothesis");
    startCustomExperiment(title, hypothesis, customDays(), customWashout());
    customTitle.set("");
    customHypothesis.set("");
    customOpen.set(false);
  };

  return (
    <View class="gap-4">
      <SegmentedTabs
        items={subTabs()}
        activeId={getInsightsSubTab()}
        onSelect={(id) => setInsightsSubTab(id)}
      />

      {/* Subview 1: Rhythm Curves & N-of-1 Experiments */}
      <If when={getInsightsSubTab() === "curves"}>
        <View class="gap-5">
          {/* Gentler Streak 式无框人文洞察总结语录 + 轻盈趋势字标 + 三联无框釉面数字瓷贴 */}
          <View class="gap-3.5 px-0.5 pt-0.5">
            <View class="gap-1.5">
              <View class="flex-row items-center justify-between gap-2">
                <Text
                  class="text-xs font-medium text-muted-foreground"
                  style={{ fontVariant: ["tabular-nums"] }}
                >
                  {tr("insights.period_subtitle", {
                    current: period().current_window,
                    prev: period().previous_window,
                  })}
                </Text>
                <VitalBadge tone="neutral" label={tr("insights.period_badge")} />
              </View>

              <Text role="header" class="text-xl font-bold tracking-tight text-foreground sm:text-2xl">
                {tr("insights.period_title")}
              </Text>

              <Text class="pt-0.5 text-[15px] leading-relaxed text-foreground/90">
                {period().summary}
              </Text>
            </View>

            <View class="flex-row gap-2">
              <BentoMetricCard
                compact={true}
                icon="heart_pulse"
                label={tr("today.metric_title_hrv_rmssd").replace(" (RMSSD)", "")}
                value={period().hrv_current === null ? "—" : `${period().hrv_current!.toFixed(1)} ms`}
                sublabel={
                  period().hrv_prev === null
                    ? tr("insights.prev_7d_empty")
                    : tr("insights.prev_7d_val", { val: `${period().hrv_prev!.toFixed(1)}` })
                }
                toneSurface="sage"
                accentColor={token("vital-prime")}
                class="min-w-0 flex-1 border-transparent"
              />

              <BentoMetricCard
                compact={true}
                icon="flame"
                label={tr("today.metric_title_rhr")}
                value={period().rhr_current === null ? "—" : `${period().rhr_current!.toFixed(1)} bpm`}
                sublabel={
                  period().rhr_prev === null
                    ? tr("insights.prev_7d_empty")
                    : tr("insights.prev_7d_val", { val: `${period().rhr_prev!.toFixed(1)}` })
                }
                toneSurface="peach"
                accentColor={token("stamp-ink")}
                class="min-w-0 flex-1 border-transparent"
              />

              <BentoMetricCard
                compact={true}
                icon="moon"
                label={tr("today.metric_sleep")}
                value={period().sleep_current_h === null ? "—" : `${period().sleep_current_h!.toFixed(1)} h`}
                sublabel={
                  period().sleep_prev_h === null
                    ? tr("insights.prev_7d_empty")
                    : tr("insights.prev_7d_val", { val: `${period().sleep_prev_h!.toFixed(1)} h` })
                }
                toneSurface="lavender"
                accentColor={token("sleep-rem")}
                class="min-w-0 flex-1 border-transparent"
              />
            </View>
          </View>

          {/* 无框沉浸有机丝带主图区（直铺宣纸背景，告别 Card 白框套娃） */}
          <View class="gap-3 px-0.5">
            <View class="flex-row items-center justify-between gap-2">
              <Text role="header" class="text-base font-bold text-foreground">
                {tr("insights.rhythm_trend_title")}
              </Text>
              <Text class="text-xs font-semibold text-vital-prime">{snap().state_badge}</Text>
            </View>

            <InteractiveRhythmPlot points={snap().trend_14d} />

            <If when={Boolean(snap().sleep_stages_last_night)}>
              <View class="gap-2 rounded-2xl bg-bento-lavender/50 px-3.5 py-3">
                <Text class="text-xs font-semibold text-foreground">
                  {tr("insights.sleep_arch_title")}
                </Text>
                <SleepArchitectureBar
                  deepMin={snap().sleep_stages_last_night!.deep_min}
                  coreMin={snap().sleep_stages_last_night!.core_min}
                  remMin={snap().sleep_stages_last_night!.rem_min}
                  awakeMin={snap().sleep_stages_last_night!.awake_min}
                />
              </View>
            </If>
          </View>

          <Card>
            <SectionHeader
              title={tr("insights.experiments_title")}
              subtitle={tr("insights.experiments_subtitle")}
              right={<VitalBadge tone="neutral" label={tr("insights.experiments_badge")} />}
            />

            <If
              when={experiments().length > 0}
              else={
                <SunkenBox>
                  <Text class="text-xs text-muted-foreground">{tr("insights.experiments_empty")}</Text>
                </SunkenBox>
              }
            >
              <View class="gap-3">
                <Each of={experiments()} key={(e) => e.id}>
                  {(exp) => (
                    <SunkenBox>
                      <View class="flex-row items-center justify-between gap-2">
                        <View class="flex-1 gap-0.5">
                          <Text class="text-sm font-semibold text-foreground">{exp().title}</Text>
                          <Text
                            class="text-xs text-muted-foreground"
                            style={{ fontVariant: ["tabular-nums"] }}
                          >
                            {tr("insights.exp_progress_line", {
                              start: exp().start_date,
                              done: exp().completed_days,
                              target: exp().target_days,
                            })}
                          </Text>
                        </View>
                        <VitalBadge
                          tone={exp().checked_today ? "prime_ready" : "neutral"}
                          label={exp().checked_today ? tr("insights.exp_checked") : tr("insights.exp_pending")}
                        />
                      </View>

                      <MeterBar
                        value={progressPercent(exp().completed_days, exp().target_days)}
                        accessibilityLabel={exp().title}
                      />

                      <Text class="text-xs leading-relaxed text-muted-foreground">{exp().hypothesis}</Text>

                      <View class="flex-row gap-3 border-y border-border/55 py-2">
                        <View class="flex-1 gap-0.5">
                          <Text class="text-[11px] text-muted-foreground">{tr("today.metric_recovery")}</Text>
                          <Text
                            class="text-xs font-semibold text-foreground"
                            style={{ fontVariant: ["tabular-nums"] }}
                          >
                            {formatDelta(exp().hrv_delta_ms, "ms")}
                          </Text>
                        </View>
                        <View class="flex-1 gap-0.5 border-l border-border/55 pl-3">
                          <Text class="text-[11px] text-muted-foreground">{tr("today.rhr_label")}</Text>
                          <Text
                            class="text-xs font-semibold text-foreground"
                            style={{ fontVariant: ["tabular-nums"] }}
                          >
                            {formatDelta(exp().rhr_delta_bpm, "bpm", true)}
                          </Text>
                        </View>
                        <View class="flex-1 gap-0.5 border-l border-border/55 pl-3">
                          <Text class="text-[11px] text-muted-foreground">{tr("today.metric_title_sleep_deep")}</Text>
                          <Text
                            class="text-xs font-semibold text-foreground"
                            style={{ fontVariant: ["tabular-nums"] }}
                          >
                            {formatDelta(exp().deep_sleep_delta_min, tr("insights.min_unit"))}
                          </Text>
                        </View>
                      </View>

                      <Text class="text-xs font-medium text-primary">
                        {tr("insights.exp_washout_line", {
                          verdict: exp().verdict_summary,
                          washout: exp().washout_days,
                          skipped: exp().washout_skipped_days,
                        })}
                      </Text>

                      <ActionButton
                        title={
                          exp().checked_today
                            ? tr("insights.exp_toggle_done")
                            : tr("insights.exp_toggle_todo")
                        }
                        icon="paw"
                        variant={exp().checked_today ? "secondary" : "primary"}
                        onPress={() => toggleExperimentToday(exp().id)}
                      />
                    </SunkenBox>
                  )}
                </Each>
              </View>
            </If>

            <View class="gap-2 pt-1">
              <Text class="text-xs font-semibold text-foreground">{tr("insights.templates_title")}</Text>
              <View class="flex-row flex-wrap gap-2">
                <Each of={templates()} key={(tpl) => tpl.id}>
                  {(tpl) => (
                    <ActionButton
                      title={tr("insights.template_btn", { title: tpl().title })}
                      variant="outline"
                      onPress={() => startExperiment(tpl().title, tpl().hypothesis, tpl().target_days)}
                    />
                  )}
                </Each>
              </View>
            </View>
          </Card>

          <CollapsibleCard
            title={tr("insights.custom_exp_title")}
            subtitle={tr("insights.custom_exp_subtitle")}
            open={customOpen()}
            onToggle={() => customOpen.set(!customOpen())}
          >
            <FieldInput
              value={customTitle()}
              placeholder={tr("insights.custom_exp_name_placeholder")}
              onChange={(v) => customTitle.set(v)}
            />
            <FieldInput
              value={customHypothesis()}
              placeholder={tr("insights.custom_exp_hypothesis_placeholder")}
              onChange={(v) => customHypothesis.set(v)}
            />
            <View class="flex-row flex-wrap items-center justify-between gap-2">
              <View class="gap-1.5">
                <View class="flex-row flex-wrap gap-1.5">
                  <Each of={[7, 14, 21]} key={(d) => d}>
                    {(d) => (
                      <SelectChip
                        label={tr("common.days_unit", { value: d() })}
                        selected={customDays() === d()}
                        onPress={() => customDays.set(d())}
                      />
                    )}
                  </Each>
                </View>
                <View class="flex-row flex-wrap gap-1.5">
                  <Each of={[0, 1, 2, 3]} key={(w) => w}>
                    {(w) => (
                      <SelectChip
                        label={tr("insights.washout_chip", { days: w() })}
                        selected={customWashout() === w()}
                        onPress={() => customWashout.set(w())}
                      />
                    )}
                  </Each>
                </View>
              </View>
              <ActionButton title={tr("insights.start_experiment")} icon="plus" onPress={submitCustomExperiment} />
            </View>
          </CollapsibleCard>
        </View>
      </If>

      {/* Subview 2: Multi-Lag Correlations */}
      <If when={getInsightsSubTab() === "correlations"}>
        <View class="gap-4">
          <Card>
            <SectionHeader
              title={tr("insights.lag_title")}
              subtitle={tr("insights.lag_subtitle")}
            />
            <If when={snap().lagged_impacts.length === 0}>
              <SunkenBox>
                <Text class="text-xs text-muted-foreground">{tr("insights.lag_empty")}</Text>
              </SunkenBox>
            </If>
            <If when={snap().lagged_impacts.length > 0}>
              <View class="gap-2">
                <Each of={snap().lagged_impacts} key={(imp) => `${imp.tag}-${imp.outcome_label}`}>
                  {(imp) => (
                    <SunkenBox>
                      <View class="flex-row items-center justify-between gap-2">
                        <Text class="text-sm font-semibold text-foreground">{`${imp().tag} → ${imp().outcome_label}`}</Text>
                        <VitalBadge
                          tone={imp().delta_value >= 0 ? "optimal" : "watch_low"}
                          label={`${imp().delta_value >= 0 ? "+" : ""}${imp().delta_value.toFixed(1)} ${imp().unit}`}
                        />
                      </View>
                      <Text class="text-xs leading-relaxed text-muted-foreground">{imp().summary}</Text>
                    </SunkenBox>
                  )}
                </Each>
              </View>
            </If>
          </Card>

          <Card>
            <SectionHeader
              title={tr("insights.radar_title")}
              subtitle={tr("insights.radar_subtitle")}
              right={
                <VitalBadge
                  tone={hasFeature("multi_lag_radar") ? "prime_ready" : "neutral"}
                  label={
                    hasFeature("multi_lag_radar")
                      ? tr("insights.radar_full_badge")
                      : tr("insights.radar_preview_badge")
                  }
                />
              }
            />
            <MultiLagRadarBars
              title={tr("insights.radar_chart_title")}
              sampleDays={snap().trend_14d.length}
              items={(hasFeature("multi_lag_radar")
                ? state().multi_lag_radar
                : state().multi_lag_radar.slice(0, 2)
              ).map((it) => ({
                lag_label: `${it.tag} · ${it.lag_label} (${it.outcome_label})`,
                delta_value: it.delta_pct,
                unit: "%",
                cohen_d: it.cohen_d,
                sample_n: it.sample_n,
                confounder_note: it.confounder_note,
              }))}
            />
            <If when={!hasFeature("multi_lag_radar")}>
              <View class="flex-row items-center justify-between gap-2 pt-1">
                <Text class="flex-1 text-xs text-muted-foreground">{tr("insights.radar_upgrade_hint")}</Text>
                <ActionButton
                  title={tr("today.open_atelier")}
                  icon="sparkles"
                  size="sm"
                  variant="outline"
                  onPress={() => setShowAtelierModal(true)}
                />
              </View>
            </If>
          </Card>
        </View>
      </If>

      {/* Subview 3: Four Seasons Handscroll & Ceramic Badges */}
      <If when={getInsightsSubTab() === "handscroll"}>
        <View class="gap-4">
          <KeepsakeShelfCard themeId={getVisualTheme()} dewSummary={getDewSummary()} />

          <Card>
            <SectionHeader
              title={tr("insights.ceramic_title")}
              subtitle={tr("insights.ceramic_subtitle")}
              right={
                <ActionButton
                  title={tr("today.open_scroll")}
                  icon="handscroll"
                  size="sm"
                  variant="secondary"
                  onPress={() => setShowFourSeasonsScrollModal(true)}
                />
              }
            />
            <View class="flex-row flex-wrap gap-2">
              <Each of={state().solar_atelier.ceramic_badges} key={(b) => b.id}>
                {(b) => (
                  <SunkenBox class="min-w-44 flex-1">
                    <View class="flex-row items-center gap-1.5">
                      <BadgeCeramicIcon
                        size={16}
                        color={b().earned ? token("ink-lapis") : token("muted-foreground")}
                      />
                      <Text class="text-xs font-semibold text-foreground">{b().title}</Text>
                      <VitalBadge
                        tone={b().earned ? "prime_ready" : "neutral"}
                        label={b().earned ? tr("insights.badge_fired") : tr("insights.badge_kiln")}
                      />
                    </View>
                    <Text class="text-[11px] leading-relaxed text-muted-foreground">{b().reason}</Text>
                  </SunkenBox>
                )}
              </Each>
            </View>
            <View class="flex-row items-center gap-1.5 pt-1">
              <HandscrollIcon size={14} color={token("ink-lapis")} />
              <Text class="text-[11px] text-muted-foreground">{tr("insights.scroll_footer_note")}</Text>
            </View>
          </Card>
        </View>
      </If>
    </View>
  );
}
