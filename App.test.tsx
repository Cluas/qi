import { expect, open, test } from "nsh/test";

test("知气 Qi v1.0 全链路：引导、元气算分、中式饮食NLP、离线OCR、N-of-1实验、E2EE同步与物理清空", async () => {
  const app = await open("./App.tsx");

  // 1. 冷启动高信任引导
  expect(app.shows("欢迎来到「知气 · Qi」")).toBe(true);
  await app.press("生成今日元气电量");
  await app.waitFor("已启用端侧健康计算，个人数据零上传");
  expect(app.shows("欢迎来到「知气 · Qi」")).toBe(false);

  // 2. 今日页：硬件降级切换 + 打卡微行动
  await app.press("Tier C 纯手机");
  await app.waitFor("晨起主观体感打分：");
  await app.press("Tier A 全穿戴");
  await app.waitFor("今日唯一高杠杆建议");

  await app.press("打卡完成今日微行动");
  await app.waitFor("已完成今日微行动 ✓");

  // 3. 记录与体检页：快捷记餐 + 一句话NLP智能解析 + 实测Apple Vision离线OCR + ZDR懒加载体检脱敏
  await app.press("记录与体检");
  await app.waitFor("3 秒记饮食与恢复因子");
  await app.press("+ 15:00后冰美式");
  await app.waitFor("已记入「15:00后冰美式」并更新 D+1 归因");

  await app.type("如：下午三点喝了生椰拿铁，晚上吃牛肉面", "下午三点喝了生椰拿铁");
  await app.press("一句话智能记入");
  await app.waitFor("已完成一句话智能解析并记入「下午三点喝了生椰拿铁」");

  await app.press("实测离线 OCR 样张");
  await app.waitFor("谷丙转氨酶(ALT)");

  await app.press("模拟导入体检单 (含隐私前置脱敏)");
  await app.waitFor("首次使用云端 AI 解读授权确认");
  await app.press("同意并继续");
  await app.waitFor("尿酸(UA)");
  expect(app.shows("偏高提醒")).toBe(true);
  expect(app.shows("31010419920518432X")).toBe(false);

  // 4. 发现实验页：7天周期对照 + N-of-1 实验打卡 + 模板发起
  await app.press("发现实验");
  await app.waitFor("近 7 天 vs 前 7 天节律对照");
  await app.press("今日实验打卡");
  await app.waitFor("今日实验已打卡 ✓");
  await app.scroll("近 7 天 vs 前 7 天节律对照", 260);
  await app.press("+ 开启：睡前 60 分钟放下屏幕");
  await app.waitFor("睡前 60 分钟放下屏幕");

  // 5. 我的与数据/AI主权页：切本地模型 + 探测算力 + E2EE 真实同步 + 加密 preview + 物理清空复位
  await app.press("我的");
  await app.waitFor("数据主权与云同步中心");
  await app.press("关闭");
  expect(app.shows("尚未执行跨端加密同步")).toBe(true);
  await app.press("立即执行 E2EE 同步");
  await app.press("关闭");
  expect(app.shows("尚未执行跨端加密同步")).toBe(false);

  await app.press("生成 XChaCha20 加密同步包预览");
  await app.waitFor("设备隔离追加分片与 AEAD 验签往返成功");

  await app.scroll("AI 引擎与隐私模式", 240);
  await app.press("② 纯本机离线大模型");
  await app.waitFor("端侧大模型硬件门禁与发热降级模拟");
  await app.press("探测本机与局域网算力");
  await app.press("关闭");

  await app.scroll("AI 引擎与隐私模式", 360);
  await app.press("物理清空本机健康数据 (5.1.1(v))");
  await app.waitFor("确认物理清空本机健康库");
  await app.press("确认清空");
  await app.waitFor("欢迎来到「知气 · Qi」");
});
