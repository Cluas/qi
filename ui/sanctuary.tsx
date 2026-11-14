import {
  Canvas,
  cleanup,
  colorScheme,
  effect,
  Image,
  type JSX,
  reduceMotion,
  Scroll,
  signal,
  Text,
  token,
  View,
} from "nsh";
import { Button } from "nsh/base";
import { hapticSelection, hapticSoftImpact, hapticSuccess } from "./haptics";
import catDayImg from "../assets/scenes/cat_day.jpg";
import catNightImg from "../assets/scenes/cat_night.jpg";
import playfulDayImg from "../assets/scenes/playful_day.jpg";
import playfulNightImg from "../assets/scenes/playful_night.jpg";
import solarSpringRainImg from "../assets/scenes/solar_spring_rain.jpg";
import solarWinterHearthImg from "../assets/scenes/solar_winter_hearth.jpg";
import zenDayImg from "../assets/scenes/zen_day.jpg";
import zenNightImg from "../assets/scenes/zen_night.jpg";
import { ActionButton, ModalSurface, SealStamp, VitalBadge } from "./components";
import { formatDurationHours, formatNumber, tr } from "./i18n";
import { QiIcon, type QiIconName } from "./icons";
import {
  AmbientSceneShader,
  createPetBounceMotion,
  createPolaroidTiltMotion,
  SceneCrossfade,
  type SceneMoodShaderKind,
} from "./motion";
import { getVisualThemeMeta, VISUAL_THEMES, type VisualThemeId } from "./themes";

export type WeatherMood = "sunny_crisp" | "misty_rain" | "first_snow" | "clear_moon";
export type SealInkId = "cinnabar" | "gold" | "lapis";
export type SceneDockTabId = "meals" | "letters" | "co_window" | "pet";

export interface CustomPetCompanionView {
  id?: string;
  name: string;
  species?: string;
  persona?: string;
  photo_data_url?: string;
  photoUrl?: string;
  die_cut_frame?: string;
  starlight_memorial?: boolean;
  paw_stamp_title?: string;
}

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

export interface SanctuaryStageProps {
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
  customPet?: CustomPetCompanionView | null;
  onOpenMeals?: () => void;
  onOpenLetters?: () => void;
  onOpenCoWindow?: () => void;
  onOpenPetStudio?: () => void;
  unreadLettersCount?: number;
  activeDockId?: SceneDockTabId | null;
}

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
    case "cat_window":
    default:
      return effectiveNight ? catNightImg : catDayImg;
  }
}

function resolveAmbientShaderConfig(
  themeId: VisualThemeId,
  isNightOrRecover: boolean,
  weatherMood?: WeatherMood,
  isStarlightMemorial?: boolean,
): { mode: SceneMoodShaderKind; warmth: number; wind: number; intensity: number; seed: number } {
  const themeSeed = themeId === "zen_garden" ? 2.3 : themeId === "playful_bento" ? 3.9 : 1.1;

  if (isStarlightMemorial) {
    return { mode: "starlight", warmth: 0.16, wind: 0.08, intensity: 0.96, seed: themeSeed + 4.1 };
  }
  if (weatherMood === "misty_rain") {
    return { mode: "rain", warmth: 0.34, wind: 0.24, intensity: 0.9, seed: themeSeed + 1.4 };
  }
  if (weatherMood === "first_snow" || (isNightOrRecover && themeId === "cat_window")) {
    return {
      mode: "hearth_steam",
      warmth: weatherMood === "first_snow" ? 0.56 : 0.88,
      wind: 0.1,
      intensity: 0.88,
      seed: themeSeed + 2.2,
    };
  }
  if (isNightOrRecover || weatherMood === "clear_moon") {
    return { mode: "starlight", warmth: 0.28, wind: 0.12, intensity: 0.82, seed: themeSeed + 3.3 };
  }
  if (themeId === "zen_garden") {
    return { mode: "hearth_steam", warmth: 0.64, wind: 0.14, intensity: 0.72, seed: themeSeed };
  }
  return { mode: "sun_motes", warmth: 0.88, wind: 0.18, intensity: 0.85, seed: themeSeed };
}

function companionActionLabel(themeId: VisualThemeId, customPet?: CustomPetCompanionView | null): string {
  if (customPet && customPet.name.trim()) {
    return tr("sanctuary.pet_action_custom", { name: customPet.name.trim() });
  }
  switch (themeId) {
    case "zen_garden":
      return tr("sanctuary.pet_action_zen");
    case "playful_bento":
      return tr("sanctuary.pet_action_playful");
    case "cat_window":
    default:
      return tr("sanctuary.pet_action_cat");
  }
}

function companionSpeech(
  themeId: VisualThemeId,
  state: string,
  petCount: number,
  checkedAction: boolean,
  isNight: boolean,
  customPet?: CustomPetCompanionView | null,
): string {
  if (customPet && customPet.name.trim()) {
    const petName = customPet.name.trim();
    if (customPet.starlight_memorial) {
      return petCount > 0
        ? tr("sanctuary.dialogue_memorial_petted", { name: petName })
        : tr("sanctuary.dialogue_memorial", { name: petName });
    }
    return petCount > 0
      ? tr("sanctuary.dialogue_custom_petted", { name: petName })
      : tr("sanctuary.dialogue_custom_pet", { name: petName });
  }

  if (petCount > 0 && petCount % 2 === 1) {
    if (themeId === "zen_garden") {
      return petCount % 3 === 0
        ? tr("sanctuary.dialogue_zen_pet_1")
        : tr("sanctuary.dialogue_zen_pet_2");
    }
    if (themeId === "playful_bento") {
      return petCount % 3 === 0
        ? tr("sanctuary.dialogue_playful_pet_1")
        : tr("sanctuary.dialogue_playful_pet_2");
    }
    return petCount % 3 === 0
      ? tr("sanctuary.dialogue_cat_pet_1")
      : tr("sanctuary.dialogue_cat_pet_2");
  }

  if (state === "recovery_priority") {
    if (isNight) {
      if (themeId === "zen_garden") return tr("sanctuary.dialogue_recover_night_zen");
      if (themeId === "playful_bento") return tr("sanctuary.dialogue_recover_night_playful");
      return tr("sanctuary.dialogue_recover_night_cat");
    }
    if (themeId === "zen_garden") return tr("sanctuary.dialogue_recover_day_zen");
    if (themeId === "playful_bento") return tr("sanctuary.dialogue_recover_day_playful");
    return tr("sanctuary.dialogue_recover_day_cat");
  }

  if (checkedAction) {
    if (themeId === "zen_garden") return tr("sanctuary.dialogue_checked_zen");
    if (themeId === "playful_bento") return tr("sanctuary.dialogue_checked_playful");
    return tr("sanctuary.dialogue_checked_cat");
  }

  if (state === "prime_ready") {
    if (themeId === "zen_garden") return tr("sanctuary.dialogue_prime_zen");
    if (themeId === "playful_bento") return tr("sanctuary.dialogue_prime_playful");
    return tr("sanctuary.dialogue_prime_cat");
  }

  if (themeId === "zen_garden") return tr("sanctuary.dialogue_steady_zen");
  if (themeId === "playful_bento") return tr("sanctuary.dialogue_steady_playful");
  return tr("sanctuary.dialogue_steady_cat");
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

function petSpeciesIcon(species?: string): QiIconName {
  switch ((species ?? "").toLowerCase()) {
    case "cat":
    case "dog":
    case "rabbit":
    case "fox":
      return "custom_pet";
    default:
      return "paw";
  }
}

/** 三境主题切换胶囊条：支持精简顶栏胶囊与详尽设置卡两种排布 */
export function ThemeSwitcherStrip(props: {
  activeId: VisualThemeId;
  onSelect: (id: VisualThemeId) => void;
  detailed?: boolean;
}) {
  const renderItems = () => (
    <Each of={VISUAL_THEMES} key={(item) => item.id}>
      {(item) => {
        const isSelected = () => props.activeId === item().id;
        if (props.detailed) {
          return (
            <Button
              hitSlop={4}
              accessibilityLabel={tr("sanctuary.switch_theme_a11y", { name: item().shortLabel })}
              onPress={() => {
                hapticSelection();
                props.onSelect(item().id);
              }}
              class={`min-h-12 flex-row items-center gap-3 rounded-2xl border px-3.5 py-2.5 transition-colors duration-150 focus-visible:ring-2 focus-visible:ring-ring motion-reduce:transition-none ${
                isSelected()
                  ? "border-primary bg-accent"
                  : "border-border/70 bg-surface-sunken hover:border-primary/45"
              }`}
            >
              <View
                class={`size-9 items-center justify-center rounded-xl border ${
                  isSelected() ? "border-primary bg-primary" : "border-border/70 bg-card"
                }`}
              >
                <QiIcon
                  name={
                    item().id === "zen_garden"
                      ? "flower_branch"
                      : item().id === "playful_bento"
                        ? "sparkles"
                        : "paw"
                  }
                  size={16}
                  color={isSelected() ? token("primary-foreground") : token("foreground")}
                />
              </View>
              <View class="flex-1 gap-0.5">
                <View class="flex-row items-center gap-1.5">
                  <Text class="text-sm font-semibold text-foreground">{item().title}</Text>
                  <If when={Boolean(item().recommended)}>
                    <View class="rounded-full bg-stamp-soft px-2 py-0.5">
                      <Text class="text-[10px] font-semibold text-stamp-ink">
                        {tr("common.recommended")}
                      </Text>
                    </View>
                  </If>
                </View>
                <Text class="text-xs text-muted-foreground">{item().subtitle}</Text>
              </View>
              <If when={isSelected()}>
                <VitalBadge tone="optimal" label={tr("common.active")} />
              </If>
            </Button>
          );
        }

        return (
          <Button
            hitSlop={4}
            accessibilityLabel={tr("sanctuary.switch_theme_a11y", { name: item().shortLabel })}
            onPress={() => {
              hapticSelection();
              props.onSelect(item().id);
            }}
            class={`min-h-8 flex-row items-center gap-1 rounded-full border px-2 py-1 transition-colors duration-150 focus-visible:ring-2 focus-visible:ring-ring motion-reduce:transition-none ${
              isSelected()
                ? "border-primary bg-primary"
                : "border-border/80 bg-card hover:border-primary/45"
            }`}
          >
            <QiIcon
              name={
                item().id === "zen_garden"
                  ? "flower_branch"
                  : item().id === "playful_bento"
                    ? "sparkles"
                    : "paw"
              }
              size={12}
              color={isSelected() ? token("primary-foreground") : token("muted-foreground")}
            />
            <Text
              class={`text-[11px] font-semibold ${
                isSelected() ? "text-primary-foreground" : "text-foreground"
              }`}
            >
              {item().shortLabel}
            </Text>
          </Button>
        );
      }}
    </Each>
  );

  if (props.detailed) {
    return <View class="gap-2">{renderItems()}</View>;
  }

  return <View class="flex-row flex-wrap items-center gap-1.5">{renderItems()}</View>;
}

function resolveVitalToneColor(state: string): string {
  switch (state) {
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
    default:
      return token("vital-recover");
  }
}

function resolveVitalSoftColor(state: string): string {
  switch (state) {
    case "prime_ready":
    case "optimal":
    case "normal":
      return token("vital-prime-soft");
    case "moderate_steady":
    case "watch_low":
    case "watch_high":
    case "shifted":
    case "low":
      return token("vital-steady-soft");
    default:
      return token("vital-recover-soft");
  }
}

/** 灵境画布叠加特效层：ベ塞尔双股茶烟 + 昼夜金桂花瓣/雨滴涟漪纸灯 + 8 帧同心水墨呼噜声波与散落金箔 */
export function SanctuaryVfxCanvas(props: {
  themeId: VisualThemeId;
  isNight: boolean;
  petTrigger: number;
  stampTrigger: number;
}) {
  const burstFrame = signal(0);
  let timerId: ReturnType<typeof setInterval> | null = null;
  let prevPet = props.petTrigger;
  let prevStamp = props.stampTrigger;

  const stopTimer = () => {
    if (timerId !== null) {
      clearInterval(timerId);
      timerId = null;
    }
  };

  cleanup(() => stopTimer());

  effect(() => {
    const nextPet = props.petTrigger;
    const nextStamp = props.stampTrigger;
    if (nextPet === prevPet && nextStamp === prevStamp) return;
    prevPet = nextPet;
    prevStamp = nextStamp;

    stopTimer();
    if (reduceMotion()) {
      burstFrame.set(4);
      return;
    }

    burstFrame.set(1);
    timerId = setInterval(() => {
      const curr = burstFrame();
      if (curr >= 8) {
        stopTimer();
      } else {
        burstFrame.set(curr + 1);
      }
    }, 42);
  });

  return (
    <Canvas
      class="pointer-events-none absolute inset-0"
      style={{ pointerEvents: "none" }}
      draw={(cx, size) => {
        const w = Math.max(240, size.width);
        const h = Math.max(180, size.height);
        const frame = burstFrame();
        const isNight = props.isNight;
        const _scheme = colorScheme();
        const phase = (props.petTrigger * 0.7 + props.stampTrigger * 1.1 + frame * 0.38) % (Math.PI * 2);

        const stageInk = token("stage-ink");
        const goldColor = token("ink-gold");
        const stampColor = token("stamp-ink");
        const dewColor = token("dew-accent");
        const atelierColor = token("atelier-accent");

        cx.clearRect(0, 0, w, h);

        // 1) 窗台青瓷茶盏上方袅袅升腾的贝塞尔双股茶烟曲线
        const cupX = w * 0.72;
        const cupY = h * 0.68;
        const swayA = Math.sin(phase) * 10;
        const swayB = Math.cos(phase * 0.85) * 8;

        cx.save();
        cx.lineCap = "round";
        cx.strokeStyle = stageInk;
        cx.lineWidth = 2.2;
        cx.globalAlpha = isNight ? 0.34 : 0.42;
        cx.beginPath();
        cx.moveTo(cupX, cupY);
        cx.bezierCurveTo(
          cupX - 14 + swayA,
          cupY - h * 0.14,
          cupX + 16 - swayB,
          cupY - h * 0.28,
          cupX - 4 + swayA * 0.7,
          cupY - h * 0.42,
        );
        cx.stroke();

        cx.lineWidth = 1.5;
        cx.globalAlpha = isNight ? 0.24 : 0.30;
        cx.beginPath();
        cx.moveTo(cupX + 7, cupY - 4);
        cx.bezierCurveTo(
          cupX + 18 - swayB,
          cupY - h * 0.12,
          cupX - 12 + swayA,
          cupY - h * 0.26,
          cupX + 8 - swayB * 0.6,
          cupY - h * 0.38,
        );
        cx.stroke();
        cx.restore();

        // 2) 日间漂浮金桂花瓣/微尘 vs 夜间窗沿雨滴涟漪圈与纸灯柔光晕
        if (!isNight) {
          // 斜射晨光微束
          cx.save();
          cx.fillStyle = goldColor;
          cx.globalAlpha = 0.08;
          cx.beginPath();
          cx.moveTo(w * 0.08, 0);
          cx.lineTo(w * 0.42, 0);
          cx.lineTo(w * 0.76, h);
          cx.lineTo(w * 0.28, h);
          cx.closePath();
          cx.fill();
          cx.restore();

          // 8 枚漂浮的金桂四瓣小花与花瓣微尘
          for (let i = 0; i < 8; i++) {
            const px = w * (0.14 + ((i * 0.11 + phase * 0.03) % 0.76));
            const py = h * (0.16 + ((i * 0.17 + phase * 0.02) % 0.52));
            const rot = i * 0.75 + phase * 0.5;

            cx.save();
            cx.translate(px, py);
            cx.rotate(rot);
            cx.fillStyle = i % 2 === 0 ? goldColor : stageInk;
            cx.globalAlpha = i % 2 === 0 ? 0.65 : 0.45;

            if (i % 3 === 0) {
              // 四瓣金桂
              for (let p = 0; p < 4; p++) {
                cx.rotate(Math.PI / 2);
                cx.beginPath();
                cx.ellipse(2.3, 0, 2.1, 1.1, 0, 0, Math.PI * 2);
                cx.fill();
              }
            } else {
              // 飘落单瓣
              cx.beginPath();
              cx.ellipse(0, 0, 2.8, 1.3, 0.3, 0, Math.PI * 2);
              cx.fill();
            }
            cx.restore();
          }
        } else {
          // 夜间案头纸灯三层温暖柔光晕
          const lanternX = w * 0.22;
          const lanternY = h * 0.34;
          const radii = [44, 28, 14];
          const alphas = [0.09, 0.16, 0.28];
          for (let i = 0; i < radii.length; i++) {
            cx.save();
            cx.fillStyle = i === 2 ? stageInk : goldColor;
            cx.globalAlpha = alphas[i];
            cx.beginPath();
            cx.arc(lanternX, lanternY, radii[i] + Math.sin(phase) * 2, 0, Math.PI * 2);
            cx.fill();
            cx.restore();
          }

          // 窗沿 3 处雨滴同心椭圆涟漪圈
          const ripples = [
            { x: w * 0.32, y: h * 0.66, rx: 14, ry: 4.5 },
            { x: w * 0.52, y: h * 0.71, rx: 18, ry: 5.5 },
            { x: w * 0.81, y: h * 0.64, rx: 12, ry: 3.8 },
          ];
          for (let i = 0; i < ripples.length; i++) {
            const rp = ripples[i];
            cx.save();
            cx.strokeStyle = i % 2 === 0 ? stageInk : dewColor;
            cx.lineWidth = 1.1;
            cx.globalAlpha = 0.34;
            cx.beginPath();
            cx.ellipse(rp.x, rp.y, rp.rx, rp.ry, 0, 0, Math.PI * 2);
            cx.stroke();

            cx.globalAlpha = 0.18;
            cx.beginPath();
            cx.ellipse(rp.x, rp.y, rp.rx * 1.55, rp.ry * 1.55, 0, 0, Math.PI * 2);
            cx.stroke();
            cx.restore();
          }
        }

        // 3) 轻抚/落印触发的 8 帧同心水墨呼噜声波涟漪环 + 散落金箔花瓣特效
        const hasTriggered = frame > 0 || props.petTrigger > 0 || props.stampTrigger > 0;
        if (hasTriggered) {
          const progress = frame > 0 ? frame / 8 : 0.55;
          const fade = frame > 0 ? Math.max(0.18, 1 - (frame - 1) * 0.1) : 0.42;
          const centerX = w * 0.54;
          const centerY = h * 0.46;

          for (let ring = 0; ring < 3; ring++) {
            const r = 16 + ring * 16 + progress * (28 + ring * 12);
            cx.save();
            cx.strokeStyle = ring === 0 ? goldColor : ring === 1 ? stageInk : atelierColor;
            cx.lineWidth = 2.0 - ring * 0.45;
            cx.globalAlpha = fade * (0.62 - ring * 0.16);
            if (ring === 1) {
              cx.setLineDash([7, 5]);
            }
            cx.beginPath();
            cx.arc(centerX, centerY, r, 0, Math.PI * 2);
            cx.stroke();
            cx.restore();
          }

          // 12 颗散落金箔花瓣与朱砂微粒
          const petalCount = 12;
          for (let k = 0; k < petalCount; k++) {
            const angle = (k * Math.PI * 2) / petalCount + progress * 0.45;
            const dist = 18 + (k % 3) * 10 + progress * (36 + (k % 4) * 8);
            const px = centerX + Math.cos(angle) * dist;
            const py = centerY + Math.sin(angle) * dist * 0.78;

            cx.save();
            cx.translate(px, py);
            cx.rotate(angle + progress * 1.2);
            cx.fillStyle = k % 3 === 0 ? stampColor : k % 2 === 0 ? goldColor : stageInk;
            cx.globalAlpha = fade * 0.78;
            cx.beginPath();
            cx.ellipse(0, 0, 3.2, 1.5, 0, 0, Math.PI * 2);
            cx.fill();
            cx.restore();
          }
        }
      }}
    />
  );
}

/** 宋瓷天青/琥珀双层圆环 + 内部双正弦波液面 + 外圈 24 枚瓷盘刻度点 + 中心等宽大字号元气分值 */
export function LiquidQiVesselCanvas(props: { score: number; state: string }) {
  return (
    <Canvas
      style={{ width: 84, height: 84 }}
      draw={(cx, size) => {
        const _scheme = colorScheme();
        const w = size.width;
        const h = size.height;
        const midX = w / 2;
        const midY = h / 2;
        const score = Math.max(0, Math.min(100, Number.isFinite(props.score) ? props.score : 0));

        const primaryTone = resolveVitalToneColor(props.state);
        const softTone = resolveVitalSoftColor(props.state);
        const borderTone = token("border");
        const subtleTone = token("ink-subtle");
        const cardTone = token("card");
        const fgTone = token("foreground");
        const goldTone = token("ink-gold");

        const outerTickR = Math.min(w, h) / 2 - 3.5;
        const outerRingR = outerTickR - 4.5;
        const vesselR = outerRingR - 4.5;

        cx.clearRect(0, 0, w, h);

        // 1) 外圈 24 枚宋瓷盘刻度点
        const tickCount = 24;
        for (let i = 0; i < tickCount; i++) {
          const ratio = i / tickCount;
          const angle = -Math.PI / 2 + ratio * Math.PI * 2;
          const tx = midX + Math.cos(angle) * outerTickR;
          const ty = midY + Math.sin(angle) * outerTickR;
          const lit = ratio * 100 <= score && score > 0;

          cx.save();
          cx.fillStyle = lit ? primaryTone : subtleTone;
          cx.globalAlpha = lit ? 0.9 : 0.35;
          cx.beginPath();
          cx.arc(tx, ty, lit ? 1.5 : 1.05, 0, Math.PI * 2);
          cx.fill();
          cx.restore();
        }

        // 2) 宋瓷天青/琥珀双层圆环
        cx.save();
        cx.strokeStyle = borderTone;
        cx.lineWidth = 1.5;
        cx.globalAlpha = 0.85;
        cx.beginPath();
        cx.arc(midX, midY, outerRingR, 0, Math.PI * 2);
        cx.stroke();

        cx.strokeStyle = primaryTone;
        cx.lineWidth = 2.2;
        cx.globalAlpha = 0.88;
        cx.beginPath();
        cx.arc(
          midX,
          midY,
          outerRingR,
          -Math.PI / 2,
          -Math.PI / 2 + (Math.PI * 2 * score) / 100,
        );
        cx.stroke();
        cx.restore();

        // 3) 内部瓷盏与双正弦波液面（随 score 高度起伏 + 细微气泡）
        cx.save();
        cx.beginPath();
        cx.arc(midX, midY, vesselR, 0, Math.PI * 2);
        cx.fillStyle = cardTone;
        cx.fill();
        cx.clip();

        const fillRatio = Math.max(0.12, Math.min(0.88, score / 100));
        const waterBaseY = midY + vesselR - vesselR * 2 * fillRatio;
        const leftX = midX - vesselR - 2;
        const rightX = midX + vesselR + 2;
        const phaseShift = (score * 0.08) % (Math.PI * 2);

        // 后浪（柔和釉色次谐波）
        cx.fillStyle = softTone;
        cx.globalAlpha = 0.95;
        cx.beginPath();
        cx.moveTo(leftX, midY + vesselR + 2);
        for (let x = leftX; x <= rightX; x += 3) {
          const norm = (x - leftX) / (rightX - leftX);
          const waveY =
            waterBaseY +
            Math.sin(norm * Math.PI * 2.2 + phaseShift + 1.1) * 3.2 +
            Math.cos(norm * Math.PI * 4.0 - phaseShift) * 1.2;
          cx.lineTo(x, waveY);
        }
        cx.lineTo(rightX, midY + vesselR + 2);
        cx.closePath();
        cx.fill();

        // 前浪（主谐波液面）
        cx.fillStyle = primaryTone;
        cx.globalAlpha = 0.28;
        cx.beginPath();
        cx.moveTo(leftX, midY + vesselR + 2);
        for (let x = leftX; x <= rightX; x += 3) {
          const norm = (x - leftX) / (rightX - leftX);
          const waveY =
            waterBaseY +
            1.8 +
            Math.sin(norm * Math.PI * 2.0 - phaseShift) * 3.4 +
            Math.sin(norm * Math.PI * 3.6 + phaseShift * 0.7) * 1.4;
          cx.lineTo(x, waveY);
        }
        cx.lineTo(rightX, midY + vesselR + 2);
        cx.closePath();
        cx.fill();

        // 前浪高光描边
        cx.strokeStyle = primaryTone;
        cx.lineWidth = 1.5;
        cx.globalAlpha = 0.75;
        cx.beginPath();
        for (let x = leftX; x <= rightX; x += 3) {
          const norm = (x - leftX) / (rightX - leftX);
          const waveY =
            waterBaseY +
            1.8 +
            Math.sin(norm * Math.PI * 2.0 - phaseShift) * 3.4 +
            Math.sin(norm * Math.PI * 3.6 + phaseShift * 0.7) * 1.4;
          if (x === leftX) cx.moveTo(x, waveY);
          else cx.lineTo(x, waveY);
        }
        cx.stroke();

        // 液面下 5 颗细微气泡
        const bubbles = [
          { dx: -10, dy: 9, r: 1.6 },
          { dx: 7, dy: 12, r: 1.3 },
          { dx: -3, dy: 16, r: 1.9 },
          { dx: 11, dy: 6, r: 1.1 },
          { dx: 2, dy: 8, r: 1.4 },
        ];
        for (let b = 0; b < bubbles.length; b++) {
          const by = Math.min(midY + vesselR - 4, waterBaseY + bubbles[b].dy);
          if (by > waterBaseY + 2) {
            cx.fillStyle = b % 2 === 0 ? cardTone : goldTone;
            cx.globalAlpha = 0.55;
            cx.beginPath();
            cx.arc(midX + bubbles[b].dx, by, bubbles[b].r, 0, Math.PI * 2);
            cx.fill();
          }
        }
        cx.restore();

        // 内圈瓷口描边
        cx.save();
        cx.strokeStyle = borderTone;
        cx.lineWidth = 1.2;
        cx.beginPath();
        cx.arc(midX, midY, vesselR, 0, Math.PI * 2);
        cx.stroke();
        cx.restore();

        // 4) 中心等宽大字号元气分值
        cx.save();
        cx.textAlign = "center";
        cx.textBaseline = "middle";
        cx.fillStyle = fgTone;
        cx.font = '700 22px "PingFang SC", -apple-system, sans-serif';
        cx.fontVariantNumeric = "tabular-nums";
        cx.fillText(`${Math.round(score)}`, midX, midY - 2);

        cx.fillStyle = subtleTone;
        cx.font = '600 9px "Menlo", monospace';
        cx.fillText("QI", midX, midY + 13);
        cx.restore();
      }}
    />
  );
}

/** 柔滑双谐波 HRV 呼吸心电波纹带与右侧发光脉搏圆点 */
export function BreathWaveCanvas(props: { state: string; activeValue?: number | null }) {
  return (
    <Canvas
      class="h-9 w-full"
      style={{ height: 36 }}
      draw={(cx, size) => {
        const _scheme = colorScheme();
        const w = Math.max(160, size.width);
        const h = size.height;
        const midY = h * 0.54;
        const padL = 4;
        const padR = 14;
        const spanW = Math.max(60, w - padL - padR);

        const toneColor = resolveVitalToneColor(props.state);
        const softColor = resolveVitalSoftColor(props.state);
        const borderColor = token("border");
        const cardColor = token("card");

        const seedVal =
          props.activeValue !== undefined && props.activeValue !== null && Number.isFinite(props.activeValue)
            ? props.activeValue
            : 58;
        const phase = (seedVal * 0.13) % (Math.PI * 2);

        cx.clearRect(0, 0, w, h);

        // 1) 柔和中轴虚线轨
        cx.save();
        cx.strokeStyle = borderColor;
        cx.lineWidth = 1;
        cx.setLineDash([3, 3]);
        cx.globalAlpha = 0.65;
        cx.beginPath();
        cx.moveTo(padL, midY);
        cx.lineTo(padL + spanW, midY);
        cx.stroke();
        cx.restore();

        const sampleY = (norm: number, harmonicShift: number, ampScale: number) => {
          const envelope = Math.sin(norm * Math.PI) * 0.75 + 0.25;
          const w1 = Math.sin(norm * Math.PI * 3.5 + phase + harmonicShift) * 6.8;
          const w2 = Math.cos(norm * Math.PI * 7.0 - phase * 0.8 + harmonicShift) * 2.8;
          return midY - (w1 + w2) * envelope * ampScale;
        };

        // 2) 次谐波伴随余韵曲线
        cx.save();
        cx.strokeStyle = toneColor;
        cx.lineWidth = 1.25;
        cx.globalAlpha = 0.32;
        cx.beginPath();
        for (let x = 0; x <= spanW; x += 3) {
          const norm = x / spanW;
          const px = padL + x;
          const py = sampleY(norm, 0.95, 0.72);
          if (x === 0) cx.moveTo(px, py);
          else cx.lineTo(px, py);
        }
        cx.stroke();
        cx.restore();

        // 3) 主谐波柔滑面积带
        cx.save();
        cx.fillStyle = softColor;
        cx.globalAlpha = 0.75;
        cx.beginPath();
        cx.moveTo(padL, h - 2);
        for (let x = 0; x <= spanW; x += 3) {
          const norm = x / spanW;
          cx.lineTo(padL + x, sampleY(norm, 0, 1.0));
        }
        cx.lineTo(padL + spanW, h - 2);
        cx.closePath();
        cx.fill();
        cx.restore();

        // 4) 主谐波高光呼吸心电线
        const endX = padL + spanW;
        const endY = sampleY(1.0, 0, 1.0);

        cx.save();
        cx.strokeStyle = toneColor;
        cx.lineWidth = 2.1;
        cx.lineCap = "round";
        cx.lineJoin = "round";
        cx.beginPath();
        for (let x = 0; x <= spanW; x += 2.5) {
          const norm = x / spanW;
          const px = padL + x;
          const py = sampleY(norm, 0, 1.0);
          if (x === 0) cx.moveTo(px, py);
          else cx.lineTo(px, py);
        }
        cx.stroke();
        cx.restore();

        // 5) 右侧发光脉搏圆点
        cx.save();
        cx.fillStyle = toneColor;
        cx.globalAlpha = 0.22;
        cx.beginPath();
        cx.arc(endX, endY, 7.5, 0, Math.PI * 2);
        cx.fill();

        cx.fillStyle = cardColor;
        cx.globalAlpha = 1;
        cx.beginPath();
        cx.arc(endX, endY, 4.6, 0, Math.PI * 2);
        cx.fill();

        cx.fillStyle = toneColor;
        cx.beginPath();
        cx.arc(endX, endY, 3.2, 0, Math.PI * 2);
        cx.fill();
        cx.restore();
      }}
    />
  );
}

/** 灵境沉浸式一体化窗台剧场（h-64 sm:h-80 画布内整合静默双角标 + 悬浮和纸对话签条 + 底缘木质窗沿器物托盘） */
export function SanctuaryStage(props: SanctuaryStageProps) {
  const petBounce = createPetBounceMotion();
  const meta = () => getVisualThemeMeta(props.themeId);
  const hourNow = new Date().getHours();
  const isEvening = hourNow >= 19 || hourNow < 6;
  const isNightMode = () =>
    colorScheme() === "dark" ||
    props.state === "recovery_priority" ||
    isEvening ||
    props.weatherMood === "clear_moon";

  const isStarlightMemorial = () => Boolean(props.customPet?.starlight_memorial);
  const sceneSource = () => resolveSceneAsset(props.themeId, isNightMode(), props.weatherMood);
  const shaderConfig = () =>
    resolveAmbientShaderConfig(
      props.themeId,
      isNightMode(),
      props.weatherMood,
      isStarlightMemorial(),
    );

  const transitionKey = () =>
    `${props.themeId}:${isNightMode() ? "night" : "day"}:${props.weatherMood ?? "sunny_crisp"}:${isStarlightMemorial() ? "star" : "norm"}`;

  const petPhotoUri = () =>
    (props.customPet?.photo_data_url || props.customPet?.photoUrl || "").trim();

  const petFrameRoundedClass = () =>
    props.customPet?.die_cut_frame === "polaroid" || props.customPet?.die_cut_frame === "celadon"
      ? "rounded-2xl"
      : "rounded-full";

  const handlePet = () => {
    hapticSoftImpact();
    void petBounce.triggerPet();
    props.onPet();
  };

  const stateToneDotColor = () => {
    if (isStarlightMemorial()) return token("ink-gold");
    switch (props.state) {
      case "prime_ready":
        return token("vital-prime");
      case "recovery_priority":
        return token("vital-recover");
      default:
        return token("vital-steady");
    }
  };

  const dockItems = () => [
    {
      id: "meals" as const,
      icon: "utensils" as QiIconName,
      label: tr("sanctuary.dock_meals"),
      hasDot: false,
      onPress: props.onOpenMeals,
    },
    {
      id: "letters" as const,
      icon: "envelope" as QiIconName,
      label: tr("sanctuary.dock_letters"),
      hasDot: (props.unreadLettersCount ?? 0) > 0,
      onPress: props.onOpenLetters,
    },
    {
      id: "co_window" as const,
      icon: "co_sanctuary" as QiIconName,
      label: tr("sanctuary.dock_cowindow"),
      hasDot: Boolean(props.partnerMoodBadge),
      onPress: props.onOpenCoWindow,
    },
    {
      id: "pet" as const,
      icon: (isStarlightMemorial() ? "starlight" : "custom_pet") as QiIconName,
      label: props.customPet?.name?.trim() || tr("sanctuary.dock_pet"),
      hasDot: isStarlightMemorial(),
      onPress: props.onOpenPetStudio,
    },
  ];

  return (
    <View class="overflow-hidden rounded-3xl border border-border/75 bg-card shadow-sm">
      {/* 1. 沉浸式插画主舞台（h-64 sm:h-80，整合 WGSL 光影、Canvas 茶烟涟漪、静默双角标与左下悬浮和纸对话签条） */}
      <View class="relative h-64 overflow-hidden bg-surface-sunken sm:h-80">
        <SceneCrossfade
          value={transitionKey()}
          warmth={shaderConfig().warmth}
          seed={shaderConfig().seed}
          class="absolute inset-0"
        >
          <Image
            src={sceneSource()}
            class="h-full w-full object-cover"
            accessibilityLabel={tr("sanctuary.scene_a11y", { theme: meta().title })}
          />
        </SceneCrossfade>

        {/* GPU WGSL 环境光影层：雨丝 / 炉火茶烟 / 日光微尘与星光守候 */}
        <AmbientSceneShader
          mode={shaderConfig().mode}
          intensity={shaderConfig().intensity}
          warmth={shaderConfig().warmth}
          wind={shaderConfig().wind}
          seed={shaderConfig().seed}
        />

        {/* Canvas 矢量动效层：茶烟曲线 + 金桂花瓣/雨夜涟漪纸灯 + 8 帧呼噜水墨声波 */}
        <SanctuaryVfxCanvas
          themeId={props.themeId}
          isNight={isNightMode()}
          petTrigger={props.petCount}
          stampTrigger={props.checkedAction ? 1 : 0}
        />

        {/* 角标一（左上）：元气读数与此刻状态（非抢眼纯景角标） */}
        <View
          ios={{ backdropMaterial: "ultraThin" }}
          style={{ pointerEvents: "none" }}
          class="absolute left-3 top-3 flex-row items-center gap-2 rounded-full border border-stage-ink/15 bg-stage-scrim/80 px-3 py-1.5"
        >
          <If
            when={isStarlightMemorial()}
            else={<View class="size-2 rounded-full" style={{ backgroundColor: stateToneDotColor() }} />}
          >
            <QiIcon name="starlight" size={12} color={token("ink-gold")} />
          </If>
          <Text
            class="text-xs font-bold text-stage-ink sm:text-sm"
            style={{ fontVariant: ["tabular-nums"] }}
          >
            {formatNumber(Math.round(props.score))}
          </Text>
          <Text class="text-[11px] font-medium text-stage-ink/85">{props.stateBadge}</Text>
        </View>

        {/* 角标二（右上）：晨露点数 + 节气/日签轻触发 */}
        <View
          ios={{ backdropMaterial: "ultraThin" }}
          class="absolute right-3 top-3 overflow-hidden rounded-full border border-stage-ink/15 bg-stage-scrim/80"
        >
          <Button
            hitSlop={6}
            accessibilityLabel={tr("poster.open_a11y")}
            onPress={() => {
              hapticSelection();
              props.onOpenPoster();
            }}
            class="min-h-11 flex-row items-center gap-1.5 rounded-full px-3 py-1.5 transition-opacity duration-150 hover:opacity-90 active:scale-[0.98] focus-visible:ring-2 focus-visible:ring-ring motion-reduce:transition-none"
          >
            <QiIcon name="droplet" size={12} color={token("stage-ink")} />
            <Text
              class="text-xs font-semibold text-stage-ink"
              style={{ fontVariant: ["tabular-nums"] }}
            >
              {formatNumber(props.dewSummary.points)}
            </Text>
            <Text class="text-[11px] text-stage-ink/60">·</Text>
            <QiIcon name="postcard" size={12} color={token("stage-ink")} />
            <Text class="text-[11px] font-medium text-stage-ink">
              {props.solarTerm ?? tr("today.open_poster")}
            </Text>
          </Button>
        </View>

        {/* 窗台轻抚热区 / 自定爱宠模切框（直接依偎于窗台焦位） */}
        <If
          when={Boolean(props.customPet)}
          else={
            <Button
              accessibilityLabel={companionActionLabel(props.themeId, null)}
              onPress={handlePet}
              class="absolute inset-x-6 bottom-20 top-14 rounded-2xl focus-visible:ring-2 focus-visible:ring-ring"
            >
              <View class="flex-1" />
            </Button>
          }
        >
          <View class="absolute bottom-20 right-3">
            <Button
              hitSlop={6}
              accessibilityLabel={companionActionLabel(props.themeId, props.customPet)}
              onPress={handlePet}
              class="focus-visible:ring-2 focus-visible:ring-ring"
            >
              <View
                style={petBounce.motion.style}
                class={`relative size-16 items-center justify-center overflow-hidden border-2 bg-card p-0.5 ${petFrameRoundedClass()} ${
                  isStarlightMemorial() ? "border-ink-gold/80" : "border-card"
                }`}
              >
                <If
                  when={petPhotoUri().length > 0}
                  else={
                    <View class="size-full items-center justify-center rounded-full bg-bento-peach">
                      <QiIcon
                        name={petSpeciesIcon(props.customPet?.species)}
                        size={26}
                        color={isStarlightMemorial() ? token("ink-gold") : token("stamp-ink")}
                      />
                    </View>
                  }
                >
                  <Image
                    src={petPhotoUri()}
                    class={`size-full object-cover ${petFrameRoundedClass()}`}
                    accessibilityLabel={props.customPet?.name ?? ""}
                  />
                </If>
                <If when={isStarlightMemorial()}>
                  <View class="absolute right-0.5 top-0.5 rounded-full bg-stage-scrim/85 p-0.5">
                    <QiIcon name="starlight" size={10} color={token("ink-gold")} />
                  </View>
                </If>
              </View>
            </Button>
          </View>
        </If>

        {/* 画内左下悬浮半透明和纸对话签条（替代画框外臃肿白底大卡片） */}
        <View
          ios={{ backdropMaterial: "ultraThin" }}
          style={{
            backdropFilter: "blur(16px)",
            glassEffect: colorScheme() === "dark" ? "none" : "regular",
            glassTintColor: token("card"),
          }}
          class="absolute inset-x-3 bottom-2.5 flex-row items-center justify-between gap-2.5 rounded-2xl border border-border/75 bg-card/88 px-3 py-2 shadow-sm"
        >
          <View
            style={petBounce.motion.style}
            class="size-8 items-center justify-center rounded-xl border border-border/60 bg-surface-sunken"
          >
            <QiIcon
              name={
                isStarlightMemorial()
                  ? "starlight"
                  : props.themeId === "zen_garden"
                    ? "flower_branch"
                    : props.themeId === "playful_bento"
                      ? "sparkles"
                      : "paw"
              }
              size={16}
              color={isStarlightMemorial() ? token("ink-gold") : token("primary")}
            />
          </View>

          <View class="min-w-0 flex-1 gap-0.5">
            <View class="flex-row flex-wrap items-center gap-x-1 gap-y-0.5">
              <Text class="text-xs font-bold text-primary">
                {props.customPet?.name?.trim() || meta().companionName}
              </Text>
              <If when={Boolean(props.atelierUnlocked)}>
                <View class="rounded-full bg-atelier-soft px-1.5 py-0.5">
                  <Text class="text-[10px] font-semibold text-atelier-accent">
                    {tr("sanctuary.atelier_tag")}
                  </Text>
                </View>
              </If>
              <If when={isStarlightMemorial()}>
                <Text class="text-[10px] font-medium text-ink-gold">
                  {`· ${tr("sanctuary.mode_starlight")}`}
                </Text>
              </If>
              <If when={!isStarlightMemorial() && props.state === "recovery_priority"}>
                <Text class="text-[10px] font-medium text-vital-recover">
                  {`· ${tr("sanctuary.mode_recovery")}`}
                </Text>
              </If>
              <If when={!isStarlightMemorial() && props.state !== "recovery_priority"}>
                <Text class="text-[10px] text-muted-foreground">
                  {`· ${isNightMode() ? tr("sanctuary.mode_night") : tr("sanctuary.mode_day")}`}
                </Text>
                <If when={Boolean(props.lunarPhase)}>
                  <Text class="text-[10px] text-muted-foreground">{`· ${props.lunarPhase}`}</Text>
                </If>
              </If>
            </View>

            <Text class="line-clamp-2 text-xs leading-snug text-foreground">
              {companionSpeech(
                props.themeId,
                props.state,
                props.petCount,
                props.checkedAction,
                isNightMode(),
                props.customPet,
              )}
            </Text>
          </View>

          <ActionButton
            title={companionActionLabel(props.themeId, props.customPet)}
            size="sm"
            variant="secondary"
            shape="pill"
            icon={isStarlightMemorial() ? "starlight" : "paw"}
            onPress={handlePet}
          />
        </View>
      </View>

      {/* 2. 紧贴画框底缘的木质窗沿器物托盘（承载 食光 / 家书 / 同窗 / 伙伴 四枚热区按钮，与窗台融为一体） */}
      <View
        role="toolbar"
        class="flex-row items-center gap-1.5 border-t border-atelier-accent/25 bg-atelier-soft/65 px-2.5 py-1.5"
      >
        <Each of={dockItems()} key={(item) => item.id}>
          {(item) => {
            const isActive = () => props.activeDockId === item().id;
            return (
              <Button
                hitSlop={2}
                accessibilityLabel={item().label}
                onPress={() => item().onPress?.()}
                class={`relative min-h-11 flex-1 flex-row items-center justify-center gap-1.5 rounded-xl px-2 py-1.5 transition-colors duration-150 focus-visible:ring-2 focus-visible:ring-ring active:scale-[0.97] motion-reduce:transition-none ${
                  isActive()
                    ? "border border-atelier-accent/35 bg-card shadow-xs"
                    : "hover:bg-card/65 active:bg-card/85"
                }`}
              >
                <View class="relative items-center justify-center">
                  <QiIcon
                    name={item().icon}
                    size={15}
                    color={isActive() ? token("primary") : token("atelier-accent")}
                  />
                  <If when={item().hasDot}>
                    <View class="absolute -right-1.5 -top-0.5 size-2 rounded-full bg-duo-accent" />
                  </If>
                </View>
                <Text
                  class={`line-clamp-1 text-xs font-semibold ${
                    isActive() ? "text-primary" : "text-foreground"
                  }`}
                >
                  {item().label}
                </Text>
              </Button>
            );
          }}
        </Each>
      </View>
    </View>
  );
}

/** 错落手作「便当小瓷贴」指标卡：柔和釉色底 + 清朗无压排版 + 可插拔四期睡眠色带 */
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
      class={`gap-1.5 rounded-2xl border border-border/60 p-3 ${bentoBgClass(props.toneSurface)} ${props.class ?? ""}`}
    >
      <View class="flex-row items-center justify-between gap-1.5">
        <View class="min-w-0 flex-1 flex-row items-center gap-1.5">
          <View class="size-6 shrink-0 items-center justify-center rounded-lg border border-border/45 bg-card/85">
            <QiIcon
              name={props.icon}
              size={13}
              color={props.accentColor ?? token("primary")}
            />
          </View>
          <Text class="flex-1 text-[11px] font-semibold leading-tight text-foreground/85">
            {props.label}
          </Text>
        </View>
      </View>

      <Text
        class="text-xl font-bold text-foreground sm:text-2xl"
        style={{ fontVariant: ["tabular-nums"] }}
      >
        {props.value}
      </Text>

      <If when={Boolean(props.children)}>
        <View class="pt-0.5">{props.children}</View>
      </If>

      <Text class="text-xs leading-snug text-muted-foreground">{props.sublabel}</Text>
    </View>
  );
}

/** 朱砂印泥晕染同心墨迹、洒金宣纸微粒与古籍方印纹饰背景层 */
export function InkBloomSealCanvas(props: { themeId: VisualThemeId }) {
  return (
    <Canvas
      class="pointer-events-none absolute inset-0"
      style={{ pointerEvents: "none" }}
      draw={(cx, size) => {
        const _scheme = colorScheme();
        const w = Math.max(200, size.width);
        const h = Math.max(96, size.height);
        const stampInk = token("stamp-ink");
        const waxSeal = token("wax-seal");
        const goldInk = token("ink-gold");

        cx.clearRect(0, 0, w, h);

        const sealCenterX = w - 68;
        const sealCenterY = h * 0.52;

        // 1) 多层朱砂印泥晕染同心水墨迹
        const blooms = [
          { rx: 56, ry: 44, alpha: 0.06, rot: -0.15 },
          { rx: 42, ry: 34, alpha: 0.09, rot: 0.12 },
          { rx: 28, ry: 24, alpha: 0.13, rot: -0.08 },
        ];
        for (let i = 0; i < blooms.length; i++) {
          const b = blooms[i];
          cx.save();
          cx.fillStyle = i % 2 === 0 ? stampInk : waxSeal;
          cx.globalAlpha = b.alpha;
          cx.beginPath();
          cx.ellipse(sealCenterX, sealCenterY, b.rx, b.ry, b.rot, 0, Math.PI * 2);
          cx.fill();
          cx.restore();
        }

        // 2) 古籍双栏方印纹饰（微倾 -8 度）
        cx.save();
        cx.translate(sealCenterX, sealCenterY);
        cx.rotate(-0.14);
        cx.strokeStyle = stampInk;
        cx.lineWidth = 1.6;
        cx.globalAlpha = 0.22;
        cx.beginPath();
        cx.roundRect(-24, -24, 48, 48, 6);
        cx.stroke();

        cx.lineWidth = 1.0;
        cx.globalAlpha = 0.18;
        cx.beginPath();
        cx.roundRect(-19, -19, 38, 38, 4);
        cx.moveTo(0, -19);
        cx.lineTo(0, 19);
        cx.moveTo(-19, 0);
        cx.lineTo(19, 0);
        cx.stroke();
        cx.restore();

        // 3) 洒金宣纸微粒
        const flecks = [
          { dx: -52, dy: -22, r: 1.5 },
          { dx: -38, dy: 18, r: 1.2 },
          { dx: -24, dy: -30, r: 1.8 },
          { dx: 18, dy: -26, r: 1.4 },
          { dx: 36, dy: -12, r: 1.7 },
          { dx: 42, dy: 16, r: 1.3 },
          { dx: 22, dy: 28, r: 1.6 },
          { dx: -14, dy: 26, r: 1.1 },
          { dx: -64, dy: 4, r: 1.4 },
          { dx: 8, dy: -34, r: 1.2 },
        ];
        for (let i = 0; i < flecks.length; i++) {
          const f = flecks[i];
          cx.save();
          cx.fillStyle = i % 3 === 0 ? stampInk : goldInk;
          cx.globalAlpha = i % 3 === 0 ? 0.25 : 0.42;
          cx.beginPath();
          cx.arc(sealCenterX + f.dx, sealCenterY + f.dy, f.r, 0, Math.PI * 2);
          cx.fill();
          cx.restore();
        }
      }}
    />
  );
}

/** 触感盖章打卡卡：每日 1 件高杠杆微行动，实体火漆/爪印弹簧落印 + 朱砂墨迹晕染收集晨露 */
export function StampCheckinCard(props: {
  themeId: VisualThemeId;
  microAction: string;
  checked: boolean;
  onToggle: () => void;
  onGoLog: () => void;
  onOpenPoster: () => void;
}) {
  const meta = () => getVisualThemeMeta(props.themeId);
  const stampIcon = (): QiIconName =>
    props.themeId === "zen_garden"
      ? "stamp_seal"
      : props.themeId === "playful_bento"
        ? "sparkles"
        : "paw";

  const handleStampToggle = () => {
    if (props.checked) {
      hapticSelection();
    } else {
      hapticSuccess();
    }
    props.onToggle();
  };

  return (
    <View
      class={`relative gap-3 overflow-hidden rounded-2xl border p-4 ${
        props.checked
          ? "border-stamp-ink/45 bg-stamp-soft"
          : "border-border/80 bg-card"
      }`}
    >
      <If when={props.checked}>
        <InkBloomSealCanvas themeId={props.themeId} />
      </If>

      <View class="flex-row items-center justify-between gap-3">
        <View class="flex-1 gap-1">
          <View class="flex-row items-center gap-2">
            <Text role="header" class="text-base font-bold text-foreground">
              {tr("sanctuary.action_stamp")}
            </Text>
            <View class="flex-row items-center gap-1 rounded-full bg-dew-soft px-2 py-0.5">
              <QiIcon name="droplet" size={11} color={token("dew-accent")} />
              <Text class="text-[11px] font-semibold text-dew-accent">
                {tr("sanctuary.dew_plus_two")}
              </Text>
            </View>
          </View>
          <Text class="text-xs text-muted-foreground">
            {tr("sanctuary.stamp_zero_pressure_hint")}
          </Text>
        </View>

        <SealStamp
          stamped={props.checked}
          label={props.checked ? meta().stampLabel : tr("sanctuary.stamp_pending_short")}
          icon={stampIcon()}
          tone="stamp"
          size="sm"
          onPress={handleStampToggle}
          accessibilityLabel={
            props.checked ? tr("sanctuary.stamped_done_btn") : tr("sanctuary.stamp_action_btn")
          }
        />
      </View>

      <Text class="text-sm font-medium leading-relaxed text-foreground">
        {props.microAction}
      </Text>

      <View class="flex-row flex-wrap items-center gap-2">
        <ActionButton
          title={
            props.checked ? tr("sanctuary.stamped_done_btn") : tr("sanctuary.stamp_action_btn")
          }
          variant={props.checked ? "outline" : "primary"}
          size="sm"
          shape="pill"
          icon={props.checked ? "check" : stampIcon()}
          onPress={handleStampToggle}
        />
        <ActionButton
          title={tr("sanctuary.quick_meal_btn")}
          variant="secondary"
          size="sm"
          shape="pill"
          icon="utensils"
          onPress={props.onGoLog}
        />
        <ActionButton
          title={tr("today.open_poster")}
          variant="ghost"
          size="sm"
          shape="pill"
          icon="postcard"
          onPress={props.onOpenPoster}
        />
      </View>
    </View>
  );
}

/** 模切白边饮食贴纸砖：点击即轻粘进今日饮食日记并自动集 1 滴晨露 */
export function StickerMealTile(props: {
  title: string;
  caption: string;
  icon: QiIconName;
  onPress: () => void;
}) {
  return (
    <Button
      hitSlop={4}
      accessibilityLabel={`${props.title} ${props.caption}`}
      onPress={props.onPress}
      class="min-w-32 flex-1 flex-row items-center gap-2 rounded-2xl border-2 border-card bg-bento-peach/70 px-2.5 py-2.5 shadow-xs transition duration-150 hover:border-primary/45 hover:bg-bento-amber active:scale-[0.98] focus-visible:ring-2 focus-visible:ring-ring motion-reduce:transition-none"
    >
      <View class="size-8 shrink-0 items-center justify-center rounded-xl border border-border/60 bg-card">
        <QiIcon name={props.icon} size={15} color={token("primary")} />
      </View>
      <View class="min-w-0 flex-1 gap-0.5">
        <Text class="text-xs font-bold leading-tight text-foreground">{props.title}</Text>
        <Text class="text-[11px] leading-tight text-muted-foreground">{props.caption}</Text>
      </View>
      <QiIcon name="plus" size={12} color={token("muted-foreground")} />
    </Button>
  );
}

/** 温柔零压力晨露藏品阁：仅随累计打卡缓慢点亮，永不惩罚或清零 */
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
            {tr("sanctuary.keepsake_shelf_title", { theme: meta().shortLabel })}
          </Text>
          <Text class="text-xs text-muted-foreground">
            {props.dewSummary.nextTarget
              ? tr("sanctuary.keepsake_next_hint", {
                  points: formatNumber(props.dewSummary.points),
                  need: formatNumber(
                    Math.max(1, props.dewSummary.nextTarget.threshold - props.dewSummary.points),
                  ),
                  name: props.dewSummary.nextTarget.name,
                })
              : tr("sanctuary.keepsake_all_unlocked")}
          </Text>
        </View>
        <View class="flex-row items-center gap-1 rounded-full bg-dew-soft px-3 py-1">
          <QiIcon name="droplet" size={13} color={token("dew-accent")} />
          <Text
            class="text-xs font-bold text-dew-accent"
            style={{ fontVariant: ["tabular-nums"] }}
          >
            {formatNumber(props.dewSummary.points)}
          </Text>
        </View>
      </View>

      <View class="flex-row flex-wrap gap-2">
        <Each of={props.dewSummary.keepsakes} key={(item) => item.id}>
          {(item) => (
            <View
              class={`min-w-36 flex-1 gap-1.5 rounded-xl border p-3 ${
                item().unlocked
                  ? "border-dew-accent/35 bg-dew-soft/55"
                  : "border-border/60 bg-surface-sunken opacity-70"
              }`}
            >
              <View class="flex-row items-center justify-between">
                <View class="size-7 items-center justify-center rounded-lg border border-border/45 bg-card">
                  <QiIcon
                    name={item().icon}
                    size={14}
                    color={item().unlocked ? token("dew-accent") : token("muted-foreground")}
                  />
                </View>
                <Text
                  class={`text-[11px] font-semibold ${
                    item().unlocked ? "text-dew-accent" : "text-muted-foreground"
                  }`}
                >
                  {item().unlocked
                    ? tr("sanctuary.keepsake_unlocked_badge")
                    : tr("sanctuary.keepsake_unlock_at", {
                        threshold: formatNumber(item().threshold),
                      })}
                </Text>
              </View>
              <Text class="text-xs font-semibold text-foreground">{item().name}</Text>
              <Text class="text-[11px] leading-snug text-muted-foreground">{item().hint}</Text>
            </View>
          )}
        </Each>
      </View>
    </View>
  );
}

/** 今日日签竖版明信片弹窗：支持三色印泥切换、无数字隐私模式与 3D 拍立得倾角动效 */
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
  customPet?: CustomPetCompanionView | null;
}) {
  const localInk = signal<SealInkId>(props.sealInk ?? "cinnabar");
  const localHideNums = signal<boolean>(Boolean(props.hideNumbers));
  const polaroid = createPolaroidTiltMotion();

  const activeInk = () => props.sealInk ?? localInk();
  const hideNums = () => (props.onToggleHideNumbers ? Boolean(props.hideNumbers) : localHideNums());

  const meta = () => getVisualThemeMeta(props.themeId);
  const companionName = () => props.customPet?.name?.trim() || meta().companionName;
  const appName = () => tr("app.name");

  const sceneImg = () =>
    resolveSceneAsset(props.themeId, props.state === "recovery_priority");

  const inkStyle = () => {
    switch (activeInk()) {
      case "gold":
        return {
          textClass: "text-ink-gold",
          badgeBgClass: "bg-bento-amber",
          borderClass: "border-ink-gold/45",
          iconColor: token("ink-gold"),
        };
      case "lapis":
        return {
          textClass: "text-ink-lapis",
          badgeBgClass: "bg-bento-lavender",
          borderClass: "border-ink-lapis/45",
          iconColor: token("ink-lapis"),
        };
      case "cinnabar":
      default:
        return {
          textClass: "text-stamp-ink",
          badgeBgClass: "bg-stamp-soft",
          borderClass: "border-stamp-ink/40",
          iconColor: token("stamp-ink"),
        };
    }
  };

  const INK_CHOICES: { id: SealInkId; labelKey: string; colorToken: string }[] = [
    { id: "cinnabar", labelKey: "poster.ink_cinnabar", colorToken: token("stamp-ink") },
    { id: "gold", labelKey: "poster.ink_gold", colorToken: token("ink-gold") },
    { id: "lapis", labelKey: "poster.ink_lapis", colorToken: token("ink-lapis") },
  ];

  const triggerInkSwitch = (inkId: SealInkId) => {
    hapticSoftImpact();
    localInk.set(inkId);
    props.onChangeSealInk?.(inkId);
    void polaroid.tilt(inkId === "lapis" ? 1 : -1, -1).then(() => {
      void polaroid.reset();
    });
  };

  const captionText = () =>
    hideNums()
      ? tr("poster.caption_hidden", {
          header: meta().posterHeader,
          date: props.targetDate,
          badge: props.stateBadge,
          sleepMood:
            props.sleepHours !== null ? tr("poster.sleep_settled") : tr("poster.sleep_easy"),
          headline: props.headline,
          action: props.microAction,
          appName: appName(),
        })
      : tr("poster.caption_full", {
          header: meta().posterHeader,
          date: props.targetDate,
          score: formatNumber(Math.round(props.score)),
          badge: props.stateBadge,
          sleepText:
            props.sleepHours !== null
              ? formatDurationHours(props.sleepHours, 1)
              : tr("poster.sleep_light_record"),
          dew: formatNumber(props.dewPoints),
          headline: props.headline,
          action: props.microAction,
          appName: appName(),
        });

  return (
    <ModalSurface open={props.open} onClose={props.onClose}>
      <View class="flex-1 bg-background p-4 sm:p-6">
        <View class="mx-auto w-full max-w-md flex-1 gap-4">
          <View class="flex-row items-center justify-between gap-2">
            <View class="flex-row items-center gap-2">
              <Text role="header" class="text-lg font-bold text-foreground">
                {tr("poster.title")}
              </Text>
              <If when={Boolean(props.atelierUnlocked)}>
                <View class="rounded-full bg-atelier-soft px-2 py-0.5">
                  <Text class="text-[10px] font-semibold text-atelier-accent">
                    {tr("poster.atelier_ink_badge")}
                  </Text>
                </View>
              </If>
            </View>
            <ActionButton
              title={tr("poster.dismiss_btn")}
              size="sm"
              variant="secondary"
              shape="pill"
              onPress={props.onClose}
            />
          </View>

          {/* 雅集印泥色与无数字隐私切换条 */}
          <View class="flex-row flex-wrap items-center justify-between gap-2 rounded-2xl border border-border/75 bg-surface-sunken px-3 py-2">
            <View class="flex-row items-center gap-1.5">
              <Text class="text-[11px] font-semibold text-muted-foreground">
                {tr("poster.seal_ink_label")}
              </Text>
              <Each of={INK_CHOICES} key={(c) => c.id}>
                {(choice) => (
                  <Button
                    hitSlop={4}
                    accessibilityLabel={tr(choice().labelKey)}
                    onPress={() => triggerInkSwitch(choice().id)}
                    class={`min-h-11 flex-row items-center gap-1 rounded-full border px-2.5 py-1.5 focus-visible:ring-2 focus-visible:ring-ring ${
                      activeInk() === choice().id
                        ? "border-primary bg-card"
                        : "border-transparent bg-transparent"
                    }`}
                  >
                    <View
                      class="size-2.5 rounded-full"
                      style={{ backgroundColor: choice().colorToken }}
                    />
                    <Text class="text-[11px] font-medium text-foreground">
                      {tr(choice().labelKey)}
                    </Text>
                  </Button>
                )}
              </Each>
            </View>

            <Button
              hitSlop={4}
              accessibilityLabel={
                hideNums() ? tr("poster.hide_numbers_active") : tr("poster.hide_numbers_toggle")
              }
              onPress={() => {
                hapticSoftImpact();
                const next = !hideNums();
                localHideNums.set(next);
                props.onToggleHideNumbers?.(next);
                void polaroid.tilt(next ? 1 : -1, 1).then(() => {
                  void polaroid.reset();
                });
              }}
              class={`min-h-11 flex-row items-center gap-1 rounded-full border px-3 py-1.5 focus-visible:ring-2 focus-visible:ring-ring ${
                hideNums() ? "border-primary bg-accent" : "border-border bg-card"
              }`}
            >
              <Text class="text-[11px] font-semibold text-foreground">
                {hideNums() ? tr("poster.hide_numbers_active") : tr("poster.hide_numbers_toggle")}
              </Text>
            </Button>
          </View>

          {/* 竖版手作明信片主体（带 3D 拍立得弹簧微倾反馈） */}
          <View
            style={polaroid.style()}
            class="overflow-hidden rounded-3xl border border-border bg-card p-3.5"
          >
            <View class="relative h-56 overflow-hidden rounded-2xl">
              <Image
                src={sceneImg()}
                class="h-full w-full object-cover"
                accessibilityLabel={meta().posterHeader}
              />
              <View class="absolute left-3 top-3 rounded-full bg-stage-scrim px-3 py-1">
                <Text class="text-xs font-semibold text-stage-ink">{meta().posterHeader}</Text>
              </View>
              <View class="absolute bottom-3 right-3 rounded-full bg-stage-scrim px-2.5 py-1">
                <Text class="text-[11px] font-medium text-stage-ink">{props.targetDate}</Text>
              </View>
            </View>

            <View class="gap-3 px-1.5 pt-4">
              <View class="flex-row items-baseline justify-between">
                <View class="gap-0.5">
                  <Text class="text-xs font-medium text-muted-foreground">
                    {hideNums() ? tr("poster.metric_kicker_hidden") : tr("poster.metric_kicker_score")}
                  </Text>
                  <If
                    when={hideNums()}
                    else={
                      <Text
                        class="text-4xl font-bold text-foreground"
                        style={{ fontVariant: ["tabular-nums"] }}
                      >
                        {formatNumber(Math.round(props.score))}
                      </Text>
                    }
                  >
                    <Text class={`text-2xl font-bold ${inkStyle().textClass}`}>
                      {props.state === "recovery_priority"
                        ? tr("poster.state_resting")
                        : tr("poster.state_clear")}
                    </Text>
                  </If>
                </View>
                <VitalBadge tone={props.state} label={props.stateBadge} />
              </View>

              <Text class="text-sm leading-relaxed text-foreground">{props.headline}</Text>

              <View class="gap-1 rounded-2xl bg-surface-sunken p-3">
                <Text class="text-xs font-medium text-muted-foreground">
                  {hideNums()
                    ? tr("poster.poetic_footer_hidden")
                    : tr("poster.metrics_summary", {
                        sleep:
                          props.sleepHours !== null
                            ? formatDurationHours(props.sleepHours, 1)
                            : tr("poster.sleep_light_record"),
                        dew: formatNumber(props.dewPoints),
                      })}
                </Text>
                <Text class="text-xs text-foreground">
                  {tr("poster.micro_action_line", { action: props.microAction })}
                </Text>
              </View>

              <View class="flex-row items-center justify-between pt-1">
                <Text class="text-xs font-medium text-muted-foreground">
                  {tr("poster.footer_attribution", {
                    companion: companionName(),
                    appName: appName(),
                  })}
                </Text>
                <View
                  class={`flex-row items-center gap-1 rounded-lg border px-2 py-0.5 ${inkStyle().borderClass} ${inkStyle().badgeBgClass}`}
                  style={{ transform: [{ rotate: "-6deg" }] }}
                >
                  <QiIcon name="stamp_seal" size={12} color={inkStyle().iconColor} />
                  <Text class={`text-[11px] font-bold ${inkStyle().textClass}`}>
                    {props.customPet?.paw_stamp_title?.trim() ||
                      (props.checkedAction
                        ? meta().stampLabel
                        : tr("poster.stamp_fallback", { appName: appName() }))}
                  </Text>
                </View>
              </View>
            </View>
          </View>

          <View class="flex-row gap-2.5">
            <ActionButton
              title={tr("poster.share_caption_btn")}
              icon="postcard"
              class="flex-1"
              onPress={() => props.onCopyCaption(captionText())}
            />
            <ActionButton title={tr("common.back")} variant="secondary" onPress={props.onClose} />
          </View>
        </View>
      </View>
    </ModalSurface>
  );
}
