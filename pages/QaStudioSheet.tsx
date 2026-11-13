import { Text, token, View } from "nsh";
import {
  clearEntitlementToken,
  generateEncryptedPreview,
  getEncryptedBatchPreview,
  getLastAuditReport,
  getPersonas,
  getShowQaStudio,
  getState,
  loadPersona,
  ocrSampleFixtureLab,
  probeCapabilities,
  purchaseHearthGiftPass,
  reseedDemoData,
  restoreApplePurchases,
  runDataIntegrityAudit,
  setEntitlementSandbox,
  setHardwareTier,
  setShowAtelierModal,
  setShowQaStudio,
  toggleAppLocale,
  updateAiConfig,
} from "../store";
import {
  ActionButton,
  Card,
  Hairline,
  SectionHeader,
  SelectChip,
  setDesktopViewportPreset,
  SheetDialog,
  SunkenBox,
  SwitchRow,
  VitalBadge,
} from "../ui/components";
import { QiIcon } from "../ui/icons";
import { tr } from "../ui/i18n";

export function QaStudioSheet() {
  const state = () => getState();
  const settings = () => state().settings;
  const probe = () => state().capability_probe;
  const auditReport = () => getLastAuditReport();
  const personas = () => getPersonas();

  const tierOptions = () => [
    { id: "tier_a", label: tr("qa.tier_a") },
    { id: "tier_b", label: tr("qa.tier_b") },
    { id: "tier_c", label: tr("qa.tier_c") },
  ];

  const sandboxTiers = () => [
    { id: "free", label: tr("atelier.tier_free_label") },
    { id: "pro_monthly", label: tr("atelier.tier_pro_monthly_label") },
    { id: "seasonal", label: tr("atelier.tier_seasonal_label") },
    { id: "pro_annual", label: tr("atelier.tier_pro_annual_label") },
    { id: "duo", label: tr("atelier.tier_duo_label") },
    { id: "family", label: tr("atelier.tier_family_label") },
    { id: "lifetime_local", label: tr("atelier.tier_lifetime_local_label") },
  ];

  const sandboxStates = () => [
    { id: "active", label: tr("qa.state_active") },
    { id: "offline_grace", label: tr("qa.state_grace") },
    { id: "expired_read_only_vault", label: tr("qa.state_expired") },
  ];

  return (
    <SheetDialog
      open={getShowQaStudio()}
      title={tr("qa.title")}
      onClose={() => setShowQaStudio(false)}
    >
      {/* Quick locale & seed controls */}
      <Card>
        <SectionHeader title={tr("qa.title")} subtitle={tr("qa.subtitle")} />
        <View class="flex-row flex-wrap gap-2">
          <ActionButton
            title={tr("qa.switch_locale_btn")}
            icon="globe"
            size="sm"
            variant="outline"
            onPress={() => toggleAppLocale()}
          />
          <ActionButton
            title={tr("qa.seed_sample")}
            icon="refresh"
            size="sm"
            variant="secondary"
            onPress={() => reseedDemoData()}
          />
        </View>
      </Card>

      {/* iPhone Portrait vs Desktop Wide preview switcher */}
      <Card>
        <SectionHeader title={tr("qa.viewport_title")} subtitle={tr("qa.viewport_subtitle")} />
        <View class="flex-row flex-wrap gap-2">
          <ActionButton
            title={tr("qa.viewport_ios_btn")}
            size="sm"
            variant="primary"
            shape="pill"
            onPress={() => setDesktopViewportPreset("ios_portrait")}
          />
          <ActionButton
            title={tr("qa.viewport_desktop_btn")}
            size="sm"
            variant="outline"
            shape="pill"
            onPress={() => setDesktopViewportPreset("desktop_wide")}
          />
        </View>
      </Card>

      {/* 1. Data integrity invariant audit */}
      <Card>
        <SectionHeader
          title={tr("qa.audit_title")}
          subtitle={tr("qa.audit_subtitle")}
          right={
            <If when={auditReport() !== null}>
              <VitalBadge
                tone={auditReport()!.passed ? "prime_ready" : "alert_high"}
                label={auditReport()!.passed ? tr("qa.audit_ok") : tr("qa.audit_warn")}
              />
            </If>
          }
        />

        <ActionButton
          title={tr("qa.audit_run_btn")}
          icon="check"
          onPress={() => runDataIntegrityAudit()}
        />

        <If when={auditReport() !== null}>
          <SunkenBox>
            <View class="flex-row items-center justify-between gap-2">
              <Text
                class={`text-sm font-semibold ${
                  auditReport()!.passed ? "text-vital-prime" : "text-vital-recover"
                }`}
              >
                {auditReport()!.passed ? tr("toast.audit_passed") : tr("toast.audit_failed")}
              </Text>
              <Text class="text-xs text-muted-foreground" style={{ fontVariant: ["tabular-nums"] }}>
                {auditReport()!.active_persona}
              </Text>
            </View>

            <Text class="text-xs text-muted-foreground" style={{ fontVariant: ["tabular-nums"] }}>
              {tr("qa.audit_counts", {
                metrics: auditReport()!.metric_rows,
                days: auditReport()!.distinct_days,
                intakes: auditReport()!.intake_rows,
                labs: auditReport()!.lab_rows,
                exps: auditReport()!.experiment_rows,
              })}
            </Text>

            <View class="gap-0 pt-1">
              <Each of={auditReport()!.checks} key={(chk) => chk.name}>
                {(chk, idx) => (
                  <View class="gap-0">
                    <If when={idx() > 0}>
                      <Hairline />
                    </If>
                    <View class="flex-row items-start justify-between gap-2.5 py-2">
                      <View class="flex-1 gap-0.5">
                        <Text class="text-xs font-semibold text-foreground">{chk().name}</Text>
                        <Text class="text-[11px] leading-relaxed text-muted-foreground">{chk().detail}</Text>
                      </View>
                      <VitalBadge
                        tone={chk().passed ? "prime_ready" : "alert_high"}
                        label={chk().passed ? "PASS" : "FAIL"}
                      />
                    </View>
                  </View>
                )}
              </Each>
            </View>
          </SunkenBox>
        </If>
      </Card>

      {/* 2. 6 longitudinal personas */}
      <Card>
        <SectionHeader
          title={tr("qa.personas_title")}
          subtitle={tr("qa.personas_subtitle")}
          right={<VitalBadge tone="neutral" label={`${personas().length}`} />}
        />

        <View class="gap-2.5">
          <Each of={personas()} key={(p) => p.id}>
            {(p) => (
              <SunkenBox>
                <View class="flex-row items-center justify-between gap-2">
                  <View class="flex-1 gap-0.5">
                    <Text class="text-sm font-semibold text-foreground">{p().name}</Text>
                    <Text class="text-xs text-muted-foreground">
                      {`${p().badge} · ${p().hardware_tier} · ${p().primary_goal} · ${p().days_count}d`}
                    </Text>
                  </View>
                  <If when={settings().active_persona === p().id}>
                    <VitalBadge tone="prime_ready" label={tr("common.active")} />
                  </If>
                </View>

                <Text class="text-xs leading-relaxed text-foreground">{p().summary}</Text>

                <View class="flex-row flex-wrap gap-1.5">
                  <Each of={p().highlight_metrics} key={(hm) => `${p().id}-${hm}`}>
                    {(hm) => (
                      <View class="rounded-full border border-border bg-card px-2.5 py-0.5">
                        <Text class="text-[11px] font-medium text-muted-foreground">{hm()}</Text>
                      </View>
                    )}
                  </Each>
                </View>

                <ActionButton
                  title={
                    settings().active_persona === p().id
                      ? tr("qa.persona_reload", { id: p().id })
                      : tr("qa.persona_switch", { id: p().id })
                  }
                  variant={settings().active_persona === p().id ? "secondary" : "outline"}
                  onPress={() => loadPersona(p().id)}
                />
              </SunkenBox>
            )}
          </Each>
        </View>
      </Card>

      {/* 3. Hardware tiers & edge compute boundary */}
      <Card>
        <SectionHeader
          title={tr("qa.hardware_title")}
          subtitle={tr("qa.hardware_subtitle")}
        />

        <View class="gap-2">
          <Text class="text-xs font-semibold text-foreground">
            {state().snapshot.hardware_tier_label}
          </Text>
          <View class="flex-row flex-wrap gap-2">
            <Each of={tierOptions()} key={(t) => t.id}>
              {(opt) => (
                <SelectChip
                  label={opt().label}
                  selected={settings().hardware_tier === opt().id}
                  onPress={() => setHardwareTier(opt().id, settings().morning_feeling)}
                />
              )}
            </Each>
          </View>
        </View>

        <SunkenBox>
          <Text class="text-xs font-semibold text-foreground">{tr("qa.ram_guard_title")}</Text>
          <View class="flex-row flex-wrap gap-2">
            <SelectChip
              label={tr("qa.ram_4gb")}
              selected={settings().simulated_ram_gb < 6}
              onPress={() =>
                updateAiConfig(
                  settings().ai_mode,
                  settings().zdr_consented,
                  settings().lan_ai_url,
                  settings().lan_ai_model,
                  settings().thermal_throttled,
                  4,
                )
              }
            />
            <SelectChip
              label={tr("qa.ram_8gb")}
              selected={settings().simulated_ram_gb >= 6}
              onPress={() =>
                updateAiConfig(
                  settings().ai_mode,
                  settings().zdr_consented,
                  settings().lan_ai_url,
                  settings().lan_ai_model,
                  settings().thermal_throttled,
                  8,
                )
              }
            />
          </View>
          <SwitchRow
            title={tr("qa.thermal_title")}
            subtitle={tr("qa.thermal_subtitle")}
            checked={settings().thermal_throttled}
            onChange={(checked) =>
              updateAiConfig(
                settings().ai_mode,
                settings().zdr_consented,
                settings().lan_ai_url,
                settings().lan_ai_model,
                checked,
                settings().simulated_ram_gb,
              )
            }
          />
        </SunkenBox>

        <View class="flex-row flex-wrap gap-2">
          <ActionButton
            title={tr("qa.probe_btn")}
            icon="sliders"
            variant="outline"
            onPress={() => probeCapabilities()}
            class="flex-1"
          />
          <ActionButton
            title={tr("qa.ocr_sample_btn")}
            icon="camera"
            variant="outline"
            onPress={() => ocrSampleFixtureLab()}
            class="flex-1"
          />
        </View>

        <If when={probe() !== null}>
          <SunkenBox>
            <Text class="text-xs font-semibold text-primary">{probe()!.platform}</Text>
            <Text class="text-xs text-foreground">{probe()!.summary}</Text>
            <Text class="text-xs text-muted-foreground">
              {`Apple Vision OCR: ${
                probe()!.vision_ocr_available ? `Rev.${probe()!.vision_revision}` : tr("common.disabled")
              } · FoundationModels: ${probe()!.foundation_models_status}`}
            </Text>
          </SunkenBox>
        </If>

        <ActionButton
          title={tr("qa.xchacha_btn")}
          icon="cloud_lock"
          variant="secondary"
          onPress={() => generateEncryptedPreview()}
        />

        <If when={Boolean(getEncryptedBatchPreview())}>
          <SunkenBox>
            <View class="flex-row items-center gap-2">
              <QiIcon name="cloud_lock" size={14} color={token("primary")} />
              <Text class="text-xs font-semibold text-primary">{tr("qa.xchacha_ok")}</Text>
            </View>
            <Text class="text-xs text-muted-foreground">{getEncryptedBatchPreview()!}</Text>
          </SunkenBox>
        </If>
      </Card>

      {/* 4. Atelier offline state machine & 7-tier sandbox */}
      <Card>
        <SectionHeader
          title={tr("qa.sandbox_title")}
          subtitle={tr("qa.sandbox_subtitle")}
          right={
            <VitalBadge
              tone={state().entitlements.is_atelier_unlocked ? "prime_ready" : "neutral"}
              label={state().entitlements.state}
            />
          }
        />

        <SunkenBox>
          <Text class="text-xs font-semibold text-foreground">
            {`${state().entitlements.tier_display} · ${
              state().entitlements.valid_signature ? tr("common.verified") : tr("common.free")
            }`}
          </Text>
          <Text class="text-xs text-muted-foreground">{state().entitlements.status_banner}</Text>
        </SunkenBox>

        <View class="flex-row flex-wrap gap-1.5">
          <Each of={sandboxTiers()} key={(t) => t.id}>
            {(t) => (
              <SelectChip
                label={t().label}
                selected={state().entitlements.tier === t().id}
                onPress={() => {
                  if (t().id === "free") {
                    clearEntitlementToken();
                  } else {
                    setEntitlementSandbox(
                      t().id,
                      "active",
                      settings().guardian_sponsor,
                      Math.max(32, settings().companion_loyalty_days),
                    );
                  }
                }}
              />
            )}
          </Each>
        </View>

        <View class="flex-row flex-wrap gap-1.5">
          <Each of={sandboxStates()} key={(st) => st.id}>
            {(st) => (
              <SelectChip
                label={st().label}
                selected={state().entitlements.state === st().id}
                onPress={() =>
                  setEntitlementSandbox(
                    state().entitlements.tier === "free" ? "pro_annual" : state().entitlements.tier,
                    st().id,
                    settings().guardian_sponsor,
                    settings().companion_loyalty_days,
                  )
                }
              />
            )}
          </Each>
        </View>

        <View class="flex-row flex-wrap gap-2">
          <ActionButton
            title={tr("qa.simulate_pro")}
            size="sm"
            variant="outline"
            onPress={() => setEntitlementSandbox("pro_annual", "active", settings().guardian_sponsor, 32)}
          />
          <ActionButton
            title={tr("qa.simulate_expired_vault")}
            size="sm"
            variant="outline"
            onPress={() =>
              setEntitlementSandbox(
                "pro_annual",
                "expired_read_only_vault",
                settings().guardian_sponsor,
                32,
              )
            }
          />
          <ActionButton
            title={tr("qa.simulate_restore")}
            size="sm"
            variant="outline"
            onPress={() => void restoreApplePurchases()}
          />
          <ActionButton
            title={tr("atelier.hearth_gift_single_btn")}
            icon="hearth_ember"
            size="sm"
            variant="outline"
            onPress={() => purchaseHearthGiftPass()}
          />
          <ActionButton
            title={tr("qa.reset_free")}
            size="sm"
            variant="ghost"
            onPress={() => clearEntitlementToken()}
          />
          <ActionButton
            title={tr("today.open_atelier")}
            icon="sparkles"
            size="sm"
            variant="secondary"
            onPress={() => {
              setShowQaStudio(false);
              setShowAtelierModal(true);
            }}
          />
        </View>
      </Card>
    </SheetDialog>
  );
}
