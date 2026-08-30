/* Drive the config builder in a real browser and print the TOML it generates.
 *
 * The page's generator is inline script, so there is nothing importable to call
 * directly — the only honest way to test what a reader actually receives is to
 * click the same buttons they do. Prints the generated document on stdout;
 * everything else goes to stderr so the caller can pipe it straight to a file.
 *
 * Usage: node tools/config_builder_smoke.js <base-url> <combination>
 * where <combination> is comma-separated group=value pairs, e.g.
 *   evolution=generational,genome=edge_edit,fitness=epi_spread
 *
 * Exit 0 with TOML on stdout, or 1 with the reason on stderr. Puppeteer comes
 * from pa11y, which the accessibility step already installs.
 */
"use strict";

const puppeteer = require("puppeteer");

const url = process.argv[2];
const combination = process.argv[3];

if (!url || !combination) {
  console.error("usage: config_builder_smoke.js <base-url> <group=value,...>");
  process.exit(1);
}

const choices = combination.split(",").map(function (pair) {
  const halves = pair.split("=");
  if (halves.length !== 2) {
    console.error(`not a group=value pair: ${pair}`);
    process.exit(1);
  }
  return { group: halves[0].trim(), value: halves[1].trim() };
});

(async function () {
  const browser = await puppeteer.launch({
    args: ["--no-sandbox", "--disable-setuid-sandbox"],
  });
  try {
    const page = await browser.newPage();

    // A page error means the builder threw while generating, which is exactly
    // the failure this exists to catch — it must not pass as an empty document.
    const failures = [];
    page.on("pageerror", (err) => failures.push(String(err)));
    page.on("console", (msg) => {
      if (msg.type() === "error") { failures.push(msg.text()); }
    });

    const response = await page.goto(`${url}/guide/config-builder.html`,
                                     { waitUntil: "networkidle0" });
    if (!response || !response.ok()) {
      throw new Error(`page did not load: HTTP ${response ? response.status() : "no response"}`);
    }

    // Download starts disabled: if it were enabled before any choice is made,
    // every later assertion about the gate would pass without meaning anything.
    const gatedAtStart = await page.$eval("#download", (b) => b.disabled);
    if (!gatedAtStart) {
      throw new Error("Download was enabled before any choice was made");
    }

    for (const choice of choices) {
      const selector = `.cb-choice[data-group="${choice.group}"][data-value="${choice.value}"]`;
      const button = await page.$(selector);
      if (!button) {
        throw new Error(`no such choice: ${choice.group}=${choice.value}`);
      }
      await button.click();
    }

    const enabled = await page.$eval("#download", (b) => !b.disabled);
    const stamp = await page.$eval("#stamp", (s) => s.textContent.trim());
    const todo = await page.$$eval("#todo li", (items) =>
      items.map((li) => li.textContent.trim()));

    if (!enabled) {
      throw new Error(
        `Download still disabled after ${combination}; stamp "${stamp}", outstanding: ` +
        (todo.length ? todo.join("; ") : "(nothing listed)"));
    }

    const toml = await page.$eval("#out", (el) => el.textContent);
    if (!toml || !toml.trim()) {
      throw new Error("Download was enabled but the generated document is empty");
    }

    if (failures.length) {
      throw new Error(`the page reported errors: ${failures.join(" | ")}`);
    }

    console.error(`${combination}: stamp "${stamp}", ${toml.split("\n").length} lines`);
    process.stdout.write(toml.endsWith("\n") ? toml : `${toml}\n`);
  } catch (err) {
    console.error(String(err && err.message ? err.message : err));
    process.exitCode = 1;
  } finally {
    await browser.close();
  }
})();
