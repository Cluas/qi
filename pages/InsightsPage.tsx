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
  setShowQuickCaptureModal,
  startCustomExperiment,
  startExperiment,
  toggleExperimentToday,
} from "../store";
import { InteractiveRhythmPlot, MultiLagRadarBars } from "../ui/charts";
import {
  ActionButton,
  Button,
  Card,
  FieldInput,
  Hairline,
  HeaderActionPill,
  IosGroupedSection,
  IosLargeHeader,
  IosListRow,
  MeterBar,
  SectionHeader,
  SegmentedTabs,
  SelectChip,
  StatMetricTile,
  VitalBadge,
} from "../ui/components";
import { BadgeCeramicIcon, HandscrollIcon, QiIcon } from "../ui/icons";
import { isEnglishLocale, tr } from "../ui/i18n";
import { KeepsakeShelfCard } from "../ui/sanctuary";

function progressPercent(completed: number, target: number): number {
  if (!Number.isFinite(target) || target <= 0) return 0;
  return Math.max(0, Math.min(100, Math.round((completed / target) * 100)));
}

export function InsightsPage() {
  const customTitle = signal("");
  const customHypothesis = signal("");
  const customDays = signal(7);
  const customWashout = signal(2);
  const expDetailOpen = signal(false);

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
    expDetailOpen.set(false);
  };

  return (
    <View class="gap-4">
      <IosLargeHeader
        title={tr("nav.insights")}
        trailing={
          <HeaderActionPill
            label={tr("nav.quick_short")}
            icon="plus"
            tone="primary"
            accessibilityLabel={tr("nav.quick_capture")}
            onPress={() => setShowQuickCaptureModal(true)}
          />
        }
      />

      <SegmentedTabs
        items={subTabs()}
        activeId={getInsightsSubTab()}
        onSelect={(id) => setInsightsSubTab(id)}
      />

      {/* Subview 1: Rhythm Curves & N-of-1 Experiments (v7.0 Radical Subtraction: 3 calm cards) */}
      <If when={getInsightsSubTab() === "curves"}>
        <View class="gap-5">
          {/* Card 1: 元气与睡眠节律（唯一主曲线，无重复睡眠结构块） */}
          <Card class="gap-3.5 rounded-3xl p-4">
            <View class="flex-row items-center justify-between gap-2">
              <Text role="header" class="text-base font-bold text-foreground">
                {tr("insights.rhythm_trend_title")}
              </Text>
              <VitalBadge
                tone={
                  snap().state === "recovery_priority"
                    ? "recovery_priority"
                    : snap().state === "moderate_steady"
                      ? "moderate_steady"
                      : "prime_ready"
                }
                label={snap().state_badge}
              />
            </View>

            <InteractiveRhythmPlot points={snap().trend_14d} />
          </Card>

          {/* Card 2: 近两周节律起伏对比 */}
          <Card class="gap-3.5 rounded-3xl p-4">
            <View class="gap-1.5">
              <View class="flex-row items-center justify-between gap-2">
                <Text role="header" class="text-xl font-bold tracking-tight text-foreground sm:text-2xl">
                  {tr("insights.period_title")}
                </Text>
                <VitalBadge tone="neutral" label={tr("insights.period_badge")} />
              </View>

              <Text class="pt-0.5 text-[15px] leading-relaxed text-foreground/90">
                {period().summary}
              </Text>
            </View>

            <View class="flex-row gap-2">
              <StatMetricTile
                icon="heart_pulse"
                label={tr("today.metric_title_hrv_rmssd").replace(" (RMSSD)", "")}
                value={period().hrv_current === null ? "—" : `${period().hrv_current!.toFixed(1)} ms`}
                sublabel={
                  period().hrv_prev === null
                    ? tr("insights.prev_7d_empty")
                    : tr("insights.prev_7d_val", { val: `${period().hrv_prev!.toFixed(1)}` })
                }
                tone="optimal"
              />

              <StatMetricTile
                icon="flame"
                label={tr("today.metric_title_rhr")}
                value={period().rhr_current === null ? "—" : `${period().rhr_current!.toFixed(1)} bpm`}
                sublabel={
                  period().rhr_prev === null
                    ? tr("insights.prev_7d_empty")
                    : tr("insights.prev_7d_val", { val: `${period().rhr_prev!.toFixed(1)}` })
                }
                tone="watch_high"
              />

              <StatMetricTile
                icon="moon"
                label={tr("today.metric_sleep")}
                value={period().sleep_current_h === null ? "—" : `${period().sleep_current_h!.toFixed(1)} h`}
                sublabel={
                  period().sleep_prev_h === null
                    ? tr("insights.prev_7d_empty")
                    : tr("insights.prev_7d_val", { val: `${period().sleep_prev_h!.toFixed(1)} h` })
                }
                tone="neutral"
              />
            </View>
          </Card>

          {/* Card 3: 个人 N-of-1 小实验（极简表层 + 折叠详情/模板/新建） */}
          <Card class="gap-3.5 rounded-3xl p-4">
            <SectionHeader
              title={tr("insights.experiments_title")}
              right={<VitalBadge tone="neutral" label={tr("insights.experiments_badge")} />}
            />

            <If
              when={experiments().length > 0}
              else={
                <View class="rounded-2xl bg-secondary/45 px-3.5 py-3">
                  <Text class="text-xs text-muted-foreground">{tr("insights.experiments_empty")}</Text>
                </View>
              }
            >
              <View class="gap-3">
                <Each of={experiments()} key={(e) => e.id}>
                  {(exp) => (
                    <View class="gap-2.5 rounded-2xl bg-secondary/45 p-3.5">
                      <View class="flex-row items-center justify-between gap-2">
                        <Text class="flex-1 text-sm font-semibold text-foreground">{exp().title}</Text>
                        <VitalBadge
                          tone={exp().checked_today ? "prime_ready" : "neutral"}
                          label={exp().checked_today ? tr("insights.exp_checked") : tr("insights.exp_pending")}
                        />
                      </View>

                      <MeterBar
                        value={progressPercent(exp().completed_days, exp().target_days)}
                        accessibilityLabel={exp().title}
                      />

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
                    </View>
                  )}
                </Each>
              </View>
            </If>
            <Hairline />

            <Button
              hitSlop={6}
              accessibilityLabel={
                isEnglishLocale()
                  ? "Trial metrics, templates & new trial"
                  : "对照指标、洗脱详情与新建实验"
              }
              onPress={() => expDetailOpen.set(!expDetailOpen())}
              class="min-h-10 flex-row items-center justify-between gap-2 rounded-xl px-1 py-1 active:opacity-75"
            >
              <View class="flex-1 gap-0.5">
                <Text class="text-xs font-semibold text-foreground">
                  {isEnglishLocale()
                    ? "Trial metrics, templates & new trial"
                    : "对照指标、洗脱详情与新建实验"}
                </Text>
                <Text class="text-[11px] text-muted-foreground">
                  {tr("insights.custom_exp_subtitle")}
                </Text>
              </View>
              <View class={`transition-transform duration-150 ${expDetailOpen() ? "rotate-180" : ""}`}>
                <QiIcon name="chevron_down" size={14} color={token("muted-foreground")} />
              </View>
            </Button>

            <If when={expDetailOpen()}>
              <View class="gap-3.5 pt-1">
                <If when={experiments().length > 0}>
              <View class="gap-2.5">
                <Each of={experiments()} key={(e) => `detail-${e.id}`}>
                  {(exp) => (
                    <View class="gap-2 rounded-2xl bg-secondary/45 p-3.5">
                      <View class="flex-row items-center justify-between gap-2">
                        <Text class="text-xs font-semibold text-foreground">{exp().title}</Text>
                        <Text
                          class="text-[11px] text-muted-foreground"
                          style={{ fontVariant: ["tabular-nums"] }}
                        >
                          {tr("insights.exp_progress_line", {
                            start: exp().start_date,
                            done: exp().completed_days,
                            target: exp().target_days,
                          })}
                        </Text>
                      </View>

                      <Text class="text-xs leading-relaxed text-muted-foreground">
                        {exp().hypothesis}
                      </Text>

                      <View class="flex-row gap-3 rounded-xl bg-card/85 px-3 py-2.5">
                        <View class="flex-1 gap-0.5">
                          <Text class="text-[11px] text-muted-foreground">{tr("today.metric_recovery")}</Text>
                          <Text
                            class="text-xs font-semibold text-foreground"
                            style={{ fontVariant: ["tabular-nums"] }}
                          >
                            {formatDelta(exp().hrv_delta_ms, "ms")}
                          </Text>
                        </View>
                        <View class="flex-1 gap-0.5">
                          <Text class="text-[11px] text-muted-foreground">{tr("today.rhr_label")}</Text>
                          <Text
                            class="text-xs font-semibold text-foreground"
                            style={{ fontVariant: ["tabular-nums"] }}
                          >
                            {formatDelta(exp().rhr_delta_bpm, "bpm", true)}
                          </Text>
                        </View>
                        <View class="flex-1 gap-0.5">
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
                    </View>
                  )}
                </Each>
              </View>
            </If>

            <View class="gap-2">
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

            <View class="gap-2.5 pt-1">
              <Text class="text-xs font-semibold text-foreground">{tr("insights.custom_exp_title")}</Text>
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
            </View>
              </View>
            </If>
          </Card>
        </View>
      </If>

      {/* Subview 2: Multi-Lag Correlations */}
      <If when={getInsightsSubTab() === "correlations"}>
        <View class="gap-4">
          <IosGroupedSection
            title={tr("insights.lag_title")}
            footer={tr("insights.lag_subtitle")}
          >
            <If
              when={snap().lagged_impacts.length > 0}
              else={
                <View class="px-4 py-4">
                  <Text class="text-xs text-muted-foreground">{tr("insights.lag_empty")}</Text>
                </View>
              }
            >
              <Each
                of={snap().lagged_impacts}
                key={(imp) => `${imp.tag}-${imp.outcome_label}`}
              >
                {(imp, idx) => (
                  <IosListRow
                    icon="heart_pulse"
                    iconBgColor={
                      imp().delta_value >= 0
                        ? token("vital-prime-soft")
                        : token("vital-recover-soft")
                    }
                    iconColor={
                      imp().delta_value >= 0
                        ? token("vital-prime")
                        : token("vital-recover")
                    }
                    title={`${imp().tag} → ${imp().outcome_label}`}
                    subtitle={imp().summary}
                    divider={idx() < snap().lagged_impacts.length - 1}
                    trailing={
                      <VitalBadge
                        tone={imp().delta_value >= 0 ? "optimal" : "watch_low"}
                        label={`${imp().delta_value >= 0 ? "+" : ""}${imp().delta_value.toFixed(1)} ${imp().unit}`}
                      />
                    }
                  />
                )}
              </Each>
            </If>
          </IosGroupedSection>

          <Card class="gap-3.5 rounded-3xl p-4">
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

          <Card class="gap-3.5 rounded-3xl p-4">
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
                  <View class="min-w-44 flex-1 gap-1.5 rounded-2xl bg-secondary/45 p-3.5">
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
                  </View>
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
