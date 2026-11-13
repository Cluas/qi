import { signal, Text, View } from "nsh";
import {
  generateEncryptedPreview,
  generateJsonlExport,
  getEncryptedBatchPreview,
  getJsonlExportPreview,
  getNoticeText,
  getState,
  reseedDemoData,
  toggleMorningPush,
  updateAiConfig,
  updateSyncConfig,
  wipeVaultAndReset,
} from "../store";
import {
  ActionButton,
  Card,
  FieldInput,
  SectionHeader,
  SelectChip,
  SheetDialog,
  SunkenBox,
  SwitchRow,
  VitalBadge,
} from "../ui/components";

const SYNC_MODES = [
  { id: "icloud_drive", label: "① iCloud 个人云 (默认免费)" },
  { id: "webdav", label: "② 跨端 WebDAV 网盘" },
  { id: "byo_server", label: "③ 私有节点 BYO-Server" },
  { id: "local_only", label: "④ 纯本地离线" },
];

const AI_MODES = [
  { id: "official_zdr_cloud", label: "① ZDR 零留存云端 (默认)" },
  { id: "on_device_local", label: "② 纯本机离线大模型" },
  { id: "lan_ollama", label: "③ 家庭局域网 Ollama" },
];

export function MeVaultPage() {
  const showRecoveryPhrase = signal(false);
  const showNoticeSheet = signal(false);
  const showWipeConfirm = signal(false);

  const settings = () => getState().settings;

  return (
    <View class="gap-4">
      {/* 1. 数据主权与个人云同步中心 */}
      <Card>
        <SectionHeader
          title="数据主权与云同步中心"
          subtitle="健康库与体检档案官方服务器零留存，密钥由本机保管"
          right={<VitalBadge tone="prime_ready" label="E2EE" />}
        />

        <View class="flex-row flex-wrap gap-2">
          <Each of={SYNC_MODES} key={(m) => m.id}>
            {(m) => (
              <SelectChip
                label={m().label}
                selected={settings().sync_mode === m().id}
                onPress={() =>
                  updateSyncConfig(m().id, settings().webdav_url, settings().byo_server_url)
                }
              />
            )}
          </Each>
        </View>

        <If when={settings().sync_mode === "webdav"}>
          <SunkenBox>
            <Text class="text-xs font-medium text-foreground">WebDAV 网盘地址（坚果云 / 群晖 NAS）</Text>
            <FieldInput
              value={settings().webdav_url}
              placeholder="https://dav.jianguoyun.com/dav/qi-vault/"
              onChange={(v) => updateSyncConfig("webdav", v, settings().byo_server_url)}
            />
          </SunkenBox>
        </If>

        <If when={settings().sync_mode === "byo_server"}>
          <SunkenBox>
            <Text class="text-xs font-medium text-foreground">BYO-Server 私有同步节点 (Puls Sync v1/v2)</Text>
            <FieldInput
              value={settings().byo_server_url}
              placeholder="https://health.home.cluas.eu.org"
              onChange={(v) => updateSyncConfig("byo_server", settings().webdav_url, v)}
            />
          </SunkenBox>
        </If>

        <View class="flex-row flex-wrap gap-2">
          <ActionButton
            title="生成 XChaCha20 加密同步包预览"
            variant="outline"
            onPress={() => generateEncryptedPreview()}
            class="flex-1"
          />
          <ActionButton
            title={showRecoveryPhrase() ? "隐藏 12 词恢复码" : "查看 12 词恢复码"}
            variant="secondary"
            onPress={() => showRecoveryPhrase.set(!showRecoveryPhrase())}
          />
        </View>

        <If when={showRecoveryPhrase()}>
          <SunkenBox>
            <Text class="text-xs font-semibold text-vital-steady">
              个人保险箱 12 词恢复码（请抄录或打印，官方无法找回）
            </Text>
            <Text class="text-sm font-semibold text-foreground">{settings().recovery_phrase}</Text>
          </SunkenBox>
        </If>

        <If when={Boolean(getEncryptedBatchPreview())}>
          <SunkenBox>
            <Text class="text-xs font-semibold text-primary">设备隔离追加分片与 AEAD 验签往返成功</Text>
            <Text class="text-xs text-muted-foreground">{getEncryptedBatchPreview()!}</Text>
          </SunkenBox>
        </If>
      </Card>

      {/* 2. AI 引擎与隐私模式 */}
      <Card>
        <SectionHeader
          title="AI 引擎与隐私模式"
          subtitle="可自由切换官方零留存云、纯本机模型或局域网算力"
        />

        <View class="flex-row flex-wrap gap-2">
          <Each of={AI_MODES} key={(m) => m.id}>
            {(m) => (
              <SelectChip
                label={m().label}
                selected={settings().ai_mode === m().id}
                onPress={() =>
                  updateAiConfig(
                    m().id,
                    settings().zdr_consented,
                    settings().lan_ai_url,
                    settings().lan_ai_model,
                    settings().thermal_throttled,
                    settings().simulated_ram_gb,
                  )
                }
              />
            )}
          </Each>
        </View>

        <If when={settings().ai_mode === "on_device_local"}>
          <SunkenBox>
            <Text class="text-xs font-semibold text-foreground">端侧大模型硬件门禁与发热降级模拟</Text>
            <Text class="text-xs text-muted-foreground">
              iOS 26+ FoundationModels 免下载；内嵌权重需 ≥6GB 内存，且后台严禁跑 Metal GPU。
            </Text>
            <View class="flex-row flex-wrap gap-2">
              <SelectChip
                label="模拟 4GB 机型 (<6GB 禁载)"
                selected={settings().simulated_ram_gb < 6}
                onPress={() =>
                  updateAiConfig(
                    "on_device_local",
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
                    "on_device_local",
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
              subtitle="开启后自动退回 Rust 确定性精简解读，首页绝不阻塞"
              checked={settings().thermal_throttled}
              onChange={(checked) =>
                updateAiConfig(
                  "on_device_local",
                  settings().zdr_consented,
                  settings().lan_ai_url,
                  settings().lan_ai_model,
                  checked,
                  settings().simulated_ram_gb,
                )
              }
            />
          </SunkenBox>
        </If>

        <If when={settings().ai_mode === "lan_ollama"}>
          <SunkenBox>
            <Text class="text-xs font-medium text-foreground">家庭局域网 Ollama / 自建网关 (OpenAI 兼容)</Text>
            <FieldInput
              value={settings().lan_ai_url}
              placeholder="http://192.168.2.18:11434/v1"
              onChange={(v) =>
                updateAiConfig(
                  "lan_ollama",
                  settings().zdr_consented,
                  v,
                  settings().lan_ai_model,
                  settings().thermal_throttled,
                  settings().simulated_ram_gb,
                )
              }
            />
            <FieldInput
              value={settings().lan_ai_model}
              placeholder="qwen3:8b"
              onChange={(v) =>
                updateAiConfig(
                  "lan_ollama",
                  settings().zdr_consented,
                  settings().lan_ai_url,
                  v,
                  settings().thermal_throttled,
                  settings().simulated_ram_gb,
                )
              }
            />
          </SunkenBox>
        </If>

        <If when={settings().ai_mode === "official_zdr_cloud"}>
          <SwitchRow
            title="已显式同意 ZDR 零留存云端 AI 授权 (Apple 5.1.2(i))"
            subtitle="关闭后将恢复按需懒加载询问；本地模型无需此授权"
            checked={settings().zdr_consented}
            onChange={(checked) =>
              updateAiConfig(
                "official_zdr_cloud",
                checked,
                settings().lan_ai_url,
                settings().lan_ai_model,
                settings().thermal_throttled,
                settings().simulated_ram_gb,
              )
            }
          />
        </If>
      </Card>

      {/* 3. 推送、导出、合规致谢与物理清空 */}
      <Card>
        <SectionHeader title="日常设置与合规工具" />

        <SwitchRow
          title={`每日 ${settings().morning_push_time} 本地早安通知`}
          subtitle="端侧 Rust 预热算分 + 本地定时推送，不依赖云端数据库"
          checked={settings().morning_push_enabled}
          onChange={(checked) => toggleMorningPush(checked)}
        />

        <View class="flex-row flex-wrap gap-2 pt-1">
          <ActionButton
            title="导出 JSONL 摘要"
            variant="secondary"
            onPress={() => generateJsonlExport()}
          />
          <ActionButton
            title="重载 35 天演示数据"
            variant="secondary"
            onPress={() => reseedDemoData()}
          />
          <ActionButton
            title="开源致谢 NOTICE"
            variant="outline"
            onPress={() => showNoticeSheet.set(true)}
          />
        </View>

        <If when={Boolean(getJsonlExportPreview())}>
          <SunkenBox>
            <Text class="text-xs font-semibold text-foreground">标准开放格式 JSONL 导出片段</Text>
            <Text class="text-xs text-muted-foreground">{getJsonlExportPreview()!}</Text>
          </SunkenBox>
        </If>

        <ActionButton
          title="物理清空本机健康数据 (5.1.1(v))"
          variant="danger"
          onPress={() => showWipeConfirm.set(true)}
        />
      </Card>

      {/* 开源致谢弹出页 */}
      <SheetDialog
        open={showNoticeSheet()}
        title="开源致谢与 Apache-2.0 NOTICE"
        onClose={() => showNoticeSheet.set(false)}
      >
        <Card>
          <Text class="text-sm leading-relaxed text-foreground">{getNoticeText()}</Text>
        </Card>
      </SheetDialog>

      {/* 物理删除二次确认 */}
      <SheetDialog
        open={showWipeConfirm()}
        title="确认物理清空本机健康库"
        onClose={() => showWipeConfirm.set(false)}
      >
        <Card>
          <Text class="text-sm leading-relaxed text-foreground">
            此操作将立即从本机 SQLite 彻底擦除所有生理指标、饮食与体检记录，并重置引导状态，符合 Apple 5.1.1(v) 账号与数据销毁规范。
          </Text>
          <View class="flex-row gap-2">
            <ActionButton
              title="取消"
              variant="secondary"
              onPress={() => showWipeConfirm.set(false)}
              class="flex-1"
            />
            <ActionButton
              title="确认清空"
              variant="danger"
              onPress={() => {
                showWipeConfirm.set(false);
                wipeVaultAndReset();
              }}
              class="flex-1"
            />
          </View>
        </Card>
      </SheetDialog>
    </View>
  );
}
