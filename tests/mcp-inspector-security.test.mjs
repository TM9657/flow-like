import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import test from "node:test";
import vm from "node:vm";
import { Window } from "happy-dom";

const template = readFileSync(new URL("../assets/mcp-inspector.html", import.meta.url), "utf8");

function inspector() {
  const window = new Window({ url: "https://example.test/mcp/inspect" });
  window.document.body.innerHTML = '<input id="endpoint"><input id="headerName"><input id="headerValue"><button id="inspect"></button><div id="error"></div><div id="result"></div>';
  const context = vm.createContext({
    window,
    document: window.document,
    URL,
    performance,
    TextDecoder,
    atob,
    fetch: async () => { throw new Error('<img src=x onerror="alert(1)">'); },
  });
  const parsedTemplate = window.document.createElement("template");
  parsedTemplate.innerHTML = template;
  const scripts = parsedTemplate.content.querySelectorAll("script");
  assert.equal(scripts.length, 1);
  const script = scripts[0].textContent.replace("__ENDPOINT_PATH_JSON__", JSON.stringify("/mcp"));
  vm.runInContext(script, context);
  return { window, context };
}

test("MCP binary previews keep MIME types and blob data inside their attributes", () => {
  const { window, context } = inspector();
  for (const mime of ['image/png', 'audio/wav', 'video/mp4', 'application/octet-stream']) {
    for (const [mimeType, blob] of [
      [`${mime}" onerror="alert(1)" data-x="`, "AA=="],
      [mime, 'AA==" onerror="alert(1)" data-x="'],
      [mime, "AA=="],
    ]) {
      for (const html of [
        context.renderResourceResponse({ contents: [{ mimeType, blob, uri: "sample" }] }),
        context.renderMcpContent({ type: "image", mimeType, data: blob }),
      ]) {
        const preview = window.document.createElement("div");
        preview.innerHTML = html;
        assert.equal(preview.querySelector("[onerror], [data-x], script"), null);
        const media = preview.querySelector("img, audio, video, a");
        assert.equal(media.getAttribute(media.tagName === "A" ? "href" : "src"), `data:${mimeType};base64,${blob}`);
      }
    }
  }
});

test("MCP initialization errors render as text", async () => {
  const { window, context } = inspector();
  await context.inspect();
  const error = window.document.getElementById("error");
  assert.equal(error.querySelector("img"), null);
  assert.equal(error.textContent, '<img src=x onerror="alert(1)">');
});
