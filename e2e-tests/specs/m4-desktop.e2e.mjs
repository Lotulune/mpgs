import { $, $$, browser, expect } from "@wdio/globals";
import path from "node:path";
import { fileURLToPath } from "node:url";
import { stopSeedServer } from "../support/runtime.mjs";

// Keep labels in sync with web/src/app/format.ts SECTION_META.
const FEEDS = ["近期正式发售", "即将发售 / Demo", "人气老游", "老牌联机"];
const packageDir = path.resolve(fileURLToPath(new URL("..", import.meta.url)));
const artifactDir = path.resolve(
  process.env.MPGS_E2E_ARTIFACT_DIR ?? path.join(packageDir, "artifacts"),
);

function exactButton(text) {
  return $(`//button[normalize-space(.)=${JSON.stringify(text)}]`);
}

async function clickTab(label) {
  const tab = await $(`//button[@role='tab' and contains(normalize-space(.),${JSON.stringify(label)})]`);
  await tab.waitForDisplayed();
  await tab.click();
  await browser.waitUntil(async () => (await tab.getAttribute("aria-selected")) === "true", {
    timeoutMsg: `expected ${label} tab to become selected`,
  });
}

async function expectVisibleText(text, rootSelector = "body") {
  const root = await $(rootSelector);
  await browser.waitUntil(async () => (await root.getText()).includes(text), {
    timeoutMsg: `expected visible text: ${text}`,
  });
}

async function waitForFeed() {
  await browser.waitUntil(
    async () => (await $$("article.card")).length > 0,
    { timeout: 20_000, timeoutMsg: "expected at least one recommendation card" },
  );
}

async function cardNames() {
  const names = [];
  for (const heading of await $$("article.card h3")) names.push(await heading.getText());
  return names;
}

async function assertNoCriticalOverflow(width, height) {
  await browser.setWindowSize(width, height);
  await browser.saveScreenshot(path.join(artifactDir, `layout-${width}x${height}.png`));
  const layout = await browser.execute(() => {
    const viewportWidth = window.innerWidth;
    const root = document.documentElement;
    const selectors = ["header.topbar", ".brand", "nav.tabs", ".topbar-controls", "main.main"];
    const outside = selectors.flatMap((selector) =>
      Array.from(document.querySelectorAll(selector))
        .filter((element) => {
          const rect = element.getBoundingClientRect();
          return rect.left < -1 || rect.right > viewportWidth + 1 || rect.width <= 0;
        })
        .map((element) => element.className || element.tagName),
    );
    const brand = document.querySelector(".brand")?.getBoundingClientRect();
    const tabs = document.querySelector("nav.tabs")?.getBoundingClientRect();
    const controls = document.querySelector(".topbar-controls")?.getBoundingClientRect();
    const overlaps = [];
    if (brand && tabs && brand.right > tabs.left + 1) overlaps.push("brand/tabs");
    if (tabs && controls && tabs.right > controls.left + 1) overlaps.push("tabs/controls");
    return {
      viewportWidth,
      documentOverflow: root.scrollWidth - viewportWidth,
      outside,
      overlaps,
    };
  });
  expect(layout.documentOverflow).toBeLessThanOrEqual(1);
  expect(layout.outside).toEqual([]);
  expect(layout.overlaps).toEqual([]);
}

describe("M4 native desktop journey", () => {
  it("completes first-run onboarding and persists preferences", async () => {
    await browser.setWindowSize(1280, 800);
    await expect($("h1=选择你的界面风格")).toBeDisplayed();
    await (await $("button[data-theme='minimal']")).click();
    await (await exactButton("继续 →")).click();
    await expect($("h1=你们通常怎么玩？")).toBeDisplayed();
    await (await exactButton("4 人")).click();
    await (await exactButton("1–2 小时")).click();
    await (await exactButton("¥150 以内")).click();
    await (await exactButton("开始探索")).click();
    await expect($("nav[aria-label='主导航']")).toBeDisplayed();
    await waitForFeed();

    // A new native WebDriver session relaunches the app. Reaching the shell
    // again proves the onboarding marker/preferences survived in client SQLite.
    await browser.reloadSession();
    await expect($("nav[aria-label='主导航']")).toBeDisplayed();
    await expect($("h1=选择你的界面风格")).not.toBeExisting();
    await waitForFeed();
  });

  it("loads all four feeds with recommendation reasons", async () => {
    for (const label of FEEDS) {
      await clickTab(label);
      await waitForFeed();
      const firstCard = (await $$("article.card"))[0];
      await expect(firstCard).toBeDisplayed();
      expect((await firstCard.$$("ul.reason-list li")).length).toBeGreaterThan(0);
      await expectVisibleText("数据更新于");
    }
  });

  it("uses the explicit natural-language fallback flow", async () => {
    await clickTab("描述推荐");
    const input = await $("#nl-input");
    await input.setValue("4 人合作，单局一小时以内，不要太竞技");
    await (await exactButton("推荐")).click();
    // AI is disabled in E2E seed mode — UI surfaces deterministic fallback chips.
    await expectVisibleText("确定性回退");
    await waitForFeed();
    await expectVisibleText("当前由确定性规则理解输入");
  });

  it("shows upcoming and recent calendar entries with honest early-data context", async () => {
    await clickTab("日历");
    await expect($("section[aria-label='发售日历']")).toBeDisplayed();
    await expect($("button.cal-row")).toBeDisplayed();
    await expectVisibleText("早期数据", "section[aria-label='发售日历']");
    await expectVisibleText("置信度", "section[aria-label='发售日历']");
    await expectVisibleText("来源更新于", "section[aria-label='发售日历']");

    await (await exactButton("近期发售")).click();
    await expect($("button.cal-row")).toBeDisplayed();
    await expectVisibleText("数据更新于", "section[aria-label='发售日历']");
  });

  it("refreshes ranking after acknowledged feedback", async () => {
    await clickTab("近期正式发售");
    await waitForFeed();
    const before = await cardNames();
    const firstCard = (await $$("article.card"))[0];
    const dismissedName = (await firstCard.$("h3")).getText();
    await (await firstCard.$(".//button[normalize-space()='不感兴趣']")).click();

    await browser.waitUntil(
      async () => {
        const names = await cardNames();
        const busy = await $("[aria-busy='true']").isExisting();
        return !busy && !names.includes(dismissedName);
      },
      { timeout: 20_000, timeoutMsg: `expected feedback to remove ${dismissedName} from the refreshed feed` },
    );
    expect(await cardNames()).not.toEqual(before);
  });

  it("has no horizontal or critical topbar overflow at supported minimum sizes", async () => {
    await assertNoCriticalOverflow(1024, 640);
    await assertNoCriticalOverflow(1280, 800);
  });

  it("serves a cached snapshot with data time after the server goes offline", async () => {
    await clickTab("老牌联机");
    await waitForFeed();
    await expectVisibleText("数据更新于");

    await stopSeedServer();
    await clickTab("人气老游");
    await expectVisibleText("离线快照");
    await expectVisibleText("数据更新于");
    await waitForFeed();
  });
});
