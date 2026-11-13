import {
  Canvas,
  colorScheme,
  createMotion,
  Image,
  type JSX,
  Scroll,
  Text,
  token,
  View,
} from "nsh";
import { Button } from "nsh/base";
import catDayImg from "../assets/scenes/cat_day.jpg";
import catNightImg from "../assets/scenes/cat_night.jpg";
import playfulDayImg from "../assets/scenes/playful_day.jpg";
import playfulNightImg from "../assets/scenes/playful_night.jpg";
import solarSpringRainImg from "../assets/scenes/solar_spring_rain.jpg";
import solarWinterHearthImg from "../assets/scenes/solar_winter_hearth.jpg";
import zenDayImg from "../assets/scenes/zen_day.jpg";
import zenNightImg from "../assets/scenes/zen_night.jpg";
import { ActionButton, ModalSurface, VitalBadge } from "./components";
import { QiIcon, type QiIconName } from "./icons";
import { getVisualThemeMeta, VISUAL_THEMES, type VisualThemeId } from "./themes";

export type WeatherMood = "sunny_crisp" | "misty_rain" | "first_snow" | "clear_moon";
export type SealInkId = "cinnabar" | "gold" | "lapis";

export interface DewItemView {
  id: string;
  name: string;
  hint: string;
  threshold: number;
  icon: QiIconName;
  unlocked: boolean;
}

export interface DewSummaryView {
  points: number;
  unlockedCount: number;
  keepsakes: DewItemView[];
  nextTarget: DewItemView | null;
}

export type BentoSurfaceTone = "sage" | "amber" | "lavender" | "peach" | "sunken";

function resolveSceneAsset(
  themeId: VisualThemeId,
  isNightOrRecover: boolean,
  weatherMood?: WeatherMood,
) {
  if (weatherMood === "misty_rain") {
    return solarSpringRainImg;
  }
  if (weatherMood === "first_snow") {
    return solarWinterHearthImg;
  }
  const effectiveNight = weatherMood === "clear_moon" ? true : isNightOrRecover;
  switch (themeId) {
    case "zen_garden":
      return effectiveNight ? zenNightImg : zenDayImg;
    case "playful_bento":
      return effectiveNight ? playfulNightImg : playfulDayImg;
    default:
      return effectiveNight ? catNightImg : catDayImg;
  }
}

function companionActionLabel(themeId: VisualThemeId): string {
  switch (themeId) {
    case "zen_garden":
      return "轻听松风";
    case "playful_bento":
      return "捏捏煤球";
    default:
      return "轻抚小满";
  }
}

function companionDialogue(
  themeId: VisualThemeId,
  state: string,
  petCount: number,
  checkedAction: boolean,
  isNight: boolean,
): string {
  if (petCount > 0) {
    const mod = petCount % 2;
    if (themeId === "zen_garden") {
      return mod === 1
        ? "松枝轻颤，香炉青烟悠悠升起一缕：慢慢来，此刻呼吸平顺便是极好。"
        : "石盘水纹微动，清风穿竹而来：心静时节律自然归位，无需急切追赶。";
    }
    if (themeId === "playful_bento") {
      return mod === 1
        ? "煤球耳朵抖了抖，晃出两颗小金星：今天也要软绵绵地照顾好自己哟。"
        : "煤球在抹茶软垫上打了个滚：摸摸头充好电啦，剩下的交给我守着吧。";
    }
    return mod === 1
      ? "小满眯起眼睛发出咕噜咕噜的声音，用毛茸茸的小爪踩了踩软垫：今天也一直在陪着你呢。"
      : "小满蹭了蹭你的指尖，转头晒了晒暖阳：不管今天步数多寡，好好吃饭睡觉就很棒啦。";
  }

  if (state === "recovery_priority") {
    if (isNight) {
      switch (themeId) {
        case "zen_garden":
          return "夜雨添凉，檐下灯暖。元气偏低时不必勉强追赶，早歇、少食辛凉、安坐饮一盏温茶就好。";
        case "playful_bento":
          return "电量低一点也没有关系呀！煤球已把薰衣草云朵枕分你一半，今晚只管安心睡个软绵好觉。";
        default:
          return "昨晚睡得有点浅没关系呀，窗外正飘着细雨，我把针织小毛毯分你一半，今晚不赶路、早点歇息吧。";
      }
    }
    switch (themeId) {
      case "zen_garden":
        return "昼暖风轻，且借半盏温茶顺气安神。元气尚在缓步回温，今日只需随缘缓步、不必追赶指标。";
      case "playful_bento":
        return "日光软绵绵晒进能量舱啦，低电量也完全没关系！煤球分你一半抹茶软垫，今天安心休整充能就好。";
      default:
        return "昨晚睡得有点浅没关系呀，趁着窗台晴光正好，小满把晒暖的小软垫分你一半，今天按自己的步调慢慢缓一缓。";
    }
  }

  if (checkedAction) {
    switch (themeId) {
      case "zen_garden":
        return "今日闲章已钤，松风入怀。且享半日清闲，留意温水补给与晚间收心。";
      case "playful_bento":
        return "星爪贴纸已经啪唧贴好啦！煤球为你点亮今日份小满足，顺其自然继续保持就好。";
      default:
        return "小满在窗台留下一枚暖呼呼的粉嫩爪印！今日这一件小事安顿好了，余下的时光轻轻松松度过吧。";
    }
  }

  if (state === "prime_ready") {
    switch (themeId) {
      case "zen_garden":
        return "晴岚初散，青瓷茗暖。今日身心节律甚佳，宜借此时辰舒展筋骨，收发自如。";
      case "playful_bento":
        return "抹茶能量舱亮晶晶满格待命中！煤球伸了个大大的懒腰，今天挑一件最想做的小事轻松出发吧。";
      default:
        return "窗台阳光晒得毛茸茸暖烘烘的，小满刚伸完长长的懒腰。今天精神正轻快，适合按自己舒服的步调舒展活动。";
    }
  }

  switch (themeId) {
    case "zen_garden":
      return "风平水静，气韵守中。今日宜按寻常步调行止，不必贪快求满，自成从容。";
    case "playful_bento":
      return "今天是平稳舒服的小确幸模式，煤球在抹茶坐垫上陪着你，喝口温水慢慢来。";
    default:
      return "微风晃着桂花枝，小满蹲在茶盏旁安静打盹。今天保持平时舒服的节奏就好，记得午后别太晚喝咖啡呀。";
  }
}

function bentoBgClass(tone: BentoSurfaceTone): string {
  switch (tone) {
    case "sage":
      return "bg-bento-sage";
    case "amber":
      return "bg-bento-amber";
    case "lavender":
      return "bg-bento-lavender";
    case "peach":
      return "bg-bento-peach";
    default:
      return "bg-surface-sunken";
  }
}

/** 三套画风胶囊切换条（支持首页紧凑胶囊条与设定/引导大卡两种形态） */
export function ThemeSwitcherStrip(props: {
  activeId: VisualThemeId;
  onSelect: (id: VisualThemeId) => void;
  detailed?: boolean;
}) {
  return (
    <View class={props.detailed ? "gap-2" : "flex-row flex-wrap items-center gap-1.5"}>
      <Each of={VISUAL_THEMES} key={(item) => item.id}>
        {(item) => {
          const isSelected = () => props.activeId === item().id;
          if (props.detailed) {
            return (
              <Button
                hitSlop={4}
                onPress={() => props.onSelect(item().id)}
                class={`min-h-12 flex-row items-center gap-3 rounded-2xl border px-3.5 py-2.5 transition-colors duration-150 focus-visible:ring-2 focus-visible:ring-ring motion-reduce:transition-none ${
                  isSelected()
                    ? "border-primary bg-accent"
                    : "border-border/70 bg-surface-sunken hover:border-primary/45"
                }`}
              >
                <View
                  class={`size-9 items-center justify-center rounded-xl ${
                    isSelected() ? "bg-primary" : "border border-border/60 bg-card"
                  }`}
                >
                  <QiIcon
                    name={item().id === "zen_garden" ? "flower_branch" : item().id === "playful_bento" ? "sparkles" : "paw"}
                    size={17}
                    color={isSelected() ? token("primary-foreground") : token("primary")}
                  />
                </View>
                <View class="flex-1 gap-0.5">
                  <View class="flex-row items-center gap-2">
                    <Text class="text-sm font-semibold text-foreground">{item().title}</Text>
                    <If when={Boolean(item().recommended)}>
                      <View class="rounded-full bg-stamp-soft px-2 py-0.5">
                        <Text class="text-[10px] font-semibold text-stamp-ink">设计师主推</Text>
                      </View>
                    </If>
                  </View>
                  <Text class="text-xs text-muted-foreground">{item().subtitle}</Text>
                </View>
                <If when={isSelected()}>
                  <QiIcon name="check" size={16} color={token("primary")} />
                </If>
              </Button>
            );
          }

          return (
            <Button
              hitSlop={4}
              accessibilityLabel={`切换画风为${item().shortLabel}`}
              onPress={() => props.onSelect(item().id)}
              class={`min-h-9 flex-row items-center gap-1.5 rounded-full border px-3 py-1.5 transition duration-150 focus-visible:ring-2 focus-visible:ring-ring active:scale-[0.98] motion-reduce:transition-none motion-reduce:active:scale-100 ${
                isSelected()
                  ? "border-primary bg-primary"
                  : "border-border/75 bg-card hover:border-primary/45 hover:bg-surface-sunken"
              }`}
            >
              <QiIcon
                name={item().id === "zen_garden" ? "flower_branch" : item().id === "playful_bento" ? "sparkles" : "paw"}
                size={13}
                color={isSelected() ? token("primary-foreground") : token("primary")}
              />
              <Text
                class={`text-xs font-semibold ${
                  isSelected() ? "text-primary-foreground" : "text-foreground"
                }`}
              >
                {item().shortLabel}
              </Text>
            </Button>
          );
        }}
      </Each>
    </View>
  );
}

/** 无框感沉浸式插画剧场（三世界昼夜/状态自适配场景 + 节气天象 + 晨露收集 + 日签入口 + 轻抚气泡） */
export function SanctuaryStage(props: {
  themeId: VisualThemeId;
  score: number;
  state: string;
  stateBadge: string;
  headline: string;
  dewSummary: DewSummaryView;
  petCount: number;
  checkedAction: boolean;
  onPet: () => void;
  onOpenPoster: () => void;
  solarTerm?: string;
  lunarPhase?: string;
  weatherMood?: WeatherMood;
  equippedKeepsakes?: string[];
  partnerMoodBadge?: string;
  atelierUnlocked?: boolean;
}) {
  const petMotion = createMotion();
  const isNightMode = () => colorScheme() === "dark";
  const meta = () => getVisualThemeMeta(props.themeId);

  const visibleKeepsakes = () => {
    const unlocked = props.dewSummary.keepsakes.filter((k) => k.unlocked);
    const equipped = props.equippedKeepsakes ?? [];
    if (equipped.length === 0) {
      return unlocked.slice(0, 3);
    }
    const matched = equipped
      .map((key) =>
        props.dewSummary.keepsakes.find((k) => k.id === key || k.name === key),
      )
      .filter((k): k is DewItemView => Boolean(k));
    if (matched.length > 0) {
      return matched.slice(0, 3);
    }
    return unlocked.slice(0, 3);
  };

  const handlePetPress = () => {
    void petMotion.play("rubberBand");
    props.onPet();
  };

  return (
    <View class="gap-2.5 rounded-3xl border border-border/80 bg-card p-2">
      {/* 内层无框感圆角场景画布 */}
      <View class="relative h-56 w-full overflow-hidden rounded-2xl bg-surface-sunken sm:h-64">
        <Image
          src={resolveSceneAsset(props.themeId, isNightMode(), props.weatherMood)}
          class="size-full"
          style={{ objectFit: "cover", objectPosition: "center" }}
        />

        {/* 轻量静态氛围微粒画布层（茶烟 / 雨丝 / 初雪 / 星芒，空闲静止零重绘） */}
        <Canvas
          class="absolute inset-0 size-full"
          style={{ pointerEvents: "none" }}
          draw={(cx, size) => {
            cx.clearRect(0, 0, size.width, size.height);
            const ink = token("stage-ink");
            cx.save();
            cx.fillStyle = ink;
            cx.strokeStyle = ink;

            if (props.weatherMood === "first_snow") {
              cx.globalAlpha = 0.45;
              for (const [sx, sy, sr] of [
                [0.12, 0.24, 1.8],
                [0.28, 0.4, 1.3],
                [0.46, 0.19, 2.0],
                [0.64, 0.34, 1.4],
                [0.78, 0.22, 1.9],
                [0.89, 0.44, 1.5],
              ]) {
                cx.beginPath();
                cx.arc(size.width * sx, size.height * sy, sr, 0, Math.PI * 2);
                cx.fill();
              }
            } else if (props.weatherMood === "misty_rain" || isNightMode() || props.weatherMood === "clear_moon") {
              // 夜息/细雨模式：轻柔细雨斜线与微光星点
              cx.globalAlpha = 0.22;
              cx.lineWidth = 1;
              for (let i = 0; i < 8; i++) {
                const rx = size.width * (0.1 + i * 0.11);
                const ry = size.height * (0.18 + (i % 3) * 0.14);
                cx.beginPath();
                cx.moveTo(rx, ry);
                cx.lineTo(rx - 4, ry + 11);
                cx.stroke();
              }
              cx.globalAlpha = 0.4;
              for (const [px, py, r] of [
                [0.18, 0.28, 1.5],
                [0.44, 0.22, 1.2],
                [0.72, 0.3, 1.8],
                [0.86, 0.42, 1.3],
              ]) {
                cx.beginPath();
                cx.arc(size.width * px, size.height * py, r, 0, Math.PI * 2);
                cx.fill();
              }
            } else {
              // 日间晴朗模式：暖阳浮尘光点
              cx.globalAlpha = 0.36;
              for (const [px, py, r] of [
                [0.16, 0.3, 1.6],
                [0.29, 0.22, 1.1],
                [0.68, 0.26, 1.7],
                [0.82, 0.38, 1.3],
              ]) {
                cx.beginPath();
                cx.arc(size.width * px, size.height * py, r, 0, Math.PI * 2);
                cx.fill();
              }
            }
            cx.restore();
          }}
        />

        {/* 顶部悬浮信息胶囊栏 */}
        <View class="absolute inset-x-2 top-2 flex-row flex-wrap items-center justify-between gap-1.5 sm:inset-x-2.5 sm:top-2.5 sm:gap-2">
          <View class="flex-row flex-wrap items-center gap-1.5">
            <View class="flex-row items-center gap-1.5 rounded-full bg-stage-scrim px-2.5 py-1 sm:gap-2 sm:px-3 sm:py-1.5">
              <Text
                class="text-[11px] font-bold text-stage-ink sm:text-xs"
                style={{ fontVariant: ["tabular-nums"] }}
              >
                {`今日元气 ${Math.round(props.score)}`}
              </Text>
              <View class="h-3 w-px bg-stage-ink/30" />
              <Text class="text-[11px] font-semibold text-stage-ink/90">{props.stateBadge}</Text>
            </View>

            <If when={Boolean(props.solarTerm || props.lunarPhase)}>
              <View class="flex-row items-center gap-1 rounded-full bg-stage-scrim px-2.5 py-1 sm:py-1.5">
                <QiIcon name="solar_term" size={12} color={token("ink-gold")} />
                <Text class="text-[11px] font-medium text-stage-ink">
                  {[props.solarTerm, props.lunarPhase].filter(Boolean).join(" · ")}
                </Text>
              </View>
            </If>
          </View>

          <View class="flex-row items-center gap-1.5">
            <View class="flex-row items-center gap-1 rounded-full bg-stage-scrim px-2 py-1 sm:px-2.5 sm:py-1.5">
              <QiIcon name="droplet" size={13} color={token("dew-accent")} />
              <Text
                class="text-[11px] font-semibold text-stage-ink sm:text-xs"
                style={{ fontVariant: ["tabular-nums"] }}
              >
                {`晨露 ${props.dewSummary.points}`}
              </Text>
            </View>

            <Button
              hitSlop={4}
              accessibilityLabel="打开今日知气日签海报"
              onPress={props.onOpenPoster}
              class="min-h-7 flex-row items-center gap-1 rounded-full bg-stage-scrim px-2.5 py-1 transition duration-150 hover:opacity-90 active:scale-[0.97] focus-visible:ring-2 focus-visible:ring-ring motion-reduce:transition-none sm:min-h-8 sm:px-3 sm:py-1.5"
            >
              <QiIcon name="postcard" size={13} color={token("stage-ink")} />
              <Text class="text-[11px] font-semibold text-stage-ink sm:text-xs">今日日签</Text>
            </Button>
          </View>
        </View>

        {/* 底部已解锁/已陈列窗台摆件胶囊 + 友邻串门徽章 + 轻抚陪伴交互按钮 */}
        <View class="absolute inset-x-2.5 bottom-2.5 flex-row items-center justify-between gap-2">
          <View class="flex-1 flex-row flex-wrap items-center gap-1.5">
            <Each of={visibleKeepsakes()} key={(k) => k.id}>
              {(item) => (
                <View class="flex-row items-center gap-1 rounded-full bg-stage-scrim px-2.5 py-1">
                  <QiIcon name={item().icon} size={12} color={token("stage-ink")} />
                  <Text class="text-[11px] font-medium text-stage-ink">{item().name}</Text>
                </View>
              )}
            </Each>
            <If when={Boolean(props.partnerMoodBadge)}>
              <View class="flex-row items-center gap-1 rounded-full bg-stage-scrim px-2.5 py-1">
                <QiIcon name="co_sanctuary" size={12} color={token("duo-accent")} />
                <Text class="text-[11px] font-medium text-stage-ink">{props.partnerMoodBadge}</Text>
              </View>
            </If>
          </View>

          <View style={petMotion.style}>
            <Button
              hitSlop={4}
              onPress={handlePetPress}
              class="min-h-9 flex-row items-center gap-1.5 rounded-full bg-primary px-3.5 py-1.5 transition duration-150 hover:opacity-92 active:scale-[0.96] focus-visible:ring-2 focus-visible:ring-ring motion-reduce:transition-none"
            >
              <QiIcon name="paw" size={14} color={token("primary-foreground")} />
              <Text class="text-xs font-semibold text-primary-foreground">
                {companionActionLabel(props.themeId)}
              </Text>
            </Button>
          </View>
        </View>
      </View>

      {/* 温暖角色对话气泡 + 今日节律一句话概括 */}
      <View class="gap-2 rounded-2xl border border-border/55 bg-surface-sunken px-3.5 py-3">
        <View class="flex-row flex-wrap items-center justify-between gap-2">
          <View class="flex-row flex-wrap items-center gap-1.5">
            <QiIcon name="sparkles" size={14} color={token("primary")} />
            <Text class="text-xs font-bold text-primary">{meta().companionName}</Text>
            <If when={Boolean(props.atelierUnlocked)}>
              <View class="rounded-full bg-atelier-soft px-2 py-0.5">
                <Text class="text-[10px] font-semibold text-atelier-accent">知气雅集</Text>
              </View>
            </If>
          </View>
          <View class="flex-row flex-wrap items-center gap-1.5">
            <If when={props.state === "recovery_priority"}>
              <View class="rounded-full bg-vital-recover-soft px-2 py-0.5">
                <Text class="text-[11px] font-semibold text-vital-recover">安心静养 · 免打卡压力</Text>
              </View>
            </If>
            <Text class="text-[11px] text-muted-foreground">
              {isNightMode() ? "柔和夜息陪护中" : "晴光舒展陪护中"}
            </Text>
          </View>
        </View>
        <Text class="text-sm font-medium leading-relaxed text-foreground">
          {companionDialogue(props.themeId, props.state, props.petCount, props.checkedAction, isNightMode())}
        </Text>
        <Text class="text-xs leading-relaxed text-muted-foreground">{`今日节律简评：${props.headline}`}</Text>
      </View>
    </View>
  );
}

/** 错落手作感「便当小瓷贴」指标卡 */
export function BentoMetricCard(props: {
  icon: QiIconName;
  label: string;
  value: string;
  sublabel: string;
  toneSurface: BentoSurfaceTone;
  accentColor?: string;
  class?: string;
  children?: JSX.Element;
}) {
  return (
    <View
      class={`gap-2 rounded-2xl border border-border/60 p-3.5 ${bentoBgClass(props.toneSurface)} ${props.class ?? ""}`}
    >
      <View class="flex-row items-center justify-between gap-2">
        <View class="flex-row items-center gap-1.5">
          <View class="size-7 items-center justify-center rounded-xl bg-card/85 border border-border/45">
            <QiIcon
              name={props.icon}
              size={14}
              color={props.accentColor ?? token("primary")}
            />
          </View>
          <Text class="text-xs font-semibold text-foreground">{props.label}</Text>
        </View>
      </View>

      <Text
        class="text-xl font-bold text-foreground"
        style={{ fontVariant: ["tabular-nums"] }}
      >
        {props.value}
      </Text>

      <Text class="text-xs leading-relaxed text-muted-foreground">{props.sublabel}</Text>

      <If when={Boolean(props.children)}>
        <View class="pt-1">{props.children}</View>
      </If>
    </View>
  );
}

/** 触感盖章打卡卡片（猫爪肉垫 / 朱砂方印 / 立体星爪贴纸 + 晨露奖励） */
export function StampCheckinCard(props: {
  themeId: VisualThemeId;
  microAction: string;
  checked: boolean;
  onToggle: () => void;
  onGoLog: () => void;
  onOpenPoster: () => void;
}) {
  const stampMotion = createMotion();
  const meta = () => getVisualThemeMeta(props.themeId);
  const stampIcon = (): QiIconName =>
    props.themeId === "zen_garden"
      ? "stamp_seal"
      : props.themeId === "playful_bento"
        ? "sparkles"
        : "paw";

  const handleCheckin = () => {
    void stampMotion.play("popIn");
    props.onToggle();
  };

  return (
    <View class="gap-3.5 rounded-2xl border border-border/80 bg-card p-4">
      <View class="flex-row items-start justify-between gap-3">
        <View class="flex-1 gap-1">
          <View class="flex-row items-center gap-2">
            <Text role="header" class="text-base font-bold text-foreground">
              今日微行动盖章
            </Text>
            <View class="flex-row items-center gap-1 rounded-full bg-dew-soft px-2 py-0.5">
              <QiIcon name="droplet" size={11} color={token("dew-accent")} />
              <Text class="text-[11px] font-semibold text-dew-accent">+2 晨露</Text>
            </View>
          </View>
          <Text class="text-xs text-muted-foreground">只安顿好这 1 件小事便算满格，断签绝不惩罚</Text>
        </View>

        {/* 实体微倾盖章徽记 */}
        <View style={stampMotion.style}>
          <View
            style={{ transform: [{ rotate: "-8deg" }] }}
            class={`items-center justify-center rounded-xl border-2 px-2.5 py-1.5 ${
              props.checked
                ? "border-stamp-ink bg-stamp-soft"
                : "border-border border-dashed bg-surface-sunken"
            }`}
          >
            <View class="flex-row items-center gap-1">
              <QiIcon
                name={stampIcon()}
                size={14}
                color={props.checked ? token("stamp-ink") : token("muted-foreground")}
              />
              <Text
                class={`text-xs font-bold ${
                  props.checked ? "text-stamp-ink" : "text-muted-foreground"
                }`}
              >
                {props.checked ? meta().stampLabel : "待盖章"}
              </Text>
            </View>
          </View>
        </View>
      </View>

      <View class="rounded-xl border border-border/55 bg-bento-amber/60 p-3.5">
        <Text class="text-sm font-medium leading-relaxed text-foreground">{props.microAction}</Text>
      </View>

      <View class="flex-row flex-wrap gap-2">
        <ActionButton
          title={props.checked ? "今日微行动已达成 ✓" : "完成今日微行动"}
          icon={stampIcon()}
          variant={props.checked ? "secondary" : "primary"}
          onPress={handleCheckin}
          class="flex-1"
        />
        <ActionButton
          title="记饮食贴纸"
          icon="record"
          variant="outline"
          onPress={props.onGoLog}
        />
        <ActionButton
          title="收藏今日日签…"
          icon="postcard"
          variant="outline"
          onPress={props.onOpenPoster}
        />
      </View>
    </View>
  );
}

/** 模切白边感「食光贴纸」快捷轻贴瓷片 */
export function StickerMealTile(props: {
  title: string;
  caption: string;
  icon: QiIconName;
  onPress: () => void;
}) {
  return (
    <Button
      hitSlop={4}
      onPress={props.onPress}
      class="min-w-40 flex-1 flex-row items-center gap-2.5 rounded-2xl border-2 border-card bg-bento-peach/70 px-3.5 py-3 transition duration-150 hover:border-primary/45 hover:bg-bento-amber active:scale-[0.98] focus-visible:ring-2 focus-visible:ring-ring motion-reduce:transition-none"
    >
      <View class="size-10 items-center justify-center rounded-xl border border-border/60 bg-card">
        <QiIcon name={props.icon} size={18} color={token("primary")} />
      </View>
      <View class="flex-1 gap-0.5">
        <Text class="text-sm font-bold text-foreground">{props.title}</Text>
        <Text class="text-xs text-muted-foreground">{props.caption}</Text>
      </View>
    </Button>
  );
}

/** 温柔零压力「晨露藏品阁」窗台摆件展示架 */
export function KeepsakeShelfCard(props: {
  themeId: VisualThemeId;
  dewSummary: DewSummaryView;
}) {
  const meta = () => getVisualThemeMeta(props.themeId);
  return (
    <View class="gap-3 rounded-2xl border border-border/80 bg-card p-4">
      <View class="flex-row items-center justify-between gap-2">
        <View class="flex-1 gap-0.5">
          <Text role="header" class="text-base font-bold text-foreground">
            {`${meta().shortLabel} · 晨露藏品阁`}
          </Text>
          <Text class="text-xs text-muted-foreground">
            {props.dewSummary.nextTarget
              ? `已集 ${props.dewSummary.points} 滴晨露 · 再积 ${Math.max(1, props.dewSummary.nextTarget.threshold - props.dewSummary.points)} 滴点亮「${props.dewSummary.nextTarget.name}」`
              : `本套灵境 4 件摆件已全部点亮，可随时切换画风继续收集`}
          </Text>
        </View>
        <View class="flex-row items-center gap-1 rounded-full bg-dew-soft px-3 py-1">
          <QiIcon name="droplet" size={13} color={token("dew-accent")} />
          <Text
            class="text-xs font-bold text-dew-accent"
            style={{ fontVariant: ["tabular-nums"] }}
          >
            {`${props.dewSummary.points} 💧`}
          </Text>
        </View>
      </View>

      <View class="flex-row flex-wrap gap-2">
        <Each of={props.dewSummary.keepsakes} key={(item) => `${props.themeId}-${item.id}`}>
          {(item) => (
            <View
              class={`min-w-36 flex-1 gap-1.5 rounded-xl border p-3 ${
                item().unlocked
                  ? "border-primary/40 bg-bento-sage/75"
                  : "border-border/60 border-dashed bg-surface-sunken opacity-75"
              }`}
            >
              <View class="flex-row items-center justify-between gap-2">
                <View
                  class={`size-8 items-center justify-center rounded-lg ${
                    item().unlocked ? "bg-primary" : "bg-card border border-border/60"
                  }`}
                >
                  <QiIcon
                    name={item().icon}
                    size={16}
                    color={item().unlocked ? token("primary-foreground") : token("muted-foreground")}
                  />
                </View>
                <Text class="text-[11px] font-semibold text-muted-foreground">
                  {item().unlocked ? "已点亮" : `${item().threshold} 💧解锁`}
                </Text>
              </View>
              <Text class="text-xs font-bold text-foreground">{item().name}</Text>
              <Text class="text-[11px] leading-relaxed text-muted-foreground">{item().hint}</Text>
            </View>
          )}
        </Each>
      </View>
    </View>
  );
}

function sealInkClasses(sealInk?: SealInkId): { box: string; text: string } {
  switch (sealInk) {
    case "gold":
      return { box: "border-ink-gold bg-atelier-soft", text: "text-ink-gold" };
    case "lapis":
      return { box: "border-ink-lapis bg-dew-soft", text: "text-ink-lapis" };
    default:
      return { box: "border-stamp-ink bg-stamp-soft", text: "text-stamp-ink" };
  }
}

/** 小红书风「今日知气日签」可分享插画海报弹窗 */
export function DailyPosterModal(props: {
  open: boolean;
  themeId: VisualThemeId;
  targetDate: string;
  score: number;
  state: string;
  stateBadge: string;
  sleepHours: number | null;
  headline: string;
  microAction: string;
  dewPoints: number;
  checkedAction: boolean;
  onCopyCaption: (caption: string) => void;
  onClose: () => void;
  sealInk?: SealInkId;
  onChangeSealInk?: (ink: SealInkId) => void;
  hideNumbers?: boolean;
  onToggleHideNumbers?: (hide: boolean) => void;
  atelierUnlocked?: boolean;
}) {
  const meta = () => getVisualThemeMeta(props.themeId);
  const isNight = () => colorScheme() === "dark";
  const hideNums = () => Boolean(props.hideNumbers);
  const sealStyle = () => sealInkClasses(props.sealInk);
  const sealInks: { id: SealInkId; label: string }[] = [
    { id: "cinnabar", label: "朱砂印" },
    { id: "gold", label: "泥金印" },
    { id: "lapis", label: "石青印" },
  ];

  const shareText = () =>
    hideNums()
      ? `【${meta().posterHeader} · ${props.targetDate}】\n今日气韵：${props.stateBadge} · 昨夜安歇${
          props.sleepHours !== null ? "安稳" : "自在"
        }\n${props.headline}\n今日微行动：${props.microAction}\n—— 知气 Qi 端侧陪伴手账`
      : `【${meta().posterHeader} · ${props.targetDate}】\n今日元气 ${Math.round(props.score)}/100（${props.stateBadge}）· 昨晚好眠 ${
          props.sleepHours !== null ? `${props.sleepHours.toFixed(1)} 小时` : "轻记录"
        } · 已收晨露 ${props.dewPoints} 💧\n${props.headline}\n今日微行动：${props.microAction}\n—— 知气 Qi 端侧陪伴手账`;

  return (
    <ModalSurface open={props.open} presentation="pageSheet" onClose={props.onClose}>
      <Scroll class="flex-1" contentStyle={{ paddingHorizontal: 20, paddingBottom: 28, gap: 16 }}>
        <View class="mx-auto w-full max-w-md gap-4">
          <View class="flex-row items-center justify-between">
            <View class="flex-row items-center gap-2">
              <Text role="header" class="text-lg font-bold text-foreground">
                今日知气日签
              </Text>
              <If when={Boolean(props.atelierUnlocked)}>
                <View class="rounded-full bg-atelier-soft px-2 py-0.5">
                  <Text class="text-[10px] font-semibold text-atelier-accent">雅集印泥</Text>
                </View>
              </If>
            </View>
            <ActionButton title="收好日签" size="sm" variant="secondary" shape="pill" onPress={props.onClose} />
          </View>

          <If when={Boolean(props.onChangeSealInk || props.onToggleHideNumbers)}>
            <View class="flex-row flex-wrap items-center justify-between gap-2 rounded-2xl border border-border/65 bg-surface-sunken px-3 py-2">
              <If when={Boolean(props.onChangeSealInk)}>
                <View class="flex-row items-center gap-1.5">
                  <Each of={sealInks} key={(s) => s.id}>
                    {(item) => {
                      const active = () => (props.sealInk ?? "cinnabar") === item().id;
                      return (
                        <Button
                          hitSlop={4}
                          onPress={() => props.onChangeSealInk?.(item().id)}
                          class={`min-h-8 rounded-full border px-2.5 py-1 transition-colors duration-150 focus-visible:ring-2 focus-visible:ring-ring motion-reduce:transition-none ${
                            active()
                              ? "border-primary bg-primary"
                              : "border-border/75 bg-card hover:border-primary/45"
                          }`}
                        >
                          <Text
                            class={`text-[11px] font-semibold ${
                              active() ? "text-primary-foreground" : "text-foreground"
                            }`}
                          >
                            {item().label}
                          </Text>
                        </Button>
                      );
                    }}
                  </Each>
                </View>
              </If>

              <If when={Boolean(props.onToggleHideNumbers)}>
                <Button
                  hitSlop={4}
                  onPress={() => props.onToggleHideNumbers?.(!hideNums())}
                  class={`min-h-8 rounded-full border px-2.5 py-1 transition-colors duration-150 focus-visible:ring-2 focus-visible:ring-ring motion-reduce:transition-none ${
                    hideNums()
                      ? "border-atelier-accent bg-atelier-soft"
                      : "border-border/75 bg-card hover:border-primary/45"
                  }`}
                >
                  <Text
                    class={`text-[11px] font-semibold ${
                      hideNums() ? "text-atelier-accent" : "text-muted-foreground"
                    }`}
                  >
                    {hideNums() ? "已隐去数字" : "隐藏数字"}
                  </Text>
                </Button>
              </If>
            </View>
          </If>

          {/* 竖版手作明信片主体 */}
          <View class="gap-4 rounded-3xl border-2 border-border bg-card p-3.5">
            <View class="flex-row items-center justify-between px-1">
              <Text class="text-xs font-semibold text-primary">{meta().posterHeader}</Text>
              <Text
                class="text-xs text-muted-foreground"
                style={{ fontVariant: ["tabular-nums"] }}
              >
                {props.targetDate}
              </Text>
            </View>

            <View class="h-52 w-full overflow-hidden rounded-2xl bg-surface-sunken">
              <Image
                src={resolveSceneAsset(props.themeId, isNight())}
                class="size-full"
                style={{ objectFit: "cover", objectPosition: "center" }}
              />
            </View>

            <View class="flex-row items-center justify-between gap-3 rounded-2xl bg-bento-sage/75 px-4 py-3">
              <View class="gap-0.5">
                <Text class="text-xs text-muted-foreground">
                  {hideNums() ? "今日气韵" : "今日元气指数"}
                </Text>
                <If
                  when={!hideNums()}
                  else={
                    <Text class="text-xl font-bold text-foreground">
                      {props.state === "recovery_priority" ? "安然静养" : "气韵清润"}
                    </Text>
                  }
                >
                  <View class="flex-row items-baseline gap-1.5">
                    <Text
                      class="text-3xl font-bold text-foreground"
                      style={{ fontVariant: ["tabular-nums"] }}
                    >
                      {`${Math.round(props.score)}`}
                    </Text>
                    <Text class="text-xs text-muted-foreground">/ 100</Text>
                  </View>
                </If>
              </View>

              <View class="items-end gap-1.5">
                <VitalBadge tone={props.state} label={props.stateBadge} />
                <Text
                  class="text-xs font-medium text-muted-foreground"
                  style={{ fontVariant: ["tabular-nums"] }}
                >
                  {hideNums()
                    ? "只留风光 · 静享此刻"
                    : `好眠 ${props.sleepHours !== null ? `${props.sleepHours.toFixed(1)}h` : "轻记录"} · 晨露 ${props.dewPoints} 💧`}
                </Text>
              </View>
            </View>

            <View class="gap-2 rounded-2xl border border-border/55 bg-surface-sunken p-3.5">
              <Text class="text-sm font-medium leading-relaxed text-foreground">{props.headline}</Text>
              <Text class="text-xs leading-relaxed text-muted-foreground">
                {`今日微行动：${props.microAction}`}
              </Text>
            </View>

            <View class="flex-row items-center justify-between px-1 pt-1">
              <Text class="text-[11px] text-muted-foreground">
                {`${meta().companionName} 陪伴 · 端侧隐私守护`}
              </Text>
              <View
                style={{ transform: [{ rotate: "-6deg" }] }}
                class={`rounded-lg border-2 px-2.5 py-1 ${sealStyle().box}`}
              >
                <Text class={`text-xs font-bold ${sealStyle().text}`}>
                  {props.checkedAction ? meta().stampLabel : "知气印记"}
                </Text>
              </View>
            </View>
          </View>

          <View class="flex-row gap-2">
            <ActionButton
              title="复制日签美文"
              icon="postcard"
              onPress={() => props.onCopyCaption(shareText())}
              class="flex-1"
            />
            <ActionButton title="返回" variant="secondary" onPress={props.onClose} />
          </View>
        </View>
      </Scroll>
    </ModalSurface>
  );
}
