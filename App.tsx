import { mount, Scroll, signal, Text, View, windowSize } from "nsh";
import { Button, Dialog } from "nsh/base";
import { InsightsPage } from "./pages/InsightsPage";
import { LogLabsPage } from "./pages/LogLabsPage";
import { MeVaultPage } from "./pages/MeVaultPage";
import { OnboardingModal } from "./pages/OnboardingModal";
import { TodayPage } from "./pages/TodayPage";
import {
  type ActiveTab,
  cancelZdrConsent,
  clearToast,
  confirmZdrConsent,
  cycleAppearance,
  getAppearancePref,
  getShowQuickCaptureModal,
  getShowZdrConsentModal,
  getState,
  getTab,
  getToast,
  logNaturalIntake,
  setShowQuickCaptureModal,
  setTab,
} from "./store";
import { ActionButton, Card, FieldInput, SheetDialog, SunkenBox, VitalBadge } from "./ui/components";

const TABS: { id: ActiveTab; label: string }[] = [
  { id: "today", label: "今日元气" },
  { id: "log", label: "记录与体检" },
  { id: "insights", label: "发现实验" },
  { id: "me", label: "我的" },
];

function App() {
  const quickText = signal("");
  const isDesktop = () => windowSize().width >= 920;

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

  const appearanceLabel = () => {
    switch (getAppearancePref()) {
      case "light":
        return "宣纸浅色";
      case "dark":
        return "松烟深色";
      default:
        return "跟随系统";
    }
  };

  const submitGlobalQuickCapture = () => {
    const text = quickText().trim();
    if (!text) return;
    logNaturalIntake(text);
    quickText.set("");
    setShowQuickCaptureModal(false);
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
      <View class="flex-row items-center justify-between gap-2 border-b border-border bg-card px-4 py-3">
        <View class="flex-1 gap-0.5">
          <Text class="text-lg font-bold text-foreground">知气 · Qi</Text>
          <Text class="text-xs text-muted-foreground">{`个人数据主权：${syncSummary()}`}</Text>
        </View>
        <View class="flex-row items-center gap-2">
          <Button
            onPress={() => setShowQuickCaptureModal(true)}
            class="min-h-9 items-center justify-center rounded-full bg-primary px-3 py-1 active:opacity-85"
          >
            <Text class="text-xs font-semibold text-primary-foreground">+ 速记</Text>
          </Button>
          <Button
            onPress={() => cycleAppearance()}
            class="min-h-9 items-center justify-center rounded-full border border-border bg-surface-sunken px-3 py-1 active:opacity-80"
          >
            <Text class="text-xs font-medium text-foreground">{`外观：${appearanceLabel()}`}</Text>
          </Button>
          <VitalBadge tone="prime_ready" label="零隐私库" />
        </View>
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

      {/* 主体：桌面端左侧边栏（>= 920pt） vs 移动端单栏视口 */}
      <View class="flex-1 flex-row">
        <If when={isDesktop()}>
          <View class="w-56 gap-2 border-r border-border bg-card p-3">
            <Text class="px-2 pb-1 text-xs font-semibold text-muted-foreground">工作区导航</Text>
            <Each of={TABS} key={(t) => `side-${t.id}`}>
              {(tab) => (
                <Button
                  onPress={() => setTab(tab().id)}
                  class={`min-h-11 items-start justify-center rounded-xl px-3.5 transition-colors duration-200 active:scale-[0.98] motion-reduce:active:scale-100 ${
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
            <SunkenBox class="mt-auto">
              <Text class="text-xs font-semibold text-foreground">端侧离线引擎运行中</Text>
              <Text class="text-[11px] leading-relaxed text-muted-foreground">
                Apple Vision OCR 与 Rust 节律计算均在本机完成，无需上传任何健康原表。
              </Text>
            </SunkenBox>
          </View>
        </If>

        <Scroll class="flex-1" contentStyle={{ padding: 16, paddingBottom: 24, gap: 14 }}>
          <View class={`mx-auto w-full gap-4 ${isDesktop() ? "max-w-3xl" : "max-w-xl"}`}>
            <If when={getTab() === "today"}>
              <TodayPage />
            </If>
            <If when={getTab() === "log"}>
              <LogLabsPage />
            </If>
            <If when={getTab() === "insights"}>
              <InsightsPage />
            </If>
            <If when={getTab() === "me"}>
              <MeVaultPage />
            </If>
          </View>
        </Scroll>
      </View>

      {/* 移动端底部 4-Tab 导航栏（< 920pt 显示） */}
      <If when={!isDesktop()}>
        <View
          class="flex-row items-center gap-1.5 border-t border-border bg-card px-3 pt-2.5"
          style={{ paddingBottom: "calc(env(safe-area-inset-bottom) + 10px)" }}
        >
          <Each of={TABS} key={(t) => `bottom-${t.id}`}>
            {(tab) => (
              <Button
                onPress={() => setTab(tab().id)}
                class={`min-h-11 flex-1 items-center justify-center rounded-xl px-1 transition-colors duration-200 active:scale-[0.98] motion-reduce:active:scale-100 ${
                  getTab() === tab().id ? "bg-primary" : "bg-surface-sunken"
                }`}
              >
                <Text
                  class={`text-xs font-semibold ${
                    getTab() === tab().id ? "text-primary-foreground" : "text-foreground"
                  }`}
                >
                  {tab().label}
                </Text>
              </Button>
            )}
          </Each>
        </View>
      </If>

      {/* 60 秒无感冷启动引导 */}
      <OnboardingModal />

      {/* 全局 + 速记弹窗 */}
      <SheetDialog
        open={getShowQuickCaptureModal()}
        title="3 秒一句话速记"
        onClose={() => setShowQuickCaptureModal(false)}
      >
        <Card>
          <Text class="text-xs leading-relaxed text-muted-foreground">
            随便说或打一句：例如“下午三点喝了生椰拿铁”、“22:30吃了烧烤小酌”、“早餐豆腐脑油条”。系统将在 0ms 内提取饮食、热量与时间节律因子。
          </Text>
          <FieldInput
            value={quickText()}
            placeholder="输入一句餐饮、咖啡、饮酒或补剂记录"
            onChange={(v) => quickText.set(v)}
          />
          <ActionButton title="立即智能解析并记入" onPress={submitGlobalQuickCapture} />
        </Card>
      </SheetDialog>

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
