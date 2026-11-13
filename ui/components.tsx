import { type JSX, Scroll, Text, View } from "nsh";
import { Button, Dialog, Input, Switch } from "nsh/base";

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
  | "normal"
  | "neutral";

/** 卡片容器：统一使用 theme.json 语义圆角与层级边框 */
export function Card(props: { class?: string; children?: JSX.Element }) {
  return (
    <View class={`gap-3 rounded-card border border-border bg-card p-4 ${props.class ?? ""}`}>
      {props.children}
    </View>
  );
}

/** 浅沉降容器：用于内部次级分组、指标单元格或代码示例预览 */
export function SunkenBox(props: { class?: string; children?: JSX.Element }) {
  return (
    <View class={`gap-2 rounded-xl border border-border/60 bg-surface-sunken p-3 ${props.class ?? ""}`}>
      {props.children}
    </View>
  );
}

/** 区块小标题 */
export function SectionHeader(props: { title: string; subtitle?: string; right?: JSX.Element }) {
  return (
    <View class="flex-row items-center justify-between gap-2">
      <View class="flex-1 gap-0.5">
        <Text class="text-base font-semibold text-foreground">{props.title}</Text>
        <If when={Boolean(props.subtitle)}>
          <Text class="text-xs text-muted-foreground">{props.subtitle}</Text>
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

/** 统一品牌按钮：保证最小 44pt 触控高度与原生按压回弹 */
export function ActionButton(props: {
  title: string;
  onPress: () => void;
  variant?: "primary" | "secondary" | "outline" | "danger";
  disabled?: boolean;
  class?: string;
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

  return (
    <Button
      disabled={props.disabled}
      onPress={props.onPress}
      class={`min-h-11 items-center justify-center rounded-xl px-4 py-2.5 transition active:scale-[0.98] motion-reduce:active:scale-100 disabled:opacity-45 ${variantClass()} ${props.class ?? ""}`}
    >
      <Text class={`text-sm font-semibold ${labelClass()}`}>{props.title}</Text>
    </Button>
  );
}

/** 单选胶囊芯片（用于硬件档位、饮食快捷因子、AI 模式选择） */
export function SelectChip(props: {
  label: string;
  selected: boolean;
  disabled?: boolean;
  onPress: () => void;
}) {
  return (
    <Button
      disabled={props.disabled}
      onPress={props.onPress}
      class={`min-h-10 items-center justify-center rounded-full border px-3.5 py-1.5 transition active:scale-[0.98] motion-reduce:active:scale-100 disabled:opacity-40 ${
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

/** 带说明文案的开关行 */
export function SwitchRow(props: {
  title: string;
  subtitle?: string;
  checked: boolean;
  onChange: (checked: boolean) => void;
}) {
  return (
    <View class="flex-row items-center justify-between gap-3 py-1">
      <View class="flex-1 gap-0.5">
        <Text class="text-sm font-medium text-foreground">{props.title}</Text>
        <If when={Boolean(props.subtitle)}>
          <Text class="text-xs text-muted-foreground">{props.subtitle}</Text>
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
            onPress={props.onClose}
            class="min-h-9 items-center justify-center rounded-full bg-secondary px-3.5 py-1.5 active:opacity-75"
          >
            <Text class="text-xs font-medium text-secondary-foreground">完成</Text>
          </Button>
        </View>
        <Scroll class="flex-1" contentStyle={{ padding: 16, gap: 14 }}>
          {props.children}
        </Scroll>
      </View>
    </Dialog>
  );
}
