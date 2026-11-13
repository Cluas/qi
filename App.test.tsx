import { expect, open, test } from "nsh/test";

test("知气 Qi v2.0 消费级全界面与 QA 工坊全链路测试", async () => {
  const app = await open("./App.tsx");

  // 1. 冷启动诗意引导：零工程术语
  expect(app.shows("欢迎来到「知气 · Qi」")).toBe(true);
  expect(app.shows("数据仅存本机与个人云")).toBe(true);
  expect(app.shows("Tier A 全穿戴")).toBe(false);

  await app.scroll("欢迎来到「知气 · Qi」", 240);
  await app.press("开启今日知气");
  await app.waitFor("已启用端侧健康计算，个人数据仅存本机");
  expect(app.shows("欢迎来到「知气 · Qi」")).toBe(false);
  await app.press("关闭");

  // 2. 今日页：主界面去工程化 + 微行动打卡 + 30 天常态折叠展开
  expect(app.shows("今日元气指数")).toBe(true);
  expect(app.shows("RMSSD 与 SDNN 严格隔离")).toBe(false);
  expect(app.shows("Tier A 全穿戴")).toBe(false);

  await app.scroll("今日元气指数", 260);
  await app.press("完成今日微行动");
  await app.waitFor("今日微行动已达成 ✓");

  await app.scroll("08:30 早安简报", 420);
  await app.press("查看 30 天个人常态对照");
  await app.waitFor("夜间恢复节律");
  expect(app.shows("Z =")).toBe(false);

  // 3. 记录页：SegmentedTabs 三子视图（饮食随记 / 体检档案 / 穿戴导入）
  await app.press("记录");
  await app.waitFor("一句话随手记饮食");

  await app.type("如：下午三点喝了生椰拿铁，晚上吃牛肉面", "下午三点喝了生椰拿铁");
  await app.press("记入今日饮食");
  await app.waitFor("已完成一句话解析并记入「下午三点喝了生椰拿铁」");
  await app.press("关闭");

  await app.press("15:00后冰美式");
  await app.waitFor("已记入「15:00后冰美式」并更新节律关联");
  await app.press("关闭");

  // 切换至「体检档案」子视图
  await app.press("体检档案");
  await app.waitFor("相册选图离线识图");
  await app.press("试用示例报告体验脱敏");
  await app.waitFor("首次使用云端 AI 解读授权确认");
  await app.press("同意并继续");
  await app.waitFor("尿酸(UA)");
  expect(app.shows("偏高提醒")).toBe(true);
  expect(app.shows("31010419920518432X")).toBe(false);
  if (app.shows("关闭")) {
    await app.press("关闭");
  }

  // 切换至「穿戴导入」子视图
  await app.press("穿戴导入");
  await app.waitFor("外部穿戴历史数据导入");
  expect(app.shows("导入历史穿戴记录")).toBe(true);

  // 4. 洞察页：双周节律对照 + 实验打卡 + 推荐模板开启
  await app.press("洞察");
  await app.waitFor("近 7 天 vs 前 7 天节律对照");
  await app.scroll("近 7 天 vs 前 7 天节律对照", 240);
  await app.press("今日实验打卡");
  await app.waitFor("今日实验已打卡 ✓");
  if (app.shows("关闭")) {
    await app.press("关闭");
  }

  await app.scroll("个人微习惯实验", 280);
  await app.press("+ 开启：睡前 60 分钟放下屏幕");
  await app.waitFor("睡前 60 分钟放下屏幕");
  if (app.shows("关闭")) {
    await app.press("关闭");
  }

  // 5. 我的页：加密同步 + AI 模式切换 + 知气验真与数据工坊
  await app.press("我的");
  await app.waitFor("个人云端到端加密同步中心");
  expect(app.shows("演示解锁 Pro")).toBe(false);
  expect(app.shows("模拟 4GB 机型")).toBe(false);

  await app.press("立即加密同步");
  if (app.shows("关闭")) {
    await app.press("关闭");
  }

  await app.scroll("个人云端到端加密同步中心", 360);
  await app.waitFor("纯本机离线模型");
  await app.press("纯本机离线模型");
  await app.waitFor("当前已选纯本机离线模型：所有简报润色都在本机完成，零数据离开设备。");

  await app.scroll("AI 引擎与隐私边界", 680);
  await app.waitFor("打开 QA 测试数据与诊断工坊…");
  await app.press("打开 QA 测试数据与诊断工坊…");
  await app.waitFor("6 大纵向测试画像一键切换");

  // 切换 4 种代表性测试画像并验证 6 项不变量全通过
  for (const personaId of ["metabolic_care", "band_commuter", "phone_minimalist", "sparse_chaotic"]) {
    await app.scroll("端侧数据完整性自检", 520);
    await app.press(`切换画像：${personaId}`);
    await app.waitFor(`重新载入：${personaId}`);
    await app.scroll("6 大纵向测试画像一键切换", -600);
    await app.press("运行全量数据自检");
    await app.waitFor("所有 6 项不变量校验通过");
  }

  // 验证 phone_minimalist (tier_c) 切回今日页时自然展现「今日晨起精神感受」，无硬件切换按钮
  await app.scroll("端侧数据完整性自检", 460);
  await app.press("切换画像：phone_minimalist");
  await app.waitFor("重新载入：phone_minimalist");
  await app.press("完成");
  await app.waitFor("知气验真与数据工坊");
  if (app.shows("关闭")) {
    await app.press("关闭");
  }

  await app.press("今日");
  await app.waitFor("今日晨起精神感受");
  expect(app.shows("Tier A 全穿戴")).toBe(false);

  // 6. 物理清空本机健康数据回归冷启动引导
  await app.press("我的");
  await app.waitFor("个人云端到端加密同步中心");
  await app.scroll("个人云端到端加密同步中心", 900);
  await app.waitFor("物理清空本机健康数据…");
  await app.press("物理清空本机健康数据…");
  await app.waitFor("确认物理清空本机健康数据");
  await app.press("确认清空");
  await app.waitFor("欢迎来到「知气 · Qi」");
});
