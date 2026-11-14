import { type App, expect, open, test } from "nsh/test";

const FORBIDDEN_BRAND = ["知气", "Qi"].join(" · ");
const FORBIDDEN_LEGACY_RE = new RegExp(
  [
    "\\u516c\\u76ca",
    "\\u6148\\u5584",
    "\\u6350",
    "\\b" + "cha" + "r(?:ity|ities)\\b",
    "\\b" + "don" + "at(?:e|ion|ions)\\b",
  ].join("|"),
  "i",
);

function assertCleanCopy(texts: string[]) {
  const joined = texts.join("\n");
  expect(joined.includes(FORBIDDEN_BRAND)).toBe(false);
  expect(FORBIDDEN_LEGACY_RE.test(joined)).toBe(false);
}

async function scrollIntoView(app: App, target: string, dy: number) {
  try {
    await app.scroll(target, dy);
  } catch (err) {
    if (!String(err).includes("cannot scroll")) throw err;
  }
}

test("知气 v5.0 确定性中文冷启动、极简顶栏、中英双语实时切换与四大空间化页面测试", async () => {
  // Even on an en-US host locale, Qi starts deterministically in zh-CN
  const app = await open("./App.tsx", { locale: "en-US" });

  expect(app.shows("欢迎来到知气")).toBe(true);
  expect(app.shows("知气")).toBe(true);
  expect(app.shows("Tier A 全穿戴")).toBe(false);
  assertCleanCopy(app.texts());

  await app.scroll("欢迎来到知气", 500);
  await app.press("推开窗扉");
  await app.waitFor("窗台已铺设好，开始安安静静记第一天吧");
  expect(app.shows("欢迎来到知气")).toBe(false);
  await app.press("关闭");

  // 1. Top header removed + TodayPage 1 Hero + 1 Focus Card + 1 Disclosure Drawer
  expect(app.shows("知气")).toBe(true);
  expect(app.shows("中 / EN")).toBe(false);
  expect(app.shows("打开体验调试台")).toBe(false);
  expect(app.shows("食光")).toBe(true);
  expect(app.shows("家书")).toBe(true);
  expect(app.shows("同窗")).toBe(true);
  expect(app.shows("伙伴")).toBe(true);
  expect(app.shows("今日关注与一念落印")).toBe(true);
  assertCleanCopy(app.texts());

  await app.press(app.count("顺顺小满毛") > 1 ? "顺顺小满毛 >> nth=1" : "顺顺小满毛");
  await app.waitUntil(() => app.texts().some((t) => t.includes("伸了个懒腰") || t.includes("松软印章")));

  // Sanctuary theme switcher & DailyPosterModal
  await app.press("掌上山水");
  await app.waitFor("养气松石灵境");
  if (app.shows("关闭")) await app.press("关闭");

  await app.press("元气手账");
  await app.waitFor("软胶猫「煤球」");
  if (app.shows("关闭")) await app.press("关闭");

  await app.press("窗畔有猫");
  await app.waitFor("三花猫「小满」");
  if (app.shows("关闭")) await app.press("关闭");

  await app.press(app.count("生成日签") > 1 ? "生成日签 >> nth=0" : "生成日签");
  await app.waitFor("节气日签卡");
  await app.press("收起");
  await app.waitFor("今日关注与一念落印");

  // Spatial Dock -> Co-window & Slow-post Letters
  await app.press("同窗");
  await app.waitFor("同窗共养 · 双人暖窗");
  await app.press("递热茶");
  await app.waitFor("已轻声向同窗窗台寄出这份心意");
  await app.press("完成");
  if (app.shows("关闭")) await app.press("关闭");

  await app.press("家书");
  await app.waitFor("周日夜话慢递");
  await app.press("收好信笺");

  // Progressive Disclosure DrawerSheet (Briefing / Curves & Baselines / Seasonal Guide)
  await app.scroll("今日关注与一念落印", 320);
  await app.press("展开");
  await app.waitFor("今日气韵简报");
  await app.press("曲线与基线");
  await app.waitFor("个人 30 日常模基线");
  await app.press("节气风物志");
  await app.waitFor("展开往期节气博物架");
  await app.press("完成");

  // 2. Toggle zh-CN -> en cleanly via Me -> 隐私与金库 settings card
  await app.press("我的");
  await app.press("隐私与金库");
  await app.waitFor("界面与习惯");
  await app.press("English");
  await app.waitFor("Appearance & language");
  if (app.shows("Close")) await app.press("Close");
  assertCleanCopy(app.texts());

  await app.press("Today");
  await app.waitFor("Qi");
  expect(app.shows("ZH / EN")).toBe(false);
  expect(app.shows("Today's focus & check-in")).toBe(true);
  assertCleanCopy(app.texts());

  // Log SegmentedTabs in English
  await app.press("Log");
  await app.waitFor("Quick log");
  expect(app.shows("Labs & panels")).toBe(true);
  expect(app.shows("Hearth vault")).toBe(true);
  await app.press("Labs & panels");
  await app.waitFor("Lab intake & privacy shield");
  await app.press("Hearth vault");
  await app.waitFor("Hearth vault summary");
  assertCleanCopy(app.texts());

  // Insights SegmentedTabs in English
  await app.press("Insights");
  await app.waitFor("Rhythm curves");
  expect(app.shows("Lag links")).toBe(true);
  expect(app.shows("Season scroll")).toBe(true);
  await app.press("Lag links");
  await app.waitFor("Intake lag correlations");
  await app.press("Season scroll");
  await app.waitFor("Kiln badges & seasonal keepsakes");
  assertCleanCopy(app.texts());

  // Me SegmentedTabs in English
  await app.press("Me");
  await app.press("Pet Studio");
  await app.waitFor("Pet Studio");
  expect(app.shows("Atelier & Billing")).toBe(true);
  expect(app.shows("Vault & Privacy")).toBe(true);
  expect(app.shows("1 custom pet sits free forever — kept intact even if Pro ever expires")).toBe(true);
  assertCleanCopy(app.texts());

  // Switch back to zh-CN inside Me -> Vault & Privacy
  await app.press("Vault & Privacy");
  await app.waitFor("Appearance & language");
  await app.press("简体中文");
  await app.waitFor("界面与习惯");
  expect(app.shows("中 / EN")).toBe(false);
  if (app.shows("关闭")) await app.press("关闭");

  // 3. LogLabs quick NLP + PII lab sample + Insights scroll
  await app.press("记录");
  await app.press("今日随记");
  await app.waitFor("一念自然语记");
  await app.type("例如：中午喝了半杯冰美式，晚饭清淡汤面，十一点前准备休息", "中午喝了黑咖啡");
  await app.press("记入手账");
  await app.waitFor("已记入随笔「中午喝了黑咖啡」");
  await app.press("关闭");

  await app.press("化验与补剂");
  await app.waitFor("化验单采集与隐私脱敏");
  await app.press("注入示例化验单");
  await app.waitFor("启用零留存云端深度梳理？");
  await app.press("同意并梳理");
  await app.waitUntil(() => app.texts().some((t) => t.includes("尿酸") || t.includes("UA")));
  if (app.shows("关闭")) await app.press("关闭");

  await app.press("炉火金库");
  await app.waitFor("炉火金库摘要");

  await app.press("今日随记");
  await app.waitFor("食光贴纸相机 · 人间烟火月历垫");
  await scrollIntoView(app, "食光贴纸相机 · 人间烟火月历垫", 360);
  await app.press("制作并贴入食光月历");
  await app.waitFor("已将「温润家常便当」贴入本月食光月历");
  await app.press("关闭");

  await app.press("洞察");
  await app.press("节律曲线");
  await app.waitFor("近两周节律起伏对比");
  await app.press("四季长卷");
  await app.waitFor("节气烧制徽章 · 陶瓷陈列柜");

  // 4. Clean reset back to onboarding for next test
  await app.press("我的");
  await app.press("隐私与金库");
  await app.waitFor("本地安全金库");
  await scrollIntoView(app, "本地安全金库", 900);
  await scrollIntoView(app, "清空本机手账数据…", 600);
  await app.waitFor("清空本机手账数据…");
  await app.press("清空本机手账数据…");
  await app.waitFor("真的要清空本机手账金库吗？");
  await app.press("确认彻底清空本机数据");
  await app.waitFor("欢迎来到知气");
}, 90000);

test("知气 v5.0 自定陪伴工坊、只读金库零删档、Apple IAP 合规三件套与窗台添柴礼赠券测试", async () => {
  const app = await open("./App.tsx", { locale: "en-US" });

  await app.scroll("欢迎来到知气", 500);
  await app.press("推开窗扉");
  await app.waitFor("窗台已铺设好，开始安安静静记第一天吧");
  await app.press("关闭");

  // 1. Custom Pet Studio: Free=1 forever, Starlight Memorial, quota enforcement
  await app.press("我的");
  await app.waitFor("自定陪伴工坊");
  expect(app.shows("免费版永久拥有 1 个自定陪伴位，到期或未订阅亦永不删档")).toBe(true);

  await app.scroll("自定陪伴工坊", 260);
  await app.type("比如：糯米、阿福、土豆", "糯米");
  await scrollIntoView(app, "安放新陪伴", 220);
  await app.press("设为星光纪念窗台");
  await scrollIntoView(app, "安放新陪伴", 160);
  await app.press("安放至窗台");
  await app.waitFor("「糯米」已在窗台安顿好");
  await app.press("关闭");

  expect(app.shows("已安放 1/1 位陪伴")).toBe(true);

  // Confirm active custom pet renders on Today Sanctuary Dock & pet action (Starlight Memorial mode)
  await app.press("今日");
  await app.waitFor("轻抚糯米");
  await app.press(app.count("轻抚糯米") > 1 ? "轻抚糯米 >> nth=1" : "轻抚糯米");
  await app.waitFor("轻抚留在星光里，糯米好像也轻轻蹭了蹭你的掌心。");

  // Attempt 2nd pet on Free tier -> blocked cleanly by Free=1 cap
  await app.press("我的");
  await app.waitFor("已安放 1/1 位陪伴");
  await scrollIntoView(app, "安放新陪伴", 320);
  await app.type("比如：糯米、阿福、土豆", "土豆");
  await scrollIntoView(app, "安放新陪伴", 360);
  await app.press("安放至窗台");
  await app.waitUntil(() =>
    app.texts().some((t) => t.includes("1/1") && t.includes("知气雅集")),
  );
  await app.press("关闭");

  // 2. Unlock Pro via QA Studio (inside 隐私与金库) -> 3 slots, Ex Libris stamp, weekly first-person letter
  await scrollIntoView(app, "自定陪伴工坊", -600);
  await app.press("隐私与金库");
  await app.waitFor("本地安全金库");
  await scrollIntoView(app, "本地安全金库", 900);
  await scrollIntoView(app, "打开体验调试台", 600);
  await app.press("打开体验调试台");
  await app.waitFor("视口形态与拇指热区预览");
  await scrollIntoView(app, "StoreKit 与会员状态沙盒", 720);
  await app.waitFor("模拟已启封手作坊 Pro");
  await app.press("模拟已启封手作坊 Pro");
  await app.waitFor("active_atelier");
  await app.press("完成");
  if (app.shows("关闭")) await app.press("关闭");

  await scrollIntoView(app, "本地安全金库", -1600);
  await app.press("自定陪伴工坊");
  await app.waitFor("已安放 1/3 位陪伴");
  await scrollIntoView(app, "安放新陪伴", 360);
  await app.clear("text:土豆");
  await app.type("比如：糯米、阿福、土豆", "阿福");
  await scrollIntoView(app, "安放新陪伴", 420);
  await app.type("例如：小满印信 / 糯米藏书", "阿福藏书印");
  await app.press("安放至窗台");
  await app.waitFor("「阿福」已在窗台安顿好");
  await app.press("关闭");

  expect(app.shows("已安放 2/3 位陪伴")).toBe(true);

  await scrollIntoView(app, "陪伴第一人称周信", 300);
  await app.waitFor("拆阅本周来信");
  await app.press("拆阅本周来信");
  await app.waitFor("「阿福」写好了一封新信，快去拆阅吧");
  await app.press("关闭");
  await app.press("读来信");
  await app.waitFor("周日夜话慢递");
  await app.press("收好信笺");

  // 3. Switch to ExpiredReadOnlyVault -> pets remain 100% intact (non-lockout)
  await scrollIntoView(app, "自定陪伴工坊", -800);
  await app.press("隐私与金库");
  await app.waitFor("本地安全金库");
  await scrollIntoView(app, "本地安全金库", 900);
  await scrollIntoView(app, "打开体验调试台", 600);
  await app.press("打开体验调试台");
  await app.waitFor("视口形态与拇指热区预览");
  await scrollIntoView(app, "StoreKit 与会员状态沙盒", 720);
  await app.press("模拟 Pro 到期只读金库");
  await app.waitFor("expired_read_only_vault");
  await app.press("完成");
  if (app.shows("关闭")) await app.press("关闭");

  await scrollIntoView(app, "本地安全金库", -1600);
  await app.press("自定陪伴工坊");
  await app.waitFor("自定陪伴工坊");
  expect(app.shows("糯米")).toBe(true);
  expect(app.shows("阿福")).toBe(true);
  expect(app.shows("即使 Pro 到期转为只读金库，你创建的全部陪伴形象、照片与往期来信都将永久完整保留。")).toBe(true);

  // 4. Atelier & Billing subtab + Commercial Hearth Gift Pass + QiAtelierModal Apple compliance triad
  await app.press("雅集与权益");
  await app.waitFor("窗台添柴礼赠券 · 赠予社区漂流席位");
  expect(app.shows("商业消耗型内购 +¥12")).toBe(true);
  assertCleanCopy(app.texts());

  await app.press("单购窗台添柴礼赠券 (+¥12)");
  await app.waitFor("窗台添柴礼赠券已加入社区漂流席位池");
  await app.press("关闭");
  expect(app.shows("本设备已发出 1 份漂流席位")).toBe(true);

  await app.press("先逛逛");
  await app.waitFor("知气雅集");
  await scrollIntoView(app, "知气雅集", 1100);
  expect(app.shows("恢复购买")).toBe(true);
  expect(app.shows("服务条款 (EULA)")).toBe(true);
  expect(app.shows("隐私政策")).toBe(true);
  expect(app.texts().join("\n").includes("24 小时")).toBe(true);
  assertCleanCopy(app.texts());

  await app.press("服务条款 (EULA)");
  await scrollIntoView(app, "知气雅集", 240);
  await app.waitFor("知气服务条款与最终用户许可协议 (EULA)");
  await app.press("收起正文");

  await app.press("隐私政策");
  await scrollIntoView(app, "知气雅集", 240);
  await app.waitFor("知气隐私政策");
  await app.press("收起正文");

  await app.press("暂不决定 · 返回今日窗台");
  await app.waitFor("雅集与权益");

  // Clean reset at end
  await app.press("隐私与金库");
  await scrollIntoView(app, "本地安全金库", 900);
  await scrollIntoView(app, "清空本机手账数据…", 600);
  await app.waitFor("清空本机手账数据…");
  await app.press("清空本机手账数据…");
  await app.waitFor("真的要清空本机手账金库吗？");
  await app.press("确认彻底清空本机数据");
  await app.waitFor("欢迎来到知气");
}, 90000);

test("知气 v5.5 iOS 竖屏 (393x852 Dynamic Island) 拇指底栏速记抽屉与 WidgetKit 矩阵测试", async () => {
  const app = await open("./App.tsx", {
    width: 393,
    height: 852,
    safeArea: { top: 59, bottom: 34, left: 0, right: 0 },
  });
  await app.waitFor("欢迎来到知气");
  await app.press("推开窗扉");
  await app.waitFor("今日手账");
  if (app.shows("关闭")) await app.press("关闭");

  expect(app.shows("知气")).toBe(true);
  expect(app.shows("中 / EN")).toBe(false);
  expect(app.shows("打开体验调试台")).toBe(false);

  // 1. 拇指弧区 5 槽底栏居中 + 速记打开底部抽屉并一步记录饮食芯片
  await app.waitFor("速记");
  await app.press("速记");
  await app.waitFor("随手速记一刻");
  await app.waitFor("单手一步记");
  await app.press("半杯冰美式");
  await app.waitFor("今日手账");
  if (app.shows("关闭")) await app.press("关闭");

  // 2. 进入 我的 -> 隐私与金库 滚动校验底部 QA 调试台并验证视口切换器按钮可呈现
  await app.press("我的");
  await app.press("隐私与金库");
  await app.waitFor("本地安全金库");
  await scrollIntoView(app, "本地安全金库", 900);
  await scrollIntoView(app, "打开体验调试台", 700);
  await app.waitFor("打开体验调试台");
  await app.press("打开体验调试台");
  await app.waitFor("视口形态与拇指热区预览");
  await app.waitFor("iPhone 竖屏 (402×874)");
  await app.press("完成");
  if (app.shows("关闭")) await app.press("关闭");

  // 3. 切回 雅集与权益 验证内置 WidgetKit 三态小组件矩阵
  await scrollIntoView(app, "本地安全金库", -1600);
  await app.press("雅集与权益");
  await scrollIntoView(app, "iOS 锁屏与桌面小组件预览（WidgetKit）", 320);
  await app.waitFor("锁屏圆环 · 元气");
  await app.waitFor("锁屏微件 · 今日节律");
  await app.waitFor("桌面 2×2 · 灵境便当");
  await app.press("精准数值");
  assertCleanCopy(app.texts());

  // 4. 进 隐私与金库 切至英文验证底部居中 Note 热区与 WidgetKit 双语一致性
  await scrollIntoView(app, "iOS 锁屏与桌面小组件预览（WidgetKit）", -600);
  await app.press("隐私与金库");
  await app.waitFor("界面与习惯");
  await app.press("English");
  await app.waitFor("Note");
  if (app.shows("Close")) await app.press("Close");
  await app.press("Atelier & Billing");
  await scrollIntoView(app, "iOS Lock Screen & Home Widgets (WidgetKit)", 320);
  await app.waitFor("iOS Lock Screen & Home Widgets (WidgetKit)");
  await app.waitFor("Lock Ring · Qi");
  assertCleanCopy(app.texts());

  // 5. 清理回归初始态
  await scrollIntoView(app, "iOS Lock Screen & Home Widgets (WidgetKit)", -600);
  await app.press("Vault & Privacy");
  await app.waitFor("Appearance & language");
  await app.press("简体中文");
  await app.waitFor("界面与习惯");
  if (app.shows("关闭")) await app.press("关闭");
  await scrollIntoView(app, "本地安全金库", 900);
  await scrollIntoView(app, "清空本机手账数据…", 700);
  await app.press("清空本机手账数据…");
  await app.waitFor("真的要清空本机手账金库吗？");
  await app.press("确认彻底清空本机数据");
  await app.waitFor("欢迎来到知气");
}, 90000);

test("知气 v5.6 Apple HealthKit 原生直连卡片与炉火金库双语入口测试", async () => {
  const app = await open("./App.tsx", { locale: "en-US" });

  await app.waitFor("欢迎来到知气");
  await app.scroll("欢迎来到知气", 500);
  await app.press("推开窗扉");
  await app.waitFor("窗台已铺设好，开始安安静静记第一天吧");
  await app.press("关闭");

  // 1. 中文模式下校验 我的 › 隐私与金库 与 记录 › 炉火金库 的 HealthKit 卡片
  await app.press("我的");
  await app.press("隐私与金库");
  await app.waitFor("界面与习惯");
  await scrollIntoView(app, "界面与习惯", 300);
  await app.waitFor("Apple 健康 (HealthKit)");
  expect(app.shows("本机只读")).toBe(true);
  assertCleanCopy(app.texts());

  await app.press("记录");
  await app.press("炉火金库");
  await app.waitFor("Apple 健康 (HealthKit) 一键直连");
  expect(app.shows("本机只读")).toBe(true);
  assertCleanCopy(app.texts());

  // 2. 切换至英文校验 Me › Vault & Privacy 与 Log › Hearth vault
  await app.press("我的");
  await app.press("隐私与金库");
  await app.waitFor("界面与习惯");
  await app.press("English");
  await app.waitFor("Appearance & language");
  if (app.shows("Close")) await app.press("Close");
  await scrollIntoView(app, "Appearance & language", 300);
  await app.waitFor("Apple HealthKit");
  expect(app.shows("Read-only")).toBe(true);
  assertCleanCopy(app.texts());

  await app.press("Log");
  await app.press("Hearth vault");
  await app.waitFor("Apple Health direct sync");
  expect(app.shows("Read-only")).toBe(true);
  assertCleanCopy(app.texts());

  // 3. 切回简体中文并清空本机手账数据回归初始态
  await app.press("Me");
  await app.press("Vault & Privacy");
  await app.waitFor("Appearance & language");
  await app.press("简体中文");
  await app.waitFor("界面与习惯");
  if (app.shows("关闭")) await app.press("关闭");
  await scrollIntoView(app, "本地安全金库", 900);
  await scrollIntoView(app, "清空本机手账数据…", 700);
  await app.waitFor("清空本机手账数据…");
  await app.press("清空本机手账数据…");
  await app.waitFor("真的要清空本机手账金库吗？");
  await app.press("确认彻底清空本机数据");
  await app.waitFor("欢迎来到知气");
}, 90000);
