import { Text, View } from "nsh";
import {
  getState,
  refreshLiveBriefing,
  requestAiAction,
  setHardwareTier,
  setTab,
  toggleTodayAction,
  updateAiConfig,
} from "../store";
import { AcwrZoneBarCanvas, RhythmSparklineCanvas, VitalityRingCanvas } from "../ui/charts";
import { ActionButton, Card, SectionHeader, SelectChip, SunkenBox, VitalBadge } from "../ui/components";

const TIER_OPTIONS = [
  { id: "tier_a", label: "Tier A 全穿戴" },
  { id: "tier_b", label: "Tier B 基础手环" },
  { id: "tier_c", label: "Tier C 纯手机" },
];

const GOAL_LABELS: Record<string, string> = {
  energy_sleep: "早安精力与睡眠修复",
  diet_balance: "3 秒记餐与吃练平衡",
  lab_archive: "体检报告红绿灯长期建档",
  smart_training: "科学防伤运动负荷管理",
};

export function TodayPage() {
  const snap = () => getState().snapshot;
  const settings = () => getState().settings;
  const briefing = () => getState().briefing;

  const goalLabel = () => GOAL_LABELS[settings().primary_goal] ?? "全面节律平衡";

  const metricCards = () => {
    const m = snap().metrics;
    const keys = ["hrv_rmssd_ms", "hrv_sdnn_ms", "resting_heart_rate_bpm", "sleep_minutes", "steps"];
    return keys.filter((k) => Boolean(m[k])).map((k) => m[k]);
  };

  return (
    <View class="gap-4">
      {/* 1. 今日元气主卡片（宋韵 Canvas 圆环 + 目标优先级 + ACWR 甜区轨道） */}
      <Card>
        <View class="flex-row items-center gap-4">
          <VitalityRingCanvas
            score={snap().qi_score}
            state={snap().state}
            badge={snap().state_badge}
          />
          <View class="flex-1 gap-1.5">
            <View class="flex-row items-center justify-between gap-2">
              <Text class="text-xs font-medium text-muted-foreground">
                {`今日元气电量 · ${snap().target_date}`}
              </Text>
              <VitalBadge tone={snap().state} label={snap().state_badge} />
            </View>
            <View class="flex-row items-baseline gap-1.5">
              <Text class="text-3xl font-bold text-foreground" style={{ fontVariant: ["tabular-nums"] }}>
                {`${snap().qi_score}`}
              </Text>
              <Text class="text-sm font-medium text-muted-foreground">/ 100</Text>
            </View>
            <Text class="text-xs text-primary">{`当前聚焦：${goalLabel()}`}</Text>
          </View>
        </View>

        <Text class="text-sm leading-relaxed text-foreground">{snap().headline}</Text>

        {/* 睡眠蓄水池与运动负荷 ACWR 速览 */}
        <View class="flex-row gap-2.5">
          <SunkenBox class="flex-1">
            <Text class="text-xs text-muted-foreground">睡眠蓄水池 (7d 负债)</Text>
            <Text class="text-base font-semibold text-foreground" style={{ fontVariant: ["tabular-nums"] }}>
              {snap().sleep_debt_7d_hours === null
                ? "纯手机不计"
                : snap().sleep_debt_7d_hours! > 0
                  ? `还差 ${snap().sleep_debt_7d_hours!.toFixed(1)} 小时`
                  : `盈余 ${Math.abs(snap().sleep_debt_7d_hours!).toFixed(1)} 小时`}
            </Text>
          </SunkenBox>
          <SunkenBox class="flex-1">
            <Text class="text-xs text-muted-foreground">负荷节律 (ACWR 7d/28d)</Text>
            <Text class="text-base font-semibold text-foreground" style={{ fontVariant: ["tabular-nums"] }}>
              {snap().acwr === null ? "未采集" : `${snap().acwr!.toFixed(2)}`}
            </Text>
          </SunkenBox>
        </View>

        {/* 原生 ACWR 甜区轨道图 */}
        <SunkenBox>
          <Text class="text-[11px] text-muted-foreground">急慢性训练负荷比轨道（0.8 – 1.3 为绿色甜区）</Text>
          <AcwrZoneBarCanvas acwr={snap().acwr} />
        </SunkenBox>

        {/* 硬件三档降级演示切换 */}
        <View class="gap-1.5 pt-1">
          <Text class="text-xs text-muted-foreground">{`当前算力口径：${snap().hardware_tier_label}`}</Text>
          <View class="flex-row flex-wrap gap-2">
            <Each of={TIER_OPTIONS} key={(t) => t.id}>
              {(opt) => (
                <SelectChip
                  label={opt().label}
                  selected={settings().hardware_tier === opt().id}
                  onPress={() => setHardwareTier(opt().id, settings().morning_feeling)}
                />
              )}
            </Each>
          </View>
          <If when={settings().hardware_tier === "tier_c"}>
            <View class="flex-row items-center justify-between gap-2 rounded-xl bg-surface-sunken px-3 py-2">
              <Text class="text-xs text-foreground">晨起主观体感打分：</Text>
              <View class="flex-row gap-1.5">
                <Each of={[1, 2, 3, 4, 5]} key={(n) => n}>
                  {(n) => (
                    <SelectChip
                      label={`${n()}`}
                      selected={settings().morning_feeling === n()}
                      onPress={() => setHardwareTier("tier_c", n())}
                    />
                  )}
                </Each>
              </View>
            </View>
          </If>
        </View>
      </Card>

      {/* 2. 今日唯一高杠杆微行动 */}
      <Card>
        <SectionHeader
          title="今日唯一高杠杆建议"
          subtitle="告别机械圆环焦虑，做完这一件就够"
          right={
            <VitalBadge
              tone={settings().checked_action_today ? "prime_ready" : "neutral"}
              label={settings().checked_action_today ? "已打卡" : "待完成"}
            />
          }
        />
        <Text class="text-sm leading-relaxed text-foreground">{snap().micro_action}</Text>
        <View class="flex-row flex-wrap gap-2">
          <ActionButton
            title={settings().checked_action_today ? "已完成今日微行动 ✓" : "打卡完成今日微行动"}
            variant={settings().checked_action_today ? "secondary" : "primary"}
            onPress={() => toggleTodayAction()}
            class="flex-1"
          />
          <ActionButton
            title="去发现实验"
            variant="outline"
            onPress={() => setTab("insights")}
          />
          <ActionButton
            title="记一餐 / 看体检"
            variant="outline"
            onPress={() => setTab("log")}
          />
        </View>
      </Card>

      {/* 3. 早安 AI 简报（端云一体零幻觉输出 + 实时润色 + 受热降级演示） */}
      <Card>
        <SectionHeader
          title="08:30 早安简报"
          subtitle={briefing().provider_label}
          right={
            <If when={briefing().degraded}>
              <VitalBadge tone="moderate_steady" label="精简降级" />
            </If>
          }
        />
        <SunkenBox>
          <Text class="text-sm leading-relaxed text-foreground">{briefing().text}</Text>
          <Text class="text-xs text-muted-foreground">{briefing().disclaimer}</Text>
        </SunkenBox>
        <View class="flex-row flex-wrap gap-2">
          <ActionButton
            title="AI 实时润色"
            variant="outline"
            onPress={() => refreshLiveBriefing()}
          />
          <If when={briefing().degraded}>
            <ActionButton
              title="临时用零留存云端润色"
              variant="outline"
              onPress={() =>
                requestAiAction(() => {
                  const s = settings();
                  updateAiConfig("official_zdr_cloud", true, s.lan_ai_url, s.lan_ai_model, false, s.simulated_ram_gb);
                })
              }
            />
          </If>
          <ActionButton
            title={settings().ai_mode === "official_zdr_cloud" && !settings().zdr_consented ? "解锁云端 AI 深度润色" : "切换 AI / 隐私模式"}
            variant="secondary"
            onPress={() => {
              if (settings().ai_mode === "official_zdr_cloud" && !settings().zdr_consented) {
                requestAiAction(() => setTab("me"));
              } else {
                setTab("me");
              }
            }}
          />
        </View>
      </Card>

      {/* 4. 物理隔离的 30 天个人基线详情 */}
      <Card>
        <SectionHeader
          title="30 天个人基线对照（RMSSD 与 SDNN 严格隔离）"
          subtitle="来自本地 Rust 引擎确定性计算"
        />
        <View class="gap-2">
          <Each of={metricCards()} key={(item) => item.key}>
            {(item) => (
              <SunkenBox>
                <View class="flex-row items-center justify-between gap-2">
                  <Text class="text-sm font-semibold text-foreground">{item().label}</Text>
                  <VitalBadge
                    tone={item().status}
                    label={item().z_score === null ? "积累基线中" : `Z = ${item().z_score! >= 0 ? "+" : ""}${item().z_score!.toFixed(2)}`}
                  />
                </View>
                <View class="flex-row items-baseline justify-between gap-2">
                  <Text class="text-base font-bold text-foreground" style={{ fontVariant: ["tabular-nums"] }}>
                    {item().value === null ? "—" : `${item().value} ${item().unit}`}
                  </Text>
                  <Text class="text-xs text-muted-foreground">
                    {item().baseline_30d_mean === null
                      ? "样本不足 4 天"
                      : `30d 个人基线 ${item().baseline_30d_mean} · 7d 均值 ${item().avg_7d ?? "—"}`}
                  </Text>
                </View>
              </SunkenBox>
            )}
          </Each>
        </View>
      </Card>

      {/* 5. 近 14 天元气节律平滑曲线（原生 Canvas 绘制） */}
      <Card>
        <SectionHeader title="近 14 天元气变化" subtitle="无打卡惩罚的动态节律曲线（原生 Canvas 渲染）" />
        <RhythmSparklineCanvas points={snap().trend_14d} />
      </Card>
    </View>
  );
}
