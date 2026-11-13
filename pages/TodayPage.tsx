import { signal, Text, token, View } from "nsh";
import {
  getState,
  refreshLiveBriefing,
  setHardwareTier,
  setTab,
  toggleTodayAction,
} from "../store";
import {
  AcwrSweetSpotCanvas,
  InteractiveRhythmPlot,
  SleepArchitectureBar,
  VitalityDialCanvas,
} from "../ui/charts";
import {
  ActionButton,
  Card,
  CollapsibleCard,
  ConsumerStatusPill,
  SectionHeader,
  SelectChip,
  StatMetricTile,
  SunkenBox,
  VitalBadge,
} from "../ui/components";
import { QiIcon } from "../ui/icons";

const GOAL_LABELS: Record<string, string> = {
  energy_sleep: "早安精力与好眠修复",
  diet_balance: "随手记餐与吃练平衡",
  lab_archive: "体检报告长期私密建档",
  smart_training: "运动节奏与温和防过劳",
};

const FEELING_LEVELS = [
  { score: 1, label: "1 疲惫" },
  { score: 2, label: "2 略倦" },
  { score: 3, label: "3 平稳" },
  { score: 4, label: "4 轻快" },
  { score: 5, label: "5 充沛" },
];

const CONSUMER_METRIC_TITLES: Record<string, string> = {
  hrv_rmssd_ms: "夜间恢复节律",
  hrv_sdnn_ms: "日常心率变异性",
  resting_heart_rate_bpm: "夜间静息心率",
  sleep_minutes: "昨晚睡眠总时长",
  sleep_deep_minutes: "深睡修复时长",
  sleep_rem_minutes: "眼动梦境时长",
  steps: "每日步行数",
  active_zone_minutes: "中高强度活动",
  calories_active_kcal: "日常活动消耗",
};

function friendlyLoadStatus(acwr: number | null): string {
  if (acwr === null || !Number.isFinite(acwr)) return "轻盈积累中";
  if (acwr < 0.8) return "处于恢复期";
  if (acwr <= 1.3) return "黄金适度区";
  return "偏累预警中";
}

function friendlySleepDebt(debt: number | null): string {
  if (debt === null || !Number.isFinite(debt)) return "未佩戴睡眠监测";
  if (debt > 0.3) return `近期少睡约 ${debt.toFixed(1)} 小时`;
  if (debt < -0.3) return `睡眠盈余 ${Math.abs(debt).toFixed(1)} 小时`;
  return "作息储备均衡";
}

function formatMetricReading(value: number | null, unit: string): string {
  if (value === null || !Number.isFinite(value)) return "—";
  const rounded = Math.abs(value) >= 100 ? `${Math.round(value)}` : `${Number(value.toFixed(1))}`;
  return unit ? `${rounded} ${unit}` : rounded;
}

export function TodayPage() {
  const baselineOpen = signal(false);

  const state = () => getState();
  const snap = () => state().snapshot;
  const settings = () => state().settings;
  const briefing = () => state().briefing;

  const goalLabel = () => GOAL_LABELS[settings().primary_goal] ?? "全面节律平衡";

  const recoveryTile = () => {
    const m = snap().metrics;
    const hrv = m.hrv_rmssd_ms ?? m.hrv_sdnn_ms;
    if (hrv && hrv.value !== null) {
      return {
        value: `${Math.round(hrv.value)} ms`,
        sublabel: hrv.baseline_30d_mean !== null
          ? `平时约 ${Math.round(hrv.baseline_30d_mean)} ms`
          : "夜间恢复节律",
        tone: hrv.status,
      };
    }
    const rhr = m.resting_heart_rate_bpm;
    if (rhr && rhr.value !== null) {
      return {
        value: `${Math.round(rhr.value)} bpm`,
        sublabel: rhr.baseline_30d_mean !== null
          ? `平时静息 ${Math.round(rhr.baseline_30d_mean)} bpm`
          : "夜间静息心率",
        tone: rhr.status,
      };
    }
    return {
      value: `${settings().morning_feeling} / 5`,
      sublabel: "晨起精神感受",
      tone: settings().morning_feeling >= 4 ? "optimal" : settings().morning_feeling >= 3 ? "normal" : "watch_low",
    };
  };

  const sleepTile = () => {
    const h = snap().sleep_last_night_hours;
    if (h === null || !Number.isFinite(h)) {
      return {
        value: "未采集",
        sublabel: "仅使用手机计步",
        tone: "no_baseline",
      };
    }
    return {
      value: `${h.toFixed(1)} 小时`,
      sublabel: friendlySleepDebt(snap().sleep_debt_7d_hours),
      tone: h >= 7.0 ? "optimal" : h >= 6.0 ? "normal" : "watch_low",
    };
  };

  const loadTile = () => {
    const m = snap().metrics;
    const steps = m.steps;
    const acwr = snap().acwr;
    if (steps && steps.value !== null) {
      return {
        value: `${Math.round(steps.value)} 步`,
        sublabel: acwr !== null ? friendlyLoadStatus(acwr) : "今日步数记录",
        tone: acwr === null ? "normal" : acwr <= 1.3 && acwr >= 0.8 ? "optimal" : acwr > 1.3 ? "alert_high" : "watch_low",
      };
    }
    return {
      value: friendlyLoadStatus(acwr),
      sublabel: "近 7 天活动负荷",
      tone: acwr === null ? "no_baseline" : acwr <= 1.3 ? "optimal" : "alert_high",
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

  return (
    <View class="gap-4">
      {/* 顶部时段问候 + 日期 + 聚焦目标 */}
      <View class="flex-row items-end justify-between gap-3 px-0.5">
        <View class="flex-1 gap-0.5">
          <Text class="text-xl font-bold text-foreground">晨安 · 今日节律</Text>
          <Text class="text-xs text-muted-foreground">{`聚焦方向：${goalLabel()}`}</Text>
        </View>
        <View class="rounded-full border border-border/70 bg-card px-3 py-1">
          <Text class="text-xs font-medium text-muted-foreground" style={{ fontVariant: ["tabular-nums"] }}>
            {snap().target_date}
          </Text>
        </View>
      </View>

      {/* 1. 今日元气主视觉卡片 */}
      <Card>
        <View class="flex-row items-center gap-4">
          <VitalityDialCanvas
            score={snap().qi_score}
            state={snap().state}
            badge={snap().state_badge}
          />
          <View class="flex-1 gap-2">
            <View class="flex-row items-center justify-between gap-2">
              <Text class="text-xs font-medium text-muted-foreground">今日元气指数</Text>
              <VitalBadge tone={snap().state} label={snap().state_badge} />
            </View>
            <View class="flex-row items-baseline gap-1.5">
              <Text class="text-3xl font-bold text-foreground" style={{ fontVariant: ["tabular-nums"] }}>
                {`${snap().qi_score}`}
              </Text>
              <Text class="text-sm font-medium text-muted-foreground">/ 100</Text>
            </View>
            <Text class="text-xs leading-relaxed text-foreground">{snap().headline}</Text>
          </View>
        </View>

        {/* 三大体征磁贴 */}
        <View class="flex-row gap-2.5">
          <StatMetricTile
            icon="heart_pulse"
            label="夜间恢复"
            value={recoveryTile().value}
            sublabel={recoveryTile().sublabel}
            tone={recoveryTile().tone}
          />
          <StatMetricTile
            icon="moon"
            label="昨晚好眠"
            value={sleepTile().value}
            sublabel={sleepTile().sublabel}
            tone={sleepTile().tone}
          />
          <StatMetricTile
            icon="flame"
            label="身体负荷"
            value={loadTile().value}
            sublabel={loadTile().sublabel}
            tone={loadTile().tone}
          />
        </View>

        {/* 睡眠四期结构条（存在睡眠分期时展示） */}
        <If when={Boolean(snap().sleep_stages_last_night)}>
          <SunkenBox>
            <View class="flex-row items-center justify-between">
              <Text class="text-xs font-semibold text-foreground">昨晚睡眠结构</Text>
              <Text class="text-xs text-muted-foreground">深度修复与梦境分布</Text>
            </View>
            <SleepArchitectureBar
              deepMin={snap().sleep_stages_last_night!.deep_min}
              coreMin={snap().sleep_stages_last_night!.core_min}
              remMin={snap().sleep_stages_last_night!.rem_min}
              awakeMin={snap().sleep_stages_last_night!.awake_min}
            />
          </SunkenBox>
        </If>

        {/* 大众版负荷甜区轨道（非纯手机模式且存在负荷数据时展示） */}
        <If when={snap().acwr !== null}>
          <SunkenBox>
            <View class="flex-row items-center justify-between">
              <Text class="text-xs font-semibold text-foreground">近期身体负荷节奏</Text>
              <Text class="text-xs font-medium text-primary">{friendlyLoadStatus(snap().acwr)}</Text>
            </View>
            <AcwrSweetSpotCanvas acwr={snap().acwr} />
          </SunkenBox>
        </If>

        {/* 唯一自然例外：当用户当前处于纯手机零穿戴模式时，自然呈现晨起精神感受 5 档选择 */}
        <If when={settings().hardware_tier === "tier_c"}>
          <SunkenBox>
            <View class="flex-row items-center justify-between gap-2">
              <Text class="text-xs font-semibold text-foreground">今日晨起精神感受</Text>
              <Text class="text-xs text-muted-foreground">轻点记录此刻状态</Text>
            </View>
            <View class="flex-row flex-wrap gap-2">
              <Each of={FEELING_LEVELS} key={(item) => item.score}>
                {(item) => (
                  <SelectChip
                    label={item().label}
                    selected={settings().morning_feeling === item().score}
                    onPress={() => setHardwareTier("tier_c", item().score)}
                  />
                )}
              </Each>
            </View>
          </SunkenBox>
        </If>
      </Card>

      {/* 2. 今日唯一高杠杆微行动 */}
      <Card>
        <SectionHeader
          title="今日微行动"
          subtitle="告别繁重任务清单，安顿好这一件事便已足够"
          right={
            <VitalBadge
              tone={settings().checked_action_today ? "prime_ready" : "neutral"}
              label={settings().checked_action_today ? "已达成" : "待打卡"}
            />
          }
        />
        <SunkenBox>
          <Text class="text-sm font-medium leading-relaxed text-foreground">{snap().micro_action}</Text>
        </SunkenBox>
        <View class="flex-row flex-wrap gap-2">
          <ActionButton
            title={settings().checked_action_today ? "今日微行动已达成 ✓" : "完成今日微行动"}
            icon="check"
            variant={settings().checked_action_today ? "secondary" : "primary"}
            onPress={() => toggleTodayAction()}
            class="flex-1"
          />
          <ActionButton
            title="记饮食与体检"
            icon="record"
            variant="outline"
            onPress={() => setTab("log")}
          />
          <ActionButton
            title="微习惯洞察"
            icon="compass"
            variant="outline"
            onPress={() => setTab("insights")}
          />
        </View>
      </Card>

      {/* 3. 08:30 早安简报 */}
      <Card>
        <View class="flex-row items-center justify-between gap-2">
          <View class="flex-row items-center gap-2">
            <View class="size-7 items-center justify-center rounded-lg bg-accent">
              <QiIcon name="sparkles" size={16} color={token("primary")} />
            </View>
            <View class="gap-0.5">
              <Text class="text-base font-semibold text-foreground">08:30 早安简报</Text>
              <Text class="text-xs text-muted-foreground">{briefing().provider_label}</Text>
            </View>
          </View>
          <If when={briefing().degraded}>
            <VitalBadge tone="moderate_steady" label="精简呈现" />
          </If>
        </View>

        <View class="flex-row gap-3 rounded-xl border border-border/60 bg-surface-sunken p-3.5">
          <View class="w-1 rounded-full bg-primary/60" />
          <Text class="flex-1 text-sm leading-relaxed text-foreground">{briefing().text}</Text>
        </View>

        <View class="flex-row flex-wrap gap-2">
          <ActionButton
            title="AI 实时润色"
            icon="sparkles"
            variant="outline"
            onPress={() => refreshLiveBriefing()}
          />
          <ActionButton
            title="切换 AI 与隐私偏好"
            icon="sliders"
            variant="secondary"
            onPress={() => setTab("me")}
          />
        </View>

        <Text class="text-[11px] leading-relaxed text-muted-foreground">{briefing().disclaimer}</Text>
      </Card>

      {/* 4. 近 14 天元气节律 */}
      <Card>
        <SectionHeader
          title="近 14 天元气节律"
          subtitle="左右轻滑或轻点曲线，可回顾每日状态变化"
        />
        <InteractiveRhythmPlot points={snap().trend_14d} />
      </Card>

      {/* 5. 渐进披露：30 天个人常态对照（默认折叠，告别裸露 Z-score） */}
      <CollapsibleCard
        title="查看 30 天个人常态对照"
        subtitle="点击展开各体征读数与个人常态区间比较"
        badgeLabel={`${baselineMetrics().length} 项体征`}
        badgeTone="neutral"
        open={baselineOpen()}
        onToggle={() => baselineOpen.set(!baselineOpen())}
      >
        <If
          when={baselineMetrics().length > 0}
          else={
            <SunkenBox>
              <Text class="text-xs text-muted-foreground">
                暂无穿戴体征记录，完成佩戴或导入数据后将自动生成 30 天个人常态对照。
              </Text>
            </SunkenBox>
          }
        >
          <View class="gap-2.5">
            <Each of={baselineMetrics()} key={(item) => item.key}>
              {(item) => (
                <SunkenBox>
                  <View class="flex-row items-center justify-between gap-2">
                    <Text class="text-sm font-semibold text-foreground">
                      {CONSUMER_METRIC_TITLES[item().key] ?? item().label}
                    </Text>
                    <ConsumerStatusPill status={item().status} />
                  </View>
                  <View class="flex-row items-baseline justify-between gap-2">
                    <Text
                      class="text-base font-bold text-foreground"
                      style={{ fontVariant: ["tabular-nums"] }}
                    >
                      {formatMetricReading(item().value, item().unit)}
                    </Text>
                    <Text class="text-xs text-muted-foreground" style={{ fontVariant: ["tabular-nums"] }}>
                      {item().baseline_30d_mean === null
                        ? "个人常态积累中"
                        : `30 天常态 ${formatMetricReading(item().baseline_30d_mean, item().unit)} · 近 7 天 ${formatMetricReading(item().avg_7d, item().unit)}`}
                    </Text>
                  </View>
                </SunkenBox>
              )}
            </Each>
          </View>
        </If>
      </CollapsibleCard>
    </View>
  );
}
