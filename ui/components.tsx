import { type JSX, Scroll, Text, token, View } from "nsh";
import { Button, Collapsible, Dialog, Input, Progress, Switch, Tab, Tabs } from "nsh/base";
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

/** 极细分割线：以留白和 1px 边界替代多层嵌套框 */
export function Hairline(props: { class?: string }) {
  return <View class={`h-px w-full bg-border/65 ${props.class ?? ""}`} />;
}

/** 宋韵瓷面一级卡片：同心外圆角 rounded-2xl 与统一内边距 p-4 */
export function Card(props: { class?: string; children?: JSX.Element }) {
  return (
    <View class={`gap-3.5 rounded-2xl border border-border/85 bg-card p-4 ${props.class ?? ""}`}>
      {props.children}
    </View>
  );
}

/** 同心沉降次级盒：内嵌于 Card 内使用 rounded-xl 与低噪声边界 */
export function SunkenBox(props: { class?: string; children?: JSX.Element }) {
  return (
    <View class={`gap-2 rounded-xl border border-border/55 bg-surface-sunken p-3.5 ${props.class ?? ""}`}>
      {props.children}
    </View>
  );
}

/** 区块小标题：主标题标注语义 header 角色便于无障碍跳转 */
export function SectionHeader(props: { title: string; subtitle?: string; right?: JSX.Element }) {
  return (
    <View class="flex-row items-center justify-between gap-2.5">
      <View class="flex-1 gap-0.5">
        <Text role="header" class="text-base font-semibold text-foreground">
          {props.title}
        </Text>
        <If when={Boolean(props.subtitle)}>
          <Text class="text-xs leading-relaxed text-muted-foreground">{props.subtitle}</Text>
        </If>
      </View>
      {props.right}
    </View>
  );
}

/** 三态节律 / 指标状态印章标签（色彩仅用于真实状态表达，默认采用中性底色） */
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
        return "bg-surface-sunken border-border/75";
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
        return "text-muted-foreground";
    }
  };

  return (
    <View class={`rounded-full border px-2.5 py-1 ${toneClass()}`}>
      <Text class={`text-xs font-semibold ${textClass()}`}>{props.label}</Text>
    </View>
  );
}

/** 统一品牌按钮：统一 size 阶梯与 hover / active / focus-visible / disabled 四态 */
export function ActionButton(props: {
  title: string;
  onPress: () => void;
  variant?: "primary" | "secondary" | "outline" | "ghost" | "danger";
  size?: "sm" | "md" | "lg";
  shape?: "rounded" | "pill";
  disabled?: boolean;
  class?: string;
  icon?: QiIconName;
  iconColor?: string;
  accessibilityLabel?: string;
}) {
  const sizeClass = () => {
    const isPill = props.shape === "pill";
    switch (props.size ?? "md") {
      case "sm":
        return `min-h-10 ${isPill ? "rounded-full" : "rounded-lg"} px-3 py-1.5`;
      case "lg":
        return `min-h-12 ${isPill ? "rounded-full" : "rounded-xl"} px-5 py-3`;
      default:
        return `min-h-11 ${isPill ? "rounded-full" : "rounded-xl"} px-4 py-2.5`;
    }
  };

  const textSizeClass = () => {
    switch (props.size ?? "md") {
      case "sm":
        return "text-xs";
      case "lg":
        return "text-base";
      default:
        return "text-sm";
    }
  };

  const iconSize = () => {
    switch (props.size ?? "md") {
      case "sm":
        return 14;
      case "lg":
        return 18;
      default:
        return 16;
    }
  };

  const variantClass = () => {
    switch (props.variant ?? "primary") {
      case "secondary":
        return "bg-secondary border border-border/80 hover:bg-muted active:opacity-85";
      case "outline":
        return "bg-card border border-primary/40 hover:bg-accent/70 active:bg-accent";
      case "ghost":
        return "bg-transparent hover:bg-surface-sunken active:bg-secondary";
      case "danger":
        return "bg-destructive hover:opacity-90 active:opacity-85";
      default:
        return "bg-primary hover:opacity-92 active:opacity-88";
    }
  };

  const labelClass = () => {
    switch (props.variant ?? "primary") {
      case "secondary":
        return "text-secondary-foreground";
      case "outline":
        return "text-primary";
      case "ghost":
        return "text-foreground";
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
      case "ghost":
        return token("foreground");
      case "danger":
        return token("destructive-foreground");
      default:
        return token("primary-foreground");
    }
  };

  return (
    <Button
      disabled={props.disabled}
      hitSlop={4}
      accessibilityLabel={props.accessibilityLabel ?? props.title}
      onPress={props.onPress}
      class={`items-center justify-center transition duration-150 focus-visible:ring-2 focus-visible:ring-ring active:scale-[0.98] motion-reduce:transition-none motion-reduce:active:scale-100 disabled:opacity-45 ${sizeClass()} ${variantClass()} ${props.class ?? ""}`}
    >
      <View class="flex-row items-center justify-center gap-1.5">
        <If when={Boolean(props.icon)}>
          <QiIcon
            name={props.icon!}
            size={iconSize()}
            color={props.iconColor ?? defaultIconColor()}
          />
        </If>
        <Text class={`font-semibold ${textSizeClass()} ${labelClass()}`}>{props.title}</Text>
      </View>
    </Button>
  );
}

/** 轻量选项行卡（引导选项、快捷饮食场景卡复用，保证对齐脊线且按压不缩放列表项） */
export function ChoiceCard(props: {
  title: string;
  subtitle?: string;
  icon?: QiIconName;
  selected?: boolean;
  disabled?: boolean;
  onPress: () => void;
  class?: string;
}) {
  return (
    <Button
      disabled={props.disabled}
      onPress={props.onPress}
      class={`min-h-11 flex-row items-center gap-3 rounded-xl border px-3.5 py-2.5 transition-colors duration-150 focus-visible:ring-2 focus-visible:ring-ring motion-reduce:transition-none disabled:opacity-45 ${
        props.selected
          ? "border-primary bg-accent hover:bg-accent"
          : "border-border/65 bg-surface-sunken hover:border-primary/40 active:bg-secondary"
      } ${props.class ?? ""}`}
    >
      <If when={Boolean(props.icon)}>
        <View
          class={`size-9 items-center justify-center rounded-lg ${
            props.selected ? "bg-primary" : "bg-card border border-border/55"
          }`}
        >
          <QiIcon
            name={props.icon!}
            size={17}
            color={props.selected ? token("primary-foreground") : token("primary")}
          />
        </View>
      </If>
      <View class="flex-1 gap-0.5">
        <Text class="text-sm font-semibold text-foreground">{props.title}</Text>
        <If when={Boolean(props.subtitle)}>
          <Text class="text-xs leading-relaxed text-muted-foreground">{props.subtitle}</Text>
        </If>
      </View>
    </Button>
  );
}

/** 单选胶囊芯片（附加 hover、focus-visible 与 hitSlop 保障 44pt+ 触控热区） */
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
      class={`min-h-10 items-center justify-center rounded-full border px-3.5 py-2 transition duration-150 focus-visible:ring-2 focus-visible:ring-ring active:scale-[0.98] motion-reduce:transition-none motion-reduce:active:scale-100 disabled:opacity-40 ${
        props.selected
          ? "border-primary bg-primary hover:opacity-92"
          : "border-border/80 bg-surface-sunken hover:border-primary/45 hover:bg-secondary/70 active:bg-secondary"
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
        class="flex-row items-center rounded-full focus-visible:ring-2 focus-visible:ring-ring"
        trackClass="w-11 rounded-full bg-muted p-0.5 transition-colors checked:bg-primary motion-reduce:transition-none"
        thumbClass="size-5 rounded-full bg-card shadow-sm transition-transform checked:translate-x-5 motion-reduce:transition-none"
      />
    </View>
  );
}

/** 统一输入框（含 focus-visible 清晰焦点环） */
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
      class={`rounded-xl border border-input bg-card px-3.5 py-2.5 text-sm text-foreground placeholder:text-muted-foreground focus-visible:border-primary focus-visible:ring-2 focus-visible:ring-ring ${
        props.multiline ? "min-h-24" : "min-h-11"
      }`}
    />
  );
}

/** 通用线性进度条原语 */
export function MeterBar(props: { value: number; accessibilityLabel: string; class?: string }) {
  return (
    <Progress
      value={props.value}
      accessibilityLabel={props.accessibilityLabel}
      class={`w-full ${props.class ?? ""}`}
      trackClass="h-2 w-full overflow-hidden rounded-full bg-border/70"
      indicatorClass="h-full rounded-full bg-primary transition-all duration-300 motion-reduce:transition-none"
    />
  );
}

/** 全局轻量 Toast 反馈横幅 */
export function ToastBanner(props: { message: string; onDismiss: () => void }) {
  return (
    <View class="mx-4 mt-2 flex-row items-center justify-between gap-2 rounded-xl border border-primary/30 bg-accent px-3.5 py-2">
      <Text class="flex-1 text-xs font-medium text-accent-foreground">{props.message}</Text>
      <Button
        onPress={props.onDismiss}
        class="min-h-8 justify-center rounded-lg px-2.5 py-1 transition-colors hover:bg-primary/10 active:opacity-70 focus-visible:ring-2 focus-visible:ring-ring"
      >
        <Text class="text-xs font-semibold text-accent-foreground">关闭</Text>
      </Button>
    </View>
  );
}

/** 桌面端原生滑动侧边导航（基于 nsh/base Tabs 原生 spring_move 指示器，无侧边装饰条） */
export function NavRail<T extends string>(props: {
  items: { id: T; label: string; icon: QiIconName }[];
  activeId: T;
  onSelect: (id: T) => void;
  title?: string;
  footer?: JSX.Element;
}) {
  return (
    <View class="w-56 gap-2.5 border-r border-border bg-card p-3">
      <If when={Boolean(props.title)}>
        <Text class="px-2.5 pt-1 text-xs font-semibold text-muted-foreground">{props.title}</Text>
      </If>
      <Tabs
        class="flex-col gap-1"
        indicatorClass="inset-x-0 rounded-xl bg-primary"
      >
        <Each of={props.items} key={(t) => `side-${t.id}`}>
          {(tab) => (
            <Tab
              selected={props.activeId === tab().id}
              onPress={() => props.onSelect(tab().id)}
              class={`min-h-11 flex-row items-center gap-2.5 rounded-xl px-3.5 transition-colors duration-150 focus-visible:ring-2 focus-visible:ring-ring motion-reduce:transition-none ${
                props.activeId === tab().id ? "" : "hover:bg-surface-sunken active:bg-secondary"
              }`}
            >
              <View class="w-5 items-center justify-center">
                <QiIcon
                  name={tab().icon}
                  size={17}
                  color={props.activeId === tab().id ? token("primary-foreground") : token("foreground")}
                />
              </View>
              <Text
                class={`text-sm font-semibold ${
                  props.activeId === tab().id ? "text-primary-foreground" : "text-foreground"
                }`}
              >
                {tab().label}
              </Text>
            </Tab>
          )}
        </Each>
      </Tabs>
      <If when={Boolean(props.footer)}>
        <View class="mt-auto">{props.footer}</View>
      </If>
    </View>
  );
}

/** 移动端底部 4-Tab 原生滑动导航栏（单选背景随 Tabs 原生弹簧滑移） */
export function BottomTabBar<T extends string>(props: {
  items: { id: T; label: string; icon: QiIconName }[];
  activeId: T;
  onSelect: (id: T) => void;
}) {
  return (
    <Tabs
      class="flex-row items-center gap-1 border-t border-border bg-card px-3 pt-2"
      style={{ paddingBottom: "calc(env(safe-area-inset-bottom) + 8px)" }}
      indicatorClass="inset-y-0 rounded-xl bg-accent"
    >
      <Each of={props.items} key={(t) => `bottom-${t.id}`}>
        {(tab) => (
          <Tab
            selected={props.activeId === tab().id}
            onPress={() => props.onSelect(tab().id)}
            class="min-h-11 flex-1 items-center justify-center gap-1 rounded-xl px-1 py-1.5 transition-colors duration-150 focus-visible:ring-2 focus-visible:ring-ring motion-reduce:transition-none"
          >
            <QiIcon
              name={tab().icon}
              size={18}
              color={props.activeId === tab().id ? token("primary") : token("muted-foreground")}
            />
            <Text
              class={`text-[11px] font-semibold ${
                props.activeId === tab().id ? "text-primary" : "text-muted-foreground"
              }`}
            >
              {tab().label}
            </Text>
          </Tab>
        )}
      </Each>
    </Tabs>
  );
}

/** 通用原生模态窗体（供冷启动全页引导及重要确认弹窗复用） */
export function ModalSurface(props: {
  open: boolean;
  presentation?: "pageSheet" | "formSheet";
  role?: "dialog" | "alertdialog";
  closeOnBackdropPress?: boolean;
  onClose?: () => void;
  children?: JSX.Element;
}) {
  return (
    <Dialog
      open={props.open}
      role={props.role ?? "dialog"}
      presentation={props.presentation ?? "pageSheet"}
      closeOnBackdropPress={props.closeOnBackdropPress ?? true}
      onOpenChange={(e) => {
        if (!e.open && props.onClose) props.onClose();
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
        {props.children}
      </View>
    </Dialog>
  );
}

/** 原生抽屉/模态对话框包装 */
export function SheetDialog(props: {
  open: boolean;
  title: string;
  role?: "dialog" | "alertdialog";
  onClose: () => void;
  children?: JSX.Element;
}) {
  return (
    <Dialog
      open={props.open}
      role={props.role ?? "dialog"}
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
          <Text role="header" class="text-lg font-semibold text-foreground">
            {props.title}
          </Text>
          <ActionButton
            title="完成"
            size="sm"
            variant="secondary"
            shape="pill"
            onPress={props.onClose}
          />
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
            class="min-h-11 flex-1 items-center justify-center rounded-xl px-3 py-2 transition-colors duration-150 focus-visible:ring-2 focus-visible:ring-ring motion-reduce:transition-none"
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

/** 渐进披露可折叠卡片（基于 nsh/base Collapsible reveal，顺滑高度动画与 44pt 语义触发头部） */
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
          class="min-h-11 flex-row items-center justify-between gap-3 rounded-xl px-1 py-1 transition-colors duration-150 hover:bg-surface-sunken/70 active:bg-surface-sunken focus-visible:ring-2 focus-visible:ring-ring motion-reduce:transition-none"
        >
          <View class="flex-1 gap-0.5">
            <Text role="header" class="text-base font-semibold text-foreground">
              {props.title}
            </Text>
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
    <View class="flex-1 gap-1.5 rounded-xl border border-border/55 bg-surface-sunken p-3 sm:p-3.5">
      <View class="flex-row items-center gap-1.5">
        <View class="w-4 items-center justify-center">
          <QiIcon
            name={props.icon}
            size={15}
            color={resolveToneSemanticColor(props.tone)}
          />
        </View>
        <Text class="text-xs font-medium text-muted-foreground">{props.label}</Text>
      </View>
      <Text
        class="text-lg font-bold text-foreground sm:text-xl"
        style={{ fontVariant: ["tabular-nums"] }}
      >
        {props.value}
      </Text>
      <Text class="text-[11px] leading-relaxed text-muted-foreground sm:text-xs">{props.sublabel}</Text>
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
