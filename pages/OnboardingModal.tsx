import { signal, Text, View } from "nsh";
import { Dialog } from "nsh/base";
import { completeOnboarding, getState } from "../store";
import { ActionButton, Card, SelectChip, SunkenBox, VitalBadge } from "../ui/components";

const GOALS = [
  { id: "energy_sleep", label: "早安精力与睡眠修复" },
  { id: "diet_balance", label: "3 秒记餐与吃练平衡" },
  { id: "lab_archive", label: "体检报告红绿灯长期建档" },
  { id: "smart_training", label: "科学防伤运动负荷管理" },
];

const TIERS = [
  { id: "tier_a", label: "Apple Watch / Garmin 等穿戴（有 HRV）" },
  { id: "tier_b", label: "基础手环（仅有睡眠时长与步数）" },
  { id: "tier_c", label: "暂不戴表（手机步数 + 晨起体感）" },
];

/** 60 秒零术语高信任冷启动引导：本地秒算高光，零弹窗轰炸 */
export function OnboardingModal() {
  const goal = signal("energy_sleep");
  const tier = signal("tier_a");
  const isOpen = () => !getState().settings.onboarding_done;

  return (
    <Dialog open={isOpen()} presentation="pageSheet" closeOnBackdropPress={false} class="flex-1 bg-background">
      <View
        class="flex-1 justify-between gap-4 px-5"
        style={{
          paddingTop: "calc(env(safe-area-inset-top) + 20px)",
          paddingBottom: "calc(env(safe-area-inset-bottom) + 20px)",
        }}
      >
        <View class="gap-4">
          <View class="flex-row items-center justify-between">
            <VitalBadge tone="prime_ready" label="Local-First 零留存" />
            <Text class="text-xs text-muted-foreground">60 秒无感开启</Text>
          </View>

          <View class="gap-1.5">
            <Text class="text-2xl font-bold text-foreground">欢迎来到「知气 · Qi」</Text>
            <Text class="text-sm leading-relaxed text-muted-foreground">
              你的生理数据与体检档案只保存在本机与你的个人 iCloud / 网盘，支持纯本地大模型。官方服务器零留存你的健康库。
            </Text>
          </View>

          <Card>
            <Text class="text-sm font-semibold text-foreground">1. 当前最想先改善哪件事？</Text>
            <View class="flex-row flex-wrap gap-2">
              <Each of={GOALS} key={(g) => g.id}>
                {(g) => (
                  <SelectChip
                    label={g().label}
                    selected={goal() === g().id}
                    onPress={() => goal.set(g().id)}
                  />
                )}
              </Each>
            </View>
          </Card>

          <Card>
            <Text class="text-sm font-semibold text-foreground">2. 你当前的穿戴设备方案</Text>
            <Text class="text-xs text-muted-foreground">
              不同设备自动匹配对应算法模型，绝不伪造不存在的 HRV 数据。
            </Text>
            <View class="gap-2">
              <Each of={TIERS} key={(t) => t.id}>
                {(t) => (
                  <SelectChip
                    label={t().label}
                    selected={tier() === t().id}
                    onPress={() => tier.set(t().id)}
                  />
                )}
              </Each>
            </View>
          </Card>

          <SunkenBox>
            <Text class="text-xs font-semibold text-primary">端侧 Rust 引擎已完成 35 天基线预热</Text>
            <Text class="text-xs text-muted-foreground">
              无需等待云端同步或繁琐配对。稍后仅在你主动启用「云端 AI 拍照 / 解读」时按需弹出授权。
            </Text>
          </SunkenBox>
        </View>

        <ActionButton
          title="生成今日元气电量"
          onPress={() => completeOnboarding(goal(), tier())}
        />
      </View>
    </Dialog>
  );
}
