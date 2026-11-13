import { signal, Text, View } from "nsh";
import { getState, startExperiment, toggleExperimentToday } from "../store";
import { ActionButton, Card, FieldInput, SectionHeader, SelectChip, SunkenBox, VitalBadge } from "../ui/components";

function formatDelta(val: number | null, unit: string, lowerIsBetter = false): string {
  if (val === null) return "样本累积中";
  const sign = val > 0 ? "+" : "";
  const formatted = `${sign}${val.toFixed(1)} ${unit}`;
  if (Math.abs(val) < 0.2) return `${formatted} (持平)`;
  const improved = lowerIsBetter ? val < 0 : val > 0;
  return `${formatted} ${improved ? "↑改善" : "↓留意"}`;
}

export function InsightsPage() {
  const customTitle = signal("");
  const customHypothesis = signal("");
  const customDays = signal(7);

  const state = () => getState();
  const snap = () => state().snapshot;
  const period = () => state().period_comparison;
  const experiments = () => state().experiments;
  const templates = () => state().experiment_templates;

  const submitCustomExperiment = () => {
    const title = customTitle().trim();
    if (!title) return;
    const hypothesis = customHypothesis().trim() || "观察该微习惯对夜间 HRV 与次日静息心率的连续影响";
    startExperiment(title, hypothesis, customDays());
    customTitle.set("");
    customHypothesis.set("");
  };

  return (
    <View class="gap-4">
      {/* 1. 近 7 天 vs 前 7 天周期对照 */}
      <Card>
        <SectionHeader
          title="近 7 天 vs 前 7 天节律对照"
          subtitle={`${period().current_window} 对比 ${period().previous_window}`}
          right={<VitalBadge tone="prime_ready" label="双周对比" />}
        />

        <View class="flex-row gap-2">
          <SunkenBox class="flex-1">
            <Text class="text-xs text-muted-foreground">夜间 HRV 均值</Text>
            <Text class="text-base font-bold text-foreground" style={{ fontVariant: ["tabular-nums"] }}>
              {period().hrv_current === null ? "—" : `${period().hrv_current!.toFixed(1)} ms`}
            </Text>
            <Text class="text-[11px] text-muted-foreground">
              {period().hrv_prev === null ? "前 7 天无数据" : `前 7 天 ${period().hrv_prev!.toFixed(1)} ms`}
            </Text>
          </SunkenBox>

          <SunkenBox class="flex-1">
            <Text class="text-xs text-muted-foreground">静息心率 RHR</Text>
            <Text class="text-base font-bold text-foreground" style={{ fontVariant: ["tabular-nums"] }}>
              {period().rhr_current === null ? "—" : `${period().rhr_current!.toFixed(1)} bpm`}
            </Text>
            <Text class="text-[11px] text-muted-foreground">
              {period().rhr_prev === null ? "前 7 天无数据" : `前 7 天 ${period().rhr_prev!.toFixed(1)} bpm`}
            </Text>
          </SunkenBox>

          <SunkenBox class="flex-1">
            <Text class="text-xs text-muted-foreground">平均夜间睡眠</Text>
            <Text class="text-base font-bold text-foreground" style={{ fontVariant: ["tabular-nums"] }}>
              {period().sleep_current_h === null ? "—" : `${period().sleep_current_h!.toFixed(1)} h`}
            </Text>
            <Text class="text-[11px] text-muted-foreground">
              {period().sleep_prev_h === null ? "前 7 天无数据" : `前 7 天 ${period().sleep_prev_h!.toFixed(1)} h`}
            </Text>
          </SunkenBox>
        </View>

        <SunkenBox>
          <Text class="text-xs leading-relaxed text-foreground">{period().summary}</Text>
        </SunkenBox>
      </Card>

      {/* 2. N-of-1 个人微习惯实验室 */}
      <Card>
        <SectionHeader
          title="N-of-1 个人微习惯实验室"
          subtitle="用你自己的身体做对照实验，自动对比打卡日与基线日的生理变化"
          right={<VitalBadge tone="moderate_steady" label="科学实证" />}
        />

        <View class="gap-2.5">
          <Each of={experiments()} key={(e) => e.id}>
            {(exp) => (
              <SunkenBox>
                <View class="flex-row items-center justify-between gap-2">
                  <View class="flex-1">
                    <Text class="text-sm font-semibold text-foreground">{exp().title}</Text>
                    <Text class="text-xs text-muted-foreground">
                      {`始于 ${exp().start_date} · 已坚持 ${exp().completed_days}/${exp().target_days} 天`}
                    </Text>
                  </View>
                  <VitalBadge
                    tone={exp().checked_today ? "prime_ready" : "neutral"}
                    label={exp().checked_today ? "今日已打卡" : "今日未打卡"}
                  />
                </View>

                <Text class="text-xs leading-relaxed text-muted-foreground">{exp().hypothesis}</Text>

                <View class="flex-row flex-wrap gap-2">
                  <View class="rounded-lg border border-border bg-card px-2.5 py-1.5">
                    <Text class="text-[11px] text-muted-foreground">HRV 变化</Text>
                    <Text class="text-xs font-semibold text-foreground">
                      {formatDelta(exp().hrv_delta_ms, "ms")}
                    </Text>
                  </View>
                  <View class="rounded-lg border border-border bg-card px-2.5 py-1.5">
                    <Text class="text-[11px] text-muted-foreground">RHR 变化</Text>
                    <Text class="text-xs font-semibold text-foreground">
                      {formatDelta(exp().rhr_delta_bpm, "bpm", true)}
                    </Text>
                  </View>
                  <View class="rounded-lg border border-border bg-card px-2.5 py-1.5">
                    <Text class="text-[11px] text-muted-foreground">深度睡眠</Text>
                    <Text class="text-xs font-semibold text-foreground">
                      {formatDelta(exp().deep_sleep_delta_min, "分钟")}
                    </Text>
                  </View>
                </View>

                <Text class="text-xs font-medium text-primary">{exp().verdict_summary}</Text>

                <ActionButton
                  title={exp().checked_today ? "今日实验已打卡 ✓" : "今日实验打卡"}
                  variant={exp().checked_today ? "secondary" : "primary"}
                  onPress={() => toggleExperimentToday(exp().id)}
                />
              </SunkenBox>
            )}
          </Each>
        </View>

        <View class="gap-2 pt-1">
          <Text class="text-xs font-semibold text-foreground">一键发起推荐实验模板</Text>
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

        <SunkenBox>
          <Text class="text-xs font-semibold text-foreground">自定义微习惯对照实验</Text>
          <FieldInput
            value={customTitle()}
            placeholder="实验名称，如：睡前温水泡脚 15 分钟"
            onChange={(v) => customTitle.set(v)}
          />
          <FieldInput
            value={customHypothesis()}
            placeholder="预期验证假设（可选），如：观察夜间静息心率与深度睡眠占比"
            onChange={(v) => customHypothesis.set(v)}
          />
          <View class="flex-row items-center justify-between gap-2">
            <View class="flex-row gap-1.5">
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
            <ActionButton title="创建实验" onPress={submitCustomExperiment} />
          </View>
        </SunkenBox>
      </Card>

      {/* 3. D日行为 -> D+1 滞后归因清单 */}
      <Card>
        <SectionHeader
          title="D → D+1 个人节律归因发现"
          subtitle="直接从你本地记录的摄入事件与次日晨起 HRV/RHR/睡眠关联统计得出"
        />
        <If when={snap().lagged_impacts.length === 0}>
          <SunkenBox>
            <Text class="text-xs text-muted-foreground">
              再连续记录 2–3 天咖啡因、夜宵或运动节奏后，系统将自动计算属于你的次日生理滞后差值。
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
                      tone={imp().delta_value >= 0 ? "prime_ready" : "recovery_priority"}
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
    </View>
  );
}
