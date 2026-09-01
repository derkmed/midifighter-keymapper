const { invoke } = window.__TAURI__.core;

// ---- state -----------------------------------------------------------------
let config = { active: null, profiles: [] };
let activeBank = 0;
let selectedCell = null;
let capturing = false;
let draft = { trigger: "tap", steps: [], color: 15 };
let engineRunning = false;
let palette = [];
let veloToHex = {};

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

function describeMouse(a) {
  if (typeof a === "string") return a.replace(/_/g, " ");
  if (a && a.move_to) return `move to ${a.move_to.x},${a.move_to.y}`;
  return "?";
}

function describeStep(s) {
  switch (s.type) {
    case "chord": return s.keys.join(" + ");
    case "text": return `type "${s.text}"`;
    case "delay": return `wait ${s.ms} ms`;
    case "mouse": return "mouse " + describeMouse(s.action);
    default: return "?";
  }
}

function isSingleChord(steps) {
  return steps.length === 1 && steps[0].type === "chord";
}

function summarizeSteps(steps) {
  if (!steps || steps.length === 0) return "";
  if (steps.length === 1) return describeStep(steps[0]);
  return `${steps.length} steps`;
}

// Display hex for a stored velocity: exact palette match, else nearest.
function hexForVelocity(v) {
  if (v in veloToHex) return veloToHex[v];
  let best = null, bestd = Infinity;
  for (const s of palette) {
    const d = Math.abs(s.velocity - v);
    if (d < bestd) { bestd = d; best = s; }
  }
  return best ? best.hex : "#000";
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
      if (b) {
        pad.classList.add("bound");
        pad.style.background = hexForVelocity(b.color);
      }
      if (c === selectedCell) pad.classList.add("selected");
      const capText = b ? (summarizeSteps(b.macro) || "color only") : "";
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
  // trigger segmented — Hold only allowed for a single-chord macro
  const single = isSingleChord(draft.steps);
  for (const btn of $("trigger-seg").children) {
    const isHold = btn.dataset.mode === "hold";
    btn.disabled = isHold && !single;
    btn.title = isHold && !single ? "Hold works only with a single-chord macro" : "";
    btn.classList.toggle("active", btn.dataset.mode === draft.trigger);
  }
  // macro steps + color swatches
  renderSteps();
  renderSwatches();
}

function renderSteps() {
  const wrap = $("steps");
  wrap.innerHTML = "";
  draft.steps.forEach((step, i) => {
    const row = document.createElement("div");
    row.className = "step";
    const desc = document.createElement("span");
    desc.className = "step-desc";
    if (step.type === "text") {
      desc.append("type ");
      const inp = document.createElement("input");
      inp.className = "step-input";
      inp.value = step.text;
      inp.onchange = () => { step.text = inp.value; persistDraft(); };
      desc.appendChild(inp);
    } else if (step.type === "delay") {
      desc.append("wait ");
      const inp = document.createElement("input");
      inp.type = "number"; inp.min = "0"; inp.className = "step-input num";
      inp.value = step.ms;
      inp.onchange = () => { step.ms = Math.max(0, Number(inp.value) || 0); persistDraft(); };
      desc.appendChild(inp);
      desc.append(" ms");
    } else if (step.type === "mouse") {
      desc.append("mouse ");
      const sel = document.createElement("select");
      for (const opt of ["left_click", "right_click", "middle_click"]) {
        const o = document.createElement("option");
        o.value = opt; o.textContent = opt.replace(/_/g, " ");
        if (typeof step.action === "string" && step.action === opt) o.selected = true;
        sel.appendChild(o);
      }
      sel.onchange = () => { step.action = sel.value; persistDraft(); };
      desc.appendChild(sel);
    } else {
      desc.textContent = describeStep(step);
    }
    row.appendChild(desc);

    const ctrls = document.createElement("div");
    ctrls.className = "step-ctrls";
    const up = miniBtn("↑", () => moveStep(i, -1)); up.disabled = i === 0;
    const dn = miniBtn("↓", () => moveStep(i, 1)); dn.disabled = i === draft.steps.length - 1;
    const del = miniBtn("✕", () => { draft.steps.splice(i, 1); persistDraft(); });
    ctrls.append(up, dn, del);
    row.appendChild(ctrls);
    wrap.appendChild(row);
  });
  if (capturing) {
    const hint = document.createElement("div");
    hint.className = "step-empty capturing";
    hint.textContent = "Press your key combo…";
    wrap.appendChild(hint);
  } else if (draft.steps.length === 0) {
    const empty = document.createElement("div");
    empty.className = "step-empty";
    empty.textContent = "No steps yet — add one below.";
    wrap.appendChild(empty);
  }
}

function miniBtn(text, fn) {
  const b = document.createElement("button");
  b.className = "mini";
  b.textContent = text;
  b.onclick = fn;
  return b;
}

function moveStep(i, d) {
  const j = i + d;
  if (j < 0 || j >= draft.steps.length) return;
  [draft.steps[i], draft.steps[j]] = [draft.steps[j], draft.steps[i]];
  persistDraft();
}

function renderSwatches() {
  const wrap = $("swatches");
  wrap.innerHTML = "";
  for (const sw of palette) {
    const b = document.createElement("button");
    b.className = "swatch-btn" + (draft.color === sw.velocity ? " sel" : "");
    b.style.background = sw.hex;
    b.title = `${sw.name} (velocity ${sw.velocity})`;
    b.onclick = () => {
      draft.color = sw.velocity;
      const p = activeProfile();
      if (p && selectedCell !== null && !engineRunning) {
        invoke("preview_color", { baseNote: p.base_note, bank: activeBank, cell: selectedCell, color: sw.velocity }).catch(() => {});
      }
      persistDraft();
    };
    wrap.appendChild(b);
  }
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
    ? { trigger: b.trigger, steps: JSON.parse(JSON.stringify(b.macro || [])), color: b.color }
    : { trigger: "tap", steps: [], color: 15 };
  capturing = false;
  renderAll();
}

// Persist the current draft to the selected pad's binding (live editing).
async function persistDraft() {
  const p = activeProfile();
  if (!p || selectedCell === null) return;
  // Hold is only valid for a single chord; otherwise fall back to Tap.
  let trigger = draft.trigger || "tap";
  if (trigger === "hold" && !isSingleChord(draft.steps)) {
    trigger = "tap";
    draft.trigger = "tap";
  }
  const pad = { bank: activeBank, cell: selectedCell, binding: { trigger, macro: draft.steps, color: draft.color } };
  config = await call("upsert_binding", { profileId: p.id, pad });
  renderAll();
}

async function clearBinding() {
  const p = activeProfile();
  if (!p || selectedCell === null) return;
  config = await call("remove_binding", { profileId: p.id, bank: activeBank, cell: selectedCell });
  draft = { trigger: "tap", steps: [], color: 15 };
  setStatus("Pad cleared", "ok");
  renderAll();
}

// Add a macro step of the given kind to the current draft.
async function addStep(kind) {
  if (selectedCell === null || !activeProfile()) return;
  if (kind === "chord") {
    capturing = true;
    renderPanel();
    return;
  }
  if (kind === "text") {
    const t = await askText("Text to type");
    if (t === null) return;
    draft.steps.push({ type: "text", text: t });
  } else if (kind === "delay") {
    const t = await askText("Delay in milliseconds", "100");
    if (t === null) return;
    draft.steps.push({ type: "delay", ms: Math.max(0, Number(t) || 0) });
  } else if (kind === "mouse") {
    draft.steps.push({ type: "mouse", action: "left_click" });
  }
  persistDraft();
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
    btn.onclick = () => {
      if (btn.disabled) return;
      draft.trigger = btn.dataset.mode;
      renderPanel();
      persistDraft();
    };
  }

  for (const btn of document.querySelectorAll("[data-add]")) {
    btn.onclick = () => addStep(btn.dataset.add);
  }
  // Chord capture: the next key combo becomes a new chord step.
  window.addEventListener("keydown", (e) => {
    if (!capturing) return;
    e.preventDefault();
    const tokens = eventToTokens(e);
    const hasNonMod = tokens.some((t) => !["ctrl", "shift", "alt", "cmd"].includes(t));
    if (hasNonMod) {
      draft.steps.push({ type: "chord", keys: tokens });
      capturing = false;
      persistDraft();
    }
  });

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
        // On macOS the engine runs fine but keystrokes silently no-op without
        // Accessibility trust, so surface the banner instead of a false success.
        if (!(await refreshAccessibility())) {
          setStatus("Mapping ON, but grant Accessibility for keystrokes to fire", "error");
        } else {
          setStatus("Mapping ON — pads now fire their combos in any app", "ok");
        }
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

// ---- macOS Accessibility permission (D12) ----------------------------------
let accessible = true;

// Reflect current trust state in the banner. Returns the state so callers can
// gate on it. On Windows/Linux the command always reports true → banner stays
// hidden.
async function refreshAccessibility() {
  try {
    accessible = await invoke("accessibility_status");
  } catch (_) {
    accessible = true; // command missing/failed → don't nag
  }
  $("ax-banner").classList.toggle("hidden", accessible);
  return accessible;
}

function wireAccessibility() {
  $("ax-grant").onclick = async () => {
    await invoke("request_accessibility").catch(() => {});
    // Trust flips only once the user toggles the switch in System Settings, so
    // poll until it does (or they close the app). Each poll updates the banner.
    const started = Date.now();
    const poll = setInterval(async () => {
      if (await refreshAccessibility() || Date.now() - started > 120000) {
        clearInterval(poll);
        if (accessible) setStatus("Accessibility granted — keystrokes enabled", "ok");
      }
    }, 1000);
  };
  // Catch revocation (or a grant done outside the button) while the app is open.
  setInterval(refreshAccessibility, 3000);
}

// ---- boot ------------------------------------------------------------------
(async function boot() {
  wire();
  wireAccessibility();
  await refreshAccessibility();
  config = await invoke("get_config");
  palette = await invoke("get_palette");
  veloToHex = Object.fromEntries(palette.map((s) => [s.velocity, s.hex]));
  engineRunning = await invoke("engine_running");
  try { $("autostart").checked = await invoke("get_autostart"); } catch (_) {}
  try { $("start-on-launch").checked = (await invoke("get_settings")).start_mapping_on_launch; } catch (_) {}
  renderEngine();
  renderAll();
})();
