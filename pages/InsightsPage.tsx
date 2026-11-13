import { signal, Text, token, View } from "nsh";
import {
  getDewSummary,
  getState,
  getVisualTheme,
  hasFeature,
  setShowAtelierModal,
  setShowFourSeasonsScrollModal,
  startCustomExperiment,
  startExperiment,
  toggleExperimentToday,
} from "../store";
import { MultiLagRadarBars } from "../ui/charts";
import {
  ActionButton,
  Card,
  CollapsibleCard,
  FieldInput,
  MeterBar,
  SectionHeader,
  SelectChip,
  SunkenBox,
  VitalBadge,
} from "../ui/components";
import { BadgeCeramicIcon, HandscrollIcon } from "../ui/icons";
import { BentoMetricCard, KeepsakeShelfCard } from "../ui/sanctuary";

function formatDelta(val: number | null, unit: string, lowerIsBetter = false): string {
  if (val === null || !Number.isFinite(val)) return "积累对比中";
  const sign = val > 0 ? "+" : "";
  const formatted = `${sign}${val.toFixed(1)} ${unit}`;
  if (Math.abs(val) < 0.2) return `${formatted} · 持平`;
  const improved = lowerIsBetter ? val < 0 : val > 0;
  return `${formatted} · ${improved ? "向好" : "留意"}`;
}

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

  const submitCustomExperiment = () => {
    const title = customTitle().trim();
    if (!title) return;
    const hypothesis = customHypothesis().trim() || "观察这项微习惯对夜间休息与次日精神状态的连续变化";
    startCustomExperiment(title, hypothesis, customDays(), customWashout());
    customTitle.set("");
    customHypothesis.set("");
    customOpen.set(false);
  };

  return (
    <View class="gap-4">
      {/* 0. 温柔零压力晨露藏品阁 */}
      <KeepsakeShelfCard themeId={getVisualTheme()} dewSummary={getDewSummary()} />

      {/* 1. 近 7 天 vs 前 7 天双周节律对照（便当贴视觉） */}
      <Card>
        <SectionHeader
          title="近 7 天 vs 前 7 天节律对照"
          subtitle={`${period().current_window} 对比 ${period().previous_window}`}
          right={<VitalBadge tone="neutral" label="双周对比" />}
        />

        <View class="flex-row flex-wrap gap-2.5">
          <BentoMetricCard
            icon="heart_pulse"
            label="夜间恢复节律"
            value={period().hrv_current === null ? "—" : `${period().hrv_current!.toFixed(1)} ms`}
            sublabel={
              period().hrv_prev === null ? "前 7 天暂无记录" : `前 7 天 ${period().hrv_prev!.toFixed(1)} ms`
            }
            toneSurface="sage"
            accentColor={token("vital-prime")}
            class="min-w-28 flex-1"
          />

          <BentoMetricCard
            icon="flame"
            label="夜间静息心率"
            value={period().rhr_current === null ? "—" : `${period().rhr_current!.toFixed(1)} bpm`}
            sublabel={
              period().rhr_prev === null ? "前 7 天暂无记录" : `前 7 天 ${period().rhr_prev!.toFixed(1)} bpm`
            }
            toneSurface="peach"
            accentColor={token("stamp-ink")}
            class="min-w-28 flex-1"
          />

          <BentoMetricCard
            icon="moon"
            label="平均夜间好眠"
            value={period().sleep_current_h === null ? "—" : `${period().sleep_current_h!.toFixed(1)} h`}
            sublabel={
              period().sleep_prev_h === null ? "前 7 天暂无记录" : `前 7 天 ${period().sleep_prev_h!.toFixed(1)} h`
            }
            toneSurface="lavender"
            accentColor={token("sleep-rem")}
            class="min-w-28 flex-1"
          />
        </View>

        <SunkenBox>
          <Text class="text-xs leading-relaxed text-foreground">{period().summary}</Text>
        </SunkenBox>
      </Card>

      {/* 2. N-of-1 个人微习惯实验室 */}
      <Card>
        <SectionHeader
          title="个人微习惯实验"
          subtitle="把好习惯变成一次轻盈小实验，看看你自己的身体如何回应"
          right={<VitalBadge tone="neutral" label="个人实证" />}
        />

        <If
          when={experiments().length > 0}
          else={
            <SunkenBox>
              <Text class="text-xs text-muted-foreground">
                暂无进行中的微习惯实验，可从下方推荐模板一键开启，或折叠展开自定义实验。
              </Text>
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
                        {`始于 ${exp().start_date} · 已坚持 ${exp().completed_days}/${exp().target_days} 天`}
                      </Text>
                    </View>
                    <VitalBadge
                      tone={exp().checked_today ? "prime_ready" : "neutral"}
                      label={exp().checked_today ? "今日已打卡" : "今日未打卡"}
                    />
                  </View>

                  <MeterBar
                    value={progressPercent(exp().completed_days, exp().target_days)}
                    accessibilityLabel={`${exp().title} 完成进度`}
                  />

                  <Text class="text-xs leading-relaxed text-muted-foreground">{exp().hypothesis}</Text>

                  <View class="flex-row gap-3 border-y border-border/55 py-2">
                    <View class="flex-1 gap-0.5">
                      <Text class="text-[11px] text-muted-foreground">夜间恢复</Text>
                      <Text
                        class="text-xs font-semibold text-foreground"
                        style={{ fontVariant: ["tabular-nums"] }}
                      >
                        {formatDelta(exp().hrv_delta_ms, "ms")}
                      </Text>
                    </View>
                    <View class="flex-1 gap-0.5 border-l border-border/55 pl-3">
                      <Text class="text-[11px] text-muted-foreground">静息心率</Text>
                      <Text
                        class="text-xs font-semibold text-foreground"
                        style={{ fontVariant: ["tabular-nums"] }}
                      >
                        {formatDelta(exp().rhr_delta_bpm, "bpm", true)}
                      </Text>
                    </View>
                    <View class="flex-1 gap-0.5 border-l border-border/55 pl-3">
                      <Text class="text-[11px] text-muted-foreground">深睡时长</Text>
                      <Text
                        class="text-xs font-semibold text-foreground"
                        style={{ fontVariant: ["tabular-nums"] }}
                      >
                        {formatDelta(exp().deep_sleep_delta_min, "分钟")}
                      </Text>
                    </View>
                  </View>

                  <Text class="text-xs font-medium text-primary">
                    {`${exp().verdict_summary} · 洗脱期 ${exp().washout_days} 天（跳过 ${exp().washout_skipped_days} 天残余期）`}
                  </Text>

                  <ActionButton
                    title={exp().checked_today ? "今日实验已打卡 ✓" : "今日实验打卡"}
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
          <Text class="text-xs font-semibold text-foreground">推荐实验模板 · 一键开启</Text>
          <View class="flex-row flex-wrap gap-2">
            <Each of={templates()} key={(tpl) => tpl.id}>
              {(tpl) => (
                <ActionButton
                  title={`+ 开启：${tpl().title}`}
                  variant="outline"
                  onPress={() => startExperiment(tpl().title, tpl().hypothesis, tpl().target_days)}
                />
              )}
            </Each>
          </View>
        </View>
      </Card>

      {/* 可折叠：创建自定义微习惯实验 */}
      <CollapsibleCard
        title="创建自定义微习惯实验"
        subtitle="按自己的生活节奏设定 7 / 14 / 21 天观察实验"
        open={customOpen()}
        onToggle={() => customOpen.set(!customOpen())}
      >
        <FieldInput
          value={customTitle()}
          placeholder="实验名称，如：睡前温水泡脚 15 分钟"
          onChange={(v) => customTitle.set(v)}
        />
        <FieldInput
          value={customHypothesis()}
          placeholder="想观察的改变（可选），如：看看夜间深睡与晨起精神是否更轻快"
          onChange={(v) => customHypothesis.set(v)}
        />
        <View class="flex-row flex-wrap items-center justify-between gap-2">
          <View class="gap-1.5">
            <View class="flex-row flex-wrap gap-1.5">
              <Each of={[7, 14, 21]} key={(d) => d}>
                {(d) => (
                  <SelectChip
                    label={`${d()} 天`}
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
                    label={`洗脱 ${w()} 天`}
                    selected={customWashout() === w()}
                    onPress={() => customWashout.set(w())}
                  />
                )}
              </Each>
            </View>
          </View>
          <ActionButton title="创建实验" icon="plus" onPress={submitCustomExperiment} />
        </View>
      </CollapsibleCard>

      {/* 3. D -> D+1 个人身体规律发现 */}
      <Card>
        <SectionHeader
          title="D → D+1 个人身体规律发现"
          subtitle="结合日常饮食、作息记录与次日晨起体征反馈自然整理"
        />
        <If when={snap().lagged_impacts.length === 0}>
          <SunkenBox>
            <Text class="text-xs text-muted-foreground">
              再随手记录 2–3 天咖啡因、晚间加餐或运动节奏，系统会自动总结你的次日身体反馈规律。
            </Text>
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

      {/* 4. v4.0 90 天 D+0/D+1/D+2 多滞后归因雷达 (Cohen's d) */}
      <Card>
        <SectionHeader
          title="90 天多滞后归因雷达 (D+0 / D+1 / D+2)"
          subtitle="剥离晚睡与酒精混杂效应，定量检验日常习惯的真实 Cohen's d 效应量"
          right={
            <VitalBadge
              tone={hasFeature("multi_lag_radar") ? "prime_ready" : "neutral"}
              label={hasFeature("multi_lag_radar") ? "雅集全量雷达" : "清欢精选预览"}
            />
          }
        />
        <MultiLagRadarBars
          title="重点行为跨天影响"
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
            <Text class="flex-1 text-xs text-muted-foreground">
              当前免费展示首条主因；开启「知气雅集」可解锁 90 天 D+0/D+1/D+2 全维度剥离雷达。
            </Text>
            <ActionButton
              title="查看雅集方案"
              icon="sparkles"
              size="sm"
              variant="outline"
              onPress={() => setShowAtelierModal(true)}
            />
          </View>
        </If>
      </Card>

      {/* 5. 宋瓷实验勋章（「证伪小达人」）与四时风物长卷 */}
      <Card>
        <SectionHeader
          title="宋瓷微习惯勋章 & 四时风物长卷"
          subtitle="哪怕实验证明「某件事其实没那么有用」，同样值得一枚清醒的「证伪小达人」勋章"
          right={
            <ActionButton
              title="展开长卷"
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
                    label={b().earned ? "已烧制" : "蓄力中"}
                  />
                </View>
                <Text class="text-[11px] leading-relaxed text-muted-foreground">{b().reason}</Text>
              </SunkenBox>
            )}
          </Each>
        </View>
        <View class="flex-row items-center gap-1.5 pt-1">
          <HandscrollIcon size={14} color={token("ink-lapis")} />
          <Text class="text-[11px] text-muted-foreground">
            四时身体风物长卷将持续收集你的春生、夏长、秋收、冬藏高光足迹。
          </Text>
        </View>
      </Card>
    </View>
  );
}
