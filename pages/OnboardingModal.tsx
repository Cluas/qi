import { Scroll, signal, Text, token, View } from "nsh";
import { completeOnboarding, getState } from "../store";
import { ActionButton, Card, ChoiceCard, ModalSurface, SunkenBox, VitalBadge } from "../ui/components";
import { QiIcon, type QiIconName } from "../ui/icons";

const GOALS: { id: string; title: string; subtitle: string; icon: QiIconName }[] = [
  {
    id: "energy_sleep",
    title: "早安精力与好眠修复",
    subtitle: "感知每日元气节律，顺应睡眠欠账与恢复节奏",
    icon: "sun",
  },
  {
    id: "diet_balance",
    title: "随手记餐与吃练平衡",
    subtitle: "3 秒一句话记录日常饮食，看见身体次日反馈",
    icon: "utensils",
  },
  {
    id: "lab_archive",
    title: "体检报告长期私密建档",
    subtitle: "端侧识图脱敏与跨期参考区间可视化对照",
    icon: "file_lab",
  },
  {
    id: "smart_training",
    title: "运动节奏与温和防过劳",
    subtitle: "把握适度运动甜区，让每次训练都恰到好处",
    icon: "heart_pulse",
  },
];

const WEAR_MODES: { id: string; title: string; subtitle: string; icon: QiIconName }[] = [
  {
    id: "tier_a",
    title: "佩戴智能手表（如 Apple Watch / Garmin）",
    subtitle: "自动融合夜间恢复节律、静息心率与四期睡眠结构",
    icon: "heart_pulse",
  },
  {
    id: "tier_b",
    title: "佩戴基础健康手环",
    subtitle: "围绕睡眠时长、静息心率与每日步数平稳评估",
    icon: "moon",
  },
  {
    id: "tier_c",
    title: "暂不佩戴穿戴设备",
    subtitle: "仅凭手机每日步数与晨起精神感受，轻盈开启记录",
    icon: "compass",
  },
];

/** 诗意极简冷启动引导：零术语高信任开启知气节律 */
export function OnboardingModal() {
  const goal = signal("energy_sleep");
  const tier = signal("tier_a");
  const isOpen = () => !getState().settings.onboarding_done;

  return (
    <ModalSurface open={isOpen()} presentation="pageSheet" closeOnBackdropPress={false}>
      <Scroll class="flex-1" contentStyle={{ paddingHorizontal: 20, paddingBottom: 24, gap: 16 }}>
        <View class="flex-row items-center justify-between pt-1">
          <View class="flex-row items-center gap-2">
            <View class="size-8 items-center justify-center rounded-xl bg-accent">
              <QiIcon name="sun" size={18} color={token("primary")} />
            </View>
            <VitalBadge tone="prime_ready" label="宋韵极简 · 私密守护" />
          </View>
          <Text class="text-xs text-muted-foreground">60 秒温润开启</Text>
        </View>

        <View class="gap-1.5">
          <Text role="header" class="text-2xl font-bold text-foreground">
            欢迎来到「知气 · Qi」
          </Text>
          <Text class="text-sm leading-relaxed text-muted-foreground">
            以温润留白的节律视角，陪伴你安顿每日睡眠、饮食与长期体征变化。无机械圆环催促，亦无繁杂统计压力。
          </Text>
        </View>

        <Card>
          <Text role="header" class="text-sm font-semibold text-foreground">
            1. 此刻最想先关照的生活方向
          </Text>
          <View class="gap-2">
            <Each of={GOALS} key={(g) => g.id}>
              {(g) => (
                <ChoiceCard
                  title={g().title}
                  subtitle={g().subtitle}
                  icon={g().icon}
                  selected={goal() === g().id}
                  onPress={() => goal.set(g().id)}
                />
              )}
            </Each>
          </View>
        </Card>

        <Card>
          <Text role="header" class="text-sm font-semibold text-foreground">
            2. 选择你的日常穿戴习惯
          </Text>
          <Text class="text-xs leading-relaxed text-muted-foreground">
            不同穿戴方式会匹配对应温和算法，绝不凭空臆造未采集到的体征。
          </Text>
          <View class="gap-2">
            <Each of={WEAR_MODES} key={(m) => m.id}>
              {(m) => (
                <ChoiceCard
                  title={m().title}
                  subtitle={m().subtitle}
                  icon={m().icon}
                  selected={tier() === m().id}
                  onPress={() => tier.set(m().id)}
                />
              )}
            </Each>
          </View>
        </Card>

        <SunkenBox>
          <View class="flex-row items-center gap-2">
            <QiIcon name="cloud_lock" size={16} color={token("primary")} />
            <Text class="text-xs font-semibold text-primary">数据仅存本机与个人云</Text>
          </View>
          <Text class="text-xs leading-relaxed text-muted-foreground">
            你的生理节律、饮食笔记与体检档案均加密保存在手机本机或你指定的个人云空间，官方服务器零留存。
          </Text>
        </SunkenBox>

        <ActionButton
          title="开启今日知气"
          icon="sparkles"
          size="lg"
          onPress={() => completeOnboarding(goal(), tier())}
        />
      </Scroll>
    </ModalSurface>
  );
}
