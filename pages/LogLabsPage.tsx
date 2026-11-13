import { Speech, signal, Text, token, View } from "nsh";
import { Button } from "nsh/base";
import {
  getDisclaimerText,
  getState,
  importExternalHealthData,
  importLabReport,
  logNaturalIntake,
  logQuickIntake,
  pickAndOcrLabPhoto,
  requestAiAction,
  searchFoods,
} from "../store";
import { LabRangeBulletCanvas } from "../ui/charts";
import {
  ActionButton,
  Card,
  FieldInput,
  SectionHeader,
  SegmentedTabs,
  SelectChip,
  SunkenBox,
  VitalBadge,
} from "../ui/components";
import { QiIcon, type QiIconName } from "../ui/icons";

type LogSubTab = "diet" | "labs" | "wearable";

const SUB_TABS: { id: LogSubTab; label: string }[] = [
  { id: "diet", label: "饮食随记" },
  { id: "labs", label: "体检档案" },
  { id: "wearable", label: "穿戴导入" },
];

const QUICK_INTAKES: {
  category: string;
  item: string;
  tag: string;
  notes: string;
  kcal: number;
  icon: QiIconName;
  caption: string;
}[] = [
  {
    category: "caffeine",
    item: "15:00后冰美式",
    tag: "晚间咖啡",
    notes: "下午提神咖啡因",
    kcal: 15,
    icon: "coffee",
    caption: "午后提神 · 约 15 kcal",
  },
  {
    category: "alcohol",
    item: "晚间精酿小酌",
    tag: "晚间饮酒",
    notes: "聚餐小酌一杯",
    kcal: 190,
    icon: "sparkles",
    caption: "夜间放松 · 约 190 kcal",
  },
  {
    category: "late_meal",
    item: "22:30夜宵烧烤",
    tag: "夜间重餐",
    notes: "睡前高油盐加餐",
    kcal: 620,
    icon: "moon",
    caption: "深夜加餐 · 约 620 kcal",
  },
  {
    category: "meal",
    item: "均衡减脂正餐",
    tag: "高蛋白轻食",
    notes: "鸡胸糙米时蔬",
    kcal: 430,
    icon: "utensils",
    caption: "轻盈正餐 · 约 430 kcal",
  },
];

const SAMPLE_LAB_OCR = `某某市第一人民医院体检中心 检验报告单
姓名：陈默 身份证号：31010419920518432X 手机：13912345678 住院号：ZY20261108
尿酸(UA) 468↑ umol/L 208-428
低密度脂蛋白胆固醇(LDL-C) 3.62↑ mmol/L 0.00-3.40
空腹血糖(GLU) 5.08 mmol/L 3.90-6.10`;

const SAMPLE_APPLE_XML = `<HealthData locale="zh_CN">
  <Record type="HKQuantityTypeIdentifierHeartRateVariabilitySDNN" sourceName="Apple Watch" unit="ms" startDate="2026-11-12 06:10:00 +0800" value="44.2" />
  <Record type="HKQuantityTypeIdentifierRestingHeartRate" sourceName="Apple Watch" unit="count/min" startDate="2026-11-12 06:15:00 +0800" value="59" />
  <Record type="HKQuantityTypeIdentifierStepCount" sourceName="iPhone" unit="count" startDate="2026-11-12 20:00:00 +0800" value="8420" />
</HealthData>`;

const SAMPLE_CSV_DATA = `date,hrv_rmssd_ms,resting_heart_rate_bpm,sleep_minutes,steps,source
2026-11-12,51.6,58,438,9120,csv_import`;

function formatLabChange(trend: string | null, delta: number | null, unit: string): string {
  if (delta === null || trend === null) return "";
  const sign = delta > 0 ? "+" : "";
  const suffix =
    trend === "improved"
      ? " · 趋势向好"
      : trend === "worsened"
        ? " · 建议留意"
        : " · 保持平稳";
  return `${sign}${delta.toFixed(2)} ${unit}${suffix}`;
}

function labFlagLabel(flag: string): string {
  if (flag === "high") return "偏高提醒";
  if (flag === "low") return "偏低提醒";
  return "参考区间内";
}

export function LogLabsPage() {
  const activeSubTab = signal<LogSubTab>("diet");
  const naturalInput = signal("");
  const foodSearchQuery = signal("");
  const speechListening = signal(false);
  const speechHint = signal<string | null>(null);

  const customLabText = signal(SAMPLE_LAB_OCR);
  const showCustomLab = signal(false);

  const externalFormat = signal<"apple_health_xml" | "csv" | "jsonl">("apple_health_xml");
  const externalText = signal(SAMPLE_APPLE_XML);

  const state = () => getState();
  const snap = () => state().snapshot;
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
        speechHint.set("当前设备暂不支持系统语音输入，可直接键入一句话速记");
        return;
      }
      speechListening.set(true);
      speechHint.set("正在聆听你的饮食描述…");
      const sess = Speech.start({ lang: "zh-CN", interim: true });
      sess.on("result", (res) => {
        if (res.text) naturalInput.set(res.text);
      });
      sess.on("end", (end) => {
        speechListening.set(false);
        if (end.text) {
          naturalInput.set(end.text);
          speechHint.set("语音听写完成，点「记入今日饮食」即可入库");
        } else if (end.error) {
          speechHint.set(`语音识别结束（${end.error}），可继续手动编辑`);
        }
      });
    } catch {
      speechListening.set(false);
      speechHint.set("麦克风权限未授权，可直接键入一句话饮食描述");
    }
  };

  return (
    <View class="gap-4">
      <SegmentedTabs
        items={SUB_TABS}
        activeId={activeSubTab()}
        onSelect={(id) => activeSubTab.set(id as LogSubTab)}
      />

      {/* 子视图 1：饮食随记 */}
      <If when={activeSubTab() === "diet"}>
        <View class="gap-4">
          <Card>
            <SectionHeader
              title="一句话随手记饮食"
              subtitle="无需称重打卡，写下吃过喝过的内容即可自动识别时段与热量"
            />

            <View class="flex-row flex-wrap items-center gap-2">
              <View class="min-w-48 flex-1">
                <FieldInput
                  value={naturalInput()}
                  placeholder="如：下午三点喝了生椰拿铁，晚上吃牛肉面"
                  onChange={(v) => naturalInput.set(v)}
                />
              </View>
              <ActionButton title="记入今日饮食" icon="plus" onPress={submitNatural} />
              <ActionButton
                title={speechListening() ? "停止语音" : "语音速记"}
                icon="mic"
                variant="outline"
                onPress={() => void toggleVoiceDictation()}
              />
            </View>

            <If when={Boolean(speechHint())}>
              <Text class="text-xs text-muted-foreground">{speechHint()!}</Text>
            </If>

            {/* 4 枚高频场景快捷卡 */}
            <View class="gap-2 pt-1">
              <Text class="text-xs font-semibold text-foreground">高频场景一键轻点</Text>
              <View class="flex-row flex-wrap gap-2">
                <Each of={QUICK_INTAKES} key={(q) => q.item}>
                  {(q) => (
                    <Button
                      onPress={() =>
                        logQuickIntake(q().category, q().item, q().tag, q().notes, q().kcal)
                      }
                      class="min-h-11 min-w-36 flex-1 flex-row items-center gap-2.5 rounded-xl border border-border/70 bg-surface-sunken px-3 py-2.5 transition active:scale-[0.98] motion-reduce:active:scale-100"
                    >
                      <View class="size-8 items-center justify-center rounded-lg bg-card">
                        <QiIcon name={q().icon} size={16} color={token("primary")} />
                      </View>
                      <View class="flex-1 gap-0.5">
                        <Text class="text-xs font-semibold text-foreground">{q().item}</Text>
                        <Text class="text-[11px] text-muted-foreground">{q().caption}</Text>
                      </View>
                    </Button>
                  )}
                </Each>
              </View>
            </View>

            {/* 中式 60+ 餐饮词典搜索 */}
            <SunkenBox>
              <Text class="text-xs font-semibold text-foreground">中式常见餐饮与补剂速查（60+ 预设）</Text>
              <FieldInput
                value={foodSearchQuery()}
                placeholder="搜菜名、饮品或补剂，如：生椰拿铁、牛肉面、酸菜鱼、镁"
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

          {/* D -> D+1 身体反馈 */}
          <If when={snap().lagged_impacts.length > 0}>
            <Card>
              <SectionHeader
                title="饮食与次日身体反馈"
                subtitle="基于过往记录统计当日饮食对次日恢复节律的温和影响"
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

          {/* 最近饮食记录 */}
          <Card>
            <SectionHeader title="近期饮食记录" subtitle="全部保存在本机加密健康库" />
            <If
              when={intakes().length > 0}
              else={
                <SunkenBox>
                  <Text class="text-xs text-muted-foreground">暂无饮食记录，点上方快捷卡或输入一句话即可开启。</Text>
                </SunkenBox>
              }
            >
              <View class="gap-2">
                <Each of={intakes().slice(0, 8)} key={(it) => it.id}>
                  {(it) => (
                    <SunkenBox>
                      <View class="flex-row items-center justify-between gap-2">
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
                    </SunkenBox>
                  )}
                </Each>
              </View>
            </If>
          </Card>
        </View>
      </If>

      {/* 子视图 2：体检档案 */}
      <If when={activeSubTab() === "labs"}>
        <View class="gap-4">
          <Card>
            <SectionHeader
              title="体检单红绿灯私密档案"
              subtitle="相册识图全程本机完成，导入前自动遮盖姓名与证件号"
              right={<VitalBadge tone="prime_ready" label="隐私脱敏守护" />}
            />

            <View class="flex-row flex-wrap gap-2">
              <ActionButton
                title="相册选图离线识图"
                icon="camera"
                onPress={() => void pickAndOcrLabPhoto()}
                class="flex-1"
              />
              <ActionButton
                title="录入 / 粘贴化验单"
                icon="file_lab"
                variant="outline"
                onPress={() => showCustomLab.set(!showCustomLab())}
                class="flex-1"
              />
            </View>

            <View class="flex-row items-center justify-between gap-2">
              <Text class="flex-1 text-xs text-muted-foreground">暂无手头化验单？可体验一次脱敏建档流程</Text>
              <Button
                onPress={() => requestAiAction(() => importLabReport("2026-11-13", SAMPLE_LAB_OCR))}
                class="min-h-10 items-center justify-center rounded-full border border-border bg-surface-sunken px-3.5 py-1.5 active:opacity-80"
              >
                <Text class="text-xs font-medium text-foreground">试用示例报告体验脱敏</Text>
              </Button>
            </View>

            <If when={showCustomLab()}>
              <SunkenBox>
                <Text class="text-xs font-semibold text-foreground">粘贴化验单文本（支持血脂、血糖、尿酸、肝功、维 D 等）</Text>
                <FieldInput
                  value={customLabText()}
                  multiline={true}
                  placeholder="粘贴化验单识别文本，系统会先在本机遮盖敏感个人信息"
                  onChange={(v) => customLabText.set(v)}
                />
                <ActionButton
                  title="脱敏并解析建档"
                  icon="check"
                  onPress={() => {
                    requestAiAction(() => {
                      importLabReport("2026-11-13", customLabText());
                      showCustomLab.set(false);
                    });
                  }}
                />
              </SunkenBox>
            </If>

            {/* PII 端侧遮盖盾牌反馈 */}
            <If when={Boolean(lastImport())}>
              <View class="gap-2 rounded-xl border border-vital-prime/30 bg-vital-prime-soft p-3.5">
                <View class="flex-row items-center gap-2">
                  <QiIcon name="user_shield" size={16} color={token("vital-prime")} />
                  <Text class="flex-1 text-xs font-semibold text-vital-prime">
                    {`端侧隐私盾牌已生效：自动遮盖 [${lastImport()!.redacted_fields.join("、") || "无个人敏感字段"}]`}
                  </Text>
                </View>
                <Text class="text-xs leading-relaxed text-foreground">{lastImport()!.scrubbed_preview}</Text>
              </View>
            </If>
          </Card>

          {/* 化验指标列表（红绿灯 + 子弹图 + 跨期对比） */}
          <Card>
            <SectionHeader
              title="已归档化验指标"
              subtitle={`共 ${labs().length} 条纵向检验记录`}
            />
            <If
              when={labs().length > 0}
              else={
                <SunkenBox>
                  <Text class="text-xs text-muted-foreground">
                    尚未归档体检指标。可点击上方「相册选图离线识图」或「试用示例报告体验脱敏」。
                  </Text>
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
                          <Text class="text-xs text-muted-foreground">{`检验日期 ${lb().test_date}`}</Text>
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
                            ? `参考区间 ${lb().ref_low} – ${lb().ref_high}`
                            : "无标准参考区间"}
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
                              {`上期 (${lb().prev_date ?? "—"}): ${lb().prev_value} ${lb().unit} · ${formatLabChange(lb().trend, lb().delta, lb().unit)}`}
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
        </View>
      </If>

      {/* 子视图 3：穿戴导入 */}
      <If when={activeSubTab() === "wearable"}>
        <Card>
          <SectionHeader
            title="外部穿戴历史数据导入"
            subtitle="支持 Apple Health 导出 XML、CSV 日报与 OpenHealthSync JSONL"
            right={<VitalBadge tone="prime_ready" label="自动口径区分" />}
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
              label="CSV 日报"
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
                externalText.set(
                  '{"kind":"daily_metric","date":"2026-11-12","metric":"steps","source":"mi_band","value":7880,"unit":"count"}',
                );
              }}
            />
          </View>

          <FieldInput
            value={externalText()}
            multiline={true}
            placeholder="粘贴 Apple Health export.xml 片段、CSV 或 JSONL"
            onChange={(v) => externalText.set(v)}
          />

          <ActionButton
            title="导入历史穿戴记录"
            icon="refresh"
            onPress={() => importExternalHealthData(externalFormat(), externalText())}
          />

          <SunkenBox>
            <Text class="text-xs leading-relaxed text-muted-foreground">
              不同手表与手环的采集指标会自动分源留档与优先级合并，并在本机重算近期节律常态。
            </Text>
          </SunkenBox>
        </Card>
      </If>
    </View>
  );
}
