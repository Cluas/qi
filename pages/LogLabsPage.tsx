import { Speech, signal, Text, token, View } from "nsh";
import {
  addFoodSticker,
  addProfileLabMarker,
  generateParentsPeaceLetter,
  getAppLocale,
  getDisclaimerText,
  getLogSubTab,
  getState,
  importExternalHealthData,
  importLabReport,
  type LogSubTab,
  logNaturalIntake,
  logQuickIntake,
  openPetStudio,
  pickAndOcrLabPhoto,
  requestAiAction,
  searchFoods,
  setLogSubTab,
  setShowAtelierModal,
  setTab,
} from "../store";
import { StickerCameraMatPanel, type StickerFrameStyle } from "../ui/atelier";
import {
  LabRangeBulletCanvas,
  UlnTrajectoryCanvas,
  type UlnProfileSeries,
} from "../ui/charts";
import {
  ActionButton,
  Card,
  FieldInput,
  Hairline,
  SectionHeader,
  SegmentedTabs,
  SelectChip,
  SunkenBox,
  VitalBadge,
} from "../ui/components";
import { QiIcon, type QiIconName } from "../ui/icons";
import { tr } from "../ui/i18n";
import { StickerMealTile } from "../ui/sanctuary";

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
  const foodSearchQuery = signal("");
  const speechListening = signal(false);
  const speechHint = signal<string | null>(null);

  const customLabDraft = signal<string | null>(null);
  const showCustomLab = signal(false);

  const externalFormat = signal<"apple_health_xml" | "csv" | "jsonl">("apple_health_xml");
  const externalText = signal(SAMPLE_APPLE_XML);
  const selectedProfileId = signal<"self" | "partner" | "parents">("self");
  const profileMarkerName = signal("");
  const profileMarkerVal = signal("34");
  const profileMarkerHigh = signal("40");

  const state = () => getState();
  const snap = () => state().snapshot;
  const solar = () => state().solar_atelier;
  const entitlements = () => state().entitlements;

  const subTabs = (): { id: LogSubTab; label: string }[] => [
    { id: "quick", label: tr("log.subtab_quick") },
    { id: "labs", label: tr("log.subtab_labs") },
    { id: "vault", label: tr("log.subtab_vault") },
  ];

  const quickIntakes = (): {
    category: string;
    item: string;
    tag: string;
    notes: string;
    kcal: number;
    icon: QiIconName;
    caption: string;
  }[] => [
    {
      category: "caffeine",
      item: tr("log.quick_coffee_item"),
      tag: tr("log.quick_coffee_tag"),
      notes: tr("log.quick_coffee_note"),
      kcal: 15,
      icon: "coffee",
      caption: tr("log.quick_coffee_caption"),
    },
    {
      category: "alcohol",
      item: tr("log.quick_drink_item"),
      tag: tr("log.quick_drink_tag"),
      notes: tr("log.quick_drink_note"),
      kcal: 190,
      icon: "sparkles",
      caption: tr("log.quick_drink_caption"),
    },
    {
      category: "late_meal",
      item: tr("log.quick_late_item"),
      tag: tr("log.quick_late_tag"),
      notes: tr("log.quick_late_note"),
      kcal: 620,
      icon: "moon",
      caption: tr("log.quick_late_caption"),
    },
    {
      category: "meal",
      item: tr("log.quick_meal_item"),
      tag: tr("log.quick_meal_tag"),
      notes: tr("log.quick_meal_note"),
      kcal: 430,
      icon: "utensils",
      caption: tr("log.quick_meal_caption"),
    },
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
  const labs = () => state().labs;
  const lastImport = () => state().last_lab_import;
  const foodHits = () => {
    const q = foodSearchQuery().trim();
    return q ? searchFoods(q, 8) : searchFoods("", 6);
  };

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

  return (
    <View class="gap-4">
      <SegmentedTabs
        items={subTabs()}
        activeId={getLogSubTab()}
        onSelect={(id) => setLogSubTab(id)}
      />

      {/* Subview 1: Quick Log */}
      <If when={getLogSubTab() === "quick"}>
        <View class="gap-4">
          <Card>
            <SectionHeader
              title={tr("log.nlp_title")}
              subtitle={tr("log.nlp_subtitle")}
            />

            <View class="flex-row flex-wrap items-center gap-2">
              <View class="min-w-48 flex-1">
                <FieldInput
                  value={naturalInput()}
                  placeholder={tr("log.nlp_placeholder")}
                  onChange={(v) => naturalInput.set(v)}
                />
              </View>
              <ActionButton title={tr("log.save_entry")} icon="plus" onPress={submitNatural} />
              <ActionButton
                title={speechListening() ? tr("log.voice_stop") : tr("log.voice_start")}
                icon="mic"
                variant="outline"
                onPress={() => void toggleVoiceDictation()}
              />
            </View>

            <If when={Boolean(speechHint())}>
              <Text class="text-xs text-muted-foreground">{speechHint()!}</Text>
            </If>

            <View class="gap-2 pt-1">
              <View class="flex-row items-center justify-between gap-2">
                <Text class="text-xs font-semibold text-foreground">{tr("log.sticker_tiles_title")}</Text>
                <Text class="text-[11px] font-medium text-dew-accent">{tr("log.sticker_tiles_dew")}</Text>
              </View>
              <View class="flex-row flex-wrap gap-2">
                <Each of={quickIntakes()} key={(q) => q.category}>
                  {(q) => (
                    <StickerMealTile
                      title={q().item}
                      caption={q().caption}
                      icon={q().icon}
                      onPress={() =>
                        logQuickIntake(q().category, q().item, q().tag, q().notes, q().kcal)
                      }
                    />
                  )}
                </Each>
              </View>
            </View>

            <SunkenBox>
              <Text class="text-xs font-semibold text-foreground">{tr("log.food_dict_title")}</Text>
              <FieldInput
                value={foodSearchQuery()}
                placeholder={tr("log.food_dict_placeholder")}
                onChange={(v) => foodSearchQuery.set(v)}
              />
              <View class="flex-row flex-wrap gap-2">
                <Each of={foodHits()} key={(hit) => hit.id}>
                  {(hit) => (
                    <SelectChip
                      label={`${hit().name} · ${hit().kcal}kcal`}
                      selected={false}
                      onPress={() =>
                        logQuickIntake(hit().category, hit().name, hit().rhythm_tag, hit().tip, hit().kcal)
                      }
                    />
                  )}
                </Each>
              </View>
            </SunkenBox>
          </Card>

          <If when={snap().lagged_impacts.length > 0}>
            <Card>
              <SectionHeader
                title={tr("log.lag_feedback_title")}
                subtitle={tr("log.lag_feedback_sub")}
              />
              <View class="gap-2">
                <Each of={snap().lagged_impacts} key={(imp) => `${imp.tag}-${imp.outcome_label}`}>
                  {(imp) => (
                    <SunkenBox>
                      <View class="flex-row items-center justify-between gap-2">
                        <Text class="text-xs font-semibold text-primary">
                          {`${imp().tag} → ${imp().outcome_label}`}
                        </Text>
                        <VitalBadge
                          tone={imp().delta_value >= 0 ? "optimal" : "watch_low"}
                          label={`${imp().delta_value >= 0 ? "+" : ""}${imp().delta_value.toFixed(1)} ${imp().unit}`}
                        />
                      </View>
                      <Text class="text-xs leading-relaxed text-foreground">{imp().summary}</Text>
                    </SunkenBox>
                  )}
                </Each>
              </View>
            </Card>
          </If>

          <Card>
            <SectionHeader title={tr("log.recent_entries")} subtitle={tr("common.local_only")} />
            <If
              when={intakes().length > 0}
              else={
                <SunkenBox>
                  <Text class="text-xs text-muted-foreground">{tr("log.recent_empty")}</Text>
                </SunkenBox>
              }
            >
              <View class="gap-0">
                <Each of={intakes().slice(0, 8)} key={(it) => it.id}>
                  {(it) => (
                    <View class="gap-0">
                      <View class="flex-row items-center justify-between gap-2 py-2.5">
                        <Text class="flex-1 text-sm font-medium text-foreground">{`${it().date} · ${it().item_name}`}</Text>
                        <Text
                          class="text-xs text-muted-foreground"
                          style={{ fontVariant: ["tabular-nums"] }}
                        >
                          {it().calories_kcal !== null
                            ? `${Math.round(it().calories_kcal!)} kcal · ${it().tags[0] ?? it().category}`
                            : `${it().tags[0] ?? it().category}`}
                        </Text>
                      </View>
                      <Hairline />
                    </View>
                  )}
                </Each>
              </View>
            </If>
          </Card>

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
          <Card>
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
              <SunkenBox>
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
              </SunkenBox>
            </If>

            <If when={Boolean(lastImport())}>
              <View class="gap-2 rounded-xl border border-vital-prime/30 bg-vital-prime-soft p-3.5">
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

          <Card>
            <SectionHeader
              title={tr("log.labs_archived_title")}
              subtitle={tr("log.labs_archived_count", { count: labs().length })}
            />
            <If
              when={labs().length > 0}
              else={
                <SunkenBox>
                  <Text class="text-xs text-muted-foreground">{tr("log.labs_archived_empty")}</Text>
                </SunkenBox>
              }
            >
              <View class="gap-3">
                <Each of={labs()} key={(lb) => `${lb.id}-${lb.test_date}-${lb.canonical_key}`}>
                  {(lb) => (
                    <SunkenBox>
                      <View class="flex-row items-center justify-between gap-2">
                        <View class="flex-1 gap-0.5">
                          <Text class="text-sm font-semibold text-foreground">{lb().marker_name}</Text>
                          <Text class="text-xs text-muted-foreground">
                            {tr("log.lab_date_row", { date: lb().test_date })}
                          </Text>
                        </View>
                        <VitalBadge tone={lb().flag} label={labFlagLabel(lb().flag)} />
                      </View>

                      <View class="flex-row items-baseline justify-between gap-2">
                        <Text
                          class="text-lg font-bold text-foreground"
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

                      <If when={lb().prev_value !== null}>
                        <View class="flex-row flex-wrap items-center gap-2">
                          <View class="rounded-full border border-border bg-card px-2.5 py-1">
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

                      <Text class="text-xs leading-relaxed text-muted-foreground">{lb().plain_note}</Text>
                    </SunkenBox>
                  )}
                </Each>
              </View>
            </If>

            <Text class="text-[11px] leading-relaxed text-muted-foreground">{getDisclaimerText()}</Text>
          </Card>

          <Card>
            <SectionHeader
              title={tr("log.family_uln_title")}
              subtitle={tr("log.family_uln_subtitle")}
              right={
                <ActionButton
                  title={tr("log.family_letter_btn")}
                  icon="family_heart"
                  size="sm"
                  onPress={() => generateParentsPeaceLetter()}
                />
              }
            />

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
              <View class="gap-1.5 pt-1">
                <Text class="text-xs font-semibold text-foreground">{tr("log.recheck_title")}</Text>
                <Each of={state().recheck_reminders.slice(0, 4)} key={(r) => `${r.profile_id}-${r.canonical_key}`}>
                  {(r) => (
                    <SunkenBox>
                      <Text class="text-xs leading-relaxed text-foreground">
                        {tr("log.recheck_row", {
                          profile: r().profile_label,
                          marker: r().marker_name,
                          days: r().days_since_last,
                          text: r().reminder_text,
                        })}
                      </Text>
                    </SunkenBox>
                  )}
                </Each>
              </View>
            </If>
          </Card>
        </View>
      </If>

      {/* Subview 3: Hearth Vault & Wearable Sources */}
      <If when={getLogSubTab() === "vault"}>
        <View class="gap-4">
          <Card>
            <SectionHeader
              title={tr("log.wearable_title")}
              subtitle={tr("log.wearable_subtitle")}
              right={<VitalBadge tone="neutral" label={tr("log.wearable_badge")} />}
            />

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

            <SunkenBox>
              <Text class="text-xs leading-relaxed text-muted-foreground">
                {tr("log.wearable_merge_note")}
              </Text>
            </SunkenBox>
          </Card>

          <Card>
            <SectionHeader
              title={tr("log.hearth_summary_title")}
              subtitle={tr("log.hearth_summary_sub", {
                intakes: intakes().length,
                labs: labs().length,
                stickers: solar().stickers.length,
              })}
            />
            <View class="flex-row flex-wrap gap-2">
              <ActionButton
                title={tr("today.open_custom_pet")}
                icon="custom_pet"
                variant="outline"
                size="sm"
                onPress={() => openPetStudio()}
              />
              <ActionButton
                title={tr("me.vault_section")}
                icon="cloud_lock"
                variant="secondary"
                size="sm"
                onPress={() => setTab("me")}
              />
            </View>
          </Card>
        </View>
      </If>
    </View>
  );
}
