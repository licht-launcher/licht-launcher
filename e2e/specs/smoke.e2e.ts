import { $, expect } from "@wdio/globals";

describe("launcher smoke", () => {
  it("leaves the startup screen for an empty library", async () => {
    await expect($("h1=Create your first instance")).toBeDisplayed();
  });

  it("opens and cancels the create instance dialog", async () => {
    await $("button=Create instance").click();
    await $('[role="menuitem"]=From scratch').click();
    const dialog = $('[role="dialog"]');
    await expect(dialog).toBeDisplayed();
    await expect(dialog.$("h2=Create instance")).toBeDisplayed();

    await dialog.$("button=Cancel").click();
    await expect(dialog).not.toBeExisting();
  });

  it("opens settings and switches sections", async () => {
    await $("button=Licht Launcher").click();
    await $('[role="menuitem"]=Settings').click();
    const page = $('section[aria-label="Settings"]');
    await expect(page.$("h2=General")).toBeDisplayed();

    await page
      .$('nav[aria-label="Settings sections"]')
      .$("button=Instances")
      .click();
    await expect(page.$("h2=Instances")).toBeDisplayed();

    await $("button=Library").click();
    await expect(page).not.toBeExisting();
  });
});
