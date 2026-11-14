import { setTheme, token, type Theme, type Tokens } from "nsh";
import type { QiIconName } from "./icons";
import { tr } from "./i18n";

export type VisualThemeId = "cat_window" | "zen_garden" | "playful_bento";

export interface WorldTheme {
  id: VisualThemeId;
  readonly atelierAccent: string;
  readonly atelierSoft: string;
  readonly waxSeal: string;
  readonly waxSealSoft: string;
  readonly duoAccent: string;
  readonly duoSoft: string;
  readonly dewAccent: string;
  readonly dewSoft: string;
  readonly inkLapis: string;
  readonly inkGold: string;
  readonly starlightAccent: string;
  readonly starlightSoft: string;
}

export function semanticColor<K extends keyof Tokens>(key: K): Tokens[K] {
  return token(key);
}

export interface KeepsakeSpec {
  id: string;
  name: string;
  hint: string;
  threshold: number;
  icon: QiIconName;
}

export interface VisualThemeMeta {
  id: VisualThemeId;
  shortLabel: string;
  title: string;
  subtitle: string;
  companionName: string;
  stampLabel: string;
  posterHeader: string;
  recommended?: boolean;
  keepsakes: KeepsakeSpec[];
}

interface KeepsakeTemplate {
  id: string;
  threshold: number;
  icon: QiIconName;
}

interface VisualThemeTemplate {
  id: VisualThemeId;
  recommended?: boolean;
  keepsakes: readonly KeepsakeTemplate[];
}

const VISUAL_THEME_SPECS: readonly VisualThemeTemplate[] = [
  {
    id: "cat_window",
    recommended: true,
    keepsakes: [
      { id: "tea_cup", threshold: 1, icon: "tea_cup" },
      { id: "osmanthus", threshold: 3, icon: "flower_branch" },
      { id: "cushion", threshold: 5, icon: "paw" },
      { id: "night_lantern", threshold: 8, icon: "lantern" },
    ],
  },
  {
    id: "zen_garden",
    keepsakes: [
      { id: "celadon_cup", threshold: 1, icon: "tea_cup" },
      { id: "pine_twig", threshold: 3, icon: "flower_branch" },
      { id: "cinnabar_seal", threshold: 5, icon: "stamp_seal" },
      { id: "stone_lantern", threshold: 8, icon: "lantern" },
    ],
  },
  {
    id: "playful_bento",
    keepsakes: [
      { id: "matcha_latte", threshold: 1, icon: "tea_cup" },
      { id: "lucky_clover", threshold: 3, icon: "flower_branch" },
      { id: "enamel_paw", threshold: 5, icon: "paw" },
      { id: "moon_pod", threshold: 8, icon: "lantern" },
    ],
  },
];

function buildLocalizedKeepsakes(
  themeId: VisualThemeId,
  templates: readonly KeepsakeTemplate[],
): KeepsakeSpec[] {
  return templates.map((item) => ({
    id: item.id,
    get name() {
      return tr(`themes.${themeId}.keepsakes.${item.id}.name`);
    },
    get hint() {
      return tr(`themes.${themeId}.keepsakes.${item.id}.hint`);
    },
    threshold: item.threshold,
    icon: item.icon,
  }));
}

export const VISUAL_THEMES: VisualThemeMeta[] = VISUAL_THEME_SPECS.map((spec) => ({
  id: spec.id,
  recommended: spec.recommended,
  get shortLabel() {
    return tr(`themes.${spec.id}.shortLabel`);
  },
  get title() {
    return tr(`themes.${spec.id}.title`);
  },
  get subtitle() {
    return tr(`themes.${spec.id}.subtitle`);
  },
  get companionName() {
    return tr(`themes.${spec.id}.companionName`);
  },
  get stampLabel() {
    return tr(`themes.${spec.id}.stampLabel`);
  },
  get posterHeader() {
    return tr(`themes.${spec.id}.posterHeader`);
  },
  get keepsakes() {
    return buildLocalizedKeepsakes(spec.id, spec.keepsakes);
  },
}));

const SHARED_THEME_VARS = {
  radius: "0.875rem",
  "font-sans": "PingFang SC, Helvetica Neue, system-ui, sans-serif",
  "font-mono": "Menlo, ui-monospace, monospace",
};

export const CAT_WINDOW_THEME: Theme = {
  cssVars: {
    theme: SHARED_THEME_VARS,
    light: {
      background: "#F3F4F6",
      foreground: "#111827",
      card: "#FFFFFF",
      "card-foreground": "#111827",
      popover: "#FFFFFF",
      "popover-foreground": "#111827",
      primary: "#1B6B5A",
      "primary-foreground": "#FFFFFF",
      secondary: "#E9ECEF",
      "secondary-foreground": "#1F2937",
      muted: "#E9ECEF",
      "muted-foreground": "#6B7280",
      accent: "#E8F3F0",
      "accent-foreground": "#145346",
      destructive: "#D93829",
      "destructive-foreground": "#FFFFFF",
      border: "#E5E7EB",
      input: "#D1D5DB",
      ring: "#1B6B5A",
      "surface-sunken": "#F7F8FA",
      "surface-elevated": "#FFFFFF",
      "ink-subtle": "#8E8E93",
      "vital-prime": "#1B6B5A",
      "vital-prime-soft": "#E8F3F0",
      "vital-steady": "#B47524",
      "vital-steady-soft": "#FBF4E8",
      "vital-recover": "#D93829",
      "vital-recover-soft": "#FCECEB",
      "sleep-deep": "#1B6B5A",
      "sleep-core": "#4B8F7E",
      "sleep-rem": "#6D5CAE",
      "sleep-awake": "#D4923A",
      "bento-sage": "#F4F8F6",
      "bento-amber": "#FBF8F1",
      "bento-lavender": "#F6F6FA",
      "bento-peach": "#FBF4F3",
      "stamp-ink": "#C84231",
      "stamp-soft": "#FDF1EF",
      "dew-accent": "#0A7EA4",
      "dew-soft": "#EAF6FA",
      "stage-scrim": "#111514B8",
      "stage-ink": "#FFFFFF",
      "atelier-accent": "#A66B2C",
      "atelier-soft": "#F7F1E8",
      "wax-seal": "#B3372B",
      "wax-seal-soft": "#FBECEB",
      "duo-accent": "#C96A50",
      "duo-soft": "#FBF0EC",
      "ink-lapis": "#2E548A",
      "ink-gold": "#BA882F",
    },
    dark: {
      background: "#000000",
      foreground: "#F5F5F7",
      card: "#1C1C1E",
      "card-foreground": "#F5F5F7",
      popover: "#2C2C2E",
      "popover-foreground": "#F5F5F7",
      primary: "#48CFAF",
      "primary-foreground": "#05201A",
      secondary: "#2C2C2E",
      "secondary-foreground": "#E5E5EA",
      muted: "#2C2C2E",
      "muted-foreground": "#98989F",
      accent: "#162C26",
      "accent-foreground": "#8CEBD3",
      destructive: "#FF453A",
      "destructive-foreground": "#FFFFFF",
      border: "#2C2C2E",
      input: "#3A3A3C",
      ring: "#48CFAF",
      "surface-sunken": "#141416",
      "surface-elevated": "#2C2C2E",
      "ink-subtle": "#6E6E73",
      "vital-prime": "#48CFAF",
      "vital-prime-soft": "#162B26",
      "vital-steady": "#FF9F0A",
      "vital-steady-soft": "#2B2214",
      "vital-recover": "#FF453A",
      "vital-recover-soft": "#2E1817",
      "sleep-deep": "#48CFAF",
      "sleep-core": "#358F79",
      "sleep-rem": "#9E8CEB",
      "sleep-awake": "#FF9F0A",
      "bento-sage": "#17231F",
      "bento-amber": "#231F16",
      "bento-lavender": "#1D1C26",
      "bento-peach": "#261918",
      "stamp-ink": "#FF6961",
      "stamp-soft": "#2C1918",
      "dew-accent": "#64D2FF",
      "dew-soft": "#15262E",
      "stage-scrim": "#000000B8",
      "stage-ink": "#F5F5F7",
      "atelier-accent": "#F0A85B",
      "atelier-soft": "#2B2116",
      "wax-seal": "#FF6961",
      "wax-seal-soft": "#2E1818",
      "duo-accent": "#F4977E",
      "duo-soft": "#2D1D1A",
      "ink-lapis": "#6AA1FF",
      "ink-gold": "#FFD60A",
    },
  },
};

export const ZEN_GARDEN_THEME: Theme = {
  cssVars: {
    theme: SHARED_THEME_VARS,
    light: {
      background: "#F1F4F2",
      foreground: "#111A17",
      card: "#FFFFFF",
      "card-foreground": "#111A17",
      popover: "#FFFFFF",
      "popover-foreground": "#111A17",
      primary: "#1F5E55",
      "primary-foreground": "#FFFFFF",
      secondary: "#E6ECE8",
      "secondary-foreground": "#1C2925",
      muted: "#E6ECE8",
      "muted-foreground": "#5F6B66",
      accent: "#E3EFEA",
      "accent-foreground": "#184A43",
      destructive: "#C93B32",
      "destructive-foreground": "#FFFFFF",
      border: "#E1E7E3",
      input: "#CFD9D3",
      ring: "#1F5E55",
      "surface-sunken": "#F5F8F6",
      "surface-elevated": "#FFFFFF",
      "ink-subtle": "#83908B",
      "vital-prime": "#1F5E55",
      "vital-prime-soft": "#E3EFEA",
      "vital-steady": "#A8742B",
      "vital-steady-soft": "#FBF4E7",
      "vital-recover": "#C93B32",
      "vital-recover-soft": "#FCECEB",
      "sleep-deep": "#1F5E55",
      "sleep-core": "#4F897F",
      "sleep-rem": "#6971A6",
      "sleep-awake": "#C79040",
      "bento-sage": "#F2F7F4",
      "bento-amber": "#FAF7F0",
      "bento-lavender": "#F4F5FA",
      "bento-peach": "#FBF3F2",
      "stamp-ink": "#B5382B",
      "stamp-soft": "#FDF0EE",
      "dew-accent": "#23788C",
      "dew-soft": "#E8F4F7",
      "stage-scrim": "#0D1614B8",
      "stage-ink": "#FFFFFF",
      "atelier-accent": "#996832",
      "atelier-soft": "#F6EFE4",
      "wax-seal": "#A23329",
      "wax-seal-soft": "#FBECEB",
      "duo-accent": "#C3725B",
      "duo-soft": "#FBF0EC",
      "ink-lapis": "#264B73",
      "ink-gold": "#B58636",
    },
    dark: {
      background: "#000000",
      foreground: "#EFF4F1",
      card: "#181C1B",
      "card-foreground": "#EFF4F1",
      popover: "#232927",
      "popover-foreground": "#EFF4F1",
      primary: "#4DC4B4",
      "primary-foreground": "#061C19",
      secondary: "#242B29",
      "secondary-foreground": "#DAE3DE",
      muted: "#242B29",
      "muted-foreground": "#8F9B96",
      accent: "#162C28",
      "accent-foreground": "#95E5DA",
      destructive: "#FF5247",
      "destructive-foreground": "#FFFFFF",
      border: "#272F2D",
      input: "#333D3A",
      ring: "#4DC4B4",
      "surface-sunken": "#111514",
      "surface-elevated": "#232A28",
      "ink-subtle": "#6B7671",
      "vital-prime": "#4DC4B4",
      "vital-prime-soft": "#162B27",
      "vital-steady": "#FF9F0A",
      "vital-steady-soft": "#2A2114",
      "vital-recover": "#FF5247",
      "vital-recover-soft": "#2E1817",
      "sleep-deep": "#4DC4B4",
      "sleep-core": "#36857A",
      "sleep-rem": "#969EDB",
      "sleep-awake": "#FF9F0A",
      "bento-sage": "#14211E",
      "bento-amber": "#211E15",
      "bento-lavender": "#1B1E28",
      "bento-peach": "#261918",
      "stamp-ink": "#FF6961",
      "stamp-soft": "#2B1817",
      "dew-accent": "#66CBE0",
      "dew-soft": "#14262B",
      "stage-scrim": "#000000B8",
      "stage-ink": "#EFF4F1",
      "atelier-accent": "#E3A764",
      "atelier-soft": "#2A2116",
      "wax-seal": "#FF6961",
      "wax-seal-soft": "#2D1817",
      "duo-accent": "#EC9A82",
      "duo-soft": "#2D1C19",
      "ink-lapis": "#6BA2FF",
      "ink-gold": "#FFD60A",
    },
  },
};

export const PLAYFUL_BENTO_THEME: Theme = {
  cssVars: {
    theme: SHARED_THEME_VARS,
    light: {
      background: "#F5F4F8",
      foreground: "#1A1826",
      card: "#FFFFFF",
      "card-foreground": "#1A1826",
      popover: "#FFFFFF",
      "popover-foreground": "#1A1826",
      primary: "#227A5E",
      "primary-foreground": "#FFFFFF",
      secondary: "#EAE8F0",
      "secondary-foreground": "#2D293D",
      muted: "#EAE8F0",
      "muted-foreground": "#6B677B",
      accent: "#E5F3EC",
      "accent-foreground": "#195E47",
      destructive: "#D9483B",
      "destructive-foreground": "#FFFFFF",
      border: "#E6E4EC",
      input: "#D4D1DE",
      ring: "#227A5E",
      "surface-sunken": "#F7F6FA",
      "surface-elevated": "#FFFFFF",
      "ink-subtle": "#8E8A9E",
      "vital-prime": "#227A5E",
      "vital-prime-soft": "#E5F3EC",
      "vital-steady": "#C47D28",
      "vital-steady-soft": "#FCF4E6",
      "vital-recover": "#D9483B",
      "vital-recover-soft": "#FDECEB",
      "sleep-deep": "#227A5E",
      "sleep-core": "#4FA286",
      "sleep-rem": "#7C6AC4",
      "sleep-awake": "#D9923B",
      "bento-sage": "#F3F8F5",
      "bento-amber": "#FBF8F1",
      "bento-lavender": "#F6F4FA",
      "bento-peach": "#FBF3F2",
      "stamp-ink": "#D44B3C",
      "stamp-soft": "#FDF0EE",
      "dew-accent": "#1B82A6",
      "dew-soft": "#E8F5FA",
      "stage-scrim": "#161420B8",
      "stage-ink": "#FFFFFF",
      "atelier-accent": "#BA7230",
      "atelier-soft": "#F8F0E5",
      "wax-seal": "#BD3F32",
      "wax-seal-soft": "#FDECEB",
      "duo-accent": "#D87358",
      "duo-soft": "#FCF0EC",
      "ink-lapis": "#34578C",
      "ink-gold": "#C49134",
    },
    dark: {
      background: "#000000",
      foreground: "#F4F2FA",
      card: "#1C1B24",
      "card-foreground": "#F4F2FA",
      popover: "#272533",
      "popover-foreground": "#F4F2FA",
      primary: "#52D1A6",
      "primary-foreground": "#071E16",
      secondary: "#2B2938",
      "secondary-foreground": "#E5E2F0",
      muted: "#2B2938",
      "muted-foreground": "#9893AD",
      accent: "#162E25",
      "accent-foreground": "#9CEBD0",
      destructive: "#FF5247",
      "destructive-foreground": "#FFFFFF",
      border: "#2C2A38",
      input: "#3A374A",
      ring: "#52D1A6",
      "surface-sunken": "#13121A",
      "surface-elevated": "#272533",
      "ink-subtle": "#706B85",
      "vital-prime": "#52D1A6",
      "vital-prime-soft": "#162D25",
      "vital-steady": "#FF9F0A",
      "vital-steady-soft": "#2C2314",
      "vital-recover": "#FF5247",
      "vital-recover-soft": "#301918",
      "sleep-deep": "#52D1A6",
      "sleep-core": "#3B9476",
      "sleep-rem": "#A998EB",
      "sleep-awake": "#FF9F0A",
      "bento-sage": "#16241F",
      "bento-amber": "#231F16",
      "bento-lavender": "#1E1C2A",
      "bento-peach": "#281918",
      "stamp-ink": "#FF6961",
      "stamp-soft": "#2E1918",
      "dew-accent": "#66CBE8",
      "dew-soft": "#152830",
      "stage-scrim": "#000000B8",
      "stage-ink": "#F4F2FA",
      "atelier-accent": "#E8A45E",
      "atelier-soft": "#2D2216",
      "wax-seal": "#FF6961",
      "wax-seal-soft": "#301918",
      "duo-accent": "#F49E87",
      "duo-soft": "#301E1B",
      "ink-lapis": "#749EE0",
      "ink-gold": "#FFD60A",
    },
  },
};

function createWorldTheme(id: VisualThemeId): WorldTheme {
  return {
    id,
    get atelierAccent() {
      return token("atelier-accent");
    },
    get atelierSoft() {
      return token("atelier-soft");
    },
    get waxSeal() {
      return token("wax-seal");
    },
    get waxSealSoft() {
      return token("wax-seal-soft");
    },
    get duoAccent() {
      return token("duo-accent");
    },
    get duoSoft() {
      return token("duo-soft");
    },
    get dewAccent() {
      return token("dew-accent");
    },
    get dewSoft() {
      return token("dew-soft");
    },
    get inkLapis() {
      return token("ink-lapis");
    },
    get inkGold() {
      return token("ink-gold");
    },
    get starlightAccent() {
      return token("ink-lapis");
    },
    get starlightSoft() {
      return token("atelier-soft");
    },
  };
}

export const WORLD_THEMES: Record<VisualThemeId, WorldTheme> = {
  cat_window: createWorldTheme("cat_window"),
  zen_garden: createWorldTheme("zen_garden"),
  playful_bento: createWorldTheme("playful_bento"),
};

export function getVisualThemeMeta(id: VisualThemeId): VisualThemeMeta {
  return VISUAL_THEMES.find((t) => t.id === id) ?? VISUAL_THEMES[0];
}

export function applyVisualTheme(id: VisualThemeId) {
  const themeObj =
    id === "zen_garden"
      ? ZEN_GARDEN_THEME
      : id === "playful_bento"
        ? PLAYFUL_BENTO_THEME
        : CAT_WINDOW_THEME;
  try {
    setTheme(themeObj);
  } catch {
    // 无头宿主不可用时静默回退默认主题
  }
}
