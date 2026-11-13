import { signal, Text, token, View } from "nsh";
import {
  getDewSummary,
  getPetCount,
  getState,
  getVisualTheme,
  petCompanion,
  refreshLiveBriefing,
  setHardwareTier,
  setShowDailyPoster,
  setTab,
  setVisualTheme,
  toggleTodayAction,
} from "../store";
import {
  AcwrSweetSpotCanvas,
  InteractiveRhythmPlot,
  SleepArchitectureBar,
} from "../ui/charts";
import {
  ActionButton,
  Card,
  CollapsibleCard,
  ConsumerStatusPill,
  Hairline,
  SelectChip,
  SunkenBox,
  VitalBadge,
} from "../ui/components";
import { QiIcon } from "../ui/icons";
import {
  BentoMetricCard,
  SanctuaryStage,
  StampCheckinCard,
  ThemeSwitcherStrip,
} from "../ui/sanctuary";

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
  const briefingOpen = signal(false);
  const trendOpen = signal(false);
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
        sublabel:
          hrv.baseline_30d_mean !== null
            ? `平时约 ${Math.round(hrv.baseline_30d_mean)} ms`
            : "夜间恢复节律",
      };
    }
    const rhr = m.resting_heart_rate_bpm;
    if (rhr && rhr.value !== null) {
      return {
        value: `${Math.round(rhr.value)} bpm`,
        sublabel:
          rhr.baseline_30d_mean !== null
            ? `平时静息 ${Math.round(rhr.baseline_30d_mean)} bpm`
            : "夜间静息心率",
      };
    }
    return {
      value: `${settings().morning_feeling} / 5`,
      sublabel: "晨起精神感受",
    };
  };

  const sleepTile = () => {
    const h = snap().sleep_last_night_hours;
    if (h === null || !Number.isFinite(h)) {
      return {
        value: "未采集",
        sublabel: "仅使用手机计步",
      };
    }
    return {
      value: `${h.toFixed(1)} 小时`,
      sublabel: friendlySleepDebt(snap().sleep_debt_7d_hours),
    };
  };

  const loadTile = () => {
    const m = snap().metrics;
    const steps = m.steps;
    const acwr = snap().acwr;
    if (steps && steps.value !== null) {
      return {
        value: `${Math.round(steps.value)} 步`,
        sublabel: acwr !== null ? `近期负荷 · ${friendlyLoadStatus(acwr)}` : "今日步数记录",
      };
    }
    return {
      value: friendlyLoadStatus(acwr),
      sublabel: "近 7 天活动负荷",
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
      {/* 顶部时段问候 + 日期 + 三套灵境画风快切 */}
      <View class="gap-2.5 px-0.5">
        <View class="flex-row items-end justify-between gap-3">
          <View class="flex-1 gap-0.5">
            <Text role="header" class="text-xl font-bold text-foreground">
              晨安 · 今日节律
            </Text>
            <Text class="text-xs text-muted-foreground">{`聚焦方向：${goalLabel()}`}</Text>
          </View>
          <View class="rounded-full border border-border/70 bg-card px-3 py-1">
            <Text class="text-xs font-medium text-muted-foreground" style={{ fontVariant: ["tabular-nums"] }}>
              {snap().target_date}
            </Text>
          </View>
        </View>

        <ThemeSwitcherStrip
          activeId={getVisualTheme()}
          onSelect={(id) => setVisualTheme(id)}
        />
      </View>

      {/* 1. 无框感沉浸式插画剧场 + 晨露收集 + 角色对话气泡 */}
      <SanctuaryStage
        themeId={getVisualTheme()}
        score={snap().qi_score}
        state={snap().state}
        stateBadge={snap().state_badge}
        headline={snap().headline}
        dewSummary={getDewSummary()}
        petCount={getPetCount()}
        checkedAction={settings().checked_action_today}
        onPet={() => petCompanion()}
        onOpenPoster={() => setShowDailyPoster(true)}
      />

      {/* 2. 错落手作感「便当小瓷贴」体征摘要 */}
      <View class="gap-2.5">
        <View class="flex-row gap-2.5">
          <BentoMetricCard
            icon="heart_pulse"
            label="夜间恢复"
            value={recoveryTile().value}
            sublabel={recoveryTile().sublabel}
            toneSurface="sage"
            accentColor={token("vital-prime")}
            class="flex-1"
          />
          <BentoMetricCard
            icon="moon"
            label="昨晚好眠"
            value={sleepTile().value}
            sublabel={sleepTile().sublabel}
            toneSurface="lavender"
            accentColor={token("sleep-rem")}
            class="flex-1"
          >
            <If when={Boolean(snap().sleep_stages_last_night)}>
              <SleepArchitectureBar
                deepMin={snap().sleep_stages_last_night!.deep_min}
                coreMin={snap().sleep_stages_last_night!.core_min}
                remMin={snap().sleep_stages_last_night!.rem_min}
                awakeMin={snap().sleep_stages_last_night!.awake_min}
              />
            </If>
          </BentoMetricCard>
        </View>

        <BentoMetricCard
          icon="flame"
          label="身体负荷"
          value={loadTile().value}
          sublabel={loadTile().sublabel}
          toneSurface="amber"
          accentColor={token("vital-steady")}
        />

        {/* 纯手机无穿戴自然呈现 5 档晨起精神感受 */}
        <If when={settings().hardware_tier === "tier_c"}>
          <Card>
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
          </Card>
        </If>
      </View>

      {/* 3. 触感盖章打卡卡片（今日唯一高杠杆微行动） */}
      <StampCheckinCard
        themeId={getVisualTheme()}
        microAction={snap().micro_action}
        checked={settings().checked_action_today}
        onToggle={() => toggleTodayAction()}
        onGoLog={() => setTab("log")}
        onOpenPoster={() => setShowDailyPoster(true)}
      />

      {/* 4. 身体手账折页（渐进披露收纳：08:30 早安简报 / 14 天趋势 / 30 天常态对照） */}
      <View class="gap-3">
        <View class="flex-row items-center gap-2 px-1 pt-1">
          <QiIcon name="record" size={15} color={token("primary")} />
          <Text role="header" class="text-sm font-bold text-foreground">
            身体手账折页
          </Text>
          <Text class="text-xs text-muted-foreground">轻点即可翻开完整简报、曲线与个人常态</Text>
        </View>

        {/* 折页一：08:30 早安简报与负荷甜区 */}
        <CollapsibleCard
          title="08:30 早安简报"
          subtitle={briefing().provider_label}
          badgeLabel={briefing().degraded ? "精简呈现" : "端侧就绪"}
          badgeTone={briefing().degraded ? "moderate_steady" : "prime_ready"}
          open={briefingOpen()}
          onToggle={() => briefingOpen.set(!briefingOpen())}
        >
          <SunkenBox>
            <Text class="text-sm leading-relaxed text-foreground">{briefing().text}</Text>
          </SunkenBox>

          <If when={snap().acwr !== null}>
            <SunkenBox>
              <View class="flex-row items-center justify-between">
                <Text class="text-xs font-semibold text-foreground">近期身体负荷节奏</Text>
                <Text class="text-xs font-medium text-primary">{friendlyLoadStatus(snap().acwr)}</Text>
              </View>
              <AcwrSweetSpotCanvas acwr={snap().acwr} />
            </SunkenBox>
          </If>

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
        </CollapsibleCard>

        {/* 折页二：近 14 天元气节律交互折线 */}
        <CollapsibleCard
          title="近 14 天元气节律"
          subtitle="翻开即可滑动查看近两周元气分数、睡眠与恢复趋势"
          badgeLabel={`${snap().trend_14d.length} 天曲线`}
          badgeTone="neutral"
          open={trendOpen()}
          onToggle={() => trendOpen.set(!trendOpen())}
        >
          <InteractiveRhythmPlot points={snap().trend_14d} />
        </CollapsibleCard>

        {/* 折页三：30 天个人常态对照 */}
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
            <View class="gap-0">
              <Each of={baselineMetrics()} key={(item) => item.key}>
                {(item, idx) => (
                  <View class="gap-1.5 py-2.5">
                    <If when={idx() > 0}>
                      <Hairline class="mb-2" />
                    </If>
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
                  </View>
                )}
              </Each>
            </View>
          </If>
        </CollapsibleCard>
      </View>
    </View>
  );
}
