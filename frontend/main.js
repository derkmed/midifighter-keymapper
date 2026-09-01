const { invoke } = window.__TAURI__.core;

// ---- state -----------------------------------------------------------------
let config = { active: null, profiles: [] };
let activeBank = 0;
let selectedCell = null;
let capturing = false;
let draft = { trigger: "tap", tokens: [], color: 7 };
let engineRunning = false;

// ---- helpers ---------------------------------------------------------------
const $ = (id) => document.getElementById(id);

function activeProfile() {
  return config.profiles.find((p) => p.id === config.active) || null;
}

// Returns the inner Binding for a pad (or null). Callers work with the Binding
// itself (trigger/macro/color), not the {bank,cell,binding} wrapper.
function bindingAt(bank, cell) {
  const p = activeProfile();
  if (!p) return null;
  const pad = p.bindings.find((b) => b.bank === bank && b.cell === cell);
  return pad ? pad.binding : null;
}

function chordTokens(binding) {
  const step = binding && binding.macro && binding.macro[0];
  return step && step.type === "chord" ? step.keys : [];
}

function colorToCss(v) {
  if (!v) return "#000";
  const hue = Math.round((v / 127) * 300);
  return `hsl(${hue} 90% 55%)`;
}

function setStatus(msg, kind = "") {
  const el = $("status");
  el.textContent = msg;
  el.className = "status " + kind;
  if (msg) setTimeout(() => { if (el.textContent === msg) { el.textContent = ""; el.className = "status"; } }, 4000);
}

async function call(cmd, args) {
  try {
    const result = await invoke(cmd, args);
    return result;
  } catch (e) {
    setStatus(String(e), "error");
    throw e;
  }
}

// A tiny modal text prompt (WebView2 disables window.prompt).
function askText(title, initial = "") {
  return new Promise((resolve) => {
    const overlay = document.createElement("div");
    overlay.style.cssText =
      "position:fixed;inset:0;background:rgba(0,0,0,.5);display:flex;align-items:center;justify-content:center;z-index:99";
    overlay.innerHTML =
      `<div style="background:#1e1e28;border:1px solid #33334a;border-radius:10px;padding:18px;width:280px">
         <div style="margin-bottom:10px">${title}</div>
         <input id="ask-input" style="width:100%;padding:7px;background:#262633;color:#e6e6f0;border:1px solid #33334a;border-radius:6px" />
         <div style="display:flex;gap:8px;justify-content:flex-end;margin-top:14px">
           <button id="ask-cancel">Cancel</button>
           <button id="ask-ok" class="primary">OK</button>
         </div>
       </div>`;
    document.body.appendChild(overlay);
    const input = overlay.querySelector("#ask-input");
    input.value = initial;
    input.focus();
    input.select();
    const done = (val) => { document.body.removeChild(overlay); resolve(val); };
    overlay.querySelector("#ask-ok").onclick = () => done(input.value.trim() || null);
    overlay.querySelector("#ask-cancel").onclick = () => done(null);
    input.onkeydown = (e) => {
      if (e.key === "Enter") done(input.value.trim() || null);
      if (e.key === "Escape") done(null);
    };
  });
}

// Map a browser KeyboardEvent to our key tokens (see engine::keys::resolve_key).
function eventToTokens(e) {
  const tokens = [];
  if (e.ctrlKey) tokens.push("ctrl");
  if (e.shiftKey) tokens.push("shift");
  if (e.altKey) tokens.push("alt");
  if (e.metaKey) tokens.push("cmd");
  const map = {
    " ": "space", Enter: "enter", Escape: "esc", Tab: "tab",
    Backspace: "backspace", Delete: "delete",
    ArrowUp: "up", ArrowDown: "down", ArrowLeft: "left", ArrowRight: "right",
    Home: "home", End: "end", PageUp: "pageup", PageDown: "pagedown",
  };
  const k = e.key;
  if (["Control", "Shift", "Alt", "Meta"].includes(k)) return tokens; // modifier only
  if (map[k]) tokens.push(map[k]);
  else if (/^F([1-9]|1[0-2])$/.test(k)) tokens.push(k.toLowerCase());
  else if (k.length === 1) tokens.push(k.toLowerCase());
  return tokens;
}

// ---- rendering -------------------------------------------------------------
function renderProfiles() {
  const sel = $("profile-select");
  sel.innerHTML = "";
  for (const p of config.profiles) {
    const opt = document.createElement("option");
    opt.value = p.id;
    opt.textContent = p.name;
    if (p.id === config.active) opt.selected = true;
    sel.appendChild(opt);
  }
  if (config.profiles.length === 0) {
    const opt = document.createElement("option");
    opt.textContent = "(no profiles)";
    sel.appendChild(opt);
  }
}

function renderBankTabs() {
  const wrap = $("bank-tabs");
  wrap.innerHTML = "";
  for (let b = 0; b < 4; b++) {
    const btn = document.createElement("button");
    btn.textContent = "Bank " + (b + 1);
    if (b === activeBank) btn.classList.add("active");
    btn.onclick = () => { activeBank = b; selectedCell = null; renderAll(); };
    wrap.appendChild(btn);
  }
}

function renderGrid() {
  const grid = $("grid");
  grid.innerHTML = "";
  // Cell 0 is bottom-left: render rows top -> bottom as 12..15, 8..11, 4..7, 0..3.
  for (const rowStart of [12, 8, 4, 0]) {
    for (let c = rowStart; c < rowStart + 4; c++) {
      const pad = document.createElement("div");
      pad.className = "pad";
      const b = bindingAt(activeBank, c);
      const keys = chordTokens(b);
      if (b) {
        pad.classList.add("bound");
        pad.style.background = colorToCss(b.color);
      }
      if (c === selectedCell) pad.classList.add("selected");
      const capText = keys.length ? keys.join(" + ") : (b ? "color only" : "");
      pad.innerHTML =
        `<span class="idx">${c}</span>
         <span class="cap">${capText}</span>`;
      pad.onclick = () => selectCell(c);
      grid.appendChild(pad);
    }
  }
}

function renderPanel() {
  const hasProfile = !!activeProfile();
  const body = $("panel-body");
  if (selectedCell === null || !hasProfile) {
    $("panel-title").textContent = hasProfile ? "No pad selected" : "Create a profile to begin";
    body.classList.add("hidden");
    return;
  }
  body.classList.remove("hidden");
  $("panel-title").textContent = `Bank ${activeBank + 1} · Cell ${selectedCell}`;
  // trigger segmented
  for (const btn of $("trigger-seg").children) {
    btn.classList.toggle("active", btn.dataset.mode === draft.trigger);
  }
  // keys
  const kd = $("keys-display");
  kd.textContent = draft.tokens.length ? draft.tokens.join(" + ") : "—";
  kd.classList.toggle("capturing", capturing);
  // color
  $("color-range").value = draft.color;
  $("color-val").textContent = draft.color;
  $("color-swatch").style.background = colorToCss(draft.color);
}

function renderAll() {
  renderProfiles();
  renderBankTabs();
  renderGrid();
  renderPanel();
}

// ---- interactions ----------------------------------------------------------
function selectCell(cell) {
  selectedCell = cell;
  const b = bindingAt(activeBank, cell);
  draft = b
    ? { trigger: b.trigger, tokens: chordTokens(b), color: b.color }
    : { trigger: "tap", tokens: [], color: 7 };
  capturing = false;
  renderAll();
}

// Persist the current draft to the selected pad's binding (live editing).
async function persistDraft() {
  const p = activeProfile();
  if (!p || selectedCell === null) return;
  const macro = draft.tokens.length ? [{ type: "chord", keys: draft.tokens }] : [];
  const pad = { bank: activeBank, cell: selectedCell, binding: { trigger: draft.trigger || "tap", macro, color: draft.color } };
  config = await call("upsert_binding", { profileId: p.id, pad });
  renderAll();
}

async function clearBinding() {
  const p = activeProfile();
  if (!p || selectedCell === null) return;
  config = await call("remove_binding", { profileId: p.id, bank: activeBank, cell: selectedCell });
  draft = { trigger: "tap", tokens: [], color: 7 };
  setStatus("Pad cleared", "ok");
  renderAll();
}

function wire() {
  $("profile-select").onchange = async (e) => {
    config = await call("set_active", { id: e.target.value });
    selectedCell = null;
    renderAll();
  };
  $("profile-new").onclick = async () => {
    const name = await askText("New profile name");
    if (!name) return;
    const id = name.toLowerCase().replace(/[^a-z0-9]+/g, "-") + "-" + Date.now().toString(36);
    config = await call("add_profile", { id, name });
    config = await call("set_active", { id });
    selectedCell = null;
    renderAll();
  };
  $("profile-rename").onclick = async () => {
    const p = activeProfile();
    if (!p) return;
    const name = await askText("Rename profile", p.name);
    if (!name) return;
    config = await call("rename_profile", { id: p.id, name });
    renderAll();
  };
  $("profile-delete").onclick = async () => {
    const p = activeProfile();
    if (!p) return;
    config = await call("delete_profile", { id: p.id });
    selectedCell = null;
    renderAll();
  };
  $("save").onclick = async () => {
    await call("save_config");
    setStatus("Saved to disk", "ok");
  };

  for (const btn of $("trigger-seg").children) {
    btn.onclick = () => { draft.trigger = btn.dataset.mode; renderPanel(); persistDraft(); };
  }
  $("capture").onclick = () => { capturing = true; renderPanel(); };
  window.addEventListener("keydown", (e) => {
    if (!capturing) return;
    e.preventDefault();
    const tokens = eventToTokens(e);
    const hasNonMod = tokens.some((t) => !["ctrl", "shift", "alt", "cmd"].includes(t));
    if (hasNonMod) { draft.tokens = tokens; capturing = false; renderPanel(); persistDraft(); }
  });

  const range = $("color-range");
  // While dragging: update UI + live device preview (cheap, no persist).
  range.oninput = () => {
    draft.color = Number(range.value);
    $("color-val").textContent = draft.color;
    $("color-swatch").style.background = colorToCss(draft.color);
    const p = activeProfile();
    if (p && selectedCell !== null && !engineRunning) {
      invoke("preview_color", { baseNote: p.base_note, bank: activeBank, cell: selectedCell, color: draft.color }).catch(() => {});
    }
  };
  // On release: persist the chosen color to the pad's binding.
  range.onchange = () => { persistDraft(); };
  $("clear").onclick = clearBinding;

  $("engine-toggle").onclick = async () => {
    try {
      if (engineRunning) {
        await invoke("stop_engine");
        engineRunning = false;
        setStatus("Mapping stopped", "ok");
      } else {
        await call("start_engine");
        engineRunning = true;
        setStatus("Mapping ON — pads now fire their combos in any app", "ok");
      }
    } catch (_) {
      engineRunning = await invoke("engine_running");
    }
    renderEngine();
  };

  $("autostart").onchange = async (e) => {
    try {
      await invoke("set_autostart", { enabled: e.target.checked });
      setStatus(e.target.checked ? "Will launch at login" : "Won't launch at login", "ok");
    } catch (err) {
      setStatus(String(err), "error");
      e.target.checked = await invoke("get_autostart").catch(() => false);
    }
  };
  $("start-on-launch").onchange = async (e) => {
    try {
      await invoke("set_start_on_launch", { enabled: e.target.checked });
    } catch (err) {
      setStatus(String(err), "error");
    }
  };
}

function renderEngine() {
  const btn = $("engine-toggle");
  btn.textContent = engineRunning ? "■ Stop mapping" : "▶ Start mapping";
  btn.classList.toggle("running", engineRunning);
}

// ---- boot ------------------------------------------------------------------
(async function boot() {
  wire();
  config = await invoke("get_config");
  engineRunning = await invoke("engine_running");
  try { $("autostart").checked = await invoke("get_autostart"); } catch (_) {}
  try { $("start-on-launch").checked = (await invoke("get_settings")).start_mapping_on_launch; } catch (_) {}
  renderEngine();
  renderAll();
})();
