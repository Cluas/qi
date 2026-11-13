import { signal, Text, View } from "nsh";
import {
  activateCloudPro,
  connectCloudSession,
  copyToClipboard,
  generateEncryptedArchive,
  generateEncryptedPreview,
  generateJsonlExport,
  getCloudEntitlements,
  getDisclaimerText,
  getEncryptedArchivePreview,
  getEncryptedBatchPreview,
  getJsonlExportPreview,
  getNoticeText,
  getState,
  probeCapabilities,
  reseedDemoData,
  restoreFromRecoveryPhrase,
  runE2eeSyncNow,
  sendMorningTestNotification,
  toggleMorningPush,
  updateAiConfig,
  updateCloudUrlAndHandle,
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
  const showRestoreSheet = signal(false);
  const restorePhraseInput = signal("");
  const restoreSourceInput = signal("");
  const restoreMode = signal<"icloud_drive" | "byo_server" | "archive">("icloud_drive");
  const showNoticeSheet = signal(false);
  const showPrivacySheet = signal(false);
  const showWipeConfirm = signal(false);

  const settings = () => getState().settings;
  const probe = () => getState().capability_probe;
  const ent = () => getCloudEntitlements();

  return (
    <View class="gap-4">
      {/* 0. 知气控制面账号与 Free/Pro 订阅状态（服务端零健康数据表） */}
      <Card>
        <SectionHeader
          title="知气控制面账号 (Free / Pro)"
          subtitle="仅管理登录态、月度 AI 配额与密文信封中继，绝无健康明文表"
          right={
            <VitalBadge
              tone={ent()?.plan_tier === "pro" ? "prime_ready" : "neutral"}
              label={ent()?.plan_tier === "pro" ? "Pro 已解锁" : settings().cloud_token ? "Free 会话已连" : "纯离线单机"}
            />
          }
        />

        <View class="gap-2">
          <FieldInput
            value={settings().cloud_url}
            placeholder="控制面地址，如 http://127.0.0.1:8788"
            onChange={(v) => updateCloudUrlAndHandle(v, settings().cloud_handle)}
          />
          <FieldInput
            value={settings().cloud_handle}
            placeholder="账号邮箱或标识（用于跨设备绑定信封命名空间）"
            onChange={(v) => updateCloudUrlAndHandle(settings().cloud_url, v)}
          />
        </View>

        <View class="flex-row flex-wrap gap-2">
          <ActionButton
            title="连接控制面会话"
            variant="outline"
            onPress={() => void connectCloudSession(settings().cloud_url, settings().cloud_handle)}
            class="flex-1"
          />
          <ActionButton
            title="演示解锁 Pro 权益"
            variant="secondary"
            onPress={() => void activateCloudPro()}
          />
        </View>

        <If when={ent() !== null}>
          <SunkenBox>
            <Text class="text-xs font-semibold text-primary">
              {`当前套餐：${ent()!.plan_tier.toUpperCase()} · 账期 ${ent()!.month}`}
            </Text>
            <Text class="text-xs text-muted-foreground">
              {ent()!.lab_parses_remaining < 0
                ? "本月 ZDR 体检深度解读：无限次 · 跨端 WebDAV & 局域网 Ollama 已解锁"
                : `本月 ZDR 体检深度解读剩余：${ent()!.lab_parses_remaining} 次（已用 ${ent()!.lab_parses_used} 次）`}
            </Text>
          </SunkenBox>
        </If>
      </Card>

      {/* 1. 数据主权与个人云同步中心 */}
      <Card>
        <SectionHeader
          title="数据主权与云同步中心"
          subtitle="健康库与体检档案官方服务器零留存，密钥由本机保管"
          right={<VitalBadge tone="prime_ready" label="E2EE" />}
        />

        <Text class="text-xs text-muted-foreground">
          {`本机设备 ID：${settings().device_id} · 本地分片序号 #${settings().sync_seq}`}
        </Text>

        <View class="flex-row flex-wrap gap-2">
          <Each of={SYNC_MODES} key={(m) => m.id}>
            {(m) => (
              <SelectChip
                label={m().label}
                selected={settings().sync_mode === m().id}
                onPress={() =>
                  updateSyncConfig(
                    m().id,
                    settings().webdav_url,
                    settings().byo_server_url,
                    settings().webdav_username,
                    settings().webdav_password,
                  )
                }
              />
            )}
          </Each>
        </View>

        <If when={settings().sync_mode === "webdav"}>
          <SunkenBox>
            <Text class="text-xs font-medium text-foreground">WebDAV 网盘配置（坚果云 / 群晖 NAS）</Text>
            <FieldInput
              value={settings().webdav_url}
              placeholder="https://dav.jianguoyun.com/dav/qi-vault/"
              onChange={(v) =>
                updateSyncConfig(
                  "webdav",
                  v,
                  settings().byo_server_url,
                  settings().webdav_username,
                  settings().webdav_password,
                )
              }
            />
            <FieldInput
              value={settings().webdav_username}
              placeholder="WebDAV 用户名（可选）"
              onChange={(v) =>
                updateSyncConfig(
                  "webdav",
                  settings().webdav_url,
                  settings().byo_server_url,
                  v,
                  settings().webdav_password,
                )
              }
            />
            <FieldInput
              value={settings().webdav_password}
              placeholder="WebDAV 应用密码（可选）"
              onChange={(v) =>
                updateSyncConfig(
                  "webdav",
                  settings().webdav_url,
                  settings().byo_server_url,
                  settings().webdav_username,
                  v,
                )
              }
            />
          </SunkenBox>
        </If>

        <If when={settings().sync_mode === "byo_server"}>
          <SunkenBox>
            <Text class="text-xs font-medium text-foreground">BYO-Server 私有同步节点 (OpenHealthSync v2)</Text>
            <FieldInput
              value={settings().byo_server_url}
              placeholder="http://127.0.0.1:8788"
              onChange={(v) =>
                updateSyncConfig(
                  "byo_server",
                  settings().webdav_url,
                  v,
                  settings().webdav_username,
                  settings().webdav_password,
                )
              }
            />
          </SunkenBox>
        </If>

        <View class="flex-row flex-wrap gap-2">
          <ActionButton
            title="立即执行 E2EE 同步"
            onPress={() => runE2eeSyncNow()}
            class="flex-1"
          />
          <ActionButton
            title="12 词跨生态恢复"
            variant="outline"
            onPress={() => {
              restorePhraseInput.set(settings().recovery_phrase);
              showRestoreSheet.set(true);
            }}
          />
        </View>

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

        <If when={Boolean(settings().last_sync_summary)}>
          <SunkenBox>
            <Text class="text-xs font-semibold text-primary">最近一次同步结果</Text>
            <Text class="text-xs text-muted-foreground">{settings().last_sync_summary}</Text>
          </SunkenBox>
        </If>

        <If when={showRecoveryPhrase()}>
          <SunkenBox>
            <Text class="text-xs font-semibold text-vital-steady">
              个人保险箱 12 词恢复码（请抄录或打印，官方无法找回）
            </Text>
            <Text class="text-sm font-semibold text-foreground">{settings().recovery_phrase}</Text>
            <ActionButton
              title="复制恢复码"
              variant="secondary"
              onPress={() => void copyToClipboard(settings().recovery_phrase, "12 词恢复码")}
            />
          </SunkenBox>
        </If>

        <If when={Boolean(getEncryptedBatchPreview())}>
          <SunkenBox>
            <Text class="text-xs font-semibold text-primary">设备隔离追加分片与 AEAD 验签往返成功</Text>
            <Text class="text-xs text-muted-foreground">{getEncryptedBatchPreview()!}</Text>
          </SunkenBox>
        </If>
      </Card>

      {/* 2. AI 引擎与隐私模式（+ 本机/局域网硬件能力探针） */}
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

        <ActionButton
          title="探测本机与局域网算力"
          variant="outline"
          onPress={() => probeCapabilities()}
        />

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
              placeholder="http://127.0.0.1:11434/v1"
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
              placeholder="qwen3:latest"
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
            title="测试 08:30 早安通知"
            variant="outline"
            onPress={() => void sendMorningTestNotification()}
          />
          <ActionButton
            title="导出 .qivault 加密归档"
            variant="secondary"
            onPress={() => generateEncryptedArchive()}
          />
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
            title="隐私政策与非医疗声明"
            variant="outline"
            onPress={() => showPrivacySheet.set(true)}
          />
          <ActionButton
            title="开源致谢 NOTICE"
            variant="outline"
            onPress={() => showNoticeSheet.set(true)}
          />
        </View>

        <If when={Boolean(getEncryptedArchivePreview())}>
          <SunkenBox>
            <Text class="text-xs font-semibold text-primary">.qivault 整库加密归档已生成（XChaCha20-Poly1305）</Text>
            <Text class="text-xs text-muted-foreground">{getEncryptedArchivePreview()!.slice(0, 260)}</Text>
            <ActionButton
              title="复制完整 .qivault JSON"
              variant="secondary"
              onPress={() => void copyToClipboard(getEncryptedArchivePreview()!, ".qivault 加密备份")}
            />
          </SunkenBox>
        </If>

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

      {/* 12 词跨生态换机恢复抽屉 */}
      <SheetDialog
        open={showRestoreSheet()}
        title="12 词跨生态换机一键恢复"
        onClose={() => showRestoreSheet.set(false)}
      >
        <Card>
          <Text class="text-xs leading-relaxed text-muted-foreground">
            输入在旧手机保存的 12 词恢复码，选择恢复源（iCloud 共享目录、BYO-Server 控制面或 .qivault 单文件 JSON），端侧自动解密合并。
          </Text>
          <View class="flex-row flex-wrap gap-2">
            <SelectChip
              label="iCloud / 本地目录"
              selected={restoreMode() === "icloud_drive"}
              onPress={() => restoreMode.set("icloud_drive")}
            />
            <SelectChip
              label="BYO-Server 中继"
              selected={restoreMode() === "byo_server"}
              onPress={() => restoreMode.set("byo_server")}
            />
            <SelectChip
              label=".qivault JSON"
              selected={restoreMode() === "archive"}
              onPress={() => restoreMode.set("archive")}
            />
          </View>
          <FieldInput
            value={restorePhraseInput()}
            placeholder="输入 12 个英文恢复词（空格分隔）"
            onChange={(v) => restorePhraseInput.set(v)}
          />
          <FieldInput
            value={restoreSourceInput()}
            multiline={restoreMode() === "archive"}
            placeholder={
              restoreMode() === "archive"
                ? "粘贴完整 .qivault JSON 内容"
                : "留空默认当前设置路径，或填入节点 URL / 本地容器路径"
            }
            onChange={(v) => restoreSourceInput.set(v)}
          />
          <ActionButton
            title="立即解密并重建健康库"
            onPress={() => {
              const ok = restoreFromRecoveryPhrase(
                restorePhraseInput(),
                restoreMode(),
                restoreSourceInput(),
              );
              if (ok) showRestoreSheet.set(false);
            }}
          />
        </Card>
      </SheetDialog>

      {/* 隐私政策与非医疗声明弹出页 */}
      <SheetDialog
        open={showPrivacySheet()}
        title="隐私保护白皮书与非医疗声明"
        onClose={() => showPrivacySheet.set(false)}
      >
        <Card>
          <Text class="text-sm font-semibold text-foreground">1. 个人健康数据零云端明文留存</Text>
          <Text class="text-xs leading-relaxed text-muted-foreground">
            所有生理基线、饮食记录与体检报告默认保存在你的本地加密 SQLite；跨端同步全部采用本地 12 词 HKDF-SHA256 + XChaCha20-Poly1305 端到端加密后才写入你的 iCloud、WebDAV 或中继信封表。
          </Text>
          <Text class="text-sm font-semibold text-foreground">2. AI 与 Apple 5.1.2(i) 按需授权</Text>
          <Text class="text-xs leading-relaxed text-muted-foreground">
            使用本地 Apple Vision OCR 与本地大模型时零字节出机；仅在你明确同意后才允许将端侧脱敏片段发往 ZDR 无留存网关。
          </Text>
          <Text class="text-sm font-semibold text-foreground">3. 非医疗器械免责声明</Text>
          <Text class="text-xs leading-relaxed text-muted-foreground">{getDisclaimerText()}</Text>
        </Card>
      </SheetDialog>

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
            此操作将立即从本机 SQLite 彻底擦除所有生理指标、饮食、实验与体检记录（若已连接控制面也将同步触发账号及信封物理销毁），并重置引导状态，符合 Apple 5.1.1(v) 规范。
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
