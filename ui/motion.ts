import {
  Animated,
  createMotion,
  Easing,
  type JSX,
  LayoutAnimation,
  type Motion,
  reduceMotion,
  Shader,
  shadersAvailable,
  ShaderTransition,
  type StyleProp,
  type ViewStyle,
} from "nsh";

/**
 * 16-byte aligned WGSL uniform contract shared across all ambient & transition shaders.
 * Host buffer size: 4 x f32 = 16 bytes (no padding drift on wgpu / Metal / WebGL).
 */
export interface SceneShaderUniforms {
  /** Visual density / opacity scaling (0..1.2). */
  intensity: number;
  /** Chromatic temperature: 0 = cool moon/frost, 1 = warm sunlight/hearth. */
  warmth: number;
  /** Drift / rain diagonal slope (-1..1). */
  wind: number;
  /** Deterministic phase offset per theme / day. */
  seed: number;
}

export type SceneMoodShaderKind = "rain" | "hearth_steam" | "sun_motes" | "starlight" | "none";

export interface AmbientSceneShaderProps {
  mode: SceneMoodShaderKind;
  intensity?: number;
  warmth?: number;
  wind?: number;
  seed?: number;
  paused?: boolean;
  class?: string;
  style?: StyleProp<ViewStyle>;
}

export interface SceneCrossfadeProps {
  value: string | number | boolean;
  duration?: number;
  intensity?: number;
  warmth?: number;
  wind?: number;
  seed?: number;
  class?: string;
  style?: StyleProp<ViewStyle>;
  children?: JSX.Element;
}

/** Canonical timing, easing, and spring tokens aligned with DESIGN.md v5.0. */
export const MOTION_TOKENS = {
  duration: {
    instant: 120,
    fast: 180,
    base: 240,
    slow: 360,
    slower: 500,
  },
  easing: {
    softSettle: Easing.bezier(0.22, 1, 0.36, 1),
  },
  spring: {
    petSquish: { stiffness: 360, damping: 18, mass: 0.85 },
    petBounce: { stiffness: 260, damping: 14, mass: 0.9 },
    sealPress: { stiffness: 420, damping: 22, mass: 0.8 },
    sealRelease: { stiffness: 290, damping: 16, mass: 0.9 },
    polaroidTilt: { stiffness: 220, damping: 18, mass: 1 },
  },
} as const;

/** Diagonal spring drizzle, multi-depth rain streaks, and gentle misty glass sheen. */
export const RAIN_GLASS_WGSL = `
struct Uniforms {
  intensity: f32,
  warmth: f32,
  wind: f32,
  seed: f32,
}

fn qi_hash21(p: vec2f) -> f32 {
  var q = fract(p * vec2f(123.34, 456.21));
  q = q + vec2f(dot(q, q + vec2f(45.32)));
  return fract(q.x * q.y);
}

fn shade(uv: vec2f) -> vec4f {
  let aspect = nsh.size.x / max(nsh.size.y, 1.0);
  let inten = clamp(uniforms.intensity, 0.0, 1.2);
  let warmth = clamp(uniforms.warmth, 0.0, 1.0);
  let wind = clamp(uniforms.wind, -1.0, 1.0);
  let p = vec2f(uv.x * aspect, uv.y);

  let slope = 0.14 + wind * 0.12;
  let slanted = vec2f(p.x + p.y * slope, p.y);

  var rain_acc = 0.0;
  for (var layer: i32 = 0; layer < 3; layer = layer + 1) {
    let fl = f32(layer);
    let cols = 20.0 + fl * 11.0;
    let rows = 1.6 + fl * 0.65;
    let speed = 0.85 + fl * 0.35;
    let cell = vec2f(slanted.x * cols, slanted.y * rows - nsh.time * speed - fl * 0.37);
    let id = floor(cell);
    let f = fract(cell);
    let rnd = qi_hash21(id + vec2f(uniforms.seed * 7.13 + fl * 19.3, fl * 11.7));
    if (rnd > 0.42) {
      let x_pos = 0.25 + 0.5 * fract(rnd * 13.7);
      let dx = abs(f.x - x_pos);
      let streak_w = 0.055 - fl * 0.01;
      let horiz = 1.0 - smoothstep(0.0, max(streak_w, 0.015), dx);
      let vert = smoothstep(0.04, 0.35, f.y) * (1.0 - smoothstep(0.55, 0.96, f.y));
      let layer_gain = 1.0 - fl * 0.24;
      rain_acc = rain_acc + horiz * vert * layer_gain * (0.45 + 0.55 * rnd);
    }
  }

  let ripple = 0.5 + 0.5 * sin(p.x * 4.5 + p.y * 2.2 + nsh.time * 0.45 + uniforms.seed);
  let window_mist = (1.0 - smoothstep(0.0, 0.72, uv.y)) * (0.04 + 0.03 * ripple);
  let cool_tint = vec3f(0.76, 0.86, 0.91);
  let warm_tint = vec3f(0.92, 0.90, 0.84);
  let col = mix(cool_tint, warm_tint, warmth);
  let alpha = clamp((rain_acc * 0.14 + window_mist) * inten, 0.0, 0.24);
  return vec4f(clamp(col, vec3f(0.0), vec3f(1.0)), alpha);
}
`;

/** Warm charcoal stove ember pulse paired with rising curling tea steam ribbons. */
export const HEARTH_STEAM_WGSL = `
struct Uniforms {
  intensity: f32,
  warmth: f32,
  wind: f32,
  seed: f32,
}

fn shade(uv: vec2f) -> vec4f {
  let aspect = nsh.size.x / max(nsh.size.y, 1.0);
  let inten = clamp(uniforms.intensity, 0.0, 1.2);
  let warmth = clamp(uniforms.warmth, 0.0, 1.0);
  let wind = clamp(uniforms.wind, -1.0, 1.0);
  let t = nsh.time;

  let ember_center = vec2f(0.46 + wind * 0.06, 0.98);
  let d_ember = length(vec2f((uv.x - ember_center.x) * aspect * 0.55, (uv.y - ember_center.y) * 1.45));
  let pulse = 0.74 + 0.26 * sin(t * 1.15 + uniforms.seed * 6.28318);
  let ember = exp(-d_ember * d_ember * 4.2) * pulse;

  let rise_h = max(0.86 - uv.y, 0.0);
  let rise_mask = smoothstep(0.14, 0.38, uv.y) * (1.0 - smoothstep(0.72, 0.90, uv.y));
  let sway_1 = sin(uv.y * 10.5 + t * 0.75 + uniforms.seed * 3.14) * (0.016 + rise_h * 0.026) + wind * 0.03 * rise_h;
  let sway_2 = cos(uv.y * 8.2 + t * 0.58 + uniforms.seed * 5.21) * (0.014 + rise_h * 0.022) + wind * 0.024 * rise_h;

  let w_1 = 0.024 + rise_h * 0.044;
  let w_2 = 0.020 + rise_h * 0.038;
  let s_1 = 1.0 - smoothstep(0.0, w_1, abs((uv.x - (0.61 + sway_1)) * aspect));
  let s_2 = 1.0 - smoothstep(0.0, w_2, abs((uv.x - (0.67 + sway_2)) * aspect));
  let steam = (s_1 * 0.65 + s_2 * 0.45) * rise_mask;

  let ember_col = mix(vec3f(0.74, 0.82, 0.94), vec3f(0.98, 0.58, 0.26), warmth);
  let steam_col = vec3f(0.96, 0.94, 0.90);
  let w_sum = ember * 0.17 + steam * 0.13 + 0.0001;
  let rgb = (ember_col * (ember * 0.17) + steam_col * (steam * 0.13)) / w_sum;
  let alpha = clamp((ember * 0.17 + steam * 0.13) * inten, 0.0, 0.26);
  return vec4f(clamp(rgb, vec3f(0.0), vec3f(1.0)), alpha);
}
`;

/** Drifting sunlight dust motes & evening fireflies (doubles as Starlight Memorial pet aura when warmth is cool). */
export const SUN_MOTES_WGSL = `
struct Uniforms {
  intensity: f32,
  warmth: f32,
  wind: f32,
  seed: f32,
}

fn shade(uv: vec2f) -> vec4f {
  let aspect = nsh.size.x / max(nsh.size.y, 1.0);
  let inten = clamp(uniforms.intensity, 0.0, 1.2);
  let warmth = clamp(uniforms.warmth, 0.0, 1.0);
  let wind = clamp(uniforms.wind, -1.0, 1.0);
  let t = nsh.time;

  let diag = clamp(1.0 - (uv.x * 0.58 + uv.y * 0.76), 0.0, 1.0);
  let ray_wave = 0.78 + 0.22 * sin((uv.x - uv.y * 0.65) * 10.0 + t * 0.22 + uniforms.seed);
  let beam = pow(diag, 1.85) * ray_wave * (0.06 + 0.07 * warmth);

  var mote_acc = 0.0;
  var gold_acc = 0.0;
  for (var i: i32 = 0; i < 10; i = i + 1) {
    let fi = f32(i);
    let bx = fract(0.17 + fi * 0.6180339 + uniforms.seed * 0.13);
    let by = 0.12 + 0.70 * fract(0.31 + fi * 0.381966 + uniforms.seed * 0.09);
    let drift_x = sin(t * (0.19 + 0.025 * fi) + fi * 1.7 + uniforms.seed * 4.0) * 0.032 + wind * sin(t * 0.14 + fi) * 0.02;
    let drift_y = cos(t * (0.16 + 0.022 * fi) + fi * 2.3) * 0.028;
    let pos = vec2f(clamp(bx + drift_x, 0.06, 0.94), clamp(by + drift_y, 0.08, 0.88));
    let d = length(vec2f((uv.x - pos.x) * aspect, uv.y - pos.y));
    let twinkle = 0.52 + 0.48 * sin(t * (0.95 + 0.21 * fi) + fi * 2.17 + uniforms.seed * 5.0);
    let core = exp(-d * d * 2400.0);
    let halo = exp(-d * d * 360.0) * 0.36;
    let energy = (core + halo) * twinkle;
    mote_acc = mote_acc + energy;
    if (i % 3 == 0) {
      gold_acc = gold_acc + energy;
    }
  }

  let warm_mote = vec3f(0.99, 0.89, 0.64);
  let star_cool = vec3f(0.84, 0.90, 0.99);
  let star_gold = vec3f(0.98, 0.86, 0.56);
  let star_ratio = clamp(gold_acc / max(mote_acc, 0.0001), 0.0, 1.0);
  let star_mix = mix(star_cool, star_gold, star_ratio);
  let mote_col = mix(star_mix, warm_mote, warmth);
  let beam_col = mix(vec3f(0.76, 0.84, 0.97), vec3f(0.99, 0.91, 0.72), warmth);

  let total_w = beam + mote_acc * 0.26 + 0.0001;
  let rgb = (beam_col * beam + mote_col * (mote_acc * 0.26)) / total_w;
  let alpha = clamp((beam + mote_acc * 0.26) * inten, 0.0, 0.28);
  return vec4f(clamp(rgb, vec3f(0.0), vec3f(1.0)), alpha);
}
`;

/** Ink-wash & warm sunlight sweep transition shader for `<ShaderTransition>`. */
export const SOLAR_CROSSFADE_WGSL = `
struct Uniforms {
  intensity: f32,
  warmth: f32,
  wind: f32,
  seed: f32,
}

fn shade(uv: vec2f) -> vec4f {
  let p = clamp(nsh.progress, 0.0, 1.0);
  let warmth = clamp(uniforms.warmth, 0.0, 1.0);
  let inten = clamp(uniforms.intensity, 0.0, 1.2);
  let wind = clamp(uniforms.wind, -0.4, 0.4);
  let from_c = sample_from(uv);
  let to_c = sample_to(uv);

  let wash = sin(uv.x * 9.0 + uniforms.seed * 3.1) * cos(uv.y * 7.0 - uniforms.seed * 1.9) * 0.045;
  let wx = 0.62 + wind * 0.12;
  let wy = 1.0 - wx;
  let diag = clamp((uv.x * wx + uv.y * wy) + wash, 0.0, 1.0);
  let edge = 0.28;
  let front = mix(-edge - 0.01, 1.0 + edge + 0.01, p);
  let blend = smoothstep(front - edge, front + edge, diag);
  let t = 1.0 - blend;

  let fp = vec4f(from_c.rgb * from_c.a, from_c.a);
  let tp = vec4f(to_c.rgb * to_c.a, to_c.a);
  let mp = mix(fp, tp, t);

  var rgb = vec3f(0.0);
  if (mp.a > 0.0001) {
    rgb = mp.rgb / mp.a;
  }

  let crest_dist = (diag - front) / edge;
  let crest = exp(-crest_dist * crest_dist * 3.2) * sin(p * 3.14159265) * 0.14 * inten;
  let sun_tint = mix(vec3f(0.86, 0.90, 0.96), vec3f(0.99, 0.92, 0.76), warmth);
  rgb = mix(rgb, sun_tint, clamp(crest, 0.0, 0.22));
  return vec4f(clamp(rgb, vec3f(0.0), vec3f(1.0)), clamp(mp.a, 0.0, 1.0));
}
`;

export function resolveSceneShaderSource(mode: SceneMoodShaderKind): string {
  switch (mode) {
    case "rain":
      return RAIN_GLASS_WGSL;
    case "hearth_steam":
      return HEARTH_STEAM_WGSL;
    case "starlight":
    case "sun_motes":
    default:
      return SUN_MOTES_WGSL;
  }
}

export function resolveSceneShaderUniforms(
  mode: SceneMoodShaderKind,
  overrides?: Partial<SceneShaderUniforms>,
): SceneShaderUniforms {
  if (mode === "none") {
    return { intensity: 0, warmth: 0.5, wind: 0, seed: 1 };
  }
  const defaults: Record<Exclude<SceneMoodShaderKind, "none">, SceneShaderUniforms> = {
    rain: { intensity: 0.85, warmth: 0.35, wind: 0.22, seed: 1.7 },
    hearth_steam: { intensity: 0.92, warmth: 0.88, wind: 0.12, seed: 2.4 },
    sun_motes: { intensity: 0.88, warmth: 0.86, wind: 0.18, seed: 3.1 },
    starlight: { intensity: 0.95, warmth: 0.18, wind: 0.1, seed: 4.6 },
  };
  const base = defaults[mode];
  return {
    intensity: overrides?.intensity ?? base.intensity,
    warmth: overrides?.warmth ?? base.warmth,
    wind: overrides?.wind ?? base.wind,
    seed: overrides?.seed ?? base.seed,
  };
}

/** GPU-accelerated ambient scene shader layer (auto-pauses on reduceMotion). */
export function AmbientSceneShader(props: AmbientSceneShaderProps): JSX.Element {
  return If({
    get when() {
      return shadersAvailable() && props.mode !== "none";
    },
    get children() {
      return Shader({
        get source() {
          return resolveSceneShaderSource(props.mode);
        },
        get uniforms() {
          return resolveSceneShaderUniforms(props.mode, {
            intensity: props.intensity,
            warmth: props.warmth,
            wind: props.wind,
            seed: props.seed,
          }) as unknown as Record<string, number>;
        },
        get paused() {
          return Boolean(props.paused || reduceMotion() || props.mode === "none");
        },
        get class() {
          return props.class ?? "pointer-events-none absolute inset-0";
        },
        get style() {
          return [{ pointerEvents: "none" as const }, props.style];
        },
        accessibilityElementsHidden: true,
      });
    },
  });
}

/** GPU ink-wash & sunlight sweep crossfade wrapper for scene/theme transitions. */
export function SceneCrossfade(props: SceneCrossfadeProps): JSX.Element {
  return ShaderTransition({
    get value() {
      return props.value;
    },
    get source() {
      return shadersAvailable() && !reduceMotion() ? SOLAR_CROSSFADE_WGSL : undefined;
    },
    get uniforms() {
      if (!shadersAvailable() || reduceMotion()) return undefined;
      return {
        intensity: props.intensity ?? 0.92,
        warmth: props.warmth ?? 0.72,
        wind: props.wind ?? 0.25,
        seed: props.seed ?? 1.2,
      } as unknown as Record<string, number>;
    },
    get duration() {
      return reduceMotion() ? 0 : (props.duration ?? MOTION_TOKENS.duration.slow);
    },
    get fallback() {
      return reduceMotion() ? "none" : "fade";
    },
    get paused() {
      return reduceMotion();
    },
    get class() {
      return props.class;
    },
    get style() {
      return props.style;
    },
    get children() {
      return props.children;
    },
  });
}

/** Spring squash-and-stretch petting controller for companion artwork and custom pet frame. */
export function createPetBounceMotion(): {
  motion: Motion;
  triggerPet: () => Promise<{ finished: boolean }>;
  reset: () => void;
} {
  const motion = createMotion();

  const triggerPet = () => {
    if (reduceMotion()) {
      motion.reset();
      return Promise.resolve({ finished: true });
    }
    motion.stop();
    const squish = MOTION_TOKENS.spring.petSquish;
    const bounce = MOTION_TOKENS.spring.petBounce;
    return Animated.sequence([
      Animated.parallel([
        Animated.spring(motion.scaleX, { toValue: 1.07, ...squish }),
        Animated.spring(motion.scaleY, { toValue: 0.92, ...squish }),
        Animated.spring(motion.translateY, { toValue: 3, ...squish }),
      ]),
      Animated.parallel([
        Animated.spring(motion.scaleX, { toValue: 0.98, ...bounce }),
        Animated.spring(motion.scaleY, { toValue: 1.04, ...bounce }),
        Animated.spring(motion.translateY, { toValue: -3, ...bounce }),
      ]),
      Animated.parallel([
        Animated.spring(motion.scaleX, { toValue: 1, ...bounce }),
        Animated.spring(motion.scaleY, { toValue: 1, ...bounce }),
        Animated.spring(motion.translateY, { toValue: 0, ...bounce }),
      ]),
    ]).start();
  };

  return {
    motion,
    triggerPet,
    reset: () => motion.reset(),
  };
}

/** Tactile wax-seal / paw-stamp spring press & tilted rest controller. */
export function createSealStampMotion(initialStamped = false, restAngle = -8): {
  motion: Motion;
  pressDown: () => Promise<{ finished: boolean }>;
  releaseStamp: (stamped?: boolean) => Promise<{ finished: boolean }>;
  snapRest: (stamped: boolean) => void;
} {
  const motion = createMotion();
  motion.rotate.set(initialStamped ? restAngle : 0);

  const snapRest = (stamped: boolean) => {
    motion.stop();
    motion.scale.set(1);
    motion.rotate.set(stamped ? restAngle : 0);
    motion.opacity.set(1);
  };

  const pressDown = () => {
    if (reduceMotion()) {
      return Promise.resolve({ finished: true });
    }
    motion.stop();
    return Animated.spring(motion.scale, {
      toValue: 0.92,
      ...MOTION_TOKENS.spring.sealPress,
    }).start();
  };

  const releaseStamp = (stamped = true) => {
    if (reduceMotion()) {
      snapRest(stamped);
      return Promise.resolve({ finished: true });
    }
    const targetAngle = stamped ? restAngle : 0;
    return Animated.sequence([
      Animated.spring(motion.scale, {
        toValue: 1.06,
        ...MOTION_TOKENS.spring.sealPress,
      }),
      Animated.parallel([
        Animated.spring(motion.scale, {
          toValue: 1,
          ...MOTION_TOKENS.spring.sealRelease,
        }),
        Animated.spring(motion.rotate, {
          toValue: targetAngle,
          ...MOTION_TOKENS.spring.sealRelease,
        }),
      ]),
    ]).start();
  };

  return {
    motion,
    pressDown,
    releaseStamp,
    snapRest,
  };
}

/** Perspective polaroid/poster tilt spring controller (uses `[{ perspective: 800 }, ...motion.transform]`). */
export function createPolaroidTiltMotion(): {
  motion: Motion;
  style: () => { opacity: Motion["opacity"]; transform: ({ perspective: number } | (typeof motion.transform)[number])[] };
  tilt: (xDir?: -1 | 0 | 1, yDir?: -1 | 0 | 1) => Promise<{ finished: boolean }>;
  reset: () => Promise<{ finished: boolean }>;
} {
  const motion = createMotion();

  const style = () => ({
    opacity: motion.opacity,
    transform: [{ perspective: 800 }, ...motion.transform],
  });

  const tilt = (xDir: -1 | 0 | 1 = 1, yDir: -1 | 0 | 1 = -1) => {
    if (reduceMotion()) {
      motion.reset();
      return Promise.resolve({ finished: true });
    }
    const cfg = MOTION_TOKENS.spring.polaroidTilt;
    return Animated.parallel([
      Animated.spring(motion.rotateY, { toValue: xDir * 5, ...cfg }),
      Animated.spring(motion.rotateX, { toValue: yDir * 3, ...cfg }),
      Animated.spring(motion.scale, { toValue: 0.985, ...cfg }),
    ]).start();
  };

  const reset = () => {
    if (reduceMotion()) {
      motion.reset();
      return Promise.resolve({ finished: true });
    }
    const cfg = MOTION_TOKENS.spring.polaroidTilt;
    return Animated.parallel([
      Animated.spring(motion.rotateY, { toValue: 0, ...cfg }),
      Animated.spring(motion.rotateX, { toValue: 0, ...cfg }),
      Animated.spring(motion.scale, { toValue: 1, ...cfg }),
    ]).start();
  };

  return {
    motion,
    style,
    tilt,
    reset,
  };
}

/** Native layout transition guard for Spatial Scene Dock drawers and accordion panels. */
export function runDrawerLayoutTransition(preset: "easeInEaseOut" | "spring" = "easeInEaseOut"): void {
  if (reduceMotion()) return;
  LayoutAnimation.configureNext(
    preset === "spring" ? LayoutAnimation.Presets.spring : LayoutAnimation.Presets.easeInEaseOut,
  );
}
