// Web Worker shim (module worker): wiring only, every computation lives in
// the wasm module built from crates/ui-calculations (make ui-calc).
// dx hashes asset file names, so the main thread sends the resolved URLs
// in one init object {calcJs, calcWasm, coursesUrl, manualJson,
// overridesJson}; after
// that, bare request strings go in and response strings come out (see
// ui-calculations::protocol), plus one initial ready/error envelope.
let calc = null;
const pending = [];

self.onmessage = (event) => {
  const message = event.data;
  if (typeof message === "string") {
    if (calc) {
      self.postMessage(calc.handle_message(message));
    } else {
      pending.push(message);
    }
    return;
  }
  boot(message).catch((error) =>
    self.postMessage(
      JSON.stringify({ kind: "error", id: 0, message: String(error) }),
    ),
  );
};

async function boot(message) {
  const mod = await import(message.calcJs);
  await mod.default({ module_or_path: message.calcWasm });
  const response = await fetch(message.coursesUrl);
  if (!response.ok) {
    throw new Error(`fetch ${message.coursesUrl} : HTTP ${response.status}`);
  }
  const snapshot = await response.text();
  const summary = mod.init_snapshot(
    snapshot,
    message.manualJson,
    message.overridesJson,
  );
  calc = mod;
  self.postMessage(
    JSON.stringify({ kind: "ready", id: 0, summary: JSON.parse(summary) }),
  );
  for (const request of pending.splice(0)) {
    self.postMessage(calc.handle_message(request));
  }
}
