import { signal, Text, View } from "nsh";
import { getDisclaimerText, getState, importLabReport, logQuickIntake, requestAiAction } from "../store";
import { ActionButton, Card, FieldInput, SectionHeader, SunkenBox, VitalBadge } from "../ui/components";

const QUICK_INTAKES = [
  { category: "caffeine", item: "15:00后冰美式", tag: "晚间咖啡", notes: "下午提神咖啡因" },
  { category: "alcohol", item: "晚间精酿小酌", tag: "晚间饮酒", notes: "聚餐小酌一杯" },
  { category: "late_meal", item: "22:30夜宵烧烤", tag: "夜间重餐", notes: "睡前高油盐加餐" },
  { category: "meal", item: "均衡减脂正餐", tag: "高蛋白轻食", notes: "鸡胸糙米时蔬" },
];

const SAMPLE_LAB_OCR = `某某市第一人民医院体检中心 检验报告单
姓名：陈默 身份证号：31010419920518432X 手机：13912345678 住院号：ZY20261108
尿酸(UA) 468↑ umol/L 208-428
低密度脂蛋白胆固醇(LDL-C) 3.62↑ mmol/L 0.00-3.40
空腹血糖(GLU) 5.08 mmol/L 3.90-6.10`;

export function LogLabsPage() {
  const customFood = signal("");
  const customLabText = signal(SAMPLE_LAB_OCR);
  const showCustomLab = signal(false);

  const state = () => getState();
  const snap = () => state().snapshot;
  const intakes = () => state().intakes;
  const labs = () => state().labs;
  const lastImport = () => state().last_lab_import;

  const submitFood = () => {
    const text = customFood().trim();
    if (!text) return;
    const tag = text.includes("咖啡")
      ? "晚间咖啡"
      : text.includes("酒")
        ? "晚间饮酒"
        : "日常饮食";
    logQuickIntake("meal", text, tag, "一句话速记");
    customFood.set("");
  };

  return (
    <View class="gap-4">
      {/* 1. 3 秒拍照/点选记餐食与节律标签 */}
      <Card>
        <SectionHeader
          title="3 秒记饮食与恢复因子"
          subtitle="无需繁琐热量库，重点捕捉影响次日恢复的关键词"
        />

        <View class="flex-row flex-wrap gap-2">
          <Each of={QUICK_INTAKES} key={(q) => q.item}>
            {(q) => (
              <ActionButton
                title={`+ ${q().item}`}
                variant="secondary"
                onPress={() => logQuickIntake(q().category, q().item, q().tag, q().notes)}
              />
            )}
          </Each>
        </View>

        <View class="flex-row items-center gap-2">
          <View class="flex-1">
            <FieldInput
              value={customFood()}
              placeholder="输入一句速记，如：晚餐吃了轻食沙拉"
              onChange={(v) => customFood.set(v)}
            />
          </View>
          <ActionButton title="记入" onPress={submitFood} />
        </View>

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
          <Each of={intakes().slice(0, 4)} key={(it) => it.id}>
            {(it) => (
              <View class="flex-row items-center justify-between gap-2">
                <Text class="flex-1 text-sm text-foreground">{`${it().date} · ${it().item_name}`}</Text>
                <Text class="text-xs text-muted-foreground">{it().tags[0] ?? it().category}</Text>
              </View>
            )}
          </Each>
        </SunkenBox>
      </Card>

      {/* 2. 体检报告红绿灯私密档案 */}
      <Card>
        <SectionHeader
          title="体检单红绿灯私密档案"
          subtitle="端侧先剔除姓名/证件号，再由 Rust 重算参考区间"
          right={<VitalBadge tone="prime_ready" label="PII 端侧脱敏" />}
        />

        <View class="flex-row flex-wrap gap-2">
          <ActionButton
            title="模拟导入体检单 (含隐私前置脱敏)"
            onPress={() =>
              requestAiAction(() => importLabReport("2026-11-13", customLabText()))
            }
            class="flex-1"
          />
          <ActionButton
            title={showCustomLab() ? "收起文本" : "编辑文本"}
            variant="outline"
            onPress={() => showCustomLab.set(!showCustomLab())}
          />
        </View>

        <If when={showCustomLab()}>
          <FieldInput
            value={customLabText()}
            multiline={true}
            placeholder="粘贴化验单 OCR 文本（含项目名、数值、单位、参考区间）"
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
                  <Text class="text-base font-bold text-foreground">
                    {`${lb().value} ${lb().unit}`}
                  </Text>
                  <Text class="text-xs text-muted-foreground">
                    {lb().ref_low !== null && lb().ref_high !== null
                      ? `参考区间 ${lb().ref_low} – ${lb().ref_high}`
                      : "无参考区间"}
                  </Text>
                </View>
                <Text class="text-xs leading-relaxed text-muted-foreground">{lb().plain_note}</Text>
              </SunkenBox>
            )}
          </Each>
        </View>

        <Text class="text-[11px] leading-relaxed text-muted-foreground">{getDisclaimerText()}</Text>
      </Card>
    </View>
  );
}
