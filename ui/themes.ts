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
      background: "#FAF6EE",
      foreground: "#241E1A",
      card: "#FFFDF9",
      "card-foreground": "#241E1A",
      popover: "#FFFDF9",
      "popover-foreground": "#241E1A",
      primary: "#2F6F5E",
      "primary-foreground": "#FAF7F0",
      secondary: "#F2ECE0",
      "secondary-foreground": "#342C26",
      muted: "#F2ECE0",
      "muted-foreground": "#71685E",
      accent: "#E7F0EB",
      "accent-foreground": "#215447",
      destructive: "#C1523E",
      "destructive-foreground": "#FFFDF9",
      border: "#E8DFCF",
      input: "#DFD4C2",
      ring: "#2F6F5E",
      "surface-sunken": "#F4EFE5",
      "surface-elevated": "#FFFDF9",
      "ink-subtle": "#948A7E",
      "vital-prime": "#2F6F5E",
      "vital-prime-soft": "#E6F1EC",
      "vital-steady": "#B87A2B",
      "vital-steady-soft": "#FBF1DF",
      "vital-recover": "#C1523E",
      "vital-recover-soft": "#FCEAE6",
      "sleep-deep": "#2F6F5E",
      "sleep-core": "#629A86",
      "sleep-rem": "#8675A8",
      "sleep-awake": "#D49A45",
      "bento-sage": "#EAF2ED",
      "bento-amber": "#FBF1DF",
      "bento-lavender": "#F1ECF8",
      "bento-peach": "#FCECE8",
      "stamp-ink": "#C94A38",
      "stamp-soft": "#FCEAE6",
      "dew-accent": "#2E7D9B",
      "dew-soft": "#E5F3F8",
      "stage-scrim": "#1F1915BF",
      "stage-ink": "#FAF7F2",
      "atelier-accent": "#B87333",
      "atelier-soft": "#F6EBE0",
      "wax-seal": "#A63428",
      "wax-seal-soft": "#F7E4E0",
      "duo-accent": "#D47A5E",
      "duo-soft": "#FBEBE5",
      "ink-lapis": "#2B4C7E",
      "ink-gold": "#C5933B",
    },
    dark: {
      background: "#151412",
      foreground: "#EFEBE3",
      card: "#201E1B",
      "card-foreground": "#EFEBE3",
      popover: "#26231F",
      "popover-foreground": "#EFEBE3",
      primary: "#58B196",
      "primary-foreground": "#101916",
      secondary: "#2B2723",
      "secondary-foreground": "#E5E0D6",
      muted: "#2B2723",
      "muted-foreground": "#A59D92",
      accent: "#20332D",
      "accent-foreground": "#98DBC6",
      destructive: "#E5735F",
      "destructive-foreground": "#1E110E",
      border: "#36312C",
      input: "#3E3832",
      ring: "#58B196",
      "surface-sunken": "#1A1815",
      "surface-elevated": "#292622",
      "ink-subtle": "#7F776E",
      "vital-prime": "#58B196",
      "vital-prime-soft": "#1B2E28",
      "vital-steady": "#DEA24B",
      "vital-steady-soft": "#302515",
      "vital-recover": "#E5735F",
      "vital-recover-soft": "#361D19",
      "sleep-deep": "#58B196",
      "sleep-core": "#3E846E",
      "sleep-rem": "#A291CB",
      "sleep-awake": "#DEA24B",
      "bento-sage": "#1D2B25",
      "bento-amber": "#2D2416",
      "bento-lavender": "#272132",
      "bento-peach": "#311F1C",
      "stamp-ink": "#EE7A68",
      "stamp-soft": "#361D19",
      "dew-accent": "#6CC3E0",
      "dew-soft": "#182B33",
      "stage-scrim": "#101211CC",
      "stage-ink": "#F5F2EB",
      "atelier-accent": "#DF9A57",
      "atelier-soft": "#342619",
      "wax-seal": "#E26B5C",
      "wax-seal-soft": "#381C19",
      "duo-accent": "#F09C82",
      "duo-soft": "#38221D",
      "ink-lapis": "#6A94D4",
      "ink-gold": "#E5B95C",
    },
  },
};

export const ZEN_GARDEN_THEME: Theme = {
  cssVars: {
    theme: SHARED_THEME_VARS,
    light: {
      background: "#F3F4EF",
      foreground: "#171D1B",
      card: "#FCFDFB",
      "card-foreground": "#171D1B",
      popover: "#FCFDFB",
      "popover-foreground": "#171D1B",
      primary: "#235E57",
      "primary-foreground": "#F6F8F5",
      secondary: "#E7EAE2",
      "secondary-foreground": "#252E2B",
      muted: "#E7EAE2",
      "muted-foreground": "#606964",
      accent: "#DFEBE7",
      "accent-foreground": "#1B4A44",
      destructive: "#B24339",
      "destructive-foreground": "#FCFDFB",
      border: "#D8DED6",
      input: "#CBD3C9",
      ring: "#235E57",
      "surface-sunken": "#ECEFE8",
      "surface-elevated": "#FFFFFF",
      "ink-subtle": "#828C86",
      "vital-prime": "#235E57",
      "vital-prime-soft": "#DFECE8",
      "vital-steady": "#A9762C",
      "vital-steady-soft": "#F6EEDF",
      "vital-recover": "#B24339",
      "vital-recover-soft": "#F9E6E4",
      "sleep-deep": "#235E57",
      "sleep-core": "#5A8E84",
      "sleep-rem": "#737AA6",
      "sleep-awake": "#C79246",
      "bento-sage": "#E4EFEA",
      "bento-amber": "#F5EFE0",
      "bento-lavender": "#EAEBF3",
      "bento-peach": "#F7EAE7",
      "stamp-ink": "#B5382B",
      "stamp-soft": "#F9E6E3",
      "dew-accent": "#2E7B8C",
      "dew-soft": "#E1F0F4",
      "stage-scrim": "#121A18BF",
      "stage-ink": "#F5F7F3",
      "atelier-accent": "#9E6A32",
      "atelier-soft": "#F3EBDC",
      "wax-seal": "#A23329",
      "wax-seal-soft": "#F6E2DF",
      "duo-accent": "#C3725B",
      "duo-soft": "#F9E9E4",
      "ink-lapis": "#264B73",
      "ink-gold": "#B88A38",
    },
    dark: {
      background: "#101413",
      foreground: "#E6EBE7",
      card: "#171D1B",
      "card-foreground": "#E6EBE7",
      popover: "#1C2321",
      "popover-foreground": "#E6EBE7",
      primary: "#4BA89B",
      "primary-foreground": "#0B1614",
      secondary: "#202825",
      "secondary-foreground": "#D8DFDA",
      muted: "#202825",
      "muted-foreground": "#929D97",
      accent: "#19302C",
      "accent-foreground": "#91D6CB",
      destructive: "#DF6F63",
      "destructive-foreground": "#1A0D0C",
      border: "#2B3532",
      input: "#333F3B",
      ring: "#4BA89B",
      "surface-sunken": "#131817",
      "surface-elevated": "#1E2623",
      "ink-subtle": "#6E7974",
      "vital-prime": "#4BA89B",
      "vital-prime-soft": "#162C28",
      "vital-steady": "#D49B49",
      "vital-steady-soft": "#2B2214",
      "vital-recover": "#DF6F63",
      "vital-recover-soft": "#311917",
      "sleep-deep": "#4BA89B",
      "sleep-core": "#36786E",
      "sleep-rem": "#8F97C9",
      "sleep-awake": "#D49B49",
      "bento-sage": "#172A26",
      "bento-amber": "#282215",
      "bento-lavender": "#1F2230",
      "bento-peach": "#2E1C1A",
      "stamp-ink": "#E87063",
      "stamp-soft": "#311917",
      "dew-accent": "#66BDD0",
      "dew-soft": "#15292E",
      "stage-scrim": "#0B100FCC",
      "stage-ink": "#EFF4F0",
      "atelier-accent": "#D49B5E",
      "atelier-soft": "#2F2519",
      "wax-seal": "#DC695C",
      "wax-seal-soft": "#351B19",
      "duo-accent": "#E6957D",
      "duo-soft": "#34211D",
      "ink-lapis": "#638FC7",
      "ink-gold": "#DDB25A",
    },
  },
};

export const PLAYFUL_BENTO_THEME: Theme = {
  cssVars: {
    theme: SHARED_THEME_VARS,
    light: {
      background: "#FFF9F0",
      foreground: "#231E2E",
      card: "#FFFFFF",
      "card-foreground": "#231E2E",
      popover: "#FFFFFF",
      "popover-foreground": "#231E2E",
      primary: "#2C8A6B",
      "primary-foreground": "#FFFDF9",
      secondary: "#F5EEE4",
      "secondary-foreground": "#352F42",
      muted: "#F5EEE4",
      "muted-foreground": "#6F687D",
      accent: "#E3F3EB",
      "accent-foreground": "#1F6B51",
      destructive: "#D95D52",
      "destructive-foreground": "#FFFFFF",
      border: "#EAE0D2",
      input: "#E0D3C3",
      ring: "#2C8A6B",
      "surface-sunken": "#F7F1E7",
      "surface-elevated": "#FFFFFF",
      "ink-subtle": "#958EA3",
      "vital-prime": "#2C8A6B",
      "vital-prime-soft": "#E2F4EC",
      "vital-steady": "#CF852E",
      "vital-steady-soft": "#FDF0DC",
      "vital-recover": "#D95D52",
      "vital-recover-soft": "#FDEAE8",
      "sleep-deep": "#2C8A6B",
      "sleep-core": "#5CAE93",
      "sleep-rem": "#8E7CC3",
      "sleep-awake": "#E09B45",
      "bento-sage": "#E6F4EC",
      "bento-amber": "#FDF1DC",
      "bento-lavender": "#EFEAF9",
      "bento-peach": "#FDECE8",
      "stamp-ink": "#D95B4A",
      "stamp-soft": "#FDEBE8",
      "dew-accent": "#2E89AC",
      "dew-soft": "#E2F4FA",
      "stage-scrim": "#1F1A2EBF",
      "stage-ink": "#FFFBF4",
      "atelier-accent": "#C67B34",
      "atelier-soft": "#FAF0E2",
      "wax-seal": "#BD3F32",
      "wax-seal-soft": "#FBE7E4",
      "duo-accent": "#E07D63",
      "duo-soft": "#FCECE7",
      "ink-lapis": "#34578C",
      "ink-gold": "#CF9B3C",
    },
    dark: {
      background: "#171622",
      foreground: "#F3EFF8",
      card: "#211F30",
      "card-foreground": "#F3EFF8",
      popover: "#272438",
      "popover-foreground": "#F3EFF8",
      primary: "#58C29C",
      "primary-foreground": "#0E1D17",
      secondary: "#2C293D",
      "secondary-foreground": "#E5DFEF",
      muted: "#2C293D",
      "muted-foreground": "#A7A0BA",
      accent: "#1F362E",
      "accent-foreground": "#9CE5C9",
      destructive: "#EE796E",
      "destructive-foreground": "#1E0F0E",
      border: "#38344C",
      input: "#423D59",
      ring: "#58C29C",
      "surface-sunken": "#1B1927",
      "surface-elevated": "#2A273B",
      "ink-subtle": "#827B96",
      "vital-prime": "#58C29C",
      "vital-prime-soft": "#193028",
      "vital-steady": "#E6A855",
      "vital-steady-soft": "#322616",
      "vital-recover": "#EE796E",
      "vital-recover-soft": "#371D1B",
      "sleep-deep": "#58C29C",
      "sleep-core": "#3E9174",
      "sleep-rem": "#AA98DD",
      "sleep-awake": "#E6A855",
      "bento-sage": "#1B3028",
      "bento-amber": "#2E2517",
      "bento-lavender": "#2B243E",
      "bento-peach": "#351F1D",
      "stamp-ink": "#F08172",
      "stamp-soft": "#371F1D",
      "dew-accent": "#6BC9E8",
      "dew-soft": "#182D38",
      "stage-scrim": "#12101ECC",
      "stage-ink": "#FAF7FD",
      "atelier-accent": "#E7A35D",
      "atelier-soft": "#36281A",
      "wax-seal": "#EA7466",
      "wax-seal-soft": "#3A1E1B",
      "duo-accent": "#F4A28A",
      "duo-soft": "#3A231F",
      "ink-lapis": "#749EE0",
      "ink-gold": "#EAC063",
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
