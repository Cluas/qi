import {
  Animated,
  Canvas,
  createAnimatedValue,
  Easing,
  effect,
  reduceMotion,
  Scroll,
  signal,
  Text,
  token,
  View,
} from "nsh";
import {
  completeOnboarding,
  getAppLocale,
  getState,
  isHealthkitSupported,
  syncAppleHealthkit,
  toggleAppLocale,
} from "../store";
import { ActionButton, Button, HeaderActionPill, VitalBadge } from "../ui/components";
import { hapticSelection, hapticSuccess } from "../ui/haptics";
import { QiIcon } from "../ui/icons";
import { tr } from "../ui/i18n";
import {
  createOrbRippleMotion,
  createPreludeEntranceMotion,
  MOTION_TOKENS,
  PreludeAuroraShader,
  SceneCrossfade,
} from "../ui/motion";

const BREATH_EASE = Easing.bezier(0.37, 0, 0.63, 1);

function PreludeBreathingOrb(props: {
  active: () => boolean;
  onTap: () => void;
}) {
  const breath = createAnimatedValue(0.45);
  const orbRipple = createOrbRippleMotion();

  const outerRadius = breath.interpolate({ inputRange: [0, 1], outputRange: [76, 95] });
  const midRadius = breath.interpolate({ inputRange: [0, 1], outputRange: [54, 69] });
  const coreRadius = breath.interpolate({ inputRange: [0, 1], outputRange: [34, 41] });
  const outerAlpha = breath.interpolate({ inputRange: [0, 1], outputRange: [0.12, 0.25] });
  const midAlpha = breath.interpolate({ inputRange: [0, 1], outputRange: [0.20, 0.36] });

  effect(() => {
    if (!props.active() || reduceMotion()) {
      breath.set(0.45);
      return;
    }
    const anim = Animated.loop(
      Animated.sequence([
        Animated.timing(breath, { toValue: 1, duration: 2200, easing: BREATH_EASE }),
        Animated.timing(breath, { toValue: 0, duration: 2200, easing: BREATH_EASE }),
      ]),
    );
    void anim.start();
    return () => anim.stop();
  });

  return (
    <Button
      hitSlop={10}
      accessibilityLabel={tr("onboarding.orb_hint")}
      onPress={() => {
        hapticSelection();
        void orbRipple.ripple();
        props.onTap();
      }}
      class="items-center justify-center rounded-full focus-visible:ring-2 focus-visible:ring-ring"
    >
      <View
        style={[{ width: 220, height: 220 }, orbRipple.motion.style]}
        class="relative items-center justify-center"
      >
        <Canvas
          style={{ width: 220, height: 220 }}
          accessibilityElementsHidden={true}
          draw={(cx, size) => {
            const cxMid = size.width / 2;
            const cyMid = size.height / 2;
            const primary = token("primary");

            cx.clearRect(0, 0, size.width, size.height);

            // Delicate outer celestial breath ticks
            const ticks = 24;
            for (let i = 0; i < ticks; i++) {
              const ang = (Math.PI * 2 * i) / ticks;
              const tx = cxMid + Math.cos(ang) * 98;
              const ty = cyMid + Math.sin(ang) * 98;
              cx.save();
              cx.fillStyle = primary;
              cx.globalAlpha = i % 3 === 0 ? 0.32 : 0.14;
              cx.beginPath();
              cx.arc(tx, ty, i % 3 === 0 ? 1.8 : 1.1, 0, Math.PI * 2);
              cx.fill();
              cx.restore();
            }

            // Outer breathing celadon halo
            cx.save();
            cx.fillStyle = primary;
            cx.globalAlpha = outerAlpha;
            cx.beginPath();
            cx.arc(cxMid, cyMid, outerRadius, 0, Math.PI * 2);
            cx.fill();
            cx.restore();

            // Mid soothing jade ring
            cx.save();
            cx.fillStyle = primary;
            cx.globalAlpha = midAlpha;
            cx.beginPath();
            cx.arc(cxMid, cyMid, midRadius, 0, Math.PI * 2);
            cx.fill();
            cx.restore();

            // Inner glowing core circle
            cx.save();
            cx.fillStyle = primary;
            cx.globalAlpha = 0.88;
            cx.beginPath();
            cx.arc(cxMid, cyMid, coreRadius, 0, Math.PI * 2);
            cx.fill();
            cx.restore();
          }}
        />

        <View class="pointer-events-none absolute inset-0 items-center justify-center">
          <Text class="text-[22px] font-bold tracking-wider text-primary-foreground">
            {tr("app.name")}
          </Text>
        </View>
      </View>
    </Button>
  );
}

export function OnboardingModal() {
  const beat = signal(0);
  const connectHealthkit = signal(true);
  const entrance = createPreludeEntranceMotion();
  const isOpen = () => !getState().settings.onboarding_done;

  const beats = () => [
    {
      kicker: tr("onboarding.beat1_kicker"),
      title: tr("onboarding.welcome_subtitle"),
      body: tr("onboarding.beat1_body"),
      warmth: 0.62,
    },
    {
      kicker: tr("onboarding.privacy_badge"),
      title: tr("onboarding.beat2_title"),
      body: tr("onboarding.beat2_body"),
      warmth: 0.86,
    },
    {
      kicker: tr("onboarding.privacy_badge"),
      title: tr("onboarding.beat3_title"),
      body: tr("onboarding.beat3_body"),
      warmth: 0.36,
    },
  ];

  const currentBeat = () => beats()[beat()] ?? beats()[0];

  effect(() => {
    if (!isOpen()) return;
    void entrance.playEntrance();
    if (reduceMotion()) return;
    const timer = setInterval(() => {
      beat.set((beat() + 1) % 3);
    }, 3800);
    return () => clearInterval(timer);
  });

  const handleOpenWindow = () => {
    hapticSuccess();
    completeOnboarding("energy_sleep", "tier_a");
    if (isHealthkitSupported() && connectHealthkit()) {
      void syncAppleHealthkit(35, false);
    }
  };

  return (
    <View class="relative flex-1 overflow-hidden bg-background">
      <PreludeAuroraShader
        intensity={0.95}
        warmth={currentBeat().warmth}
        breath={beat() * 0.45}
        paused={!isOpen()}
      />

      <View
        class="flex-1"
        style={{
          paddingTop: "env(safe-area-inset-top)",
          paddingBottom: "env(safe-area-inset-bottom)",
        }}
      >
        <Scroll
          class="flex-1"
          contentStyle={{
            flexGrow: 1,
            justifyContent: "space-between",
            paddingHorizontal: 24,
            paddingTop: 12,
            paddingBottom: 28,
          }}
        >
        {/* Top bar: zero-account pill & instant locale switch */}
        <View class="flex-row items-center justify-between gap-2">
          <VitalBadge tone="prime_ready" label={tr("onboarding.privacy_badge")} />
          <HeaderActionPill
            label={tr("common.lang_toggle_compact")}
            icon="globe"
            tone="neutral"
            accessibilityLabel={`${tr("common.language")} (${getAppLocale()})`}
            onPress={() => toggleAppLocale()}
          />
        </View>

        {/* Centerpiece: breathing celadon orb + poetic 3-beat progression */}
        <View style={entrance.motion.style} class="my-auto items-center gap-6 py-4">
          <PreludeBreathingOrb
            active={isOpen}
            onTap={() => beat.set((beat() + 1) % 3)}
          />

          <View class="w-full max-w-sm items-center gap-3">
            <Text role="header" class="text-[28px] font-bold tracking-tight text-foreground">
              {tr("onboarding.welcome_title")}
            </Text>

            <SceneCrossfade
              value={beat()}
              duration={MOTION_TOKENS.duration.slow}
              warmth={currentBeat().warmth}
              class="w-full items-center"
            >
              <View class="items-center gap-2 px-2">
                <Text class="text-center text-lg font-semibold leading-snug text-foreground">
                  {currentBeat().title}
                </Text>
                <Text class="text-center text-sm leading-relaxed text-muted-foreground">
                  {currentBeat().body}
                </Text>
              </View>
            </SceneCrossfade>
          </View>
        </View>

        {/* Bottom dock: interactive beat dots, whisper HealthKit pill & 1-tap CTA */}
        <View class="w-full max-w-sm items-center gap-3.5 self-center">
          <View class="flex-row items-center gap-2">
            <Each of={[0, 1, 2]} key={(i) => i}>
              {(idx) => (
                <Button
                  hitSlop={10}
                  accessibilityLabel={`${idx() + 1} / 3`}
                  onPress={() => {
                    hapticSelection();
                    beat.set(idx());
                  }}
                  class={`h-2 rounded-full transition-all duration-200 ${
                    beat() === idx() ? "w-6 bg-primary" : "w-2 bg-primary/25"
                  }`}
                />
              )}
            </Each>
          </View>

          <Text class="text-center text-xs text-muted-foreground">
            {tr("onboarding.orb_hint")}
          </Text>

          <If when={isHealthkitSupported()}>
            <Button
              hitSlop={6}
              accessibilityLabel={
                connectHealthkit()
                  ? tr("onboarding.healthkit_pill_on")
                  : tr("onboarding.healthkit_pill_off")
              }
              onPress={() => {
                hapticSelection();
                connectHealthkit.update((v) => !v);
              }}
              class={`min-h-9 flex-row items-center gap-1.5 rounded-full border px-3.5 py-1.5 transition-colors duration-150 active:scale-[0.97] motion-reduce:transition-none ${
                connectHealthkit()
                  ? "border-primary/35 bg-accent/80"
                  : "border-border/55 bg-card/80"
              }`}
            >
              <QiIcon
                name={connectHealthkit() ? "heart_pulse" : "cloud_lock"}
                size={13}
                color={connectHealthkit() ? token("primary") : token("muted-foreground")}
              />
              <Text
                class={`text-xs font-medium ${
                  connectHealthkit() ? "text-primary" : "text-muted-foreground"
                }`}
              >
                {connectHealthkit()
                  ? tr("onboarding.healthkit_pill_on")
                  : tr("onboarding.healthkit_pill_off")}
              </Text>
            </Button>
          </If>

          <ActionButton
            title={tr("onboarding.begin_btn")}
            icon="sparkles"
            size="lg"
            shape="pill"
            onPress={handleOpenWindow}
            class="w-full"
          />
        </View>
        </Scroll>
      </View>
    </View>
  );
}
