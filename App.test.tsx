import { expect, open, test } from "nsh/test";

test("知气 Qi v3.0 三套灵境画风、场景陪伴、晨露日签与 QA 工坊全链路测试", async () => {
  const app = await open("./App.tsx");

  // 1. 冷启动诗意引导：零工程术语 + 灵境画风预览选择
  expect(app.shows("欢迎来到「知气 · Qi」")).toBe(true);
  expect(app.shows("数据仅存本机与个人云")).toBe(true);
  expect(app.shows("Tier A 全穿戴")).toBe(false);

  await app.scroll("欢迎来到「知气 · Qi」", 420);
  await app.press("开启今日知气");
  await app.waitFor("已启用端侧健康计算，个人数据仅存本机");
  expect(app.shows("欢迎来到「知气 · Qi」")).toBe(false);
  await app.press("关闭");

  // 2. 今日页：插画剧场 + 轻抚陪伴 + 三套画风实时热切换 + 今日日签海报
  expect(app.shows("三花猫「小满」")).toBe(true);
  expect(app.shows("RMSSD 与 SDNN 严格隔离")).toBe(false);
  expect(app.shows("Tier A 全穿戴")).toBe(false);

  await app.press("轻抚小满");
  await app.waitFor("小满眯起眼睛发出咕噜咕噜的声音，用毛茸茸的小爪踩了踩软垫：今天也一直在陪着你呢。");

  await app.press("掌上山水");
  await app.waitFor("灵境画风已切换为：掌上山水 · 养气灵境");
  expect(app.shows("养气松石灵境")).toBe(true);
  await app.press("关闭");

  await app.press("元气手账");
  await app.waitFor("灵境画风已切换为：元气手账机 · 煤球");
  expect(app.shows("软胶猫「煤球」")).toBe(true);
  await app.press("关闭");

  await app.press("窗畔有猫");
  await app.waitFor("灵境画风已切换为：窗畔有猫 · 小满");
  expect(app.shows("三花猫「小满」")).toBe(true);
  await app.press("关闭");

  // 今日日签海报打开与关闭
  await app.press("今日日签");
  await app.waitFor("今日知气日签");
  expect(app.shows("立冬微晴 · 窗畔知气日签")).toBe(true);
  await app.press("收好日签");
  await app.waitFor("晨安 · 今日节律");

  // 微行动盖章打卡与身体手账折页渐进披露
  await app.scroll("晨安 · 今日节律", 360);
  await app.press("完成今日微行动");
  await app.waitFor("今日微行动已达成 ✓");
  expect(app.shows("已盖爪印")).toBe(true);
  if (app.shows("关闭")) {
    await app.press("关闭");
  }

  await app.scroll("今日微行动盖章", 380);
  await app.press("08:30 早安简报");
  await app.waitFor("AI 实时润色");

  await app.press("查看 30 天个人常态对照");
  await app.waitFor("夜间恢复节律");
  expect(app.shows("Z =")).toBe(false);

  // 3. 记录页：食光贴纸餐垫 / 体检档案 / 穿戴导入
  await app.press("记录");
  await app.waitFor("一句话随手记饮食");

  await app.type("如：下午三点喝了生椰拿铁，晚上吃牛肉面", "下午三点喝了生椰拿铁");
  await app.press("记入今日饮食");
  await app.waitFor("已完成一句话解析并记入「下午三点喝了生椰拿铁」");
  await app.press("关闭");

  await app.press("15:00后冰美式");
  await app.waitFor("已记入「15:00后冰美式」并更新节律关联");
  await app.press("关闭");

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

  await app.press("穿戴导入");
  await app.waitFor("外部穿戴历史数据导入");
  expect(app.shows("导入历史穿戴记录")).toBe(true);

  // 4. 洞察页：晨露藏品阁 + 双周便当对照 + 实验打卡
  await app.press("洞察");
  await app.waitFor("窗畔有猫 · 晨露藏品阁");
  expect(app.shows("天青茶盏")).toBe(true);
  await app.waitFor("近 7 天 vs 前 7 天节律对照");
  await app.scroll("近 7 天 vs 前 7 天节律对照", 340);
  await app.press("今日实验打卡");
  await app.waitFor("今日实验已打卡 ✓");
  if (app.shows("关闭")) {
    await app.press("关闭");
  }

  await app.scroll("个人微习惯实验", 300);
  await app.press("+ 开启：睡前 60 分钟放下屏幕");
  await app.waitFor("睡前 60 分钟放下屏幕");
  if (app.shows("关闭")) {
    await app.press("关闭");
  }

  // 5. 我的页：画风设置 + 加密同步 + AI 模式 + QA 数据工坊
  await app.press("我的");
  await app.waitFor("灵境画风与陪伴小筑");
  await app.scroll("灵境画风与陪伴小筑", 340);
  await app.waitFor("个人云端到端加密同步中心");
  expect(app.shows("演示解锁 Pro")).toBe(false);
  expect(app.shows("模拟 4GB 机型")).toBe(false);

  await app.press("立即加密同步");
  await app.waitFor("关闭");
  await app.press("关闭");

  await app.scroll("个人云端到端加密同步中心", 360);
  await app.waitFor("纯本机离线模型");
  await app.press("纯本机离线模型");
  await app.waitFor("当前已选纯本机离线模型：所有简报润色都在本机完成，零数据离开设备。");

  await app.scroll("AI 引擎与隐私边界", 700);
  await app.waitFor("打开 QA 测试数据与诊断工坊…");
  await app.press("打开 QA 测试数据与诊断工坊…");
  await app.waitFor("6 大纵向测试画像一键切换");

  for (const personaId of ["metabolic_care", "band_commuter", "phone_minimalist", "sparse_chaotic"]) {
    await app.scroll("端侧数据完整性自检", 520);
    await app.press(`切换画像：${personaId}`);
    await app.waitFor(`重新载入：${personaId}`);
    await app.scroll("6 大纵向测试画像一键切换", -600);
    await app.press("运行全量数据自检");
    await app.waitFor("所有 6 项不变量校验通过");
  }

  // 验证 phone_minimalist (tier_c) 切回今日页自然展示晨起感受，无工程术语
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
  await app.waitFor("灵境画风与陪伴小筑");
  await app.scroll("灵境画风与陪伴小筑", 1100);
  await app.waitFor("物理清空本机健康数据…");
  await app.press("物理清空本机健康数据…");
  await app.waitFor("确认物理清空本机健康数据");
  await app.press("确认清空");
  await app.waitFor("欢迎来到「知气 · Qi」");
}, 90000);

test("知气雅集 v4.0 会员沙盒、节气天象、食光贴纸、慢递读信、多滞后归因与只读保险箱不锁门测试", async () => {
  const app = await open("./App.tsx");
  await app.scroll("欢迎来到「知气 · Qi」", 420);
  await app.press("开启今日知气");
  await app.waitFor("已启用端侧健康计算，个人数据仅存本机");
  await app.press("关闭");

  // 1. 打开知气雅集温和权益弹窗：验证 7 档位、7 天透明时间轴与道德红线
  await app.press("知气雅集");
  await app.waitFor("7 天透明体验时间轴");
  expect(app.shows("素心清欢 · 永久免费")).toBe(true);
  expect(app.shows("双人同窗 · Duo")).toBe(true);
  expect(app.shows("四时家书 · Family")).toBe(true);
  expect(app.shows("买断本地 · 终身版")).toBe(true);
  await app.scroll("7 天透明体验时间轴", 420);
  await app.press("暂不决定 · 返回今日窗台");
  await app.waitFor("晨安 · 今日节律");

  // 2. 今日页：周日夜话慢递、同窗串门盖爪印、四时风物长卷与岁时季票
  await app.scroll("今日微行动盖章", 950);
  await app.waitFor("周日夜话慢递");
  await app.press("周日夜话慢递");
  await app.waitFor("第一封 · 立冬初见记");
  expect(app.shows("节气藏书票 · 冬初雪笺 · 藏书票 No.01")).toBe(true);
  await app.press("收好信笺");

  await app.waitFor("四时风物长卷");
  await app.press("四时风物长卷");
  await app.waitFor("我的四时身体风物长卷");
  await app.press("卷起长卷");

  await app.press("岁时风物双轨季票与窗外天色");
  await app.waitFor("岁时风物季票 · 立冬");
  expect(app.shows("展开往期节气博物架")).toBe(true);
  await app.scroll("岁时风物季票 · 立冬", 480);
  await app.press("烟雨江南");
  await app.waitFor("已切换窗外天色心境");
  if (app.shows("关闭")) {
    await app.press("关闭");
  }

  // 3. 记录页：食光贴纸造物机 & 亲友 %ULN 与大字平安家书
  await app.press("记录");
  await app.waitFor("一句话随手记饮食");
  await app.scroll("近期饮食记录", 680);
  await app.waitFor("食光贴纸相机 · 人间烟火月历垫");
  await app.press("制作并贴入食光月历");
  await app.waitFor("已定格食光贴纸「清蒸鲈鱼 · 温热汤饭」(+1 💧)");
  if (app.shows("关闭")) {
    await app.press("关闭");
  }

  await app.scroll("食光贴纸相机 · 人间烟火月历垫", -700);
  await app.press("体检档案");
  await app.waitFor("亲友多档案 %ULN 归一化趋势 & 平安家书");
  await app.scroll("体检单红绿灯私密档案", 680);
  await app.press("生成《气韵平安家书》");
  await app.waitFor("气韵平安家书（大字版）");
  await app.press("合上家书");

  // 4. 洞察页：90 天多滞后归因雷达 & 宋瓷勋章（「证伪小达人」）
  await app.press("洞察");
  await app.waitFor("近 7 天 vs 前 7 天节律对照");
  await app.scroll("近 7 天 vs 前 7 天节律对照", 1100);
  await app.waitFor("90 天多滞后归因雷达 (D+0 / D+1 / D+2)");
  expect(app.shows("宋瓷微习惯勋章 & 四时风物长卷")).toBe(true);
  expect(app.shows("证伪小达人")).toBe(true);

  // 5. 我的 -> QA 工坊：验证 Ed25519 状态机沙盒切换至「到期只读保险箱」，历史数据与导出 100% 永不锁门
  await app.press("我的");
  await app.waitFor("灵境画风与陪伴小筑");
  await app.scroll("灵境画风与陪伴小筑", 820);
  await app.press("打开 QA 测试数据与诊断工坊…");
  await app.waitFor("知气雅集 · 离线状态机与 7 档位沙盒");
  await app.scroll("硬件与端侧算力边界工坊", 680);
  await app.waitFor("知气雅集 · 离线状态机与 7 档位沙盒");
  await app.press("年卡 ¥98");
  await app.waitFor("active_atelier");
  await app.press("状态：到期只读保险箱");
  await app.waitFor("expired_read_only_vault");
  await app.press("完成");
  await app.waitFor("导出 JSONL 摘要");
  if (app.shows("关闭")) {
    await app.press("关闭");
  }

  // 到期只读保险箱下，导出 JSONL 依然畅通无阻
  await app.press("导出 JSONL 摘要");
  await app.waitFor("开放标准 JSONL 个人摘要");
}, 90000);
