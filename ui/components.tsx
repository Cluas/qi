import { type JSX, Scroll, Text, token, View } from "nsh";
import { Button, Collapsible, Dialog, Input, Switch, Tab, Tabs } from "nsh/base";
import { QiIcon, type QiIconName } from "./icons";

export type VitalTone =
  | "prime_ready"
  | "moderate_steady"
  | "recovery_priority"
  | "optimal"
  | "high"
  | "alert_high"
  | "watch_high"
  | "low"
  | "alert_low"
  | "watch_low"
  | "shifted"
  | "no_baseline"
  | "normal"
  | "neutral";

function resolveToneSemanticColor(tone?: string): string {
  switch (tone) {
    case "prime_ready":
    case "optimal":
    case "normal":
      return token("vital-prime");
    case "moderate_steady":
    case "watch_low":
    case "watch_high":
    case "shifted":
    case "low":
      return token("vital-steady");
    case "recovery_priority":
    case "high":
    case "alert_high":
    case "alert_low":
      return token("vital-recover");
    default:
      return token("muted-foreground");
  }
}

/** 宋韵瓷面卡片容器：统一使用 theme.json 语义圆角与温和描边 */
export function Card(props: { class?: string; children?: JSX.Element }) {
  return (
    <View class={`gap-3.5 rounded-card border border-border/90 bg-card p-4 ${props.class ?? ""}`}>
      {props.children}
    </View>
  );
}

/** 浅沉降容器：用于内部次级分组、指标单元格或辅助阅读区 */
export function SunkenBox(props: { class?: string; children?: JSX.Element }) {
  return (
    <View class={`gap-2 rounded-xl border border-border/60 bg-surface-sunken p-3.5 ${props.class ?? ""}`}>
      {props.children}
    </View>
  );
}

/** 区块小标题 */
export function SectionHeader(props: { title: string; subtitle?: string; right?: JSX.Element }) {
  return (
    <View class="flex-row items-center justify-between gap-2.5">
      <View class="flex-1 gap-0.5">
        <Text class="text-base font-semibold text-foreground">{props.title}</Text>
        <If when={Boolean(props.subtitle)}>
          <Text class="text-xs leading-relaxed text-muted-foreground">{props.subtitle}</Text>
        </If>
      </View>
      {props.right}
    </View>
  );
}

/** 三态节律 / 指标状态印章标签 */
export function VitalBadge(props: { tone: VitalTone | string; label: string }) {
  const toneClass = () => {
    switch (props.tone) {
      case "prime_ready":
      case "optimal":
      case "normal":
        return "bg-vital-prime-soft border-vital-prime/30";
      case "moderate_steady":
      case "watch_low":
      case "watch_high":
      case "shifted":
      case "low":
        return "bg-vital-steady-soft border-vital-steady/30";
      case "recovery_priority":
      case "high":
      case "alert_high":
      case "alert_low":
        return "bg-vital-recover-soft border-vital-recover/30";
      default:
        return "bg-secondary border-border";
    }
  };
  const textClass = () => {
    switch (props.tone) {
      case "prime_ready":
      case "optimal":
      case "normal":
        return "text-vital-prime";
      case "moderate_steady":
      case "watch_low":
      case "watch_high":
      case "shifted":
      case "low":
        return "text-vital-steady";
      case "recovery_priority":
      case "high":
      case "alert_high":
      case "alert_low":
        return "text-vital-recover";
      default:
        return "text-secondary-foreground";
    }
  };

  return (
    <View class={`rounded-full border px-2.5 py-1 ${toneClass()}`}>
      <Text class={`text-xs font-semibold ${textClass()}`}>{props.label}</Text>
    </View>
  );
}

/** 统一品牌按钮：保证最小 44pt 触控高度、可选矢量图标与原生按压回弹 */
export function ActionButton(props: {
  title: string;
  onPress: () => void;
  variant?: "primary" | "secondary" | "outline" | "danger";
  disabled?: boolean;
  class?: string;
  icon?: QiIconName;
  iconColor?: string;
}) {
  const variantClass = () => {
    switch (props.variant ?? "primary") {
      case "secondary":
        return "bg-secondary border border-border active:opacity-80";
      case "outline":
        return "bg-card border border-primary/40 active:bg-accent";
      case "danger":
        return "bg-destructive active:opacity-85";
      default:
        return "bg-primary active:opacity-90";
    }
  };
  const labelClass = () => {
    switch (props.variant ?? "primary") {
      case "secondary":
        return "text-secondary-foreground";
      case "outline":
        return "text-primary";
      case "danger":
        return "text-destructive-foreground";
      default:
        return "text-primary-foreground";
    }
  };
  const defaultIconColor = () => {
    switch (props.variant ?? "primary") {
      case "secondary":
        return token("secondary-foreground");
      case "outline":
        return token("primary");
      case "danger":
        return token("destructive-foreground");
      default:
        return token("primary-foreground");
    }
  };

  return (
    <Button
      disabled={props.disabled}
      onPress={props.onPress}
      class={`min-h-11 items-center justify-center rounded-xl px-4 py-2.5 transition active:scale-[0.98] motion-reduce:active:scale-100 disabled:opacity-45 ${variantClass()} ${props.class ?? ""}`}
    >
      <View class="flex-row items-center justify-center gap-2">
        <If when={Boolean(props.icon)}>
          <QiIcon
            name={props.icon!}
            size={16}
            color={props.iconColor ?? defaultIconColor()}
          />
        </If>
        <Text class={`text-sm font-semibold ${labelClass()}`}>{props.title}</Text>
      </View>
    </Button>
  );
}

/** 单选胶囊芯片（用于快捷因子、体感打分与模式选择，附加 hitSlop 保障 44pt+ 触控热区） */
export function SelectChip(props: {
  label: string;
  selected: boolean;
  disabled?: boolean;
  onPress: () => void;
}) {
  return (
    <Button
      disabled={props.disabled}
      hitSlop={4}
      onPress={props.onPress}
      class={`min-h-10 items-center justify-center rounded-full border px-3.5 py-2 transition active:scale-[0.98] motion-reduce:active:scale-100 disabled:opacity-40 ${
        props.selected
          ? "border-primary bg-primary"
          : "border-border bg-surface-sunken active:bg-secondary"
      }`}
    >
      <Text class={`text-xs font-medium ${props.selected ? "text-primary-foreground" : "text-foreground"}`}>
        {props.label}
      </Text>
    </Button>
  );
}

/** 带说明文案的开关行（最小 44pt 行高） */
export function SwitchRow(props: {
  title: string;
  subtitle?: string;
  checked: boolean;
  onChange: (checked: boolean) => void;
}) {
  return (
    <View class="min-h-11 flex-row items-center justify-between gap-3 py-1">
      <View class="flex-1 gap-0.5">
        <Text class="text-sm font-medium text-foreground">{props.title}</Text>
        <If when={Boolean(props.subtitle)}>
          <Text class="text-xs leading-relaxed text-muted-foreground">{props.subtitle}</Text>
        </If>
      </View>
      <Switch
        checked={props.checked}
        onChange={({ checked }) => props.onChange(checked)}
        class="flex-row items-center"
        trackClass="w-11 rounded-full bg-muted p-0.5 transition-colors checked:bg-primary"
        thumbClass="size-5 rounded-full bg-card shadow-sm transition-transform checked:translate-x-5"
      />
    </View>
  );
}

/** 统一输入框 */
export function FieldInput(props: {
  value: string;
  placeholder?: string;
  multiline?: boolean;
  onChange: (value: string) => void;
}) {
  return (
    <Input
      value={props.value}
      placeholder={props.placeholder}
      multiline={props.multiline}
      onChange={({ value }) => props.onChange(value)}
      class={`rounded-xl border border-input bg-card px-3.5 py-2.5 text-sm text-foreground placeholder:text-muted-foreground ${
        props.multiline ? "min-h-24" : "min-h-11"
      }`}
    />
  );
}

/** 原生抽屉/模态对话框包装 */
export function SheetDialog(props: {
  open: boolean;
  title: string;
  onClose: () => void;
  children?: JSX.Element;
}) {
  return (
    <Dialog
      open={props.open}
      presentation="pageSheet"
      onOpenChange={(e) => {
        if (!e.open) props.onClose();
      }}
      class="flex-1 bg-background"
    >
      <View
        class="flex-1"
        style={{
          paddingTop: "calc(env(safe-area-inset-top) + 12px)",
          paddingBottom: "calc(env(safe-area-inset-bottom) + 16px)",
        }}
      >
        <View class="flex-row items-center justify-between border-b border-border px-4 pb-3">
          <Text class="text-lg font-semibold text-foreground">{props.title}</Text>
          <Button
            hitSlop={4}
            onPress={props.onClose}
            class="min-h-10 items-center justify-center rounded-full bg-secondary px-4 py-1.5 active:opacity-75"
          >
            <Text class="text-xs font-semibold text-secondary-foreground">完成</Text>
          </Button>
        </View>
        <Scroll class="flex-1" contentStyle={{ padding: 16, gap: 14 }}>
          {props.children}
        </Scroll>
      </View>
    </Dialog>
  );
}

/** 消费级分段切换标签栏（基于 nsh/base Tabs，自带原生弹簧滑动指示器） */
export function SegmentedTabs(props: {
  items: { id: string; label: string }[];
  activeId: string;
  onSelect: (id: string) => void;
}) {
  return (
    <Tabs
      class="flex-row rounded-2xl border border-border/70 bg-surface-sunken p-1"
      indicatorClass="inset-y-1 rounded-xl bg-card border border-border/50"
    >
      <Each of={props.items} key={(item) => item.id}>
        {(item) => (
          <Tab
            selected={props.activeId === item().id}
            onPress={() => props.onSelect(item().id)}
            class="min-h-11 flex-1 items-center justify-center rounded-xl px-3 py-2"
          >
            <Text
              class={`text-xs font-semibold ${
                props.activeId === item().id ? "text-foreground" : "text-muted-foreground"
              }`}
            >
              {item().label}
            </Text>
          </Tab>
        )}
      </Each>
    </Tabs>
  );
}

/** 渐进披露可折叠卡片（基于 nsh/base Collapsible reveal，顺滑高度动画与 44pt 触控头部） */
export function CollapsibleCard(props: {
  title: string;
  subtitle?: string;
  badgeLabel?: string;
  badgeTone?: string;
  open: boolean;
  onToggle: () => void;
  children?: JSX.Element;
}) {
  return (
    <Card class="gap-0">
      <Collapsible
        open={props.open}
        reveal={true}
        class="gap-0"
        content={
          <View class="mt-3 gap-3 border-t border-border/60 pt-3">
            {props.children}
          </View>
        }
      >
        <View
          role="button"
          accessible={true}
          accessibilityState={{ expanded: props.open }}
          onPress={props.onToggle}
          class="min-h-11 flex-row items-center justify-between gap-3 active:opacity-80"
        >
          <View class="flex-1 gap-0.5">
            <Text class="text-base font-semibold text-foreground">{props.title}</Text>
            <If when={Boolean(props.subtitle)}>
              <Text class="text-xs leading-relaxed text-muted-foreground">{props.subtitle}</Text>
            </If>
          </View>
          <View class="flex-row items-center gap-2">
            <If when={Boolean(props.badgeLabel)}>
              <VitalBadge tone={props.badgeTone ?? "neutral"} label={props.badgeLabel!} />
            </If>
            <QiIcon
              name={props.open ? "chevron_down" : "chevron_right"}
              size={18}
              color={token("muted-foreground")}
            />
          </View>
        </View>
      </Collapsible>
    </Card>
  );
}

/** 宋韵沉降微圆角指标磁贴（带矢量图标、等宽数字与大众化副标题） */
export function StatMetricTile(props: {
  icon: QiIconName;
  label: string;
  value: string;
  sublabel: string;
  tone?: string;
}) {
  return (
    <View class="flex-1 gap-1.5 rounded-xl border border-border/60 bg-surface-sunken p-3.5">
      <View class="flex-row items-center gap-1.5">
        <QiIcon
          name={props.icon}
          size={15}
          color={resolveToneSemanticColor(props.tone)}
        />
        <Text class="text-xs font-medium text-muted-foreground">{props.label}</Text>
      </View>
      <Text
        class="text-xl font-bold text-foreground"
        style={{ fontVariant: ["tabular-nums"] }}
      >
        {props.value}
      </Text>
      <Text class="text-xs leading-relaxed text-muted-foreground">{props.sublabel}</Text>
    </View>
  );
}

/** 消费级状态标签：将底层统计状态翻译为普通人秒懂的温润标签，隐藏 Z-score 等算法术语 */
export function ConsumerStatusPill(props: { status: string }) {
  const mapped = () => {
    switch (props.status) {
      case "optimal":
        return { tone: "optimal", label: "状态充沛" };
      case "normal":
        return { tone: "normal", label: "处于常态" };
      case "watch_low":
        return { tone: "watch_low", label: "略偏低于平时" };
      case "alert_low":
        return { tone: "alert_low", label: "需多留意休息" };
      case "watch_high":
        return { tone: "watch_high", label: "略偏高于平时" };
      case "alert_high":
        return { tone: "alert_high", label: "波动偏明显" };
      case "shifted":
        return { tone: "shifted", label: "节律有所波动" };
      case "no_baseline":
        return { tone: "no_baseline", label: "基线积累中" };
      default:
        return { tone: "neutral", label: "平缓观察中" };
    }
  };

  return <VitalBadge tone={mapped().tone} label={mapped().label} />;
}
