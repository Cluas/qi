import { Canvas, createMotion, Image, Scroll, signal, Text, token, View } from "nsh";
import { Button } from "nsh/base";
import fourSeasonsHandscrollImg from "../assets/scenes/four_seasons_handscroll.jpg";
import {
  ActionButton,
  Card,
  FieldInput,
  Hairline,
  MeterBar,
  ModalSurface,
  SectionHeader,
  SelectChip,
  SheetDialog,
  SunkenBox,
  SwitchRow,
  VitalBadge,
} from "./components";
import {
  BadgeCeramicIcon,
  CoSanctuaryIcon,
  EnvelopeIcon,
  FamilyHeartIcon,
  HandscrollIcon,
  QiIcon,
  SolarTermIcon,
  WaxSealIcon,
  WidgetIcon,
} from "./icons";
import type { SealInkId, WeatherMood } from "./sanctuary";
import { getVisualThemeMeta, type VisualThemeId } from "./themes";

export type AtelierTierId =
  | "free"
  | "pro_monthly"
  | "seasonal"
  | "pro_annual"
  | "duo"
  | "family"
  | "lifetime_local"
  | string;

export interface AtelierPlanOption {
  tier: AtelierTierId;
  title: string;
  subtitle: string;
  priceCny: number;
  originalPriceCny?: number | null;
  billingCycle: string;
  seats: number;
  highlights: string[];
  badge: string;
  loyaltyApplied?: boolean;
}

export interface TransparentTrialStep {
  dayLabel: string;
  title: string;
  detail: string;
}

const DEFAULT_ATELIER_PLANS: AtelierPlanOption[] = [
  {
    tier: "free",
    title: "素心清欢 · 永久免费",
    subtitle: "原始数据、单人化验单 OCR、Ollama 本地晨报、静养双倍露与 JSONL 导出永久免费",
    priceCny: 0,
    billingCycle: "永久免费",
    seats: 1,
    highlights: [
      "完整生理记录 + 单人化验解析 + 100% 明文导出",
      "第 1 封伴侣周信 + 每月 5 张拍立得饮食贴纸 + 清欢轨节气风物",
    ],
    badge: "健康底线 · 永不涨价",
  },
  {
    tier: "pro_monthly",
    title: "知气月雅 · 按月轻享",
    subtitle: "无自动续费套路，按月随心开启全套雅集陪伴",
    priceCny: 18,
    billingCycle: "30 天",
    seats: 1,
    highlights: [
      "24 节气昼夜书房天象 + 雅集限定服饰、泥金/石青印泥",
      "无限饮食贴纸 + 90 天 D+0/D+1/D+2 归因雷达",
    ],
    badge: "无连包套路",
  },
  {
    tier: "seasonal",
    title: "岁时节气季卡",
    subtitle: "陪你完整度过春 / 夏 / 秋 / 冬一整季 6 个节气",
    priceCny: 38,
    originalPriceCny: 54,
    billingCycle: "一季 (3 个月)",
    seats: 1,
    highlights: [
      "解锁当季全部 6 个节气「雅集轨」摆件与藏书票",
      "四时风物长卷 + WebDAV 双向加密同步 + 自定义洗脱期",
    ],
    badge: "应季首选",
  },
  {
    tier: "pro_annual",
    title: "知气雅集 · 年卡",
    subtitle: "一年 24 节气、52 封周信、12 张节气月历垫全收录",
    priceCny: 98,
    originalPriceCny: 148,
    billingCycle: "365 天",
    seats: 1,
    highlights: [
      "解锁全部 24 节气雅集风物、52 封周信与往期博物架",
      "多亲友化验 %ULN 归一化对比 + 大字平安家书 + 同窗房主权",
    ],
    badge: "30 天陪伴礼遇价 ¥98",
    loyaltyApplied: true,
  },
  {
    tier: "duo",
    title: "双人同窗 · Duo",
    subtitle: "两人各享独立私密 Vault，只互传 1 字节心情与合种窗台金桂",
    priceCny: 168,
    originalPriceCny: 236,
    billingCycle: "365 天 · 2 席",
    seats: 2,
    highlights: [
      "2 个完整知气雅集席位，双方互递热茶、盖薄被、按爪印",
      "绝不共享日记、作息数字与化验单内容",
    ],
    badge: "情侣 / 挚友同窗",
  },
  {
    tier: "family",
    title: "四时家书 · Family",
    subtitle: "至多 4 位家人共享：把温和气色化作爸妈看得懂的大字平安家书",
    priceCny: 228,
    originalPriceCny: 472,
    billingCycle: "365 天 · 4 席",
    seats: 4,
    highlights: [
      "4 个独立端侧保险箱 + %ULN 亲友化验同屏趋势",
      "一键生成端到端加密大字版《气韵平安家书》",
    ],
    badge: "全家安心 · 4 席",
  },
  {
    tier: "lifetime_local",
    title: "买断本地 · 终身版",
    subtitle: "一次买断本地全部雅集与统计引擎（自有 WebDAV 同步永久免费）",
    priceCny: 368,
    billingCycle: "永久买断",
    seats: 1,
    highlights: [
      "离线 Ed25519 终身凭证，永不依赖任何续费服务器",
      "本地全部节气美术、贴纸滤镜与 .qivault 全速运行",
    ],
    badge: "买断党挚爱",
  },
];

const DEFAULT_TRIAL_TIMELINE: TransparentTrialStep[] = [
  { dayLabel: "第 1 天", title: "此刻起畅享全部雅集", detail: "24 节气剧场、慢递周信、多滞后雷达即时开启。" },
  { dayLabel: "第 5 天", title: "提前 48h 温柔提醒", detail: "应用内轻声提醒一次，绝不发短信轰炸或弹窗拦路。" },
  { dayLabel: "第 7 天", title: "退订无阻，资产永存", detail: "随时一键切回素心清欢；历史贴纸、周信与档案永久可读可导出。" },
];

const DEFAULT_ETHICAL_PROMISES: string[] = [
  "绝不锁死原始生理数据、单人化验 OCR 与 JSONL / .qivault 明文导出",
  "安心静养模式（recovery_priority）下绝不弹出付费推广或制造焦虑",
  "拒绝暗黑模式（Dark Patterns）：无倒计时逼单、无羞辱式拒绝文案",
  "停费后进入「只读保险箱」，既往风物与手账永不销毁",
];

/** 1. 温柔雅集订阅弹窗：零压力上下文沟通 + 30天陪伴礼遇 + 7方案 + 添柴公益 + 透明7日线 */
export function QiAtelierModal(props: {
  open: boolean;
  companionThemeId?: VisualThemeId;
  currentTier: AtelierTierId;
  selectedTier?: AtelierTierId;
  onSelectTier?: (tier: AtelierTierId) => void;
  guardianSponsor: boolean;
  onToggleGuardian: (enabled: boolean) => void;
  loyaltyDays?: number;
  offlineStateBanner?: string;
  plans?: AtelierPlanOption[];
  timelineSteps?: TransparentTrialStep[];
  ethicalPromises?: string[];
  onConfirm: (tier: AtelierTierId, guardianSponsor: boolean) => void;
  onClose: () => void;
}) {
  const localSelected = signal<AtelierTierId>(
    props.selectedTier ?? (props.currentTier === "free" ? "pro_annual" : props.currentTier),
  );
  const activeTier = () => props.selectedTier ?? localSelected();
  const selectTier = (tier: AtelierTierId) => {
    localSelected.set(tier);
    props.onSelectTier?.(tier);
  };

  const themeMeta = () => getVisualThemeMeta(props.companionThemeId ?? "cat_window");
  const loyaltyDays = () => props.loyaltyDays ?? 30;
  const loyaltyEligible = () => loyaltyDays() >= 30;
  const plans = () => props.plans ?? DEFAULT_ATELIER_PLANS;
  const timeline = () => props.timelineSteps ?? DEFAULT_TRIAL_TIMELINE;
  const promises = () => props.ethicalPromises ?? DEFAULT_ETHICAL_PROMISES;

  const selectedPlan = () => plans().find((p) => p.tier === activeTier()) ?? plans()[0];

  return (
    <ModalSurface open={props.open} presentation="pageSheet" onClose={props.onClose}>
      <Scroll class="flex-1" contentStyle={{ paddingHorizontal: 18, paddingBottom: 32, gap: 14 }}>
        <View class="mx-auto w-full max-w-2xl gap-3.5">
          {/* 顶部标题与礼遇胶囊 */}
          <View class="flex-row items-center justify-between gap-2">
            <View class="flex-row items-center gap-2">
              <View class="size-9 items-center justify-center rounded-xl bg-atelier-soft">
                <SolarTermIcon size={18} color={token("atelier-accent")} />
              </View>
              <View class="gap-0.5">
                <Text role="header" class="text-lg font-bold text-foreground">
                  知气雅集 · Qi Atelier
                </Text>
                <Text class="text-xs text-muted-foreground">四时风物与家庭手账工坊 · 温和陪伴不催促</Text>
              </View>
            </View>
            <ActionButton title="先逛逛" size="sm" variant="secondary" shape="pill" onPress={props.onClose} />
          </View>

          {/* 伙伴轻声耳语 + 30天陪伴礼遇 */}
          <View class="gap-2.5 rounded-2xl border border-atelier-accent/35 bg-atelier-soft p-3.5">
            <View class="flex-row flex-wrap items-center justify-between gap-2">
              <View class="flex-row items-center gap-1.5">
                <QiIcon name="sparkles" size={14} color={token("atelier-accent")} />
                <Text class="text-xs font-bold text-atelier-accent">
                  {`${themeMeta().companionName}的窗台便笺`}
                </Text>
              </View>
              <View class="rounded-full border border-atelier-accent/35 bg-card px-2.5 py-0.5">
                <Text
                  class="text-[11px] font-semibold text-atelier-accent"
                  style={{ fontVariant: ["tabular-nums"] }}
                >
                  {loyaltyEligible()
                    ? `已享连续陪伴 ${loyaltyDays()} 天礼遇 · 年卡立减 ¥20`
                    : `轻陪伴第 ${loyaltyDays()}/30 天 · 满 30 天解锁礼遇价`}
                </Text>
              </View>
            </View>
            <Text class="text-xs leading-relaxed text-foreground">
              你的全部原始生理记录、单人化验单与明文导出永远留在这里。如果愿意给窗台添一盏长明夜灯，欢迎按自己的节奏选择雅集方案；哪怕一直保持素心清欢，我也一样陪着你。
            </Text>
            <If when={Boolean(props.offlineStateBanner)}>
              <View class="rounded-xl border border-border/60 bg-card/80 px-3 py-1.5">
                <Text class="text-[11px] font-medium text-muted-foreground">{props.offlineStateBanner}</Text>
              </View>
            </If>
          </View>

          {/* 7 档方案卡片 */}
          <View class="gap-2">
            <Text class="text-xs font-semibold text-muted-foreground">选择适合此刻的陪伴方式（共 7 档透明方案）</Text>
            <Each of={plans()} key={(p) => p.tier}>
              {(plan) => {
                const isSelected = () => activeTier() === plan().tier;
                const isCurrent = () => props.currentTier === plan().tier;
                return (
                  <Button
                    onPress={() => selectTier(plan().tier)}
                    class={`gap-2 rounded-2xl border p-3.5 transition-colors duration-150 focus-visible:ring-2 focus-visible:ring-ring motion-reduce:transition-none ${
                      isSelected()
                        ? "border-atelier-accent bg-atelier-soft/75"
                        : "border-border/75 bg-card hover:border-atelier-accent/45"
                    }`}
                  >
                    <View class="flex-row items-start justify-between gap-2">
                      <View class="flex-1 gap-1">
                        <View class="flex-row flex-wrap items-center gap-1.5">
                          <Text class="text-sm font-bold text-foreground">{plan().title}</Text>
                          <View class="rounded-full bg-surface-sunken px-2 py-0.5 border border-border/60">
                            <Text class="text-[10px] font-semibold text-atelier-accent">{plan().badge}</Text>
                          </View>
                          <If when={isCurrent()}>
                            <View class="rounded-full bg-vital-prime-soft px-2 py-0.5">
                              <Text class="text-[10px] font-semibold text-vital-prime">当前方案</Text>
                            </View>
                          </If>
                        </View>
                        <Text class="text-xs leading-relaxed text-muted-foreground">{plan().subtitle}</Text>
                      </View>

                      <View class="items-end">
                        <View class="flex-row items-baseline gap-1">
                          <Text
                            class="text-lg font-bold text-foreground"
                            style={{ fontVariant: ["tabular-nums"] }}
                          >
                            {plan().priceCny === 0 ? "¥0" : `¥${plan().priceCny}`}
                          </Text>
                          <If when={Boolean(plan().originalPriceCny)}>
                            <Text
                              class="text-[11px] text-muted-foreground line-through"
                              style={{ fontVariant: ["tabular-nums"] }}
                            >
                              {`¥${plan().originalPriceCny}`}
                            </Text>
                          </If>
                        </View>
                        <Text class="text-[10px] text-muted-foreground">{plan().billingCycle}</Text>
                      </View>
                    </View>

                    <View class="gap-1 pt-0.5">
                      <Each of={plan().highlights} key={(h) => `${plan().tier}-${h}`}>
                        {(line) => (
                          <View class="flex-row items-center gap-1.5">
                            <QiIcon name="check" size={12} color={token("vital-prime")} />
                            <Text class="flex-1 text-[11px] text-foreground">{line()}</Text>
                          </View>
                        )}
                      </Each>
                    </View>
                  </Button>
                );
              }}
            </Each>
          </View>

          {/* 窗台添柴计划 (+¥12) */}
          <Card class="border-wax-seal/30 bg-wax-seal-soft/50">
            <View class="flex-row items-center gap-2">
              <FamilyHeartIcon size={16} color={token("wax-seal")} />
              <Text class="text-xs font-bold text-wax-seal">公益共建 · 窗台添柴计划</Text>
            </View>
            <SwitchRow
              title="随单加入「窗台添柴计划」(+¥12)"
              subtitle="为正在经历高压力备考、住院或康复期的陌生用户资助 1 个月素心雅集暖灯。"
              checked={props.guardianSponsor}
              onChange={props.onToggleGuardian}
            />
          </Card>

          {/* Blinkist 风格 3 步透明 7 天提醒时间轴 */}
          <Card>
            <SectionHeader title="7 天透明体验时间轴" subtitle="没有暗扣、没有小字，把每一步都写在明面上" />
            <View class="gap-2.5">
              <Each of={timeline()} key={(t) => t.dayLabel}>
                {(step, idx) => (
                  <View class="flex-row items-start gap-2.5">
                    <View class="size-6 items-center justify-center rounded-full bg-atelier-soft">
                      <Text
                        class="text-[11px] font-bold text-atelier-accent"
                        style={{ fontVariant: ["tabular-nums"] }}
                      >
                        {`${idx() + 1}`}
                      </Text>
                    </View>
                    <View class="flex-1 gap-0.5">
                      <View class="flex-row items-center gap-2">
                        <Text class="text-xs font-bold text-foreground">{step().dayLabel}</Text>
                        <Text class="text-xs font-semibold text-atelier-accent">{step().title}</Text>
                      </View>
                      <Text class="text-[11px] leading-relaxed text-muted-foreground">{step().detail}</Text>
                    </View>
                  </View>
                )}
              </Each>
            </View>
          </Card>

          {/* 伦理红线承诺 */}
          <SunkenBox>
            <View class="flex-row items-center gap-1.5">
              <QiIcon name="user_shield" size={14} color={token("vital-prime")} />
              <Text class="text-xs font-bold text-vital-prime">知气伦理红线承诺</Text>
            </View>
            <Each of={promises()} key={(p) => p}>
              {(item) => (
                <View class="flex-row items-start gap-1.5">
                  <QiIcon name="check" size={12} color={token("vital-prime")} />
                  <Text class="flex-1 text-[11px] leading-relaxed text-muted-foreground">{item()}</Text>
                </View>
              )}
            </Each>
          </SunkenBox>

          {/* 柔和确认 CTA */}
          <View class="gap-2 pt-1">
            <ActionButton
              title={
                activeTier() === "free"
                  ? "继续素心清欢（永久免费）"
                  : `开通 / 切换：${selectedPlan().title}${
                      props.guardianSponsor ? " + 添柴 ¥12" : ""
                    }`
              }
              icon={activeTier() === "free" ? "tea_cup" : "wax_seal"}
              size="lg"
              onPress={() => props.onConfirm(activeTier(), props.guardianSponsor)}
            />
            <ActionButton
              title="暂不决定 · 返回今日窗台"
              variant="ghost"
              size="sm"
              onPress={props.onClose}
            />
          </View>
        </View>
      </Scroll>
    </ModalSurface>
  );
}

export interface SeasonalPassNode {
  id: string;
  title: string;
  track: "free" | "atelier";
  dewCost: number;
  claimed: boolean;
  description?: string;
}

export interface SeasonalArchiveItem {
  solarTerm: string;
  title: string;
  badge: string;
  unlocked: boolean;
}

/** 2. 岁时风物季票面板：清欢/雅集双轨 + 晨露余额 + 往期节气博物架 + 服饰/印泥/天象/3格陈列 */
export function SeasonalPassPanel(props: {
  solarTerm: string;
  solarSubtitle?: string;
  dewBalance: number;
  atelierUnlocked: boolean;
  nodes: SeasonalPassNode[];
  archiveItems?: SeasonalArchiveItem[];
  activeOutfit?: string;
  outfits?: { id: string; label: string; atelierOnly?: boolean }[];
  onSelectOutfit?: (id: string) => void;
  activeSealInk?: SealInkId;
  onSelectSealInk?: (ink: SealInkId) => void;
  weatherMood?: WeatherMood;
  onSelectWeatherMood?: (mood: WeatherMood) => void;
  equippedKeepsakes?: string[];
  availableKeepsakes?: { id: string; name: string; unlocked: boolean }[];
  onToggleEquipKeepsake?: (id: string) => void;
  onClaim: (nodeId: string) => void;
  onOpenAtelier?: () => void;
}) {
  const showArchive = signal<boolean>(false);

  const weatherOptions: { id: WeatherMood; label: string }[] = [
    { id: "sunny_crisp", label: "晴岚暖阳" },
    { id: "misty_rain", label: "烟雨江南" },
    { id: "first_snow", label: "初雪炉暖" },
    { id: "clear_moon", label: "静夜澄月" },
  ];

  const sealOptions: { id: SealInkId; label: string }[] = [
    { id: "cinnabar", label: "朱砂印泥" },
    { id: "gold", label: "泥金印泥" },
    { id: "lapis", label: "石青印泥" },
  ];

  const freeNodes = () => props.nodes.filter((n) => n.track === "free");
  const atelierNodes = () => props.nodes.filter((n) => n.track === "atelier");

  return (
    <Card>
      <SectionHeader
        title={`岁时风物季票 · ${props.solarTerm}`}
        subtitle={props.solarSubtitle ?? "晨露只因好好生活而积攒，不设限时焦虑"}
        right={
          <View class="flex-row items-center gap-1 rounded-full bg-dew-soft px-3 py-1">
            <QiIcon name="droplet" size={13} color={token("dew-accent")} />
            <Text
              class="text-xs font-bold text-dew-accent"
              style={{ fontVariant: ["tabular-nums"] }}
            >
              {`${props.dewBalance} 💧`}
            </Text>
          </View>
        }
      />

      {/* 双轨奖励：清欢轨 vs 雅集轨 */}
      <View class="gap-3">
        <View class="gap-2">
          <Text class="text-xs font-semibold text-vital-prime">清欢轨 · 人人可领</Text>
          <View class="flex-row flex-wrap gap-2">
            <Each of={freeNodes()} key={(n) => n.id}>
              {(node) => (
                <View class="min-w-40 flex-1 gap-1.5 rounded-xl border border-border/70 bg-bento-sage/60 p-3">
                  <View class="flex-row items-center justify-between gap-1">
                    <Text class="text-xs font-bold text-foreground">{node().title}</Text>
                    <Text
                      class="text-[11px] font-semibold text-dew-accent"
                      style={{ fontVariant: ["tabular-nums"] }}
                    >
                      {`${node().dewCost} 💧`}
                    </Text>
                  </View>
                  <If when={Boolean(node().description)}>
                    <Text class="text-[11px] text-muted-foreground">{node().description}</Text>
                  </If>
                  <ActionButton
                    title={node().claimed ? "已收入窗台" : "采撷风物"}
                    size="sm"
                    variant={node().claimed ? "secondary" : "primary"}
                    disabled={node().claimed || props.dewBalance < node().dewCost}
                    onPress={() => props.onClaim(node().id)}
                  />
                </View>
              )}
            </Each>
          </View>
        </View>

        <View class="gap-2">
          <View class="flex-row items-center justify-between">
            <Text class="text-xs font-semibold text-atelier-accent">雅集轨 · 四时珍藏</Text>
            <If when={!props.atelierUnlocked && Boolean(props.onOpenAtelier)}>
              <ActionButton
                title="了解雅集"
                size="sm"
                variant="outline"
                shape="pill"
                onPress={() => props.onOpenAtelier?.()}
              />
            </If>
          </View>
          <View class="flex-row flex-wrap gap-2">
            <Each of={atelierNodes()} key={(n) => n.id}>
              {(node) => (
                <View
                  class={`min-w-40 flex-1 gap-1.5 rounded-xl border p-3 ${
                    props.atelierUnlocked
                      ? "border-atelier-accent/40 bg-atelier-soft/65"
                      : "border-border/65 border-dashed bg-surface-sunken opacity-85"
                  }`}
                >
                  <View class="flex-row items-center justify-between gap-1">
                    <Text class="text-xs font-bold text-foreground">{node().title}</Text>
                    <Text
                      class="text-[11px] font-semibold text-atelier-accent"
                      style={{ fontVariant: ["tabular-nums"] }}
                    >
                      {`${node().dewCost} 💧`}
                    </Text>
                  </View>
                  <If when={Boolean(node().description)}>
                    <Text class="text-[11px] text-muted-foreground">{node().description}</Text>
                  </If>
                  <ActionButton
                    title={
                      !props.atelierUnlocked
                        ? "雅集解锁"
                        : node().claimed
                          ? "已珍藏"
                          : "领取雅集风物"
                    }
                    size="sm"
                    variant={node().claimed ? "secondary" : "outline"}
                    disabled={
                      props.atelierUnlocked && (node().claimed || props.dewBalance < node().dewCost)
                    }
                    onPress={() => {
                      if (!props.atelierUnlocked) props.onOpenAtelier?.();
                      else props.onClaim(node().id);
                    }}
                  />
                </View>
              )}
            </Each>
          </View>
        </View>
      </View>

      <Hairline />

      {/* 窗台天象、服饰、印泥与 3 格摆件槽位 */}
      <If when={Boolean(props.onSelectWeatherMood)}>
        <View class="gap-1.5">
          <Text class="text-xs font-semibold text-foreground">窗外岁时天象</Text>
          <View class="flex-row flex-wrap gap-1.5">
            <Each of={weatherOptions} key={(w) => w.id}>
              {(w) => (
                <SelectChip
                  label={w().label}
                  selected={(props.weatherMood ?? "sunny_crisp") === w().id}
                  onPress={() => props.onSelectWeatherMood?.(w().id)}
                />
              )}
            </Each>
          </View>
        </View>
      </If>

      <If when={Boolean(props.outfits && props.outfits.length > 0 && props.onSelectOutfit)}>
        <View class="gap-1.5">
          <Text class="text-xs font-semibold text-foreground">伙伴节气小衫</Text>
          <View class="flex-row flex-wrap gap-1.5">
            <Each of={props.outfits ?? []} key={(o) => o.id}>
              {(o) => (
                <SelectChip
                  label={o().atelierOnly && !props.atelierUnlocked ? `${o().label} · 雅集` : o().label}
                  selected={props.activeOutfit === o().id}
                  onPress={() => props.onSelectOutfit?.(o().id)}
                />
              )}
            </Each>
          </View>
        </View>
      </If>

      <If when={Boolean(props.onSelectSealInk)}>
        <View class="gap-1.5">
          <Text class="text-xs font-semibold text-foreground">日签与手账印泥</Text>
          <View class="flex-row flex-wrap gap-1.5">
            <Each of={sealOptions} key={(s) => s.id}>
              {(s) => (
                <SelectChip
                  label={s().label}
                  selected={(props.activeSealInk ?? "cinnabar") === s().id}
                  onPress={() => props.onSelectSealInk?.(s().id)}
                />
              )}
            </Each>
          </View>
        </View>
      </If>

      <If when={Boolean(props.availableKeepsakes && props.onToggleEquipKeepsake)}>
        <View class="gap-1.5">
          <View class="flex-row items-center justify-between">
            <Text class="text-xs font-semibold text-foreground">窗台 3 格陈列摆件</Text>
            <Text
              class="text-[11px] text-muted-foreground"
              style={{ fontVariant: ["tabular-nums"] }}
            >
              {`已陈列 ${(props.equippedKeepsakes ?? []).length}/3`}
            </Text>
          </View>
          <View class="flex-row flex-wrap gap-1.5">
            <Each of={props.availableKeepsakes ?? []} key={(k) => k.id}>
              {(k) => (
                <SelectChip
                  label={k().name}
                  selected={(props.equippedKeepsakes ?? []).includes(k().id)}
                  disabled={!k().unlocked}
                  onPress={() => props.onToggleEquipKeepsake?.(k().id)}
                />
              )}
            </Each>
          </View>
        </View>
      </If>

      {/* 往期节气博物架抽屉切换 */}
      <If when={Boolean(props.archiveItems && props.archiveItems.length > 0)}>
        <View class="gap-2 pt-1">
          <ActionButton
            title={showArchive() ? "收起往期节气博物架" : "展开往期节气博物架"}
            icon="handscroll"
            size="sm"
            variant="secondary"
            onPress={() => showArchive.update((v) => !v)}
          />
          <If when={showArchive()}>
            <View class="flex-row flex-wrap gap-2 rounded-xl border border-border/65 bg-surface-sunken p-3">
              <Each of={props.archiveItems ?? []} key={(a) => `${a.solarTerm}-${a.title}`}>
                {(item) => (
                  <View class="min-w-36 flex-1 gap-1 rounded-xl border border-border/60 bg-card p-2.5">
                    <View class="flex-row items-center justify-between">
                      <Text class="text-xs font-bold text-foreground">{item().solarTerm}</Text>
                      <Text class="text-[10px] font-semibold text-atelier-accent">
                        {item().unlocked ? item().badge : "可补采"}
                      </Text>
                    </View>
                    <Text class="text-[11px] text-muted-foreground">{item().title}</Text>
                  </View>
                )}
              </Each>
            </View>
          </If>
        </View>
      </If>
    </Card>
  );
}

export type StickerFrameStyle = "watercolor" | "polaroid" | "celadon";

export interface FoodStickerItem {
  id: string;
  date: string;
  dayNumber: number;
  title: string;
  frameStyle: StickerFrameStyle;
  purineHint?: string;
  sodiumHint?: string;
  kcal?: number;
}

/** 3. 模切食光贴纸造物机 & 人间烟火月历垫 */
export function StickerCameraMatPanel(props: {
  stickers: FoodStickerItem[];
  monthlyQuotaUsed: number;
  monthlyQuotaLimit?: number;
  atelierUnlocked: boolean;
  onCreateSticker: (input: {
    title: string;
    frameStyle: StickerFrameStyle;
    purineHint?: string;
  }) => void;
  onOpenAtelier?: () => void;
}) {
  const draftTitle = signal<string>("清蒸鲈鱼 · 温热汤饭");
  const draftFrame = signal<StickerFrameStyle>("watercolor");
  const draftHint = signal<string>("中低嘌呤 · 宜多饮温水");
  const limit = () => props.monthlyQuotaLimit ?? 5;
  const quotaReached = () => !props.atelierUnlocked && props.monthlyQuotaUsed >= limit();

  const frameStyles: { id: StickerFrameStyle; label: string }[] = [
    { id: "watercolor", label: "水彩模切" },
    { id: "polaroid", label: "拍立得相纸" },
    { id: "celadon", label: "青瓷釉框" },
  ];

  const days = Array.from({ length: 28 }, (_, i) => i + 1);

  return (
    <Card>
      <SectionHeader
        title="食光贴纸相机 · 人间烟火月历垫"
        subtitle="把三餐变成可收藏的模切贴纸，附温柔嘌呤/钠盐小贴士"
        right={
          <View
            class={`rounded-full px-2.5 py-1 ${
              props.atelierUnlocked ? "bg-atelier-soft" : "bg-surface-sunken border border-border/70"
            }`}
          >
            <Text
              class={`text-[11px] font-semibold ${
                props.atelierUnlocked ? "text-atelier-accent" : "text-foreground"
              }`}
              style={{ fontVariant: ["tabular-nums"] }}
            >
              {props.atelierUnlocked
                ? "雅集无限贴纸"
                : `本月免费 ${props.monthlyQuotaUsed}/${limit()}`}
            </Text>
          </View>
        }
      />

      {/* Canvas 实时模切贴纸预览框 */}
      <View class="flex-row flex-wrap items-center gap-3 rounded-2xl border border-border/65 bg-surface-sunken p-3">
        <Canvas
          style={{ width: 122, height: 122 }}
          draw={(cx, size) => {
            const w = size.width;
            const h = size.height;
            const style = draftFrame();
            cx.clearRect(0, 0, w, h);

            // 外层模切白边
            cx.save();
            cx.fillStyle = token("card");
            cx.strokeStyle =
              style === "celadon"
                ? token("primary")
                : style === "watercolor"
                  ? token("atelier-accent")
                  : token("border");
            cx.lineWidth = 2;
            cx.beginPath();
            cx.roundRect(6, 6, w - 12, h - 12, style === "polaroid" ? 8 : 18);
            cx.fill();
            cx.stroke();
            cx.restore();

            // 内框美食画布区
            cx.save();
            cx.fillStyle =
              style === "celadon"
                ? token("bento-sage")
                : style === "watercolor"
                  ? token("bento-peach")
                  : token("bento-amber");
            const innerH = style === "polaroid" ? h - 42 : h - 28;
            cx.beginPath();
            cx.roundRect(14, 14, w - 28, innerH, style === "polaroid" ? 4 : 12);
            cx.fill();
            cx.restore();

            // 中心食物器皿矢量简绘
            cx.save();
            cx.strokeStyle = token("primary");
            cx.lineWidth = 2;
            cx.beginPath();
            cx.arc(w / 2, style === "polaroid" ? 50 : 58, 20, 0, Math.PI * 2);
            cx.stroke();
            cx.beginPath();
            cx.arc(w / 2, style === "polaroid" ? 50 : 58, 12, 0.2, Math.PI * 1.4);
            cx.stroke();
            cx.restore();

            // 底部手写标签
            cx.save();
            cx.fillStyle = token("foreground");
            cx.font = '600 10px "PingFang SC", sans-serif';
            cx.textAlign = "center";
            cx.textBaseline = "middle";
            const shortTitle = (draftTitle() || "今日好食").slice(0, 8);
            cx.fillText(shortTitle, w / 2, h - 18);
            cx.restore();
          }}
        />

        <View class="min-w-52 flex-1 gap-2">
          <FieldInput
            value={draftTitle()}
            placeholder="例如：清炒时蔬 · 糙米饭"
            onChange={(val) => draftTitle.set(val)}
          />
          <View class="flex-row flex-wrap gap-1.5">
            <Each of={frameStyles} key={(f) => f.id}>
              {(f) => (
                <SelectChip
                  label={f().label}
                  selected={draftFrame() === f().id}
                  onPress={() => draftFrame.set(f().id)}
                />
              )}
            </Each>
          </View>
          <View class="flex-row flex-wrap items-center gap-1.5">
            <View class="rounded-full bg-bento-amber px-2.5 py-0.5">
              <Text class="text-[11px] font-medium text-foreground">{draftHint()}</Text>
            </View>
          </View>
          <ActionButton
            title={quotaReached() ? "本月 5 张免费贴纸已满 · 了解雅集无限贴纸" : "制作并贴入食光月历"}
            icon="camera"
            size="sm"
            variant={quotaReached() ? "outline" : "primary"}
            onPress={() => {
              if (quotaReached()) {
                props.onOpenAtelier?.();
                return;
              }
              const trimmed = draftTitle().trim() || "温润家常便当";
              props.onCreateSticker({
                title: trimmed,
                frameStyle: draftFrame(),
                purineHint: draftHint(),
              });
            }}
          />
        </View>
      </View>

      {/* 人间烟火月历垫网格 */}
      <View class="gap-2 rounded-2xl border border-border/65 bg-bento-amber/40 p-3">
        <View class="flex-row items-center justify-between">
          <Text class="text-xs font-bold text-foreground">本月食光月历垫</Text>
          <Text class="text-[11px] text-muted-foreground" style={{ fontVariant: ["tabular-nums"] }}>
            {`已收录 ${props.stickers.length} 枚贴纸`}
          </Text>
        </View>
        <View class="flex-row flex-wrap gap-1.5">
          <Each of={days} key={(d) => `cal-day-${d}`}>
            {(day) => {
              const matched = () => props.stickers.filter((s) => s.dayNumber === day());
              return (
                <View
                  class={`h-11 w-10 items-center justify-center rounded-xl border ${
                    matched().length > 0
                      ? "border-atelier-accent/45 bg-card"
                      : "border-border/45 bg-surface-sunken/70"
                  }`}
                >
                  <Text
                    class="text-[10px] font-semibold text-muted-foreground"
                    style={{ fontVariant: ["tabular-nums"] }}
                  >
                    {`${day()}`}
                  </Text>
                  <If when={matched().length > 0}>
                    <View class="mt-0.5 rounded-full bg-atelier-soft px-1.5">
                      <Text class="text-[9px] font-bold text-atelier-accent">
                        {`${matched().length}贴`}
                      </Text>
                    </View>
                  </If>
                </View>
              );
            }}
          </Each>
        </View>
      </View>
    </Card>
  );
}

export interface CompanionLetterItem {
  weekIndex: number;
  title: string;
  solarTerm: string;
  dateLabel: string;
  body: string;
  exLibrisTitle: string;
  exLibrisQuote: string;
  unlocked: boolean;
}

/** 4. 《周日夜话慢递》火漆信封与藏书票读信弹窗 */
export function CompanionLetterModal(props: {
  open: boolean;
  letters: CompanionLetterItem[];
  selectedWeek?: number;
  onSelectWeek?: (weekIndex: number) => void;
  atelierUnlocked: boolean;
  onOpenAtelier?: () => void;
  onClose: () => void;
}) {
  const localWeek = signal<number>(props.selectedWeek ?? 1);
  const sealMotion = createMotion();
  const isUnsealed = signal<boolean>(true);

  const activeWeek = () => props.selectedWeek ?? localWeek();
  const currentLetter = () =>
    props.letters.find((l) => l.weekIndex === activeWeek()) ?? props.letters[0] ?? null;

  const chooseWeek = (week: number) => {
    localWeek.set(week);
    props.onSelectWeek?.(week);
  };

  return (
    <ModalSurface open={props.open} presentation="pageSheet" onClose={props.onClose}>
      <Scroll class="flex-1" contentStyle={{ paddingHorizontal: 18, paddingBottom: 28, gap: 14 }}>
        <View class="mx-auto w-full max-w-xl gap-4">
          <View class="flex-row items-center justify-between">
            <View class="flex-row items-center gap-2">
              <EnvelopeIcon size={18} color={token("wax-seal")} />
              <Text role="header" class="text-lg font-bold text-foreground">
                周日夜话慢递
              </Text>
            </View>
            <ActionButton title="收好信笺" size="sm" variant="secondary" shape="pill" onPress={props.onClose} />
          </View>

          {/* 周次选择器 */}
          <View class="flex-row flex-wrap gap-1.5">
            <Each of={props.letters} key={(l) => `week-${l.weekIndex}`}>
              {(letter) => (
                <SelectChip
                  label={`第 ${letter().weekIndex} 周 · ${letter().solarTerm}`}
                  selected={activeWeek() === letter().weekIndex}
                  onPress={() => chooseWeek(letter().weekIndex)}
                />
              )}
            </Each>
          </View>

          <If when={currentLetter()}>
            {(letter) => (
              <View class="gap-3.5">
                {/* 火漆信封封面 */}
                <View class="items-center gap-2.5 rounded-2xl border border-wax-seal/30 bg-wax-seal-soft/65 p-4">
                  <View style={sealMotion.style}>
                    <Button
                      hitSlop={4}
                      onPress={() => {
                        void sealMotion.play("popIn");
                        isUnsealed.update((v) => !v);
                      }}
                      class="size-14 items-center justify-center rounded-full border-2 border-wax-seal bg-card"
                    >
                      <WaxSealIcon size={26} color={token("wax-seal")} />
                    </Button>
                  </View>
                  <Text class="text-sm font-bold text-foreground">{letter().title}</Text>
                  <Text class="text-xs text-muted-foreground">
                    {`${letter().dateLabel} · ${letter().solarTerm}来信`}
                  </Text>
                </View>

                <If
                  when={letter().unlocked || letter().weekIndex === 1 || props.atelierUnlocked}
                  else={
                    <Card class="items-center gap-3 py-6">
                      <Text class="text-sm font-semibold text-foreground">
                        第 1 周初遇信永久免费可读，第 2 周起收录于知气雅集
                      </Text>
                      <Text class="text-xs text-muted-foreground">
                        你的生活轨迹不会被清空，随时开启即可读到陪伴伙伴为你写的长信。
                      </Text>
                      <If when={Boolean(props.onOpenAtelier)}>
                        <ActionButton
                          title="开启知气雅集"
                          icon="wax_seal"
                          onPress={() => props.onOpenAtelier?.()}
                        />
                      </If>
                    </Card>
                  }
                >
                  <If when={isUnsealed()}>
                    {/* 暖米色信纸正文 */}
                    <View class="gap-3 rounded-2xl border border-border bg-card p-5">
                      <Text class="text-sm leading-loose text-foreground">{letter().body}</Text>
                    </View>

                    {/* 底部节气藏书票 */}
                    <View class="flex-row items-center gap-3 rounded-2xl border-2 border-ink-lapis/40 bg-dew-soft/55 p-3.5">
                      <View class="size-11 items-center justify-center rounded-xl border border-ink-gold/50 bg-card">
                        <BadgeCeramicIcon size={22} color={token("ink-gold")} />
                      </View>
                      <View class="flex-1 gap-0.5">
                        <Text class="text-xs font-bold text-ink-lapis">
                          {`节气藏书票 · ${letter().exLibrisTitle}`}
                        </Text>
                        <Text class="text-xs italic text-foreground">{letter().exLibrisQuote}</Text>
                      </View>
                    </View>
                  </If>
                </If>
              </View>
            )}
          </If>
        </View>
      </Scroll>
    </ModalSurface>
  );
}

/** 5. 隐私优先《同窗共养》抽屉：1字节心情 + 互助动作 + 金桂盆栽 + 绝不共享声明 */
export function CoSanctuarySheet(props: {
  open: boolean;
  myMood: "sunny_full" | "breeze_steady" | "rest_recharge" | string;
  onSelectMood: (mood: "sunny_full" | "breeze_steady" | "rest_recharge") => void;
  partnerName?: string;
  partnerMood?: string;
  osmanthusLevel: number;
  osmanthusProgressPct: number;
  careHistory?: { id: string; label: string; timeLabel: string }[];
  onSendCare: (action: "warm_tea" | "blanket" | "paw_print") => void;
  onClose: () => void;
}) {
  const moodOptions: { id: "sunny_full" | "breeze_steady" | "rest_recharge"; label: string }[] = [
    { id: "sunny_full", label: "晴朗满格" },
    { id: "breeze_steady", label: "微风平稳" },
    { id: "rest_recharge", label: "静养蓄力" },
  ];

  const careActions: { id: "warm_tea" | "blanket" | "paw_print"; label: string }[] = [
    { id: "warm_tea", label: "递热茶" },
    { id: "blanket", label: "盖薄被" },
    { id: "paw_print", label: "按爪印" },
  ];

  return (
    <SheetDialog open={props.open} title="同窗共养 · 双人暖窗" onClose={props.onClose}>
      {/* 隐私信任盾牌 */}
      <View class="flex-row items-center gap-2 rounded-2xl border border-vital-prime/35 bg-vital-prime-soft px-3.5 py-2.5">
        <QiIcon name="user_shield" size={16} color={token("vital-prime")} />
        <Text class="flex-1 text-xs font-semibold text-vital-prime">
          绝不共享日记与化验 · 仅加密交换 1 字节心情枚举与合养盆栽成长值
        </Text>
      </View>

      {/* 我的 1 字节心情状态 */}
      <Card>
        <SectionHeader title="今日我的窗台心情" subtitle="只同步一句话的气象隐喻，不暴露具体分数或作息" />
        <View class="flex-row flex-wrap gap-2">
          <Each of={moodOptions} key={(m) => m.id}>
            {(m) => (
              <SelectChip
                label={m().label}
                selected={props.myMood === m().id}
                onPress={() => props.onSelectMood(m().id)}
              />
            )}
          </Each>
        </View>
      </Card>

      {/* 窗台岁时金桂盆栽 */}
      <Card class="border-duo-accent/35 bg-duo-soft/45">
        <View class="flex-row items-center justify-between gap-2">
          <View class="flex-row items-center gap-2">
            <CoSanctuaryIcon size={18} color={token("duo-accent")} />
            <View class="gap-0.5">
              <Text class="text-sm font-bold text-foreground">窗台岁时金桂盆栽</Text>
              <Text class="text-xs text-muted-foreground">
                {`${props.partnerName ?? "同窗好友"}当前状态：${props.partnerMood ?? "微风平稳"}`}
              </Text>
            </View>
          </View>
          <View class="rounded-full bg-card px-2.5 py-1 border border-duo-accent/35">
            <Text
              class="text-xs font-bold text-duo-accent"
              style={{ fontVariant: ["tabular-nums"] }}
            >
              {`Lv.${props.osmanthusLevel}`}
            </Text>
          </View>
        </View>

        <MeterBar value={props.osmanthusProgressPct} accessibilityLabel="金桂盆栽共养成长进度" />

        <View class="flex-row flex-wrap gap-2 pt-1">
          <Each of={careActions} key={(c) => c.id}>
            {(act) => (
              <ActionButton
                title={act().label}
                size="sm"
                variant="outline"
                icon={act().id === "warm_tea" ? "tea_cup" : act().id === "blanket" ? "moon" : "paw"}
                onPress={() => props.onSendCare(act().id)}
                class="flex-1"
              />
            )}
          </Each>
        </View>
      </Card>

      <If when={Boolean(props.careHistory && props.careHistory.length > 0)}>
        <SunkenBox>
          <Text class="text-xs font-semibold text-foreground">近期暖窗互寄记录</Text>
          <Each of={props.careHistory ?? []} key={(h) => h.id}>
            {(row) => (
              <View class="flex-row flex-wrap items-center justify-between gap-1.5">
                <Text class="flex-1 text-xs text-foreground">{row().label}</Text>
                <Text class="text-[11px] text-muted-foreground">{row().timeLabel}</Text>
              </View>
            )}
          </Each>
        </SunkenBox>
      </If>
    </SheetDialog>
  );
}

export interface SeasonScrollSummary {
  season: string;
  solarRange: string;
  highlight: string;
  avgQiScore: number;
  dewCollected: number;
}

export interface CeramicBadgeItem {
  id: string;
  title: string;
  subtitle: string;
  unlocked: boolean;
}

/** 6. 《我的四时身体风物长卷》横向长卷弹窗 + 宋瓷勋章架 */
export function FourSeasonsScrollModal(props: {
  open: boolean;
  seasons: SeasonScrollSummary[];
  badges: CeramicBadgeItem[];
  onClose: () => void;
}) {
  return (
    <ModalSurface open={props.open} presentation="pageSheet" onClose={props.onClose}>
      <Scroll class="flex-1" contentStyle={{ paddingHorizontal: 18, paddingBottom: 28, gap: 16 }}>
        <View class="mx-auto w-full max-w-3xl gap-4">
          <View class="flex-row items-center justify-between">
            <View class="flex-row items-center gap-2">
              <HandscrollIcon size={18} color={token("ink-lapis")} />
              <Text role="header" class="text-lg font-bold text-foreground">
                我的四时身体风物长卷
              </Text>
            </View>
            <ActionButton title="卷起长卷" size="sm" variant="secondary" shape="pill" onPress={props.onClose} />
          </View>

          {/* 横向卷轴插画 */}
          <Scroll horizontal={true} class="w-full rounded-2xl border border-border bg-card">
            <View class="flex-row items-center gap-3 p-2.5">
              <Image
                src={fourSeasonsHandscrollImg}
                class="h-44 w-160 rounded-xl"
                style={{ objectFit: "cover", objectPosition: "center" }}
              />
            </View>
          </Scroll>

          {/* 四季身体摘要卡 */}
          <View class="flex-row flex-wrap gap-2.5">
            <Each of={props.seasons} key={(s) => s.season}>
              {(item) => (
                <View class="min-w-44 flex-1 gap-1.5 rounded-2xl border border-border/75 bg-card p-3.5">
                  <View class="flex-row items-center justify-between">
                    <Text class="text-sm font-bold text-foreground">{item().season}</Text>
                    <Text class="text-[11px] font-semibold text-atelier-accent">{item().solarRange}</Text>
                  </View>
                  <Text
                    class="text-xs font-semibold text-primary"
                    style={{ fontVariant: ["tabular-nums"] }}
                  >
                    {`均元气 ${Math.round(item().avgQiScore)} · 晨露 ${item().dewCollected} 💧`}
                  </Text>
                  <Text class="text-xs leading-relaxed text-muted-foreground">{item().highlight}</Text>
                </View>
              )}
            </Each>
          </View>

          {/* 宋瓷成就徽章 */}
          <Card>
            <SectionHeader title="宋瓷风物印章" subtitle="记录每一次自我觉察与温和证伪" />
            <View class="flex-row flex-wrap gap-2">
              <Each of={props.badges} key={(b) => b.id}>
                {(badge) => (
                  <View
                    class={`min-w-40 flex-1 flex-row items-center gap-2.5 rounded-xl border p-3 ${
                      badge().unlocked
                        ? "border-ink-gold/45 bg-atelier-soft/60"
                        : "border-border/60 border-dashed bg-surface-sunken opacity-70"
                    }`}
                  >
                    <BadgeCeramicIcon
                      size={22}
                      color={badge().unlocked ? token("ink-gold") : token("muted-foreground")}
                    />
                    <View class="flex-1 gap-0.5">
                      <Text class="text-xs font-bold text-foreground">{badge().title}</Text>
                      <Text class="text-[11px] text-muted-foreground">{badge().subtitle}</Text>
                    </View>
                  </View>
                )}
              </Each>
            </View>
          </Card>
        </View>
      </Scroll>
    </ModalSurface>
  );
}

/** 7. 《抬腕嘴替》Apple Watch / 桌面微件预览卡（诗意隐喻 vs 精准数值切换） */
export function WristWidgetPreviewCard(props: {
  score: number;
  state: string;
  stateBadge: string;
  poeticLine: string;
  numericLine?: string;
  solarTerm?: string;
  mode?: "poetic" | "numeric";
  onChangeMode?: (mode: "poetic" | "numeric") => void;
}) {
  const localMode = signal<"poetic" | "numeric">(props.mode ?? "poetic");
  const activeMode = () => props.mode ?? localMode();
  const setMode = (m: "poetic" | "numeric") => {
    localMode.set(m);
    props.onChangeMode?.(m);
  };

  return (
    <Card>
      <SectionHeader
        title="抬腕嘴替 · 表盘与桌面微件"
        subtitle="一抬腕就看懂今天该冲还是该歇，不制造评分负担"
        right={<WidgetIcon size={18} color={token("primary")} />}
      />

      <View class="flex-row gap-1.5">
        <SelectChip
          label="诗意隐喻"
          selected={activeMode() === "poetic"}
          onPress={() => setMode("poetic")}
        />
        <SelectChip
          label="精准数值"
          selected={activeMode() === "numeric"}
          onPress={() => setMode("numeric")}
        />
      </View>

      {/* 模拟表盘卡片预览 */}
      <View class="mx-auto w-full max-w-xs gap-2 rounded-3xl border-2 border-border bg-surface-sunken p-4">
        <View class="flex-row items-center justify-between">
          <Text class="text-[11px] font-semibold text-atelier-accent">
            {props.solarTerm ? `知气 · ${props.solarTerm}` : "知气 · Qi"}
          </Text>
          <VitalBadge tone={props.state} label={props.stateBadge} />
        </View>

        <If
          when={activeMode() === "poetic"}
          else={
            <View class="gap-1 py-1">
              <Text
                class="text-2xl font-bold text-foreground"
                style={{ fontVariant: ["tabular-nums"] }}
              >
                {`元气 ${Math.round(props.score)} / 100`}
              </Text>
              <Text
                class="text-xs text-muted-foreground"
                style={{ fontVariant: ["tabular-nums"] }}
              >
                {props.numericLine ?? `当前节律 ${props.stateBadge}`}
              </Text>
            </View>
          }
        >
          <View class="gap-1 py-1">
            <Text class="text-base font-bold leading-snug text-foreground">{props.poeticLine}</Text>
            <Text class="text-[11px] text-muted-foreground">数字已温柔隐去 · 顺从体感节律</Text>
          </View>
        </If>
      </View>
    </Card>
  );
}

/** 8. 父母大字端到端加密《气韵平安家书》阅读与一键发送弹窗 */
export function FamilyPeaceLetterModal(props: {
  open: boolean;
  recipientLabel?: string;
  dateLabel: string;
  peaceHeadline: string;
  bodyParagraphs: string[];
  signatureLine?: string;
  onCopyLetter: (fullText: string) => void;
  onClose: () => void;
}) {
  const fullLetterText = () =>
    [
      `【气韵平安家书 · ${props.dateLabel}】`,
      `${props.recipientLabel ?? "亲爱的爸妈"}：`,
      props.peaceHeadline,
      ...props.bodyParagraphs,
      props.signatureLine ?? "一切安好，勿念。—— 端侧加密温柔生成",
    ].join("\n\n");

  return (
    <ModalSurface open={props.open} presentation="pageSheet" onClose={props.onClose}>
      <Scroll class="flex-1" contentStyle={{ paddingHorizontal: 20, paddingBottom: 28, gap: 16 }}>
        <View class="mx-auto w-full max-w-lg gap-4">
          <View class="flex-row items-center justify-between">
            <View class="flex-row items-center gap-2">
              <FamilyHeartIcon size={20} color={token("wax-seal")} />
              <Text role="header" class="text-lg font-bold text-foreground">
                气韵平安家书（大字版）
              </Text>
            </View>
            <ActionButton title="合上家书" size="sm" variant="secondary" shape="pill" onPress={props.onClose} />
          </View>

          <View class="flex-row items-center gap-2 rounded-2xl border border-vital-prime/30 bg-vital-prime-soft px-3.5 py-2">
            <QiIcon name="cloud_lock" size={14} color={token("vital-prime")} />
            <Text class="flex-1 text-xs font-semibold text-vital-prime">
              端侧本地摘要生成 · 不外泄任何隐私明细与化验原始数值
            </Text>
          </View>

          <View class="gap-4 rounded-3xl border-2 border-border bg-card p-6">
            <View class="flex-row items-center justify-between">
              <Text class="text-base font-bold text-wax-seal">
                {`${props.recipientLabel ?? "亲爱的爸妈"}：`}
              </Text>
              <Text class="text-xs text-muted-foreground">{props.dateLabel}</Text>
            </View>

            <Text class="text-xl font-bold leading-relaxed text-foreground">
              {props.peaceHeadline}
            </Text>

            <View class="gap-3">
              <Each of={props.bodyParagraphs} key={(p, idx) => `peace-${idx}`}>
                {(para) => (
                  <Text class="text-base font-medium leading-loose text-foreground">{para()}</Text>
                )}
              </Each>
            </View>

            <Hairline />
            <Text class="text-right text-sm font-semibold text-muted-foreground">
              {props.signatureLine ?? "记挂你们，一切安好 · 知气护航"}
            </Text>
          </View>

          <View class="flex-row gap-2">
            <ActionButton
              title="一键复制发给爸妈"
              icon="family_heart"
              size="lg"
              onPress={() => props.onCopyLetter(fullLetterText())}
              class="flex-1"
            />
            <ActionButton title="返回" variant="secondary" size="lg" onPress={props.onClose} />
          </View>
        </View>
      </Scroll>
    </ModalSurface>
  );
}
