import { effect, type JSX, Keyboard, platform, Scroll, signal, Text, token, View, windowSize } from "nsh";
import { Button, Collapsible, Dialog, Input, Progress, Sheet, Switch, Tab, Tabs } from "nsh/base";
import { isAvailable as isDesktopHost, Window } from "nsh/desktop";
import { hapticSelection, hapticStampImpact, hapticSuccess } from "./haptics";
import { tr } from "./i18n";
import { QiIcon, type QiIconName } from "./icons";
import { createSealStampMotion } from "./motion";

export type DesktopViewportPreset = "ios_portrait" | "desktop_wide";

let iosInitialViewportApplied = false;

/** 判断当前宿主是否为原生 iOS 运行时 */
export function isIosPlatform(): boolean {
  return platform.os === "ios";
}

/** 桌面宿主在调试台一键切换 iPhone 竖屏 / 桌面宽屏预览视口 */
export function setDesktopViewportPreset(preset: DesktopViewportPreset): void {
  try {
    if (!isDesktopHost) return;
    const win = Window.get("main");
    if (!win) return;
    if (preset === "ios_portrait") {
      win.setBounds({ size: { width: 402, height: 874 }, animated: true });
    } else {
      win.setBounds({ size: { width: 1200, height: 820 }, animated: true });
    }
    win.center();
  } catch {
    // 非桌面运行时或无窗口权限时静默跳过
  }
}

/** 桌面端默认以 iPhone 16 Pro 竖屏尺寸居中冷启动（不影响 headless test 与自定义尺寸） */
export function ensureDefaultIosPortraitWindow(): void {
  if (iosInitialViewportApplied) return;
  iosInitialViewportApplied = true;
  try {
    if (isDesktopHost && windowSize().width >= 1150) {
      setDesktopViewportPreset("ios_portrait");
    }
  } catch {
    // 忽略无窗口场景
  }
}

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
        return `min-h-11 ${isPill ? "rounded-full" : "rounded-lg"} px-3 py-2`;
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
      onPress={() => {
        hapticSelection();
        props.onPress();
      }}
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

/** 轻量图标按钮（44pt 触控热区、焦点环与三境画风语义令牌适配） */
export function IconButton(props: {
  icon: QiIconName;
  accessibilityLabel: string;
  onPress: () => void;
  variant?: "ghost" | "secondary" | "outline" | "scrim";
  size?: "sm" | "md";
  shape?: "pill" | "rounded";
  disabled?: boolean;
  iconColor?: string;
  class?: string;
}) {
  const sizeClass = () => {
    const isPill = (props.shape ?? "pill") === "pill";
    if (props.size === "sm") {
      return `min-h-11 min-w-11 ${isPill ? "rounded-full" : "rounded-lg"} p-2`;
    }
    return `min-h-11 min-w-11 ${isPill ? "rounded-full" : "rounded-xl"} p-2.5`;
  };

  const variantClass = () => {
    switch (props.variant ?? "ghost") {
      case "scrim":
        return "border border-stage-ink/20 bg-stage-scrim/85 hover:bg-stage-scrim active:opacity-85";
      case "secondary":
        return "border border-border/80 bg-secondary hover:bg-muted active:opacity-85";
      case "outline":
        return "border border-primary/40 bg-card hover:bg-accent/70 active:bg-accent";
      default:
        return "bg-transparent hover:bg-surface-sunken active:bg-secondary";
    }
  };

  const defaultColor = () => {
    if (props.iconColor) return props.iconColor;
    switch (props.variant ?? "ghost") {
      case "scrim":
        return token("stage-ink");
      case "outline":
        return token("primary");
      default:
        return token("foreground");
    }
  };

  return (
    <Button
      disabled={props.disabled}
      hitSlop={4}
      accessibilityLabel={props.accessibilityLabel}
      onPress={() => {
        hapticSelection();
        props.onPress();
      }}
      class={`items-center justify-center transition duration-150 focus-visible:ring-2 focus-visible:ring-ring active:scale-[0.95] motion-reduce:transition-none motion-reduce:active:scale-100 disabled:opacity-45 ${sizeClass()} ${variantClass()} ${props.class ?? ""}`}
    >
      <QiIcon name={props.icon} size={props.size === "sm" ? 16 : 18} color={defaultColor()} />
    </Button>
  );
}

/** 火漆与爪印落印徽章：支持原生弹簧压印手势与减弱动态降级 */
export function SealStamp(props: {
  stamped: boolean;
  label: string;
  sublabel?: string;
  icon?: QiIconName;
  tone?: "stamp" | "wax" | "gold" | "lapis";
  size?: "sm" | "md" | "lg";
  disabled?: boolean;
  onPress?: () => void;
  accessibilityLabel?: string;
  class?: string;
}) {
  const seal = createSealStampMotion(props.stamped, -8);

  let prevStamped = Boolean(props.stamped);
  effect(() => {
    const nextStamped = Boolean(props.stamped);
    if (nextStamped === prevStamped) return;
    prevStamped = nextStamped;
    if (nextStamped) {
      void seal.releaseStamp(true);
    } else {
      seal.snapRest(false);
    }
  });

  const toneStyle = () => {
    if (!props.stamped) {
      return {
        outerClass: "border-dashed border-border bg-surface-sunken",
        innerClass: "border-border/65 bg-card",
        textClass: "text-muted-foreground",
        iconColor: token("muted-foreground"),
      };
    }
    switch (props.tone ?? "stamp") {
      case "wax":
        return {
          outerClass: "border-wax-seal/45 bg-wax-seal-soft",
          innerClass: "border-wax-seal/55 bg-wax-seal-soft",
          textClass: "text-wax-seal",
          iconColor: token("wax-seal"),
        };
      case "gold":
        return {
          outerClass: "border-ink-gold/50 bg-bento-amber",
          innerClass: "border-ink-gold/55 bg-card/70",
          textClass: "text-ink-gold",
          iconColor: token("ink-gold"),
        };
      case "lapis":
        return {
          outerClass: "border-ink-lapis/45 bg-bento-lavender",
          innerClass: "border-ink-lapis/55 bg-card/70",
          textClass: "text-ink-lapis",
          iconColor: token("ink-lapis"),
        };
      default:
        return {
          outerClass: "border-stamp-ink/45 bg-stamp-soft",
          innerClass: "border-stamp-ink/55 bg-stamp-soft",
          textClass: "text-stamp-ink",
          iconColor: token("stamp-ink"),
        };
    }
  };

  const dims = () => {
    switch (props.size ?? "md") {
      case "sm":
        return { outer: "min-h-11 px-3 py-1.5 gap-1.5", icon: 14, text: "text-[11px]" };
      case "lg":
        return { outer: "min-h-14 px-4 py-2.5 gap-2", icon: 20, text: "text-xs" };
      default:
        return { outer: "min-h-12 px-3.5 py-2 gap-2", icon: 16, text: "text-xs" };
    }
  };

  const renderMedal = () => {
    const t = toneStyle();
    const d = dims();
    return (
      <View
        style={seal.motion.style}
        class={`flex-row items-center justify-center rounded-2xl border-2 ${d.outer} ${t.outerClass} ${props.class ?? ""}`}
      >
        <View class={`items-center justify-center rounded-full border p-1 ${t.innerClass}`}>
          <QiIcon name={props.icon ?? "paw"} size={d.icon} color={t.iconColor} />
        </View>
        <View class="gap-0.5">
          <Text class={`font-bold ${d.text} ${t.textClass}`}>{props.label}</Text>
          <If when={Boolean(props.sublabel)}>
            <Text class="text-[10px] font-medium text-muted-foreground">{props.sublabel}</Text>
          </If>
        </View>
      </View>
    );
  };

  if (props.onPress) {
    return (
      <Button
        disabled={props.disabled}
        hitSlop={4}
        accessibilityLabel={props.accessibilityLabel ?? props.label}
        onPressDown={() => {
          hapticStampImpact();
          void seal.pressDown();
        }}
        onPressUp={() => {
          void seal.releaseStamp(props.stamped);
        }}
        onPress={() => {
          if (!props.stamped) {
            hapticSuccess();
          } else {
            hapticSelection();
          }
          props.onPress?.();
        }}
        class="min-h-11 min-w-11 rounded-2xl focus-visible:ring-2 focus-visible:ring-ring disabled:opacity-45"
      >
        {renderMedal()}
      </Button>
    );
  }

  return renderMedal();
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
      onPress={() => {
        hapticSelection();
        props.onPress();
      }}
      class={`min-h-11 items-center justify-center rounded-full border px-3.5 py-2 transition duration-150 focus-visible:ring-2 focus-visible:ring-ring active:scale-[0.98] motion-reduce:transition-none motion-reduce:active:scale-100 disabled:opacity-40 ${
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
        onChange={({ checked }) => {
          hapticSelection();
          props.onChange(checked);
        }}
        class="flex-row items-center rounded-full focus-visible:ring-2 focus-visible:ring-ring"
        trackClass="w-11 rounded-full bg-muted p-0.5 transition-colors checked:bg-primary motion-reduce:transition-none"
        thumbClass="size-5 rounded-full bg-card shadow-sm transition-transform checked:translate-x-5 motion-reduce:transition-none"
      />
    </View>
  );
}

/** 统一输入框（含 focus-visible 清晰焦点环，单行 Done 提交收起 + 多行聚焦收起胶囊） */
export function FieldInput(props: {
  value: string;
  placeholder?: string;
  multiline?: boolean;
  onChange: (value: string) => void;
}) {
  const focused = signal(false);

  return (
    <View class="gap-1.5">
      <Input
        value={props.value}
        placeholder={props.placeholder}
        multiline={props.multiline}
        returnKeyType={props.multiline ? undefined : "done"}
        submitBehavior={props.multiline ? "newline" : "blurAndSubmit"}
        onFocus={() => focused.set(true)}
        onBlur={() => focused.set(false)}
        onChange={({ value }) => props.onChange(value)}
        class={`rounded-xl border border-input bg-card px-3.5 py-2.5 text-sm text-foreground placeholder:text-muted-foreground focus-visible:border-primary focus-visible:ring-2 focus-visible:ring-ring ${
          props.multiline ? "min-h-24" : "min-h-11"
        }`}
      />
      <If when={Boolean(props.multiline) && focused()}>
        <View class="flex-row justify-end">
          <Button
            hitSlop={4}
            accessibilityLabel={tr("common.hide_keyboard")}
            onPress={() => {
              hapticSelection();
              Keyboard.dismiss();
              focused.set(false);
            }}
            class="min-h-11 items-center justify-center rounded-full bg-secondary px-3.5 py-1.5 transition-opacity duration-150 active:opacity-85"
          >
            <Text class="text-xs font-semibold text-foreground">
              {tr("common.hide_keyboard")}
            </Text>
          </Button>
        </View>
      </If>
    </View>
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
        hitSlop={4}
        accessibilityLabel={tr("common.close")}
        onPress={props.onDismiss}
        class="min-h-9 justify-center rounded-lg px-2.5 py-1 transition-colors hover:bg-primary/10 active:opacity-70 focus-visible:ring-2 focus-visible:ring-ring"
      >
        <Text class="text-xs font-semibold text-accent-foreground">{tr("common.close")}</Text>
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

/** 移动端拇指弧区 5 槽底部导航栏（居中 + 速记主按钮 + ultraThin 毛玻璃 + 原生弹簧滑移） */
export function BottomTabBar<T extends string>(props: {
  items: { id: T; label: string; icon: QiIconName; sfSymbol?: string }[];
  activeId: T;
  onSelect: (id: T) => void;
  onQuickCapture?: () => void;
  quickCaptureLabel?: string;
}) {
  const leftItems = () => props.items.slice(0, 2);
  const rightItems = () => props.items.slice(2);

  return (
    <View
      ios={{ backdropMaterial: "ultraThin" }}
      class="border-t border-border/80 bg-card/90"
      style={{ paddingBottom: "calc(env(safe-area-inset-bottom) + 6px)" }}
    >
      <Tabs
        class="flex-row items-center gap-1 px-2.5 pt-2"
        indicatorClass="inset-y-0 rounded-2xl bg-accent"
      >
        <Each of={leftItems()} key={(t) => `bottom-${t.id}`}>
          {(tab) => (
            <Tab
              selected={props.activeId === tab().id}
              onPress={() => {
                hapticSelection();
                props.onSelect(tab().id);
              }}
              class="min-h-12 flex-1 items-center justify-center gap-0.5 rounded-2xl px-0.5 py-1.5 transition-colors duration-150 focus-visible:ring-2 focus-visible:ring-ring motion-reduce:transition-none"
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

        <If when={Boolean(props.onQuickCapture)}>
          <Button
            hitSlop={6}
            accessibilityLabel={props.quickCaptureLabel ?? tr("nav.quick_capture")}
            onPress={() => {
              hapticSelection();
              props.onQuickCapture?.();
            }}
            class="mx-1 min-h-12 min-w-14 items-center justify-center gap-0.5 rounded-2xl bg-primary px-3 py-1.5 shadow-sm transition duration-150 active:scale-[0.96] focus-visible:ring-2 focus-visible:ring-ring motion-reduce:transition-none"
          >
            <QiIcon name="plus" size={18} color={token("primary-foreground")} />
            <Text class="text-[10px] font-bold text-primary-foreground">
              {tr("nav.quick_short")}
            </Text>
          </Button>
        </If>

        <Each of={rightItems()} key={(t) => `bottom-${t.id}`}>
          {(tab) => (
            <Tab
              selected={props.activeId === tab().id}
              onPress={() => {
                hapticSelection();
                props.onSelect(tab().id);
              }}
              class="min-h-12 flex-1 items-center justify-center gap-0.5 rounded-2xl px-0.5 py-1.5 transition-colors duration-150 focus-visible:ring-2 focus-visible:ring-ring motion-reduce:transition-none"
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
    </View>
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
          paddingTop: "calc(env(safe-area-inset-top) + 10px)",
          paddingBottom: "calc(env(safe-area-inset-bottom) + 16px)",
        }}
      >
        <View class="mb-2 items-center">
          <View class="h-1 w-10 rounded-full bg-border/90" />
        </View>
        {props.children}
      </View>
    </Dialog>
  );
}

/** 原生抽屉/模态对话框包装 */
export function SheetDialog(props: {
  open: boolean;
  title: string;
  subtitle?: string;
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
          paddingTop: "calc(env(safe-area-inset-top) + 10px)",
          paddingBottom: "calc(env(safe-area-inset-bottom) + 16px)",
        }}
      >
        <View class="mb-2 items-center">
          <View class="h-1 w-10 rounded-full bg-border/90" />
        </View>
        <View class="flex-row items-center justify-between gap-3 border-b border-border px-4 pb-3">
          <View class="flex-1 gap-0.5">
            <Text role="header" class="text-lg font-semibold text-foreground">
              {props.title}
            </Text>
            <If when={Boolean(props.subtitle)}>
              <Text class="text-xs text-muted-foreground">{props.subtitle}</Text>
            </If>
          </View>
          <ActionButton
            title={tr("common.done")}
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

/** 原生边缘滑入抽屉（基于 nsh/base Sheet，支持底缘拖拽关闭、空间收纳与减弱动态偏好） */
export function DrawerSheet(props: {
  open: boolean;
  title: string;
  subtitle?: string;
  side?: "bottom" | "right" | "left" | "top";
  closeOnSwipe?: boolean;
  onClose: () => void;
  footer?: JSX.Element;
  children?: JSX.Element;
}) {
  const edge = () => props.side ?? "bottom";
  const panelShapeClass = () => {
    switch (edge()) {
      case "right":
        return "h-full w-full max-w-md rounded-l-3xl border-l border-border bg-background";
      case "left":
        return "h-full w-full max-w-md rounded-r-3xl border-r border-border bg-background";
      case "top":
        return "h-[64%] max-h-[82%] w-full rounded-b-3xl border-b border-border bg-background";
      default:
        return "h-[74%] max-h-[86%] w-full self-center sm:max-w-2xl rounded-t-3xl border-t border-border bg-background";
    }
  };

  return (
    <Sheet
      open={props.open}
      side={edge()}
      closeOnSwipe={props.closeOnSwipe ?? true}
      backdropClass="bg-stage-scrim/55"
      popupClass={panelShapeClass()}
      onOpenChange={(e) => {
        if (!e.open) props.onClose();
      }}
    >
      <View
        class="flex-1"
        style={{
          paddingTop: edge() === "bottom" ? 10 : "calc(env(safe-area-inset-top) + 12px)",
          paddingBottom: "calc(env(safe-area-inset-bottom) + 14px)",
        }}
      >
        <If when={edge() === "bottom"}>
          <View class="mb-2 items-center">
            <View class="h-1 w-10 rounded-full bg-border" />
          </View>
        </If>
        <View class="flex-row items-center justify-between gap-3 border-b border-border/80 px-4 pb-3">
          <View class="flex-1 gap-0.5">
            <Text role="header" class="text-base font-semibold text-foreground">
              {props.title}
            </Text>
            <If when={Boolean(props.subtitle)}>
              <Text class="text-xs leading-relaxed text-muted-foreground">{props.subtitle}</Text>
            </If>
          </View>
          <ActionButton
            title={tr("common.done")}
            size="sm"
            variant="secondary"
            shape="pill"
            onPress={props.onClose}
          />
        </View>
        <Scroll class="flex-1" contentStyle={{ padding: 16, gap: 14 }}>
          {props.children}
        </Scroll>
        <If when={Boolean(props.footer)}>
          <View class="border-t border-border/75 px-4 pt-3">{props.footer}</View>
        </If>
      </View>
    </Sheet>
  );
}

/** 消费级分段切换标签栏（基于 nsh/base Tabs，自带原生弹簧滑动指示器） */
export function SegmentedTabs<T extends string = string>(props: {
  items: { id: T; label: string; icon?: QiIconName }[];
  activeId: T;
  onSelect: (id: T) => void;
  size?: "sm" | "md";
  class?: string;
}) {
  return (
    <Tabs
      class={`flex-row rounded-2xl border border-border/70 bg-surface-sunken p-1 ${props.class ?? ""}`}
      indicatorClass="inset-y-1 rounded-xl bg-card border border-border/50"
    >
      <Each of={props.items} key={(item) => item.id}>
        {(item) => (
          <Tab
            selected={props.activeId === item().id}
            onPress={() => {
              hapticSelection();
              props.onSelect(item().id);
            }}
            class="group min-h-11 flex-1 flex-row items-center justify-center gap-1.5 rounded-xl px-2.5 py-2 focus-visible:ring-2 focus-visible:ring-ring"
          >
            <If when={Boolean(item().icon)}>
              <QiIcon
                name={item().icon!}
                size={14}
                color={props.activeId === item().id ? token("foreground") : token("muted-foreground")}
              />
            </If>
            <Text class="text-xs font-semibold text-muted-foreground group-selected:text-foreground">
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
        return { tone: "optimal", label: tr("status.metric_optimal") };
      case "normal":
        return { tone: "normal", label: tr("status.metric_normal") };
      case "watch_low":
        return { tone: "watch_low", label: tr("status.metric_watch_low") };
      case "alert_low":
        return { tone: "alert_low", label: tr("status.metric_alert_low") };
      case "watch_high":
        return { tone: "watch_high", label: tr("status.metric_watch_high") };
      case "alert_high":
        return { tone: "alert_high", label: tr("status.metric_alert_high") };
      case "shifted":
        return { tone: "shifted", label: tr("status.metric_shifted") };
      case "no_baseline":
        return { tone: "no_baseline", label: tr("status.metric_no_baseline") };
      default:
        return { tone: "neutral", label: tr("status.metric_neutral") };
    }
  };

  return <VitalBadge tone={mapped().tone} label={mapped().label} />;
}

export { MOTION_TOKENS, runDrawerLayoutTransition } from "./motion";
