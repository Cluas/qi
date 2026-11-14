import { Speech, signal, Text, token, View } from "nsh";
import {
  addFoodSticker,
  addProfileLabMarker,
  generateEncryptedArchive,
  generateJsonlExport,
  generateParentsPeaceLetter,
  getAppLocale,
  getDisclaimerText,
  getEncryptedArchivePreview,
  getJsonlExportPreview,
  getLogSubTab,
  getState,
  importExternalHealthData,
  importLabReport,
  isHealthkitSupported,
  isSyncingHealthkit,
  type LogSubTab,
  logNaturalIntake,
  logQuickIntake,
  openPetStudio,
  pickAndOcrLabPhoto,
  requestAiAction,
  setLogSubTab,
  setShowAtelierModal,
  setShowQuickCaptureModal,
  setTab,
  syncAppleHealthkit,
} from "../store";
import { StickerCameraMatPanel, type StickerFrameStyle } from "../ui/atelier";
import {
  LabRangeBulletCanvas,
  UlnTrajectoryCanvas,
  type UlnProfileSeries,
} from "../ui/charts";
import {
  ActionButton,
  Button,
  Card,
  CollapsibleCard,
  FieldInput,
  Hairline,
  HeaderActionPill,
  IconButton,
  IosGroupedSection,
  IosLargeHeader,
  IosListRow,
  runDrawerLayoutTransition,
  SectionHeader,
  SegmentedTabs,
  SelectChip,
  SwitchRow,
  VitalBadge,
} from "../ui/components";
import { QiIcon, type QiIconName } from "../ui/icons";
import { isEnglishLocale, tr } from "../ui/i18n";

const SAMPLE_APPLE_XML = `<HealthData locale="en_US">
  <Record type="HKQuantityTypeIdentifierHeartRateVariabilitySDNN" sourceName="Apple Watch" unit="ms" startDate="2026-11-12 06:10:00 +0800" value="44.2" />
  <Record type="HKQuantityTypeIdentifierRestingHeartRate" sourceName="Apple Watch" unit="count/min" startDate="2026-11-12 06:15:00 +0800" value="59" />
  <Record type="HKQuantityTypeIdentifierStepCount" sourceName="iPhone" unit="count" startDate="2026-11-12 20:00:00 +0800" value="8420" />
</HealthData>`;

const SAMPLE_CSV_DATA = `date,hrv_rmssd_ms,resting_heart_rate_bpm,sleep_minutes,steps,source
2026-11-12,51.6,58,438,9120,csv_import`;

const SAMPLE_JSONL_DATA =
  '{"kind":"daily_metric","date":"2026-11-12","metric":"steps","source":"mi_band","value":7880,"unit":"count"}';

export function LogLabsPage() {
  const naturalInput = signal("");
  const showAllIntakes = signal(false);
  const speechListening = signal(false);
  const speechHint = signal<string | null>(null);

  const customLabDraft = signal<string | null>(null);
  const showCustomLab = signal(false);
  const expandedLabKey = signal<string | null>(null);
  const familyUlnOpen = signal(false);
  const wearableImportOpen = signal(false);

  const externalFormat = signal<"apple_health_xml" | "csv" | "jsonl">("apple_health_xml");
  const externalText = signal(SAMPLE_APPLE_XML);
  const selectedProfileId = signal<"self" | "partner" | "parents">("self");
  const profileMarkerName = signal("");
  const profileMarkerVal = signal("34");
  const profileMarkerHigh = signal("40");
  const supplementToggles = signal<Record<string, boolean>>({});

  const state = () => getState();
  const snap = () => state().snapshot;
  const solar = () => state().solar_atelier;
  const entitlements = () => state().entitlements;

  const subTabs = (): { id: LogSubTab; label: string }[] => [
    { id: "quick", label: tr("log.subtab_quick") },
    { id: "labs", label: tr("log.subtab_labs") },
    { id: "vault", label: tr("log.subtab_vault") },
  ];

  const effectiveLabText = () => customLabDraft() ?? tr("log.sample_lab_text");
  const effectiveMarkerName = () => profileMarkerName() || tr("log.default_marker_alt");

  const formatLabChange = (trend: string | null, delta: number | null, unit: string): string => {
    if (delta === null || trend === null) return "";
    const sign = delta > 0 ? "+" : "";
    const suffix =
      trend === "improved"
        ? tr("log.lab_trend_improved")
        : trend === "worsened"
          ? tr("log.lab_trend_worsened")
          : tr("log.lab_trend_stable");
    return `${sign}${delta.toFixed(2)} ${unit} · ${suffix}`;
  };

  const labFlagLabel = (flag: string): string => {
    if (flag === "high") return tr("log.flag_high");
    if (flag === "low") return tr("log.flag_low");
    return tr("log.flag_normal");
  };

  const ulnSeries = (): UlnProfileSeries[] => {
    const map = new Map<string, UlnProfileSeries>();
    for (const pt of state().multi_profile_uln_trends) {
      let bucket = map.get(pt.profile_id);
      if (!bucket) {
        bucket = { profile_id: pt.profile_id, label: pt.profile_label, points: [] };
        map.set(pt.profile_id, bucket);
      }
      bucket.points.push({
        date: pt.test_date,
        short_label: `${pt.marker_name.slice(0, 6)} ${pt.test_date.slice(5)}`,
        uln_pct: pt.uln_pct,
        raw_value: pt.value,
        unit: pt.unit,
      });
    }
    return Array.from(map.values());
  };

  const intakes = () => state().intakes;
  const visibleIntakes = () => intakes().slice(0, showAllIntakes() ? 12 : 3);
  const labs = () => state().labs;
  const lastImport = () => state().last_lab_import;

  const submitNatural = () => {
    const text = naturalInput().trim();
    if (!text) return;
    logNaturalIntake(text);
    naturalInput.set("");
  };

  const toggleVoiceDictation = async () => {
    if (speechListening()) {
      speechListening.set(false);
      return;
    }
    try {
      const ok = await Speech.available();
      if (!ok) {
        speechHint.set(tr("log.speech_unavailable"));
        return;
      }
      speechListening.set(true);
      speechHint.set(tr("log.speech_listening"));
      const sess = Speech.start({
        lang: getAppLocale() === "en" ? "en-US" : "zh-CN",
        interim: true,
      });
      sess.on("result", (res) => {
        if (res.text) naturalInput.set(res.text);
      });
      sess.on("end", (end) => {
        speechListening.set(false);
        if (end.text) {
          naturalInput.set(end.text);
          speechHint.set(tr("log.speech_done"));
        } else if (end.error) {
          speechHint.set(tr("log.speech_error", { error: end.error }));
        }
      });
    } catch {
      speechListening.set(false);
      speechHint.set(tr("log.speech_denied"));
    }
  };

  const intakeCategoryBadge = (
    category: string,
  ): { icon: QiIconName; bg: string; fg: string } => {
    switch (category) {
      case "caffeine":
        return {
          icon: "coffee",
          bg: token("atelier-soft"),
          fg: token("atelier-accent"),
        };
      case "alcohol":
        return {
          icon: "sparkles",
          bg: token("vital-recover-soft"),
          fg: token("vital-recover"),
        };
      case "late_meal":
        return {
          icon: "moon",
          bg: token("vital-recover-soft"),
          fg: token("vital-recover"),
        };
      case "supplement":
        return {
          icon: "heart_pulse",
          bg: token("vital-prime-soft"),
          fg: token("vital-prime"),
        };
      case "sleep":
        return {
          icon: "moon",
          bg: token("secondary"),
          fg: token("primary"),
        };
      default:
        return {
          icon: "utensils",
          bg: token("vital-prime-soft"),
          fg: token("vital-prime"),
        };
    }
  };

  const dailySupplements = () =>
    isEnglishLocale()
      ? [
          {
            id: "mag_glycinate",
            title: "Magnesium Glycinate 200mg",
            subtitle: "30m before sleep · deep sleep & HRV support",
            tag: "magnesium",
          },
          {
            id: "vit_d3_k2",
            title: "Vitamin D3 + K2",
            subtitle: "With morning meal · metabolic & seasonal balance",
            tag: "vitamin_d",
          },
          {
            id: "omega3",
            title: "Omega-3 Fish Oil",
            subtitle: "With dinner · recovery & vascular calm",
            tag: "omega3",
          },
        ]
      : [
          {
            id: "mag_glycinate",
            title: "甘氨酸镁 200mg",
            subtitle: "睡前 30 分钟 · 支持深睡与 HRV 恢复",
            tag: "镁剂助眠",
          },
          {
            id: "vit_d3_k2",
            title: "维生素 D3 + K2",
            subtitle: "随早餐脂溶吸收 · 骨骼与节气免疫",
            tag: "维D3",
          },
          {
            id: "omega3",
            title: "高纯度 Omega-3 鱼油",
            subtitle: "随晚餐 · 温和抗炎与心率平稳",
            tag: "Omega-3",
          },
        ];

  const isSupplementChecked = (id: string, title: string): boolean => {
    const manual = supplementToggles()[id];
    if (manual !== undefined) return manual;
    const today = snap().target_date;
    return intakes().some((it) => it.date === today && it.item_name === title);
  };

  return (
    <View class="gap-4">
      <IosLargeHeader
        title={tr("nav.log")}
        trailing={
          <HeaderActionPill
            label={tr("nav.quick_short")}
            icon="plus"
            tone="primary"
            accessibilityLabel={tr("nav.quick_capture")}
            onPress={() => setShowQuickCaptureModal(true)}
          />
        }
      />

      <SegmentedTabs
        items={subTabs()}
        activeId={getLogSubTab()}
        onSelect={(id) => setLogSubTab(id)}
      />

      {/* Subview 1: Quick Log (v7.0 Radical Subtraction: 3 calm blocks) */}
      <If when={getLogSubTab() === "quick"}>
        <View class="gap-5">
          <Card class="gap-3.5 rounded-3xl p-4">
            <SectionHeader
              title={tr("log.nlp_title")}
              subtitle={tr("log.nlp_subtitle")}
            />

            <View class="flex-row items-center gap-2">
              <View class="min-w-0 flex-1">
                <FieldInput
                  value={naturalInput()}
                  placeholder={tr("log.nlp_placeholder")}
                  onChange={(v) => naturalInput.set(v)}
                  onSubmit={submitNatural}
                />
              </View>
              <IconButton
                icon="mic"
                size="sm"
                variant="outline"
                accessibilityLabel={
                  speechListening() ? tr("log.voice_stop") : tr("log.voice_start")
                }
                onPress={() => void toggleVoiceDictation()}
              />
            </View>

            <View class="flex-row flex-wrap items-center justify-between gap-2">
              <View class="flex-1 flex-row flex-wrap gap-1.5">
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
                      onPress={() => logNaturalIntake(tr(key()))}
                    />
                  )}
                </Each>
              </View>
              <ActionButton
                title={tr("log.save_entry")}
                icon="plus"
                size="sm"
                onPress={submitNatural}
              />
            </View>

            <If when={Boolean(speechHint())}>
              <Text class="text-xs text-muted-foreground">{speechHint()!}</Text>
            </If>
          </Card>

          <IosGroupedSection
            title={tr("log.recent_entries")}
            footer={tr("common.local_only")}
          >
            <If
              when={intakes().length > 0}
              else={
                <View class="px-4 py-4">
                  <Text class="text-xs text-muted-foreground">{tr("log.recent_empty")}</Text>
                </View>
              }
            >
              <Each of={visibleIntakes()} key={(it) => it.id}>
                {(it, idx) => {
                  const badge = () => intakeCategoryBadge(it().category);
                  return (
                    <IosListRow
                      icon={badge().icon}
                      iconBgColor={badge().bg}
                      iconColor={badge().fg}
                      title={it().item_name}
                      subtitle={it().notes || undefined}
                      divider={idx() < visibleIntakes().length - 1 || intakes().length > 3}
                      trailing={
                        <Text
                          class="text-xs text-muted-foreground"
                          style={{ fontVariant: ["tabular-nums"] }}
                        >
                          {it().date.slice(5)}
                        </Text>
                      }
                    />
                  );
                }}
              </Each>
              <If when={intakes().length > 3}>
                <IosListRow
                  title={
                    showAllIntakes()
                      ? isEnglishLocale()
                        ? "Show less"
                        : "收起"
                      : isEnglishLocale()
                        ? `Show all (${intakes().length})`
                        : `展开全部 (${intakes().length})`
                  }
                  divider={false}
                  showChevron={false}
                  trailing={
                    <QiIcon
                      name={showAllIntakes() ? "chevron_down" : "chevron_right"}
                      size={16}
                      color={token("primary")}
                    />
                  }
                  onPress={() => {
                    runDrawerLayoutTransition("easeInEaseOut");
                    showAllIntakes.set(!showAllIntakes());
                  }}
                />
              </If>
            </If>
          </IosGroupedSection>

          <StickerCameraMatPanel
            stickers={solar().stickers.map((st, idx) => ({
              id: String(st.id),
              date: st.date,
              dayNumber: Number(st.date.slice(8, 10)) || idx + 1,
              title: st.title,
              frameStyle: (st.filter === "watercolor" || st.filter === "celadon"
                ? st.filter
                : "polaroid") as StickerFrameStyle,
              purineHint: st.nutrition_hint,
            }))}
            monthlyQuotaUsed={solar().sticker_quota.used_count}
            monthlyQuotaLimit={solar().sticker_quota.limit ?? 999}
            atelierUnlocked={entitlements().is_atelier_unlocked}
            onCreateSticker={(input) =>
              addFoodSticker(snap().target_date, input.title, input.frameStyle, input.purineHint ?? "")
            }
            onOpenAtelier={() => setShowAtelierModal(true)}
          />
        </View>
      </If>

      {/* Subview 2: Labs & Supplements */}
      <If when={getLogSubTab() === "labs"}>
        <View class="gap-4">
          <Card class="gap-3.5 rounded-3xl p-4">
            <SectionHeader
              title={tr("log.labs_card_title")}
              subtitle={tr("log.ocr_privacy_hint")}
              right={<VitalBadge tone="prime_ready" label={tr("log.pii_shield_badge")} />}
            />

            <View class="flex-row flex-wrap gap-2">
              <ActionButton
                title={tr("log.import_ocr_btn")}
                icon="camera"
                onPress={() => void pickAndOcrLabPhoto()}
                class="flex-1"
              />
              <ActionButton
                title={tr("log.paste_lab_btn")}
                icon="file_lab"
                variant="outline"
                onPress={() => showCustomLab.set(!showCustomLab())}
                class="flex-1"
              />
            </View>

            <View class="flex-row items-center justify-between gap-2">
              <Text class="flex-1 text-xs text-muted-foreground">{tr("log.sample_lab_prompt")}</Text>
              <ActionButton
                title={tr("log.sample_lab_btn")}
                size="sm"
                variant="secondary"
                shape="pill"
                onPress={() => requestAiAction(() => importLabReport("2026-11-13", tr("log.sample_lab_text")))}
              />
            </View>

            <If when={showCustomLab()}>
              <View class="gap-2.5 rounded-2xl bg-secondary/45 p-3.5">
                <Text class="text-xs font-semibold text-foreground">{tr("log.paste_lab_title")}</Text>
                <FieldInput
                  value={effectiveLabText()}
                  multiline={true}
                  placeholder={tr("log.paste_lab_placeholder")}
                  onChange={(v) => customLabDraft.set(v)}
                />
                <ActionButton
                  title={tr("log.paste_lab_submit")}
                  icon="check"
                  onPress={() => {
                    requestAiAction(() => {
                      importLabReport("2026-11-13", effectiveLabText());
                      showCustomLab.set(false);
                    });
                  }}
                />
              </View>
            </If>

            <If when={Boolean(lastImport())}>
              <View class="gap-2 rounded-2xl bg-vital-prime-soft p-3.5">
                <View class="flex-row items-center gap-2">
                  <QiIcon name="user_shield" size={16} color={token("vital-prime")} />
                  <Text class="flex-1 text-xs font-semibold text-vital-prime">
                    {tr("log.pii_redacted_summary", {
                      fields: lastImport()!.redacted_fields.join(", ") || tr("common.no_data"),
                    })}
                  </Text>
                </View>
                <Text class="text-xs leading-relaxed text-foreground">{lastImport()!.scrubbed_preview}</Text>
              </View>
            </If>
          </Card>

          <IosGroupedSection
            title={tr("log.labs_archived_title")}
            right={
              <Text
                class="text-[11px] font-medium text-muted-foreground"
                style={{ fontVariant: ["tabular-nums"] }}
              >
                {tr("log.labs_archived_count", { count: labs().length })}
              </Text>
            }
            footer={getDisclaimerText()}
          >
            <If
              when={labs().length > 0}
              else={
                <View class="px-4 py-4">
                  <Text class="text-xs text-muted-foreground">{tr("log.labs_archived_empty")}</Text>
                </View>
              }
            >
              <Each of={labs()} key={(lb) => `${lb.id}-${lb.test_date}-${lb.canonical_key}`}>
                {(lb, idx) => {
                  const rowKey = () => `${lb().id}-${lb().test_date}-${lb().canonical_key}`;
                  const isExpanded = () => expandedLabKey() === rowKey();
                  return (
                    <View>
                      <Button
                        onPress={() => {
                          runDrawerLayoutTransition("easeInEaseOut");
                          expandedLabKey.set(isExpanded() ? null : rowKey());
                        }}
                        class="w-full px-4 py-3.5 text-left"
                        accessibilityLabel={lb().marker_name}
                      >
                        <View class="w-full gap-2">
                          <View class="flex-row items-center justify-between gap-2">
                            <View class="min-w-0 flex-1 gap-0.5">
                              <Text class="text-[15px] font-semibold text-foreground">
                                {lb().marker_name}
                              </Text>
                              <Text
                                class="text-xs text-muted-foreground"
                                style={{ fontVariant: ["tabular-nums"] }}
                              >
                                {tr("log.lab_date_row", { date: lb().test_date })}
                              </Text>
                            </View>
                            <View class="flex-row items-center gap-2">
                              <VitalBadge tone={lb().flag} label={labFlagLabel(lb().flag)} />
                              <QiIcon
                                name={isExpanded() ? "chevron_down" : "chevron_right"}
                                size={15}
                                color={token("muted-foreground")}
                              />
                            </View>
                          </View>

                          <View class="flex-row items-baseline justify-between gap-2">
                            <Text
                              class="text-xl font-bold text-foreground"
                              style={{ fontVariant: ["tabular-nums"] }}
                            >
                              {`${lb().value} ${lb().unit}`}
                            </Text>
                            <Text
                              class="text-xs text-muted-foreground"
                              style={{ fontVariant: ["tabular-nums"] }}
                            >
                              {lb().ref_low !== null && lb().ref_high !== null
                                ? tr("log.lab_ref_bounds", {
                                    low: lb().ref_low ?? "--",
                                    high: lb().ref_high ?? "--",
                                  })
                                : tr("log.lab_no_ref")}
                            </Text>
                          </View>

                          <LabRangeBulletCanvas
                            value={lb().value}
                            refLow={lb().ref_low}
                            refHigh={lb().ref_high}
                            flag={lb().flag}
                            prevValue={lb().prev_value}
                          />

                          <If when={isExpanded()}>
                            <View class="gap-1.5 pt-1">
                              <If when={lb().prev_value !== null}>
                                <View class="flex-row flex-wrap items-center gap-2">
                                  <View class="rounded-full bg-secondary/55 px-2.5 py-1">
                                    <Text
                                      class="text-[11px] font-medium text-primary"
                                      style={{ fontVariant: ["tabular-nums"] }}
                                    >
                                      {tr("log.lab_prev_line", {
                                        date: lb().prev_date ?? "—",
                                        val: lb().prev_value ?? 0,
                                        unit: lb().unit,
                                        change: formatLabChange(lb().trend, lb().delta, lb().unit),
                                      })}
                                    </Text>
                                  </View>
                                </View>
                              </If>
                              <Text class="text-xs leading-relaxed text-muted-foreground">
                                {lb().plain_note}
                              </Text>
                            </View>
                          </If>
                        </View>
                      </Button>
                      <If when={idx() < labs().length - 1}>
                        <Hairline class="ml-4" />
                      </If>
                    </View>
                  );
                }}
              </Each>
            </If>
          </IosGroupedSection>

          <IosGroupedSection
            title={isEnglishLocale() ? "Daily supplements" : "每日补剂打卡"}
            footer={
              isEnglishLocale()
                ? "Toggling records today's supplement adherence in your local log."
                : "开启即记录今日补剂服用，参与节律与恢复关联统计。"
            }
          >
            <Each of={dailySupplements()} key={(sup) => sup.id}>
              {(sup, idx) => (
                <View>
                  <View class="px-4 py-1">
                    <SwitchRow
                      title={sup().title}
                      subtitle={sup().subtitle}
                      checked={isSupplementChecked(sup().id, sup().title)}
                      onChange={(next) => {
                        supplementToggles.set({
                          ...supplementToggles(),
                          [sup().id]: next,
                        });
                        if (next) {
                          logQuickIntake(
                            "supplement",
                            sup().title,
                            sup().tag,
                            sup().subtitle,
                            0,
                          );
                        }
                      }}
                    />
                  </View>
                  <If when={idx() < dailySupplements().length - 1}>
                    <Hairline class="ml-4" />
                  </If>
                </View>
              )}
            </Each>
          </IosGroupedSection>

          <CollapsibleCard
            title={tr("log.family_uln_title")}
            subtitle={tr("log.family_uln_subtitle")}
            open={familyUlnOpen()}
            onToggle={() => familyUlnOpen.set(!familyUlnOpen())}
          >
            <View class="flex-row items-center justify-between gap-2">
              <View class="flex-row flex-wrap gap-1.5">
                <Each
                  of={[
                    { id: "self", label: tr("log.profile_self") },
                    { id: "partner", label: tr("log.profile_partner") },
                    { id: "parents", label: tr("log.profile_parents_default") },
                  ]}
                  key={(p) => p.id}
                >
                  {(p) => (
                    <SelectChip
                      label={p().label}
                      selected={selectedProfileId() === p().id}
                      onPress={() => selectedProfileId.set(p().id as "self" | "partner" | "parents")}
                    />
                  )}
                </Each>
              </View>
              <ActionButton
                title={tr("log.family_letter_btn")}
                icon="family_heart"
                size="sm"
                variant="outline"
                onPress={() => generateParentsPeaceLetter()}
              />
            </View>

            <View class="flex-row flex-wrap items-center gap-2">
              <View class="min-w-36 flex-1">
                <FieldInput
                  value={effectiveMarkerName()}
                  placeholder={tr("log.marker_name_placeholder")}
                  onChange={(v) => profileMarkerName.set(v)}
                />
              </View>
              <View class="w-24">
                <FieldInput
                  value={profileMarkerVal()}
                  placeholder={tr("log.lab_value")}
                  onChange={(v) => profileMarkerVal.set(v)}
                />
              </View>
              <View class="w-24">
                <FieldInput
                  value={profileMarkerHigh()}
                  placeholder={tr("log.uln_upper_placeholder")}
                  onChange={(v) => profileMarkerHigh.set(v)}
                />
              </View>
              <ActionButton
                title={tr("log.add_lab_btn")}
                icon="plus"
                size="sm"
                onPress={() => {
                  const v = Number(profileMarkerVal());
                  const h = Number(profileMarkerHigh());
                  if (Number.isFinite(v)) {
                    addProfileLabMarker(
                      selectedProfileId(),
                      snap().target_date,
                      effectiveMarkerName(),
                      v,
                      "U/L",
                      null,
                      Number.isFinite(h) && h > 0 ? h : null,
                    );
                  }
                }}
              />
            </View>

            <If when={ulnSeries().length > 0}>
              <UlnTrajectoryCanvas
                series={ulnSeries()}
                markerName={tr("log.uln_chart_title")}
              />
            </If>

            <If when={state().recheck_reminders.length > 0}>
              <View class="gap-2 pt-1">
                <Text class="text-xs font-semibold text-foreground">{tr("log.recheck_title")}</Text>
                <Each of={state().recheck_reminders.slice(0, 4)} key={(r) => `${r.profile_id}-${r.canonical_key}`}>
                  {(r) => (
                    <View class="rounded-2xl bg-secondary/45 px-3.5 py-2.5">
                      <Text class="text-xs leading-relaxed text-foreground">
                        {tr("log.recheck_row", {
                          profile: r().profile_label,
                          marker: r().marker_name,
                          days: r().days_since_last,
                          text: r().reminder_text,
                        })}
                      </Text>
                    </View>
                  )}
                </Each>
              </View>
            </If>
          </CollapsibleCard>
        </View>
      </If>

      {/* Subview 3: Hearth Vault & Wearable Sources */}
      <If when={getLogSubTab() === "vault"}>
        <View class="gap-4">
          <IosGroupedSection
            title={tr("log.hearth_summary_title")}
            footer={tr("log.hearth_summary_sub", {
              intakes: intakes().length,
              labs: labs().length,
              stickers: solar().stickers.length,
            })}
          >
            <IosListRow
              icon="utensils"
              iconBgColor={token("vital-prime-soft")}
              iconColor={token("vital-prime")}
              title={isEnglishLocale() ? "Daily log entries" : "食光与日常随记"}
              divider={true}
              trailing={
                <Text
                  class="text-sm font-semibold text-foreground"
                  style={{ fontVariant: ["tabular-nums"] }}
                >
                  {String(intakes().length)}
                </Text>
              }
            />
            <IosListRow
              icon="file_lab"
              iconBgColor={token("vital-steady-soft")}
              iconColor={token("vital-steady")}
              title={isEnglishLocale() ? "Archived lab biomarkers" : "已归档化验指标"}
              divider={true}
              trailing={
                <Text
                  class="text-sm font-semibold text-foreground"
                  style={{ fontVariant: ["tabular-nums"] }}
                >
                  {String(labs().length)}
                </Text>
              }
            />
            <IosListRow
              icon="camera"
              iconBgColor={token("atelier-soft")}
              iconColor={token("atelier-accent")}
              title={isEnglishLocale() ? "Food stickers & mats" : "食光贴纸月历收藏"}
              divider={true}
              trailing={
                <Text
                  class="text-sm font-semibold text-foreground"
                  style={{ fontVariant: ["tabular-nums"] }}
                >
                  {String(solar().stickers.length)}
                </Text>
              }
            />
            <IosListRow
              icon="custom_pet"
              iconBgColor={token("secondary")}
              iconColor={token("primary")}
              title={tr("today.open_custom_pet")}
              showChevron={true}
              divider={false}
              onPress={() => openPetStudio()}
            />
          </IosGroupedSection>

          <Card class="gap-3.5 rounded-3xl p-4">
            <SectionHeader
              title={tr("log.healthkit_card_title")}
              subtitle={tr("log.healthkit_card_sub")}
              right={
                <VitalBadge
                  tone={state().settings.healthkit_authorized ? "prime_ready" : "neutral"}
                  label={
                    state().settings.healthkit_authorized
                      ? tr("me.healthkit_badge_connected")
                      : tr("me.healthkit_badge_local")
                  }
                />
              }
            />

            <If when={isHealthkitSupported()}>
              <View class="gap-2.5">
                <View class="flex-row flex-wrap gap-2">
                  <ActionButton
                    title={
                      isSyncingHealthkit()
                        ? tr("me.healthkit_syncing_btn")
                        : state().settings.healthkit_authorized
                          ? tr("me.healthkit_sync_35d_btn")
                          : tr("me.healthkit_connect_btn")
                    }
                    icon="heart_pulse"
                    disabled={isSyncingHealthkit()}
                    onPress={() => void syncAppleHealthkit(35, false)}
                    class="flex-1"
                  />
                  <ActionButton
                    title={tr("me.healthkit_sync_90d_btn")}
                    icon="refresh"
                    variant="outline"
                    disabled={isSyncingHealthkit()}
                    onPress={() => void syncAppleHealthkit(90, false)}
                  />
                </View>

                <If when={Boolean(state().settings.healthkit_last_summary)}>
                  <View class="rounded-2xl bg-secondary/45 px-3.5 py-2.5">
                    <Text class="text-xs leading-relaxed text-foreground">
                      {state().settings.healthkit_last_summary}
                    </Text>
                  </View>
                </If>
              </View>
            </If>

            <If when={!isHealthkitSupported()}>
              <View class="rounded-2xl bg-secondary/45 px-3.5 py-2.5">
                <Text class="text-xs leading-relaxed text-muted-foreground">
                  {tr("me.healthkit_desktop_hint")}
                </Text>
              </View>
            </If>
          </Card>

          <IosGroupedSection
            title={tr("me.vault_section")}
            footer={
              getJsonlExportPreview() || getEncryptedArchivePreview()
                ? isEnglishLocale()
                  ? "Export payload ready — view or copy full archive in Me › Privacy & Vault."
                  : "已生成导出快照，可前往「我的 › 隐私与金库」查看或复制完整归档。"
                : undefined
            }
          >
            <IosListRow
              icon="file_lab"
              iconBgColor={token("vital-prime-soft")}
              iconColor={token("vital-prime")}
              title={tr("me.export_jsonl_btn")}
              subtitle={
                getJsonlExportPreview()
                  ? isEnglishLocale()
                    ? "JSONL export generated"
                    : "已生成明文 JSONL 快照"
                  : isEnglishLocale()
                    ? "Plain human-readable log & biomarker timeline"
                    : "可自由迁移的明文随记与指标时间线"
              }
              showChevron={true}
              divider={true}
              onPress={() => generateJsonlExport()}
            />
            <IosListRow
              icon="cloud_lock"
              iconBgColor={token("secondary")}
              iconColor={token("primary")}
              title={tr("me.export_backup")}
              subtitle={
                getEncryptedArchivePreview()
                  ? isEnglishLocale()
                    ? "Encrypted archive ready"
                    : "已生成加密金库归档"
                  : isEnglishLocale()
                    ? "Local encrypted archive for backup & restore"
                    : "端侧加密归档，用于备份与跨机迁移"
              }
              showChevron={true}
              divider={true}
              onPress={() => generateEncryptedArchive()}
            />
            <IosListRow
              icon="user_shield"
              iconBgColor={token("secondary")}
              iconColor={token("muted-foreground")}
              title={tr("me.vault_section")}
              subtitle={isEnglishLocale() ? "Open full security & sync settings" : "前往完整隐私与同步设置"}
              showChevron={true}
              divider={false}
              onPress={() => setTab("me")}
            />
          </IosGroupedSection>

          <CollapsibleCard
            title={tr("log.wearable_title")}
            subtitle={tr("log.wearable_subtitle")}
            badgeLabel={tr("log.wearable_badge")}
            badgeTone="neutral"
            open={wearableImportOpen()}
            onToggle={() => wearableImportOpen.set(!wearableImportOpen())}
          >
            <View class="flex-row flex-wrap gap-2">
              <SelectChip
                label="Apple Health XML"
                selected={externalFormat() === "apple_health_xml"}
                onPress={() => {
                  externalFormat.set("apple_health_xml");
                  externalText.set(SAMPLE_APPLE_XML);
                }}
              />
              <SelectChip
                label={tr("log.format_csv")}
                selected={externalFormat() === "csv"}
                onPress={() => {
                  externalFormat.set("csv");
                  externalText.set(SAMPLE_CSV_DATA);
                }}
              />
              <SelectChip
                label="OpenHealthSync JSONL"
                selected={externalFormat() === "jsonl"}
                onPress={() => {
                  externalFormat.set("jsonl");
                  externalText.set(SAMPLE_JSONL_DATA);
                }}
              />
            </View>

            <FieldInput
              value={externalText()}
              multiline={true}
              placeholder={tr("log.wearable_placeholder")}
              onChange={(v) => externalText.set(v)}
            />

            <ActionButton
              title={tr("log.wearable_import_btn")}
              icon="refresh"
              onPress={() => importExternalHealthData(externalFormat(), externalText())}
            />

            <View class="rounded-2xl bg-secondary/45 px-3.5 py-2.5">
              <Text class="text-xs leading-relaxed text-muted-foreground">
                {tr("log.wearable_merge_note")}
              </Text>
            </View>
          </CollapsibleCard>
        </View>
      </If>
    </View>
  );
}
