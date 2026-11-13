import { Scroll, signal, Text, token, View } from "nsh";
import {
  completeOnboarding,
  getAppLocale,
  getState,
  getVisualTheme,
  setVisualTheme,
  toggleAppLocale,
} from "../store";
import { ActionButton, Card, ChoiceCard, ModalSurface, SunkenBox, VitalBadge } from "../ui/components";
import { QiIcon, type QiIconName } from "../ui/icons";
import { tr } from "../ui/i18n";
import { ThemeSwitcherStrip } from "../ui/sanctuary";

export function OnboardingModal() {
  const goal = signal("energy_sleep");
  const tier = signal("tier_a");
  const isOpen = () => !getState().settings.onboarding_done;

  const goals = (): { id: string; title: string; subtitle: string; icon: QiIconName }[] => [
    {
      id: "energy_sleep",
      title: tr("onboarding.goal_energy_title"),
      subtitle: tr("onboarding.goal_energy_sub"),
      icon: "sun",
    },
    {
      id: "diet_balance",
      title: tr("onboarding.goal_diet_title"),
      subtitle: tr("onboarding.goal_diet_sub"),
      icon: "utensils",
    },
    {
      id: "lab_archive",
      title: tr("onboarding.goal_lab_title"),
      subtitle: tr("onboarding.goal_lab_sub"),
      icon: "file_lab",
    },
    {
      id: "smart_training",
      title: tr("onboarding.goal_train_title"),
      subtitle: tr("onboarding.goal_train_sub"),
      icon: "heart_pulse",
    },
  ];

  const wearModes = (): { id: string; title: string; subtitle: string; icon: QiIconName }[] => [
    {
      id: "tier_a",
      title: tr("onboarding.tier_a_title"),
      subtitle: tr("onboarding.tier_a_sub"),
      icon: "heart_pulse",
    },
    {
      id: "tier_b",
      title: tr("onboarding.tier_b_title"),
      subtitle: tr("onboarding.tier_b_sub"),
      icon: "moon",
    },
    {
      id: "tier_c",
      title: tr("onboarding.tier_c_title"),
      subtitle: tr("onboarding.tier_c_sub"),
      icon: "compass",
    },
  ];

  return (
    <ModalSurface open={isOpen()} presentation="pageSheet" closeOnBackdropPress={false}>
      <Scroll class="flex-1" contentStyle={{ paddingHorizontal: 20, paddingBottom: 24, gap: 16 }}>
        <View class="flex-row items-center justify-between pt-1">
          <View class="flex-row items-center gap-2">
            <View class="size-8 items-center justify-center rounded-xl bg-accent">
              <QiIcon name="sun" size={18} color={token("primary")} />
            </View>
            <VitalBadge tone="prime_ready" label={tr("onboarding.privacy_badge")} />
          </View>
          <ActionButton
            title={tr("common.lang_toggle_compact")}
            icon="globe"
            size="sm"
            variant="ghost"
            shape="pill"
            accessibilityLabel={`${tr("common.language")} (${getAppLocale()})`}
            onPress={() => toggleAppLocale()}
          />
        </View>

        <View class="gap-1.5">
          <Text role="header" class="text-2xl font-bold text-foreground">
            {tr("onboarding.welcome_title")}
          </Text>
          <Text class="text-sm leading-relaxed text-muted-foreground">
            {tr("onboarding.welcome_subtitle")}
          </Text>
        </View>

        <Card>
          <Text role="header" class="text-sm font-semibold text-foreground">
            {tr("onboarding.step_goal_title")}
          </Text>
          <View class="gap-2">
            <Each of={goals()} key={(g) => g.id}>
              {(g) => (
                <ChoiceCard
                  title={g().title}
                  subtitle={g().subtitle}
                  icon={g().icon}
                  selected={goal() === g().id}
                  onPress={() => goal.set(g().id)}
                />
              )}
            </Each>
          </View>
        </Card>

        <Card>
          <Text role="header" class="text-sm font-semibold text-foreground">
            {tr("onboarding.step_wear_title")}
          </Text>
          <Text class="text-xs leading-relaxed text-muted-foreground">
            {tr("onboarding.step_wear_subtitle")}
          </Text>
          <View class="gap-2">
            <Each of={wearModes()} key={(m) => m.id}>
              {(m) => (
                <ChoiceCard
                  title={m().title}
                  subtitle={m().subtitle}
                  icon={m().icon}
                  selected={tier() === m().id}
                  onPress={() => tier.set(m().id)}
                />
              )}
            </Each>
          </View>
        </Card>

        <SunkenBox>
          <View class="flex-row items-center gap-2">
            <QiIcon name="cloud_lock" size={16} color={token("primary")} />
            <Text class="text-xs font-semibold text-primary">
              {tr("onboarding.step_privacy_title")}
            </Text>
          </View>
          <Text class="text-xs leading-relaxed text-muted-foreground">
            {tr("onboarding.step_privacy_desc")}
          </Text>
        </SunkenBox>

        <Card>
          <Text role="header" class="text-sm font-semibold text-foreground">
            {tr("onboarding.step_theme_title")}
          </Text>
          <ThemeSwitcherStrip
            activeId={getVisualTheme()}
            detailed={true}
            onSelect={(id) => setVisualTheme(id, true)}
          />
        </Card>

        <ActionButton
          title={tr("onboarding.begin_btn")}
          icon="sparkles"
          size="lg"
          onPress={() => completeOnboarding(goal(), tier())}
        />
      </Scroll>
    </ModalSurface>
  );
}
