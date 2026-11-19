// The Rust root tested from TypeScript: `nsh test` runs this file in a
// headless runner it builds with the crate, and `open(".")` starts the root
// (`nsh.json` "root").
import { expect, open, test } from "nsh/test";

test("renders reminder app", async () => {
  const app = await open(".");
  // If the app is currently in mini capsule mode, expand to full card
  if (!app.texts().includes("reminder 提醒")) {
    await app.press("role:button[展开为卡片]");
  }
  expect(app.texts()).toContain("reminder 提醒");
  expect(app.texts()).toContain("喝水");
  expect(app.texts()).toContain("起身活动");
});

test("shows the global hotkey in the settings drawer", async () => {
  const app = await open(".");
  if (!app.texts().includes("reminder 提醒")) {
    await app.press("role:button[展开为卡片]");
  }
  await app.press("role:button[偏好设置]");
  await app.waitFor("text:全局热键");
  const texts = app.texts();
  expect(texts).toContain("全局热键");
  // 修饰键按平台显示 Ctrl / ⌘、Alt / ⌥，末位键帽跨平台一致
  expect(texts).toContain("R");
});

test("offers launch at login in the settings drawer", async () => {
  const app = await open(".");
  if (!app.texts().includes("reminder 提醒")) {
    await app.press("role:button[展开为卡片]");
  }
  await app.press("role:button[偏好设置]");
  await app.waitFor("text:开机自启");
  const texts = app.texts();
  expect(texts).toContain("开机自启");
  // 开 / 关两种状态共用同一句前缀，说明这块确实由系统状态驱动
  expect(texts.join(" ")).toContain("登录系统后自动");
});

test("offers fixed-time daily reminders in the custom form", async () => {
  const app = await open(".");
  if (!app.texts().includes("reminder 提醒")) {
    await app.press("role:button[展开为卡片]");
  }
  await app.press("role:button[新建提醒]");
  await app.waitFor("role:button[自定义新提醒]");
  // 展开自定义新提醒：默认按循环周期配置
  await app.press("role:button[自定义新提醒]");
  await app.waitFor("role:button[每天定点]");
  expect(app.texts()).toContain("循环周期");

  // 切到「每天定点」：出现时:分编辑与常用定点时刻
  await app.press("role:button[每天定点]");
  await app.waitFor("text:每天提醒时间");
  await app.waitFor("text:常用时刻:");
  const texts = app.texts();
  expect(texts).toContain("每天提醒时间");
  // 用户示例里的 12:00 午餐、18:30 下班打卡、23:00 睡觉都能一键填入
  expect(texts).toContain("12:00");
  expect(texts).toContain("18:30");
  expect(texts).toContain("23:00");

  // 切回循环周期，原来的分钟快捷填入还在
  await app.press("role:button[循环周期]");
  await app.waitFor("text:快捷填入:");
  expect(app.texts()).toContain("循环周期");
});

test("clears the add drawer's tab and draft when reopened", async () => {
  const app = await open(".");
  if (!app.texts().includes("reminder 提醒")) {
    await app.press("role:button[展开为卡片]");
  }
  // 1. 打开新建抽屉，切到「自定义新提醒」，填名称并改成「每天定点」
  await app.press("role:button[新建提醒]");
  await app.waitFor("role:button[自定义新提醒]");
  await app.press("role:button[自定义新提醒]");
  await app.waitFor("role:button[每天定点]");
  await app.type("custom-name", "站姿切换");
  await app.press("role:button[每天定点]");
  await app.waitFor("text:每天提醒时间");
  expect(app.value("custom-name")).toBe("站姿切换");

  // 2. 收起再重新打开：页签回到「常用推荐」
  await app.press("role:button[新建提醒]");
  await app.press("role:button[新建提醒]");
  await app.waitFor("text:循环打卡");
  expect(app.texts()).toContain("常用推荐");

  // 3. 再次切到「自定义新提醒」：名称、提醒方式也已复位成干净首屏
  await app.press("role:button[自定义新提醒]");
  await app.waitFor("text:提醒事项名称");
  expect(app.value("custom-name")).toBe("");
  expect(app.texts()).toContain("快捷填入:");
  expect(app.texts()).not.toContain("每天提醒时间");
});
