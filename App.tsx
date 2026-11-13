import { mount, Scroll, Text, View } from "nsh";
import { Button, Dialog } from "nsh/base";
import { LogLabsPage } from "./pages/LogLabsPage";
import { MeVaultPage } from "./pages/MeVaultPage";
import { OnboardingModal } from "./pages/OnboardingModal";
import { TodayPage } from "./pages/TodayPage";
import {
  type ActiveTab,
  cancelZdrConsent,
  clearToast,
  confirmZdrConsent,
  getShowZdrConsentModal,
  getState,
  getTab,
  getToast,
  setTab,
} from "./store";
import { ActionButton, Card, SunkenBox, VitalBadge } from "./ui/components";

const TABS: { id: ActiveTab; label: string }[] = [
  { id: "today", label: "今日元气" },
  { id: "log", label: "记录与体检" },
  { id: "me", label: "我的" },
];

function App() {
  const syncSummary = () => {
    const mode = getState().settings.sync_mode;
    switch (mode) {
      case "icloud_drive":
        return "iCloud 个人云";
      case "webdav":
        return "WebDAV 网盘";
      case "byo_server":
        return "BYO-Server";
      default:
        return "纯本地离线";
    }
  };

  return (
    <View
      class="flex-1 bg-background"
      style={{
        paddingTop: "env(safe-area-inset-top)",
        paddingLeft: "env(safe-area-inset-left)",
        paddingRight: "env(safe-area-inset-right)",
      }}
    >
      {/* 顶部宋韵极简栏 */}
      <View class="flex-row items-center justify-between border-b border-border bg-card px-4 py-3">
        <View class="gap-0.5">
          <Text class="text-lg font-bold text-foreground">知气 · Qi</Text>
          <Text class="text-xs text-muted-foreground">{`个人数据主权：${syncSummary()}`}</Text>
        </View>
        <VitalBadge tone="prime_ready" label="零隐私库" />
      </View>

      {/* 轻量 Toast 反馈条 */}
      <If when={Boolean(getToast())}>
        <View class="mx-4 mt-2 flex-row items-center justify-between gap-2 rounded-xl border border-primary/30 bg-accent px-3.5 py-2">
          <Text class="flex-1 text-xs font-medium text-accent-foreground">{getToast()!}</Text>
          <Button onPress={() => clearToast()} class="px-2 py-1 active:opacity-70">
            <Text class="text-xs font-semibold text-accent-foreground">关闭</Text>
          </Button>
        </View>
      </If>

      {/* 主滚动视口 */}
      <Scroll class="flex-1" contentStyle={{ padding: 16, paddingBottom: 24, gap: 14 }}>
        <View class="mx-auto w-full max-w-xl gap-4">
          <If when={getTab() === "today"}>
            <TodayPage />
          </If>
          <If when={getTab() === "log"}>
            <LogLabsPage />
          </If>
          <If when={getTab() === "me"}>
            <MeVaultPage />
          </If>
        </View>
      </Scroll>

      {/* 底部 3-Tab 导航栏（V1.0 移动端收敛设计） */}
      <View
        class="flex-row items-center gap-2 border-t border-border bg-card px-4 pt-2.5"
        style={{ paddingBottom: "calc(env(safe-area-inset-bottom) + 10px)" }}
      >
        <Each of={TABS} key={(t) => t.id}>
          {(tab) => (
            <Button
              onPress={() => setTab(tab().id)}
              class={`min-h-11 flex-1 items-center justify-center rounded-xl transition-colors duration-200 active:scale-[0.98] motion-reduce:active:scale-100 ${
                getTab() === tab().id ? "bg-primary" : "bg-surface-sunken"
              }`}
            >
              <Text
                class={`text-sm font-semibold ${
                  getTab() === tab().id ? "text-primary-foreground" : "text-foreground"
                }`}
              >
                {tab().label}
              </Text>
            </Button>
          )}
        </Each>
      </View>

      {/* 60 秒无感冷启动引导 */}
      <OnboardingModal />

      {/* Apple 5.1.2(i) 懒加载 AI 显式授权弹窗 */}
      <Dialog
        open={getShowZdrConsentModal()}
        presentation="formSheet"
        onOpenChange={(e) => {
          if (!e.open) cancelZdrConsent();
        }}
        class="flex-1 bg-background"
      >
        <View
          class="flex-1 justify-center gap-4 px-5"
          style={{
            paddingTop: "calc(env(safe-area-inset-top) + 16px)",
            paddingBottom: "calc(env(safe-area-inset-bottom) + 16px)",
          }}
        >
          <Card>
            <VitalBadge tone="prime_ready" label="ZDR 零留存授权 (5.1.2i)" />
            <Text class="text-lg font-bold text-foreground">首次使用云端 AI 解读授权确认</Text>
            <Text class="text-sm leading-relaxed text-muted-foreground">
              仅在你主动触发云端识图或体检解读时，本应用会先在手机端遮盖姓名、身份证号与 EXIF 信息，再将脱敏片段通过 Zero-Data-Retention 网关发往已备案大模型，阅后即焚且绝不用于模型训练。
            </Text>
            <SunkenBox>
              <Text class="text-xs text-foreground">
                你也可以随时在「我的 → AI 引擎」切换为“纯本机离线大模型”或“家庭局域网 Ollama”，实现数据 100% 不出机。
              </Text>
            </SunkenBox>
            <View class="flex-row gap-2">
              <ActionButton
                title="暂不授权"
                variant="secondary"
                onPress={() => cancelZdrConsent()}
                class="flex-1"
              />
              <ActionButton
                title="同意并继续"
                onPress={() => confirmZdrConsent()}
                class="flex-1"
              />
            </View>
          </Card>
        </View>
      </Dialog>
    </View>
  );
}

mount(App);
