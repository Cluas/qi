import { signal, Text, token, View } from "nsh";
import {
  connectCloudSession,
  copyToClipboard,
  generateEncryptedArchive,
  generateJsonlExport,
  getCloudEntitlements,
  getDisclaimerText,
  getEncryptedArchivePreview,
  getJsonlExportPreview,
  getNoticeText,
  getState,
  restoreFromRecoveryPhrase,
  runE2eeSyncNow,
  sendMorningTestNotification,
  setShowQaStudio,
  toggleMorningPush,
  updateAiConfig,
  updateCloudUrlAndHandle,
  updateSyncConfig,
  wipeVaultAndReset,
} from "../store";
import {
  ActionButton,
  Card,
  CollapsibleCard,
  FieldInput,
  SectionHeader,
  SelectChip,
  SheetDialog,
  SunkenBox,
  SwitchRow,
  VitalBadge,
} from "../ui/components";
import { QiIcon } from "../ui/icons";
import { QaStudioSheet } from "./QaStudioSheet";

const SYNC_MODES = [
  { id: "icloud_drive", label: "iCloud 个人云" },
  { id: "webdav", label: "WebDAV 网盘" },
  { id: "byo_server", label: "私有同步节点" },
  { id: "local_only", label: "纯本地离线" },
];

const AI_MODES = [
  { id: "official_zdr_cloud", label: "零留存云端 AI" },
  { id: "on_device_local", label: "纯本机离线模型" },
  { id: "lan_ollama", label: "家庭局域网 Ollama" },
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
  const cloudAccountOpen = signal(false);

  const settings = () => getState().settings;
  const ent = () => getCloudEntitlements();

  return (
    <View class="gap-4">
      {/* 1. 个人云端到端加密同步中心 */}
      <Card>
        <SectionHeader
          title="个人云端到端加密同步中心"
          subtitle="健康记录与体检档案仅存本机或你的个人云，密钥只归你所有"
          right={<VitalBadge tone="prime_ready" label="端到端加密" />}
        />

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
            <Text class="text-xs font-medium text-foreground">私有同步节点地址</Text>
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
            title="立即加密同步"
            icon="cloud_lock"
            onPress={() => runE2eeSyncNow()}
            class="flex-1"
          />
          <ActionButton
            title="跨设备恢复…"
            icon="refresh"
            variant="outline"
            onPress={() => {
              restorePhraseInput.set(settings().recovery_phrase);
              showRestoreSheet.set(true);
            }}
          />
        </View>

        <ActionButton
          title={showRecoveryPhrase() ? "隐藏 12 词恢复密钥" : "查看 12 词恢复密钥"}
          icon="user_shield"
          variant="secondary"
          onPress={() => showRecoveryPhrase.set(!showRecoveryPhrase())}
        />

        <If when={Boolean(settings().last_sync_summary)}>
          <SunkenBox>
            <Text class="text-xs font-semibold text-primary">同步状态</Text>
            <Text class="text-xs text-muted-foreground">{settings().last_sync_summary}</Text>
          </SunkenBox>
        </If>

        <If when={showRecoveryPhrase()}>
          <SunkenBox>
            <Text class="text-xs font-semibold text-vital-steady">
              个人保险箱 12 词恢复密钥（请妥善抄录保存，官方无法找回）
            </Text>
            <Text class="text-sm font-semibold text-foreground">{settings().recovery_phrase}</Text>
            <ActionButton
              title="复制恢复密钥"
              variant="secondary"
              onPress={() => void copyToClipboard(settings().recovery_phrase, "12 词恢复密钥")}
            />
          </SunkenBox>
        </If>
      </Card>

      {/* 2. AI 引擎与隐私边界选择 */}
      <Card>
        <SectionHeader
          title="AI 引擎与隐私边界"
          subtitle="自由选择零留存云端解读、纯本机离线模型或家庭局域网算力"
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

        <If when={settings().ai_mode === "official_zdr_cloud"}>
          <SwitchRow
            title="允许脱敏片段通过零留存云端解读"
            subtitle="开启前与每次上传前都会先在本机遮盖姓名与证件信息；关闭则按需询问"
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

        <If when={settings().ai_mode === "on_device_local"}>
          <SunkenBox>
            <Text class="text-xs leading-relaxed text-muted-foreground">
              当前已选纯本机离线模型：所有简报润色都在本机完成，零数据离开设备。
            </Text>
          </SunkenBox>
        </If>

        <If when={settings().ai_mode === "lan_ollama"}>
          <SunkenBox>
            <Text class="text-xs font-medium text-foreground">家庭局域网 Ollama 地址与模型</Text>
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
      </Card>

      {/* 3. 每日 08:30 早安提醒 */}
      <Card>
        <SectionHeader title="每日早安提醒" subtitle="温和唤醒今日节律，不制造高频打扰" />
        <SwitchRow
          title={`每日 ${settings().morning_push_time} 早安节律提醒`}
          subtitle="在本机计算完成后准时轻推今日微行动"
          checked={settings().morning_push_enabled}
          onChange={(checked) => toggleMorningPush(checked)}
        />
        <ActionButton
          title="预览 08:30 早安提醒"
          icon="sun"
          variant="outline"
          onPress={() => void sendMorningTestNotification()}
        />
      </Card>

      {/* 可折叠控制面账号卡 */}
      <CollapsibleCard
        title="控制面账号与云端配额"
        subtitle="仅用于跨设备密文信封绑定与配额查看，不存健康明文"
        badgeLabel={ent()?.plan_tier === "pro" ? "Pro" : settings().cloud_token ? "已连接" : "未登录"}
        badgeTone={ent()?.plan_tier === "pro" ? "prime_ready" : "neutral"}
        open={cloudAccountOpen()}
        onToggle={() => cloudAccountOpen.set(!cloudAccountOpen())}
      >
        <FieldInput
          value={settings().cloud_url}
          placeholder="服务节点地址，如 http://127.0.0.1:8788"
          onChange={(v) => updateCloudUrlAndHandle(v, settings().cloud_handle)}
        />
        <FieldInput
          value={settings().cloud_handle}
          placeholder="账号邮箱或标识"
          onChange={(v) => updateCloudUrlAndHandle(settings().cloud_url, v)}
        />
        <ActionButton
          title="连接控制面账号"
          variant="outline"
          onPress={() => void connectCloudSession(settings().cloud_url, settings().cloud_handle)}
        />
        <If when={ent() !== null}>
          <SunkenBox>
            <Text class="text-xs font-semibold text-primary">
              {`当前状态：${ent()!.plan_tier.toUpperCase()} · 账期 ${ent()!.month}`}
            </Text>
            <Text class="text-xs text-muted-foreground">
              {ent()!.lab_parses_remaining < 0
                ? "本月云端深度解读：无限制"
                : `本月云端深度解读剩余：${ent()!.lab_parses_remaining} 次（已用 ${ent()!.lab_parses_used} 次）`}
            </Text>
          </SunkenBox>
        </If>
      </CollapsibleCard>

      {/* 4. 数据导出、合规说明与物理清空 */}
      <Card>
        <SectionHeader title="数据导出与隐私合规" subtitle="你随时可以带走全部记录，或彻底擦除本机数据" />

        <View class="flex-row flex-wrap gap-2">
          <ActionButton
            title="导出 .qivault 加密归档"
            variant="secondary"
            onPress={() => generateEncryptedArchive()}
            class="flex-1"
          />
          <ActionButton
            title="导出 JSONL 摘要"
            variant="secondary"
            onPress={() => generateJsonlExport()}
            class="flex-1"
          />
        </View>

        <View class="flex-row flex-wrap gap-2">
          <ActionButton
            title="隐私政策与非医疗声明"
            variant="outline"
            onPress={() => showPrivacySheet.set(true)}
            class="flex-1"
          />
          <ActionButton
            title="开源致谢 NOTICE"
            variant="outline"
            onPress={() => showNoticeSheet.set(true)}
            class="flex-1"
          />
        </View>

        <If when={Boolean(getEncryptedArchivePreview())}>
          <SunkenBox>
            <Text class="text-xs font-semibold text-primary">.qivault 整库加密归档已就绪</Text>
            <Text class="text-xs text-muted-foreground">{getEncryptedArchivePreview()!.slice(0, 220)}</Text>
            <ActionButton
              title="复制完整 .qivault 备份"
              variant="secondary"
              onPress={() => void copyToClipboard(getEncryptedArchivePreview()!, ".qivault 加密备份")}
            />
          </SunkenBox>
        </If>

        <If when={Boolean(getJsonlExportPreview())}>
          <SunkenBox>
            <Text class="text-xs font-semibold text-foreground">开放标准 JSONL 个人摘要</Text>
            <Text class="text-xs text-muted-foreground">{getJsonlExportPreview()!}</Text>
          </SunkenBox>
        </If>

        <ActionButton
          title="物理清空本机健康数据…"
          variant="danger"
          onPress={() => showWipeConfirm.set(true)}
        />
      </Card>

      {/* 5. 知气验真与数据工坊专属入口 */}
      <Card>
        <View class="flex-row items-center gap-3">
          <View class="size-10 items-center justify-center rounded-xl bg-accent">
            <QiIcon name="sliders" size={20} color={token("primary")} />
          </View>
          <View class="flex-1 gap-0.5">
            <Text role="header" class="text-base font-semibold text-foreground">
              知气验真与数据工坊
            </Text>
            <Text class="text-xs leading-relaxed text-muted-foreground">
              面向验收与开发者：快速切换纵向测试画像、执行数据完整性自检与端侧能力诊断
            </Text>
          </View>
        </View>
        <ActionButton
          title="打开 QA 测试数据与诊断工坊…"
          icon="sliders"
          variant="outline"
          onPress={() => setShowQaStudio(true)}
        />
      </Card>

      {/* QA 验真工坊抽屉 */}
      <QaStudioSheet />

      {/* 跨设备恢复抽屉 */}
      <SheetDialog
        open={showRestoreSheet()}
        title="12 词跨设备恢复"
        onClose={() => showRestoreSheet.set(false)}
      >
        <Card>
          <Text class="text-xs leading-relaxed text-muted-foreground">
            输入在旧设备保存的 12 词恢复密钥，选择恢复源后将在本机自动解密并重建你的健康库。
          </Text>
          <View class="flex-row flex-wrap gap-2">
            <SelectChip
              label="iCloud / 本地目录"
              selected={restoreMode() === "icloud_drive"}
              onPress={() => restoreMode.set("icloud_drive")}
            />
            <SelectChip
              label="私有节点"
              selected={restoreMode() === "byo_server"}
              onPress={() => restoreMode.set("byo_server")}
            />
            <SelectChip
              label=".qivault 文件"
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
                ? "粘贴完整 .qivault 内容"
                : "留空使用默认路径，或填入节点地址"
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

      {/* 隐私保护白皮书抽屉 */}
      <SheetDialog
        open={showPrivacySheet()}
        title="隐私保护白皮书与非医疗声明"
        onClose={() => showPrivacySheet.set(false)}
      >
        <Card>
          <Text class="text-sm font-semibold text-foreground">1. 个人健康数据零云端明文留存</Text>
          <Text class="text-xs leading-relaxed text-muted-foreground">
            所有生理基线、饮食记录与体检报告默认保存在你的本地加密存储；跨端同步均先由你专属的 12 词密钥端到端加密后才写入你的个人云空间。
          </Text>
          <Text class="text-sm font-semibold text-foreground">2. AI 按需隐私护栏</Text>
          <Text class="text-xs leading-relaxed text-muted-foreground">
            使用本机离线识图与本地模型时零字节出机；仅在你明确同意后才允许将脱敏后的化验文本片段发往零留存云端引擎。
          </Text>
          <Text class="text-sm font-semibold text-foreground">3. 非医疗器械免责声明</Text>
          <Text class="text-xs leading-relaxed text-muted-foreground">{getDisclaimerText()}</Text>
        </Card>
      </SheetDialog>

      {/* 开源致谢抽屉 */}
      <SheetDialog
        open={showNoticeSheet()}
        title="开源致谢与 Apache-2.0 NOTICE"
        onClose={() => showNoticeSheet.set(false)}
      >
        <Card>
          <Text class="text-sm leading-relaxed text-foreground">{getNoticeText()}</Text>
        </Card>
      </SheetDialog>

      {/* 物理清空二次确认抽屉 */}
      <SheetDialog
        open={showWipeConfirm()}
        role="alertdialog"
        title="确认物理清空本机健康数据"
        onClose={() => showWipeConfirm.set(false)}
      >
        <Card>
          <Text class="text-sm leading-relaxed text-foreground">
            此操作将立即从本机彻底擦除所有生理指标、饮食、微习惯实验与体检档案记录，并重置引导流程，完成后无法撤销。
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
