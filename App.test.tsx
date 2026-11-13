import { expect, open, test } from "nsh/test";

test("知气 Qi 全链路：引导、元气算分、饮食归因、体检PII脱敏与本地AI切换", async () => {
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

  // 3. 记录与体检页：快捷记餐 + 触发 ZDR 懒加载弹窗导入含身份证号体检单
  await app.press("记录与体检");
  await app.waitFor("3 秒记饮食与恢复因子");
  await app.press("+ 15:00后冰美式");
  await app.waitFor("已记入「15:00后冰美式」并更新 D+1 归因");

  await app.press("模拟导入体检单 (含隐私前置脱敏)");
  await app.waitFor("首次使用云端 AI 解读授权确认");
  await app.press("同意并继续");
  await app.waitFor("尿酸(UA)");
  expect(app.shows("偏高提醒")).toBe(true);
  expect(app.shows("31010419920518432X")).toBe(false);

  // 4. 我的与数据/AI主权页：切本地大模型 + 生成 E2EE 分片 + 物理清空复位
  await app.press("我的");
  await app.waitFor("数据主权与云同步中心");
  await app.press("② 纯本机离线大模型");
  await app.waitFor("端侧大模型硬件门禁与发热降级模拟");

  await app.press("生成 XChaCha20 加密同步包预览");
  await app.waitFor("设备隔离追加分片与 AEAD 验签往返成功");

  await app.press("物理清空本机健康数据 (5.1.1(v))");
  await app.waitFor("确认物理清空本机健康库");
  await app.press("确认清空");
  await app.waitFor("欢迎来到「知气 · Qi」");
});
