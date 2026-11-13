import { mount, Scroll, signal, Text, token, View, windowSize } from "nsh";
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
  copyToClipboard,
  cycleAppearance,
  cycleVisualTheme,
  getAppearancePref,
  getDewSummary,
  getShowDailyPoster,
  getShowQuickCaptureModal,
  getShowZdrConsentModal,
  getState,
  getTab,
  getToast,
  getVisualTheme,
  getVisualThemeMeta,
  logNaturalIntake,
  setShowDailyPoster,
  setShowQuickCaptureModal,
  setTab,
} from "./store";
import {
  ActionButton,
  BottomTabBar,
  Card,
  FieldInput,
  ModalSurface,
  NavRail,
  SheetDialog,
  SunkenBox,
  ToastBanner,
  VitalBadge,
} from "./ui/components";
import { QiIcon, type QiIconName } from "./ui/icons";
import { DailyPosterModal } from "./ui/sanctuary";

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

  const snap = () => getState().snapshot;
  const settings = () => getState().settings;
  const themeMeta = () => getVisualThemeMeta(getVisualTheme());

  return (
    <View
      class="flex-1 bg-background"
      style={{
        paddingTop: "env(safe-area-inset-top)",
        paddingLeft: "env(safe-area-inset-left)",
        paddingRight: "env(safe-area-inset-right)",
      }}
    >
      {/* 顶部宋韵手作陪伴品牌栏 */}
      <View class="flex-col gap-2 border-b border-border bg-card px-4 py-2.5 sm:flex-row sm:items-center sm:justify-between">
        <View class="gap-0.5 sm:flex-1">
          <View class="flex-row items-center gap-2">
            <Text role="header" class="text-lg font-bold text-foreground">
              知气 · Qi
            </Text>
            <View class="flex-row items-center gap-1 rounded-full border border-vital-prime/30 bg-vital-prime-soft px-2 py-0.5">
              <QiIcon name="user_shield" size={12} color={token("vital-prime")} />
              <Text class="text-[11px] font-semibold text-vital-prime">隐私盾</Text>
            </View>
          </View>
          <Text class="text-xs text-muted-foreground">{syncSummary()}</Text>
        </View>

        <View class="flex-row flex-wrap items-center justify-between gap-1.5 sm:justify-end">
          <ActionButton
            title={`画风：${themeMeta().shortLabel}`}
            icon="palette"
            variant="outline"
            size="sm"
            shape="pill"
            accessibilityLabel={`切换灵境画风，当前${themeMeta().shortLabel}`}
            onPress={() => cycleVisualTheme()}
          />
          <ActionButton
            title="速记"
            icon="plus"
            size="sm"
            shape="pill"
            accessibilityLabel="打开一句话饮食速记"
            onPress={() => setShowQuickCaptureModal(true)}
          />
          <ActionButton
            title={appearanceLabel()}
            icon={appearanceIcon()}
            variant="secondary"
            size="sm"
            shape="pill"
            accessibilityLabel={`切换外观主题，当前${appearanceLabel()}`}
            onPress={() => cycleAppearance()}
          />
        </View>
      </View>

      {/* 轻量 Toast 反馈条 */}
      <If when={Boolean(getToast())}>
        <ToastBanner message={getToast()!} onDismiss={() => clearToast()} />
      </If>

      {/* 主体：桌面端左侧滑动导航栏（>= 920pt） vs 移动端单栏视口 */}
      <View class="flex-1 flex-row">
        <If when={isDesktop()}>
          <NavRail
            title="节律导航"
            items={TABS}
            activeId={getTab()}
            onSelect={(id) => setTab(id)}
            footer={
              <SunkenBox>
                <View class="flex-row items-center gap-1.5">
                  <QiIcon name="cloud_lock" size={14} color={token("primary")} />
                  <Text class="text-xs font-semibold text-foreground">端侧私密计算中</Text>
                </View>
                <Text class="text-[11px] leading-relaxed text-muted-foreground">
                  个人生理基线、饮食随记与体检档案均在本机守护，零强制上传。
                </Text>
              </SunkenBox>
            }
          />
        </If>

        <If when={getTab() === "today"}>
          <Scroll class="flex-1" contentStyle={{ padding: 16, paddingBottom: 24, gap: 16 }}>
            <View class={`mx-auto w-full gap-4 ${isDesktop() ? "max-w-3xl" : "max-w-xl"}`}>
              <TodayPage />
            </View>
          </Scroll>
        </If>
        <If when={getTab() === "log"}>
          <Scroll class="flex-1" contentStyle={{ padding: 16, paddingBottom: 24, gap: 16 }}>
            <View class={`mx-auto w-full gap-4 ${isDesktop() ? "max-w-3xl" : "max-w-xl"}`}>
              <LogLabsPage />
            </View>
          </Scroll>
        </If>
        <If when={getTab() === "insights"}>
          <Scroll class="flex-1" contentStyle={{ padding: 16, paddingBottom: 24, gap: 16 }}>
            <View class={`mx-auto w-full gap-4 ${isDesktop() ? "max-w-3xl" : "max-w-xl"}`}>
              <InsightsPage />
            </View>
          </Scroll>
        </If>
        <If when={getTab() === "me"}>
          <Scroll class="flex-1" contentStyle={{ padding: 16, paddingBottom: 24, gap: 16 }}>
            <View class={`mx-auto w-full gap-4 ${isDesktop() ? "max-w-3xl" : "max-w-xl"}`}>
              <MeVaultPage />
            </View>
          </Scroll>
        </If>
      </View>

      {/* 移动端底部 4-Tab 原生滑动导航栏（< 920pt 显示，非绝对定位防遮挡） */}
      <If when={!isDesktop()}>
        <BottomTabBar items={TABS} activeId={getTab()} onSelect={(id) => setTab(id)} />
      </If>

      {/* 冷启动诗意引导 */}
      <OnboardingModal />

      {/* 小红书风今日知气日签海报 */}
      <DailyPosterModal
        open={getShowDailyPoster()}
        themeId={getVisualTheme()}
        targetDate={snap().target_date}
        score={snap().qi_score}
        state={snap().state}
        stateBadge={snap().state_badge}
        sleepHours={snap().sleep_last_night_hours}
        headline={snap().headline}
        microAction={snap().micro_action}
        dewPoints={getDewSummary().points}
        checkedAction={settings().checked_action_today}
        onCopyCaption={(caption) => void copyToClipboard(caption, "今日知气日签")}
        onClose={() => setShowDailyPoster(false)}
      />

      {/* 全局 + 速记抽屉 */}
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
      <ModalSurface
        open={getShowZdrConsentModal()}
        role="alertdialog"
        presentation="formSheet"
        onClose={() => cancelZdrConsent()}
      >
        <View class="flex-1 justify-center gap-4 px-5">
          <Card>
            <VitalBadge tone="prime_ready" label="零留存隐私授权" />
            <Text role="header" class="text-lg font-bold text-foreground">
              首次使用云端 AI 解读授权确认
            </Text>
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
      </ModalSurface>
    </View>
  );
}

mount(App);
