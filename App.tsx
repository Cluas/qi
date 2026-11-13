import { mount, Scroll, signal, Text, token, View, windowSize } from "nsh";
import { InsightsPage } from "./pages/InsightsPage";
import { LogLabsPage } from "./pages/LogLabsPage";
import { MeVaultPage } from "./pages/MeVaultPage";
import { OnboardingModal } from "./pages/OnboardingModal";
import { QaStudioSheet } from "./pages/QaStudioSheet";
import { TodayPage } from "./pages/TodayPage";
import {
  type ActiveTab,
  cancelZdrConsent,
  clearToast,
  confirmAtelierApplePurchase,
  confirmZdrConsent,
  copyToClipboard,
  coSanctuaryAction,
  cycleAppearance,
  getAppearancePref,
  getAppLocale,
  getDewSummary,
  getHearthGiftCount,
  getHidePosterNumbers,
  getParentsPeaceLetterResult,
  getRestoringPurchases,
  getRestoreStatusMessage,
  getShowAtelierModal,
  getShowCompanionLetterModal,
  getShowCoSanctuarySheet,
  getShowDailyPoster,
  getShowFamilyPeaceLetterModal,
  getShowFourSeasonsScrollModal,
  getShowQuickCaptureModal,
  getShowZdrConsentModal,
  getState,
  getTab,
  getToast,
  getVisualTheme,
  logNaturalIntake,
  restoreApplePurchases,
  setHidePosterNumbers,
  setSealInk,
  setShowAtelierModal,
  setShowCompanionLetterModal,
  setShowCoSanctuarySheet,
  setShowDailyPoster,
  setShowFamilyPeaceLetterModal,
  setShowFourSeasonsScrollModal,
  setShowQaStudio,
  setShowQuickCaptureModal,
  setTab,
  toggleAppLocale,
} from "./store";
import {
  CompanionLetterModal,
  CoSanctuarySheet,
  FamilyPeaceLetterModal,
  FourSeasonsScrollModal,
  QiAtelierModal,
} from "./ui/atelier";
import {
  ActionButton,
  BottomTabBar,
  Card,
  DrawerSheet,
  ensureDefaultIosPortraitWindow,
  FieldInput,
  IconButton,
  ModalSurface,
  NavRail,
  SelectChip,
  SunkenBox,
  ToastBanner,
  VitalBadge,
} from "./ui/components";
import { QiIcon, type QiIconName } from "./ui/icons";
import { tr } from "./ui/i18n";
import { MOTION_TOKENS, SceneCrossfade } from "./ui/motion";
import { DailyPosterModal, type SealInkId } from "./ui/sanctuary";

function App() {
  ensureDefaultIosPortraitWindow();

  const quickText = signal("");
  const includeHearthGift = signal(getState().entitlements.guardian_sponsor);
  const myMoodLocal = signal<"sunny_full" | "breeze_steady" | "rest_recharge" | null>(null);
  const isDesktop = () => windowSize().width >= 920;

  const navTabs = (): { id: ActiveTab; label: string; icon: QiIconName; sfSymbol: string }[] => [
    { id: "today", label: tr("nav.today"), icon: "sun", sfSymbol: "sun.max.fill" },
    { id: "log", label: tr("nav.log"), icon: "record", sfSymbol: "square.and.pencil" },
    { id: "insights", label: tr("nav.insights"), icon: "compass", sfSymbol: "safari" },
    { id: "me", label: tr("nav.me"), icon: "user_shield", sfSymbol: "person.crop.circle" },
  ];

  const syncSummary = () => {
    const mode = getState().settings.sync_mode;
    switch (mode) {
      case "icloud_drive":
        return tr("sync.mode_icloud_summary");
      case "webdav":
        return tr("sync.mode_webdav_summary");
      case "byo_server":
        return tr("sync.mode_byo_summary");
      default:
        return tr("sync.mode_local_summary");
    }
  };

  const appearanceIcon = (): QiIconName => {
    return getAppearancePref() === "dark" ? "moon" : "sun";
  };

  const submitGlobalQuickCapture = () => {
    const text = quickText().trim();
    if (!text) return;
    logNaturalIntake(text);
    quickText.set("");
    setShowQuickCaptureModal(false);
  };

  const snap = () => getState().snapshot;
  const settings = () => getState().settings;
  const customPets = () => getState().solar_atelier;

  return (
    <View
      class="flex-1 bg-background"
      style={{
        paddingTop: "env(safe-area-inset-top)",
        paddingLeft: "env(safe-area-inset-left)",
        paddingRight: "env(safe-area-inset-right)",
      }}
    >
      {/* Decluttered sanctuary header: brand, quiet privacy status, compact locale toggle & utilities */}
      <View
        ios={{ backdropMaterial: "ultraThin" }}
        class="flex-row items-center justify-between gap-2 border-b border-border/80 bg-card/90 px-4 py-2"
      >
        <View class="flex-1 flex-row items-center gap-2.5">
          <Text role="header" class="text-lg font-bold text-foreground">
            {tr("app.name")}
          </Text>
          <View class="flex-row items-center gap-1 rounded-full border border-vital-prime/25 bg-vital-prime-soft px-2 py-0.5">
            <QiIcon name="user_shield" size={11} color={token("vital-prime")} />
            <Text class="text-[11px] font-semibold text-vital-prime">
              {settings().sync_mode === "local_only"
                ? tr("status.local_sealed")
                : tr("status.synced")}
            </Text>
          </View>
          <If when={isDesktop()}>
            <Text class="text-xs text-muted-foreground">{syncSummary()}</Text>
          </If>
        </View>

        <View class="flex-row items-center gap-1">
          <ActionButton
            title={tr("common.lang_toggle_compact")}
            icon="globe"
            variant="ghost"
            size="sm"
            shape="pill"
            accessibilityLabel={`${tr("common.language")} (${getAppLocale()})`}
            onPress={() => toggleAppLocale()}
          />
          <IconButton
            icon={appearanceIcon()}
            size="sm"
            variant="ghost"
            accessibilityLabel={tr("common.appearance")}
            onPress={() => cycleAppearance()}
          />
          <If when={isDesktop()}>
            <IconButton
              icon="plus"
              size="sm"
              variant="secondary"
              accessibilityLabel={tr("nav.quick_capture")}
              onPress={() => setShowQuickCaptureModal(true)}
            />
          </If>
          <IconButton
            icon="sliders"
            size="sm"
            variant="ghost"
            accessibilityLabel={tr("me.qa_studio_btn")}
            onPress={() => setShowQaStudio(true)}
          />
        </View>
      </View>

      <If when={Boolean(getToast())}>
        <ToastBanner message={getToast()!} onDismiss={() => clearToast()} />
      </If>

      {/* Main layout: desktop NavRail vs mobile single column with shader crossfade */}
      <View class="flex-1 flex-row">
        <If when={isDesktop()}>
          <NavRail
            title={tr("nav.rail_title")}
            items={navTabs()}
            activeId={getTab()}
            onSelect={(id) => setTab(id)}
            footer={
              <SunkenBox>
                <View class="flex-row items-center gap-1.5">
                  <QiIcon name="cloud_lock" size={14} color={token("primary")} />
                  <Text class="text-xs font-semibold text-foreground">
                    {tr("nav.local_guard_title")}
                  </Text>
                </View>
                <Text class="text-[11px] leading-relaxed text-muted-foreground">
                  {tr("nav.local_guard_desc")}
                </Text>
              </SunkenBox>
            }
          />
        </If>

        <SceneCrossfade
          value={getTab()}
          duration={MOTION_TOKENS.duration.base}
          class="flex-1"
        >
          <Scroll class="flex-1" contentStyle={{ padding: 16, paddingBottom: 24, gap: 16 }}>
            <View class={`mx-auto w-full gap-4 ${isDesktop() ? "max-w-3xl" : "max-w-xl"}`}>
              {getTab() === "today" ? (
                <TodayPage />
              ) : getTab() === "log" ? (
                <LogLabsPage />
              ) : getTab() === "insights" ? (
                <InsightsPage />
              ) : (
                <MeVaultPage />
              )}
            </View>
          </Scroll>
        </SceneCrossfade>
      </View>

      <If when={!isDesktop()}>
        <BottomTabBar
          items={navTabs()}
          activeId={getTab()}
          onSelect={(id) => setTab(id)}
          onQuickCapture={() => setShowQuickCaptureModal(true)}
          quickCaptureLabel={tr("nav.quick_capture")}
        />
      </If>

      <OnboardingModal />
      <QaStudioSheet />

      <DailyPosterModal
        open={getShowDailyPoster()}
        themeId={getVisualTheme()}
        targetDate={snap().target_date}
        score={snap().qi_score}
        state={snap().state}
        stateBadge={snap().state_badge}
        sleepHours={snap().sleep_last_night_hours}
        headline={snap().headline}
        microAction={snap().micro_action}
        dewPoints={getDewSummary().points}
        checkedAction={settings().checked_action_today}
        sealInk={(settings().active_seal_ink || "cinnabar") as SealInkId}
        onChangeSealInk={(ink) => setSealInk(ink)}
        hideNumbers={getHidePosterNumbers()}
        onToggleHideNumbers={(hide) => setHidePosterNumbers(hide)}
        atelierUnlocked={getState().entitlements.is_atelier_unlocked}
        customPet={customPets().active_custom_pet}
        onCopyCaption={(caption) => void copyToClipboard(caption, tr("poster.title"))}
        onClose={() => setShowDailyPoster(false)}
      />

      <QiAtelierModal
        open={getShowAtelierModal()}
        companionThemeId={getVisualTheme()}
        currentTier={getState().entitlements.tier}
        guardianSponsor={includeHearthGift()}
        onToggleGuardian={(enabled) => includeHearthGift.set(enabled)}
        loyaltyDays={
          getState().entitlements.companion_loyalty_eligible
            ? 30
            : settings().companion_loyalty_days
        }
        offlineStateBanner={getState().entitlements.status_banner}
        plans={getState().entitlement_catalog.plans.map((p) => ({
          tier: p.tier,
          title: p.title,
          subtitle: p.subtitle,
          priceCny: p.price_cny,
          originalPriceCny: p.original_price_cny,
          billingCycle: p.billing_cycle,
          seats: p.seats,
          highlights: p.highlights,
          badge: p.badge,
          loyaltyApplied: p.loyalty_applied,
        }))}
        timelineSteps={getState().entitlement_catalog.transparent_timeline.map((t) => ({
          dayLabel: t.day_label,
          title: t.title,
          detail: t.detail,
        }))}
        ethicalPromises={getState().entitlement_catalog.ethical_promises}
        hearthGiftCount={getHearthGiftCount()}
        restoringPurchases={getRestoringPurchases()}
        restoreStatusMessage={getRestoreStatusMessage()}
        onRestorePurchases={() => void restoreApplePurchases()}
        onConfirm={(tier, withHearthGift) => {
          confirmAtelierApplePurchase(tier, withHearthGift);
          setShowAtelierModal(false);
        }}
        onClose={() => setShowAtelierModal(false)}
      />

      <CompanionLetterModal
        open={getShowCompanionLetterModal()}
        atelierUnlocked={getState().entitlements.is_atelier_unlocked}
        letters={(() => {
          const petLetters = customPets().pet_weekly_letters;
          const byWeek = new Map(
            petLetters.map((p) => [
              p.week_number,
              {
                weekIndex: p.week_number,
                title: p.title,
                solarTerm: p.solar_term,
                dateLabel: tr("atelier.letter.week_chip", {
                  week: p.week_number,
                  solarTerm: p.pet_name,
                }),
                body: p.body,
                exLibrisTitle: p.pet_name,
                exLibrisQuote: p.ex_libris_seal,
                unlocked: true,
              },
            ]),
          );
          const merged = customPets().weekly_letters.map((w) => {
            const petOverride = byWeek.get(w.week_number);
            if (petOverride) {
              byWeek.delete(w.week_number);
              return petOverride;
            }
            return {
              weekIndex: w.week_number,
              title: w.title,
              solarTerm: w.solar_term,
              dateLabel: tr("atelier.letter.week_chip", {
                week: w.week_number,
                solarTerm: w.companion_name,
              }),
              body: w.body,
              exLibrisTitle: w.ex_libris_title,
              exLibrisQuote: w.ex_libris_seal,
              unlocked: w.unlocked,
            };
          });
          for (const extra of byWeek.values()) {
            merged.push(extra);
          }
          return merged;
        })()}
        onOpenAtelier={() => {
          setShowCompanionLetterModal(false);
          setShowAtelierModal(true);
        }}
        onClose={() => setShowCompanionLetterModal(false)}
      />

      <CoSanctuarySheet
        open={getShowCoSanctuarySheet()}
        myMood={
          myMoodLocal() ??
          (customPets().co_sanctuary.self_mood_byte === 2
            ? "rest_recharge"
            : customPets().co_sanctuary.self_mood_byte === 1
              ? "breeze_steady"
              : "sunny_full")
        }
        onSelectMood={(mood) => {
          myMoodLocal.set(mood);
          coSanctuaryAction(`sync_mood_${mood}`);
        }}
        partnerName={customPets().co_sanctuary.partner_name}
        partnerMood={customPets().co_sanctuary.partner_mood_label}
        osmanthusLevel={Math.max(1, Math.floor(customPets().co_sanctuary.bonsai_dew / 5) + 1)}
        osmanthusProgressPct={
          customPets().co_sanctuary.next_stage_dew
            ? Math.min(
                100,
                Math.round(
                  (customPets().co_sanctuary.bonsai_dew /
                    Math.max(1, customPets().co_sanctuary.next_stage_dew!)) *
                    100,
                ),
              )
            : 100
        }
        careHistory={customPets()
          .dew_ledger.filter((d) => d.source === "co_sanctuary")
          .slice(0, 6)
          .map((d) => ({
            id: String(d.id),
            label: d.note,
            timeLabel: d.created_at.slice(0, 16).replace("T", " "),
          }))}
        onSendCare={(act) => {
          const mapped =
            act === "blanket" ? "tuck_blanket" : act === "paw_print" ? "paw_stamp" : "warm_tea";
          coSanctuaryAction(mapped);
        }}
        onClose={() => setShowCoSanctuarySheet(false)}
      />

      <FourSeasonsScrollModal
        open={getShowFourSeasonsScrollModal()}
        seasons={customPets().four_seasons_scroll.map((s) => ({
          season: s.season_cn,
          solarRange: s.solar_term,
          highlight: `${s.title} · ${s.summary}`,
          avgQiScore: snap().qi_score,
          dewCollected: customPets().dew_balance,
        }))}
        badges={customPets().ceramic_badges.map((b) => ({
          id: b.id,
          title: b.title,
          subtitle: b.reason,
          unlocked: b.earned,
        }))}
        onClose={() => setShowFourSeasonsScrollModal(false)}
      />

      <FamilyPeaceLetterModal
        open={getShowFamilyPeaceLetterModal()}
        recipientLabel={
          (getParentsPeaceLetterResult() ?? getState().parents_peace_letter_preview)
            .recipient_label
        }
        dateLabel={snap().target_date}
        peaceHeadline={
          (getParentsPeaceLetterResult() ?? getState().parents_peace_letter_preview).title
        }
        bodyParagraphs={[
          (getParentsPeaceLetterResult() ?? getState().parents_peace_letter_preview).greeting,
          (getParentsPeaceLetterResult() ?? getState().parents_peace_letter_preview).body,
          ...(getParentsPeaceLetterResult() ?? getState().parents_peace_letter_preview)
            .highlights,
          ...(getParentsPeaceLetterResult() ?? getState().parents_peace_letter_preview)
            .recheck_notes,
        ]}
        signatureLine={`${
          (getParentsPeaceLetterResult() ?? getState().parents_peace_letter_preview).closing
        } · ${(getParentsPeaceLetterResult() ?? getState().parents_peace_letter_preview).e2ee_note}`}
        onCopyLetter={(txt) => void copyToClipboard(txt, tr("atelier.family_letter.copy_label"))}
        onClose={() => setShowFamilyPeaceLetterModal(false)}
      />

      <DrawerSheet
        open={getShowQuickCaptureModal()}
        title={tr("log.quick_sheet_title")}
        side="bottom"
        closeOnSwipe={true}
        onClose={() => setShowQuickCaptureModal(false)}
      >
        <Card>
          <Text class="text-xs font-semibold text-muted-foreground">
            {tr("log.quick_chips_label")}
          </Text>
          <View class="flex-row flex-wrap gap-2">
            <Each
              of={[
                "log.quick_chip_coffee",
                "log.quick_chip_tea",
                "log.quick_chip_meal",
                "log.quick_chip_sleep",
              ]}
              key={(k) => k}
            >
              {(key) => (
                <SelectChip
                  label={tr(key())}
                  selected={false}
                  onPress={() => {
                    logNaturalIntake(tr(key()));
                    setShowQuickCaptureModal(false);
                  }}
                />
              )}
            </Each>
          </View>
          <Text class="text-xs leading-relaxed text-muted-foreground">
            {tr("log.quick_sheet_hint")}
          </Text>
          <FieldInput
            value={quickText()}
            placeholder={tr("log.quick_sheet_placeholder")}
            onChange={(v) => quickText.set(v)}
          />
          <ActionButton
            title={tr("log.quick_sheet_submit")}
            icon="check"
            onPress={submitGlobalQuickCapture}
          />
        </Card>
      </DrawerSheet>

      <ModalSurface
        open={getShowZdrConsentModal()}
        role="alertdialog"
        presentation="formSheet"
        onClose={() => cancelZdrConsent()}
      >
        <View class="flex-1 justify-center gap-4 px-5">
          <Card>
            <VitalBadge tone="prime_ready" label={tr("insights.zdr_badge")} />
            <Text role="header" class="text-lg font-bold text-foreground">
              {tr("ai.zdr_dialog_title")}
            </Text>
            <Text class="text-sm leading-relaxed text-muted-foreground">
              {tr("ai.zdr_dialog_body", { days: 14 })}
            </Text>
            <SunkenBox>
              <Text class="text-xs leading-relaxed text-foreground">
                {tr("ai.zdr_consent_alt")}
              </Text>
            </SunkenBox>
            <View class="flex-row gap-2">
              <ActionButton
                title={tr("ai.zdr_cancel")}
                variant="secondary"
                onPress={() => cancelZdrConsent()}
                class="flex-1"
              />
              <ActionButton
                title={tr("ai.zdr_confirm")}
                onPress={() => confirmZdrConsent()}
                class="flex-1"
              />
            </View>
          </Card>
        </View>
      </ModalSurface>
    </View>
  );
}

mount(App);
