import { mount, Scroll, signal, Text, token, View, windowSize } from "nsh";
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
import { QiIcon, type QiIconName } from "./ui/icons";

const TABS: { id: ActiveTab; label: string; icon: QiIconName }[] = [
  { id: "today", label: "今日", icon: "sun" },
  { id: "log", label: "记录", icon: "record" },
  { id: "insights", label: "洞察", icon: "compass" },
  { id: "me", label: "我的", icon: "user_shield" },
];

function App() {
  const quickText = signal("");
  const isDesktop = () => windowSize().width >= 920;

  const syncSummary = () => {
    const mode = getState().settings.sync_mode;
    switch (mode) {
      case "icloud_drive":
        return "iCloud 个人云 · 隐私守护";
      case "webdav":
        return "WebDAV 网盘 · 端到端加密";
      case "byo_server":
        return "私有同步节点 · 端到端加密";
      default:
        return "纯本地离线 · 仅留本机";
    }
  };

  const appearanceLabel = () => {
    switch (getAppearancePref()) {
      case "light":
        return "宣纸";
      case "dark":
        return "松烟";
      default:
        return "自动";
    }
  };

  const appearanceIcon = (): QiIconName => {
    return getAppearancePref() === "dark" ? "moon" : "sun";
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
      {/* 顶部宋韵品牌栏 */}
      <View class="flex-row items-center justify-between gap-2 border-b border-border bg-card px-4 py-2.5">
        <View class="flex-1 gap-0.5">
          <View class="flex-row items-center gap-2">
            <Text class="text-lg font-bold text-foreground">知气 · Qi</Text>
            <View class="flex-row items-center gap-1 rounded-full border border-vital-prime/30 bg-vital-prime-soft px-2 py-0.5">
              <QiIcon name="user_shield" size={12} color={token("vital-prime")} />
              <Text class="text-[11px] font-semibold text-vital-prime">隐私盾</Text>
            </View>
          </View>
          <Text class="text-xs text-muted-foreground">{syncSummary()}</Text>
        </View>

        <View class="flex-row items-center gap-2">
          <Button
            hitSlop={4}
            onPress={() => setShowQuickCaptureModal(true)}
            class="min-h-10 flex-row items-center justify-center gap-1.5 rounded-full bg-primary px-3.5 py-1.5 transition active:scale-[0.98] motion-reduce:active:scale-100"
          >
            <QiIcon name="plus" size={14} color={token("primary-foreground")} />
            <Text class="text-xs font-semibold text-primary-foreground">速记</Text>
          </Button>
          <Button
            hitSlop={4}
            onPress={() => cycleAppearance()}
            class="min-h-10 flex-row items-center justify-center gap-1.5 rounded-full border border-border bg-surface-sunken px-3 py-1.5 transition active:opacity-80"
          >
            <QiIcon name={appearanceIcon()} size={14} color={token("foreground")} />
            <Text class="text-xs font-medium text-foreground">{appearanceLabel()}</Text>
          </Button>
        </View>
      </View>

      {/* 轻量 Toast 反馈条 */}
      <If when={Boolean(getToast())}>
        <View class="mx-4 mt-2 flex-row items-center justify-between gap-2 rounded-xl border border-primary/30 bg-accent px-3.5 py-2">
          <Text class="flex-1 text-xs font-medium text-accent-foreground">{getToast()!}</Text>
          <Button onPress={() => clearToast()} class="min-h-8 justify-center px-2 py-1 active:opacity-70">
            <Text class="text-xs font-semibold text-accent-foreground">关闭</Text>
          </Button>
        </View>
      </If>

      {/* 主体：桌面端左侧边栏（>= 920pt） vs 移动端单栏视口 */}
      <View class="flex-1 flex-row">
        <If when={isDesktop()}>
          <View class="w-56 gap-2 border-r border-border bg-card p-3">
            <Text class="px-2 pb-1 text-xs font-semibold text-muted-foreground">节律导航</Text>
            <Each of={TABS} key={(t) => `side-${t.id}`}>
              {(tab) => (
                <Button
                  onPress={() => setTab(tab().id)}
                  class={`min-h-11 flex-row items-center gap-2.5 rounded-xl px-3.5 transition-colors duration-200 active:scale-[0.98] motion-reduce:active:scale-100 ${
                    getTab() === tab().id ? "bg-primary" : "bg-surface-sunken"
                  }`}
                >
                  <QiIcon
                    name={tab().icon}
                    size={17}
                    color={getTab() === tab().id ? token("primary-foreground") : token("foreground")}
                  />
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
              <View class="flex-row items-center gap-1.5">
                <QiIcon name="cloud_lock" size={14} color={token("primary")} />
                <Text class="text-xs font-semibold text-foreground">端侧私密计算中</Text>
              </View>
              <Text class="text-[11px] leading-relaxed text-muted-foreground">
                个人生理基线、饮食随记与体检档案均在本机守护，零强制上传。
              </Text>
            </SunkenBox>
          </View>
        </If>

        <Scroll class="flex-1" contentStyle={{ padding: 16, paddingBottom: 24, gap: 16 }}>
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

      {/* 移动端底部 4-Tab 导航栏（< 920pt 显示，非绝对定位防遮挡） */}
      <If when={!isDesktop()}>
        <View
          class="flex-row items-center gap-1.5 border-t border-border bg-card px-3 pt-2"
          style={{ paddingBottom: "calc(env(safe-area-inset-bottom) + 8px)" }}
        >
          <Each of={TABS} key={(t) => `bottom-${t.id}`}>
            {(tab) => (
              <Button
                onPress={() => setTab(tab().id)}
                class={`min-h-11 flex-1 items-center justify-center gap-1 rounded-xl px-1 py-1.5 transition-colors duration-200 active:scale-[0.98] motion-reduce:active:scale-100 ${
                  getTab() === tab().id ? "bg-accent" : "bg-transparent"
                }`}
              >
                <QiIcon
                  name={tab().icon}
                  size={18}
                  color={getTab() === tab().id ? token("primary") : token("muted-foreground")}
                />
                <Text
                  class={`text-[11px] font-semibold ${
                    getTab() === tab().id ? "text-primary" : "text-muted-foreground"
                  }`}
                >
                  {tab().label}
                </Text>
              </Button>
            )}
          </Each>
        </View>
      </If>

      {/* 冷启动诗意引导 */}
      <OnboardingModal />

      {/* 全局 + 速记弹窗 */}
      <SheetDialog
        open={getShowQuickCaptureModal()}
        title="3 秒一句话速记"
        onClose={() => setShowQuickCaptureModal(false)}
      >
        <Card>
          <Text class="text-xs leading-relaxed text-muted-foreground">
            随手输入一句：例如“下午三点喝了生椰拿铁”、“22:30吃了烧烤小酌”、“早餐豆腐脑油条”。系统将在本机自动识别餐食、热量与时间节奏。
          </Text>
          <FieldInput
            value={quickText()}
            placeholder="输入一句餐饮、咖啡、饮酒或补剂记录"
            onChange={(v) => quickText.set(v)}
          />
          <ActionButton
            title="立即解析并记入"
            icon="check"
            onPress={submitGlobalQuickCapture}
          />
        </Card>
      </SheetDialog>

      {/* Apple 5.1.2(i) 懒加载云端 AI 显式授权弹窗 */}
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
            <VitalBadge tone="prime_ready" label="零留存隐私授权" />
            <Text class="text-lg font-bold text-foreground">首次使用云端 AI 解读授权确认</Text>
            <Text class="text-sm leading-relaxed text-muted-foreground">
              仅在你主动触发云端解读时，应用会先在手机端遮盖姓名、身份证号与个人敏感信息，再将脱敏片段通过零留存通道发送，阅后即焚且绝不用于模型训练。
            </Text>
            <SunkenBox>
              <Text class="text-xs leading-relaxed text-foreground">
                你也可以随时在「我的 → AI 引擎与隐私边界」切换为纯本机离线模型或家庭局域网算力，实现数据完全不出机。
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
