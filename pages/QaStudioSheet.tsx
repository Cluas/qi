import { Text, token, View } from "nsh";
import {
  generateEncryptedPreview,
  getEncryptedBatchPreview,
  getLastAuditReport,
  getPersonas,
  getShowQaStudio,
  getState,
  loadPersona,
  ocrSampleFixtureLab,
  probeCapabilities,
  reseedDemoData,
  runDataIntegrityAudit,
  setHardwareTier,
  setShowQaStudio,
  updateAiConfig,
} from "../store";
import {
  ActionButton,
  Card,
  Hairline,
  SectionHeader,
  SelectChip,
  SheetDialog,
  SunkenBox,
  SwitchRow,
  VitalBadge,
} from "../ui/components";
import { QiIcon } from "../ui/icons";

const TIER_OPTIONS = [
  { id: "tier_a", label: "Tier A 全穿戴" },
  { id: "tier_b", label: "Tier B 基础手环" },
  { id: "tier_c", label: "Tier C 纯手机" },
];

export function QaStudioSheet() {
  const state = () => getState();
  const settings = () => state().settings;
  const probe = () => state().capability_probe;
  const auditReport = () => getLastAuditReport();
  const personas = () => getPersonas();

  return (
    <SheetDialog
      open={getShowQaStudio()}
      title="知气验真与数据工坊"
      onClose={() => setShowQaStudio(false)}
    >
      {/* 1. 端侧数据完整性实时自检 */}
      <Card>
        <SectionHeader
          title="端侧数据完整性自检"
          subtitle="实时校验统计边界、RMSSD/SDNN 口径隔离、硬件契约、化验单趋势与 PII 脱敏"
          right={
            <If when={auditReport() !== null}>
              <VitalBadge
                tone={auditReport()!.passed ? "prime_ready" : "alert_high"}
                label={auditReport()!.passed ? "全部通过" : "发现异常"}
              />
            </If>
          }
        />

        <ActionButton
          title="运行全量数据自检"
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
                {auditReport()!.passed ? "所有 6 项不变量校验通过" : "存在未通过的校验项"}
              </Text>
              <Text class="text-xs text-muted-foreground" style={{ fontVariant: ["tabular-nums"] }}>
                {`画像：${auditReport()!.active_persona}`}
              </Text>
            </View>

            <Text class="text-xs text-muted-foreground" style={{ fontVariant: ["tabular-nums"] }}>
              {`指标 ${auditReport()!.metric_rows} 条 · 覆盖 ${auditReport()!.distinct_days} 天 · 饮食 ${auditReport()!.intake_rows} 条 · 化验 ${auditReport()!.lab_rows} 条 · 实验 ${auditReport()!.experiment_rows} 项`}
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

      {/* 2. 6 大真实测试画像一键切换 */}
      <Card>
        <SectionHeader
          title="6 大纵向测试画像一键切换"
          subtitle="切换后清空业务表并注入确定性纵向数据，保留云会话与 12 词恢复密钥"
          right={<VitalBadge tone="neutral" label={`${personas().length} 套画像`} />}
        />

        <View class="gap-2.5">
          <Each of={personas()} key={(p) => p.id}>
            {(p) => (
              <SunkenBox>
                <View class="flex-row items-center justify-between gap-2">
                  <View class="flex-1 gap-0.5">
                    <Text class="text-sm font-semibold text-foreground">{p().name}</Text>
                    <Text class="text-xs text-muted-foreground">
                      {`${p().badge} · ${p().hardware_tier} · ${p().primary_goal} · ${p().days_count} 天`}
                    </Text>
                  </View>
                  <If when={settings().active_persona === p().id}>
                    <VitalBadge tone="prime_ready" label="当前活跃" />
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
                  title={settings().active_persona === p().id ? `重新载入：${p().id}` : `切换画像：${p().id}`}
                  variant={settings().active_persona === p().id ? "secondary" : "outline"}
                  onPress={() => loadPersona(p().id)}
                />
              </SunkenBox>
            )}
          </Each>
        </View>
      </Card>

      {/* 3. 硬件与端侧算力边界工坊 */}
      <Card>
        <SectionHeader
          title="硬件与端侧算力边界工坊"
          subtitle="验证 Tier A/B/C 降级、4GB/8GB 内存门禁、发热降频、离线 OCR 样张与 XChaCha20 分片"
        />

        <View class="gap-2">
          <Text class="text-xs font-semibold text-foreground">
            {`当前硬件档位覆盖：${state().snapshot.hardware_tier_label}`}
          </Text>
          <View class="flex-row flex-wrap gap-2">
            <Each of={TIER_OPTIONS} key={(t) => t.id}>
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
          <Text class="text-xs font-semibold text-foreground">端侧大模型内存门禁与发热降频模拟</Text>
          <View class="flex-row flex-wrap gap-2">
            <SelectChip
              label="模拟 4GB 机型 (<6GB 禁载)"
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
              label="模拟 8GB 旗舰机型"
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
            title="模拟设备发热降频"
            subtitle="开启后自动退回 Rust 确定性精简解读"
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
            title="探测本机与局域网算力"
            icon="sliders"
            variant="outline"
            onPress={() => probeCapabilities()}
            class="flex-1"
          />
          <ActionButton
            title="实测离线 OCR 样张"
            icon="camera"
            variant="outline"
            onPress={() => ocrSampleFixtureLab()}
            class="flex-1"
          />
        </View>

        <If when={probe() !== null}>
          <SunkenBox>
            <Text class="text-xs font-semibold text-primary">{`运行平台：${probe()!.platform}`}</Text>
            <Text class="text-xs text-foreground">{probe()!.summary}</Text>
            <Text class="text-xs text-muted-foreground">
              {`Apple Vision OCR: ${probe()!.vision_ocr_available ? `就绪 (Rev.${probe()!.vision_revision})` : "不可用"} · FoundationModels: ${probe()!.foundation_models_status}`}
            </Text>
            <Text class="text-xs text-muted-foreground">
              {probe()!.ollama_reachable
                ? `局域网 Ollama 模型：${probe()!.ollama_models.join("、") || "已连通"}`
                : "局域网 Ollama 未检测到响应"}
            </Text>
          </SunkenBox>
        </If>

        <View class="flex-row flex-wrap gap-2">
          <ActionButton
            title="生成 XChaCha20 加密同步包预览"
            icon="cloud_lock"
            variant="secondary"
            onPress={() => generateEncryptedPreview()}
            class="flex-1"
          />
          <ActionButton
            title="重载 35 天演示数据"
            icon="refresh"
            variant="secondary"
            onPress={() => reseedDemoData()}
          />
        </View>

        <If when={Boolean(getEncryptedBatchPreview())}>
          <SunkenBox>
            <View class="flex-row items-center gap-2">
              <QiIcon name="cloud_lock" size={14} color={token("primary")} />
              <Text class="text-xs font-semibold text-primary">设备隔离追加分片与 AEAD 验签往返成功</Text>
            </View>
            <Text class="text-xs text-muted-foreground">{getEncryptedBatchPreview()!}</Text>
          </SunkenBox>
        </If>
      </Card>
    </SheetDialog>
  );
}
