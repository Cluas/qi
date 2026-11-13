import { Speech, signal, Text, View } from "nsh";
import {
  getDisclaimerText,
  getState,
  importExternalHealthData,
  importLabReport,
  logNaturalIntake,
  logQuickIntake,
  ocrSampleFixtureLab,
  pickAndOcrLabPhoto,
  requestAiAction,
  searchFoods,
} from "../store";
import { ActionButton, Card, FieldInput, SectionHeader, SelectChip, SunkenBox, VitalBadge } from "../ui/components";

const QUICK_INTAKES = [
  { category: "caffeine", item: "15:00后冰美式", tag: "晚间咖啡", notes: "下午提神咖啡因", kcal: 15 },
  { category: "alcohol", item: "晚间精酿小酌", tag: "晚间饮酒", notes: "聚餐小酌一杯", kcal: 190 },
  { category: "late_meal", item: "22:30夜宵烧烤", tag: "夜间重餐", notes: "睡前高油盐加餐", kcal: 620 },
  { category: "meal", item: "均衡减脂正餐", tag: "高蛋白轻食", notes: "鸡胸糙米时蔬", kcal: 430 },
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

function trendLabel(trend: string | null, delta: number | null, unit: string): string {
  if (delta === null || trend === null) return "";
  const sign = delta > 0 ? "+" : "";
  const suffix =
    trend === "improved"
      ? " · 较上次已改善"
      : trend === "worsened"
        ? " · 较上次需留意"
        : " · 较上次持平";
  return `${sign}${delta.toFixed(2)} ${unit}${suffix}`;
}

export function LogLabsPage() {
  const naturalInput = signal("");
  const foodSearchQuery = signal("");
  const speechListening = signal(false);
  const speechHint = signal<string | null>(null);
  const customLabText = signal(SAMPLE_LAB_OCR);
  const showCustomLab = signal(false);
  const externalFormat = signal<"apple_health_xml" | "csv" | "jsonl">("apple_health_xml");
  const externalText = signal(SAMPLE_APPLE_XML);
  const showExternalImport = signal(false);

  const state = () => getState();
  const snap = () => state().snapshot;
  const intakes = () => state().intakes;
  const labs = () => state().labs;
  const lastImport = () => state().last_lab_import;
  const foodHits = () => {
    const q = foodSearchQuery().trim();
    return q ? searchFoods(q, 6) : searchFoods("", 4);
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
        speechHint.set("当前运行环境暂不支持系统麦克风识别，可直接键盘输入一句话智能解析");
        return;
      }
      speechListening.set(true);
      speechHint.set("正在聆听中文饮食描述…");
      const sess = Speech.start({ lang: "zh-CN", interim: true });
      sess.on("result", (res) => {
        if (res.text) naturalInput.set(res.text);
      });
      sess.on("end", (end) => {
        speechListening.set(false);
        if (end.text) {
          naturalInput.set(end.text);
          speechHint.set("语音听写完成，点「一句话智能记入」即可入库");
        } else if (end.error) {
          speechHint.set(`语音识别结束（${end.error}），可继续手动编辑`);
        }
      });
    } catch {
      speechListening.set(false);
      speechHint.set("麦克风权限未授权或不可用，可直接输入一句自然语言饮食描述");
    }
  };

  return (
    <View class="gap-4">
      {/* 1. 3 秒记餐食、一句话 NLP 槽位提取与 60+ 中式词典检索 */}
      <Card>
        <SectionHeader
          title="3 秒记饮食与恢复因子"
          subtitle="内置 60+ 中式高频餐饮词典 + 离线自然语言解析，无需繁琐称重"
        />

        <View class="flex-row flex-wrap gap-2">
          <Each of={QUICK_INTAKES} key={(q) => q.item}>
            {(q) => (
              <ActionButton
                title={`+ ${q().item}`}
                variant="secondary"
                onPress={() => logQuickIntake(q().category, q().item, q().tag, q().notes, q().kcal)}
              />
            )}
          </Each>
        </View>

        <View class="flex-row flex-wrap items-center gap-2">
          <View class="min-w-48 flex-1">
            <FieldInput
              value={naturalInput()}
              placeholder="如：下午三点喝了生椰拿铁，晚上吃牛肉面"
              onChange={(v) => naturalInput.set(v)}
            />
          </View>
          <ActionButton title="一句话智能记入" onPress={submitNatural} />
          <ActionButton
            title={speechListening() ? "停止语音" : "语音速记"}
            variant="outline"
            onPress={() => void toggleVoiceDictation()}
          />
        </View>

        <If when={Boolean(speechHint())}>
          <Text class="text-xs text-muted-foreground">{speechHint()!}</Text>
        </If>

        {/* 60+ 中式饮食词典检索 */}
        <SunkenBox>
          <Text class="text-xs font-semibold text-foreground">中式高频餐饮与补剂词典检索（60+ 预设，0ms 离线）</Text>
          <FieldInput
            value={foodSearchQuery()}
            placeholder="搜菜名/饮品/补剂，如：生椰拿铁、牛肉面、酸菜鱼、镁、烧烤"
            onChange={(v) => foodSearchQuery.set(v)}
          />
          <View class="flex-row flex-wrap gap-2">
            <Each of={foodHits()} key={(hit) => hit.id}>
              {(hit) => (
                <ActionButton
                  title={`${hit().name} (${hit().kcal}kcal)`}
                  variant="outline"
                  onPress={() =>
                    logQuickIntake(hit().category, hit().name, hit().rhythm_tag, hit().tip, hit().kcal)
                  }
                />
              )}
            </Each>
          </View>
        </SunkenBox>

        <If when={snap().lagged_impacts.length > 0}>
          <SunkenBox>
            <Text class="text-xs font-semibold text-primary">我的身体规律归因（D 日摄入 → D+1 恢复）</Text>
            <Each of={snap().lagged_impacts} key={(imp) => `${imp.tag}-${imp.outcome_label}`}>
              {(imp) => (
                <Text class="text-xs leading-relaxed text-foreground">{`• ${imp().summary}`}</Text>
              )}
            </Each>
          </SunkenBox>
        </If>

        <SunkenBox>
          <Text class="text-xs font-medium text-muted-foreground">最近记录（仅存本地加密库）</Text>
          <Each of={intakes().slice(0, 5)} key={(it) => it.id}>
            {(it) => (
              <View class="flex-row items-center justify-between gap-2">
                <Text class="flex-1 text-sm text-foreground">{`${it().date} · ${it().item_name}`}</Text>
                <Text class="text-xs text-muted-foreground">
                  {it().calories_kcal !== null ? `${Math.round(it().calories_kcal!)} kcal · ${it().tags[0] ?? it().category}` : `${it().tags[0] ?? it().category}`}
                </Text>
              </View>
            )}
          </Each>
        </SunkenBox>
      </Card>

      {/* 2. 体检报告红绿灯私密档案（支持 Apple Vision 离线 OCR + 25+ 指标纵向对比） */}
      <Card>
        <SectionHeader
          title="体检单红绿灯私密档案"
          subtitle="支持 Apple Vision 纯离线 OCR 识图 + 25 项指标跨期趋势追踪"
          right={<VitalBadge tone="prime_ready" label="PII 端侧脱敏" />}
        />

        <View class="flex-row flex-wrap gap-2">
          <ActionButton
            title="模拟导入体检单 (含隐私前置脱敏)"
            onPress={() => requestAiAction(() => importLabReport("2026-11-13", customLabText()))}
            class="flex-1"
          />
          <ActionButton
            title="实测离线 OCR 样张"
            variant="outline"
            onPress={() => ocrSampleFixtureLab()}
          />
          <ActionButton
            title="相册选图离线 OCR"
            variant="outline"
            onPress={() => void pickAndOcrLabPhoto()}
          />
          <ActionButton
            title={showCustomLab() ? "收起文本" : "编辑文本"}
            variant="secondary"
            onPress={() => showCustomLab.set(!showCustomLab())}
          />
        </View>

        <If when={showCustomLab()}>
          <FieldInput
            value={customLabText()}
            multiline={true}
            placeholder="粘贴化验单 OCR 文本（支持 ALT / AST / TSH / HbA1c / 尿酸 / 血脂 / 铁蛋白等 25+ 项）"
            onChange={(v) => customLabText.set(v)}
          />
        </If>

        <If when={Boolean(lastImport())}>
          <SunkenBox>
            <Text class="text-xs font-semibold text-vital-prime">
              {`端侧脱敏完成：已自动遮盖 [${lastImport()!.redacted_fields.join("、") || "无敏感字段"}]`}
            </Text>
            <Text class="text-xs text-muted-foreground">{lastImport()!.scrubbed_preview}</Text>
          </SunkenBox>
        </If>

        <View class="gap-2">
          <Each of={labs()} key={(lb) => `${lb.id}-${lb.test_date}-${lb.canonical_key}`}>
            {(lb) => (
              <SunkenBox>
                <View class="flex-row items-center justify-between gap-2">
                  <View class="flex-1">
                    <Text class="text-sm font-semibold text-foreground">{lb().marker_name}</Text>
                    <Text class="text-xs text-muted-foreground">{`检验日期 ${lb().test_date}`}</Text>
                  </View>
                  <VitalBadge
                    tone={lb().flag}
                    label={
                      lb().flag === "high"
                        ? "偏高提醒"
                        : lb().flag === "low"
                          ? "偏低提醒"
                          : "参考区间内"
                    }
                  />
                </View>
                <View class="flex-row items-baseline justify-between gap-2">
                  <Text class="text-base font-bold text-foreground" style={{ fontVariant: ["tabular-nums"] }}>
                    {`${lb().value} ${lb().unit}`}
                  </Text>
                  <Text class="text-xs text-muted-foreground">
                    {lb().ref_low !== null && lb().ref_high !== null
                      ? `参考区间 ${lb().ref_low} – ${lb().ref_high}`
                      : "无参考区间"}
                  </Text>
                </View>
                <If when={lb().prev_value !== null}>
                  <Text class="text-xs font-medium text-primary">
                    {`上次 (${lb().prev_date ?? "—"}): ${lb().prev_value} ${lb().unit} (${trendLabel(lb().trend, lb().delta, lb().unit)})`}
                  </Text>
                </If>
                <Text class="text-xs leading-relaxed text-muted-foreground">{lb().plain_note}</Text>
              </SunkenBox>
            )}
          </Each>
        </View>

        <Text class="text-[11px] leading-relaxed text-muted-foreground">{getDisclaimerText()}</Text>
      </Card>

      {/* 3. 外部穿戴数据导入器（Apple Health XML / CSV / OpenHealthSync JSONL） */}
      <Card>
        <SectionHeader
          title="Apple Health / CSV 外部数据导入"
          subtitle="支持直接导入导出片段，RMSSD 与 SDNN 自动分轨合并"
          right={
            <ActionButton
              title={showExternalImport() ? "收起导入" : "展开导入"}
              variant="secondary"
              onPress={() => showExternalImport.set(!showExternalImport())}
            />
          }
        />

        <If when={showExternalImport()}>
          <View class="gap-2.5">
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
                  externalText.set('{"kind":"daily_metric","date":"2026-11-12","metric":"steps","source":"mi_band","value":7880,"unit":"count"}');
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
              title="导入外部穿戴记录"
              onPress={() => importExternalHealthData(externalFormat(), externalText())}
            />
          </View>
        </If>
      </Card>
    </View>
  );
}
