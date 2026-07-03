const invoke = window.__TAURI__.core.invoke;
const listen = window.__TAURI__.event.listen;

const RAM_MIN = 512;
const RAM_STEP = 64;

const state = {
  busy: false,
  versions: [],
  installed: [],
  systemRamMb: 16384,
  config: {
    username: "",
    version_id: "",
    ram_min_mb: 1024,
    ram_max_mb: 2048,
    jvm_args: "",
    filters: {
      show_releases: true,
      show_snapshots: false,
      show_old: false,
    },
  },
};

const $ = (id) => document.getElementById(id);

const DEFAULT_FILTERS = {
  show_releases: true,
  show_snapshots: false,
  show_old: false,
};

function injectIcons() {
  document.querySelectorAll("[data-icon]").forEach((el) => {
    const name = el.dataset.icon;
    if (Icons[name]) el.innerHTML = Icons[name];
  });
}

function setStatusbar(msg) {
  $("statusbar-msg").textContent = msg;
}

function log(message, type = "info") {
  const logEl = $("output-log");
  const line = document.createElement("div");
  line.className = `log-line ${type === "info" ? "" : type}`;
  if (type === "info") line.classList.add("info");
  const time = new Date().toLocaleTimeString("es-ES", { hour12: false });
  line.textContent = `[${time}] ${message}`;
  logEl.appendChild(line);
  logEl.scrollTop = logEl.scrollHeight;
  setStatusbar(message);
}

function setBusy(busy) {
  state.busy = busy;
  const noVersion = state.versions.length === 0;
  $("btn-play").disabled = busy || noVersion;
  $("btn-install").disabled = busy || noVersion;
  $("btn-refresh").disabled = busy;
  $("btn-open-root").disabled = busy;
  $("btn-open-versions").disabled = busy;
  $("btn-open-logs").disabled = busy;
  $("btn-open-crashes").disabled = busy;
  if (!busy) setStatusbar("Ready");
}

function showProgress(completed, total, current) {
  const wrap = $("progress-wrap");
  const bar = $("progress-bar");
  const text = $("progress-text");
  wrap.classList.remove("hidden");
  const frac = total > 0 ? (completed / total) * 100 : 0;
  bar.style.width = `${frac}%`;
  const label = current || "Downloading…";
  text.textContent = total > 0 ? `${completed}/${total} — ${label}` : label;
  setStatusbar(text.textContent);
}

function hideProgress() {
  $("progress-wrap").classList.add("hidden");
}

function ensureFilters() {
  if (!state.config.filters) {
    state.config.filters = { ...DEFAULT_FILTERS };
  }
}

function versionTypeAllowed(type) {
  ensureFilters();
  const f = state.config.filters;
  if (type === "release") return f.show_releases;
  if (type === "snapshot") return f.show_snapshots;
  if (type === "old_beta" || type === "old_alpha") return f.show_old;
  return false;
}

function anyFilterActive() {
  ensureFilters();
  const f = state.config.filters;
  return f.show_releases || f.show_snapshots || f.show_old;
}

function filteredVersions() {
  const filter = $("version-filter").value.trim().toLowerCase();
  if (!anyFilterActive()) return [];

  return state.versions.filter((v) => {
    if (!versionTypeAllowed(v.type)) return false;
    if (filter && !v.id.toLowerCase().includes(filter)) return false;
    return true;
  });
}

function isInstalled(id) {
  return state.installed.some((v) => v.id === id);
}

function makeTreeItem(id, extra = {}) {
  const li = document.createElement("li");
  if (extra.selected) li.classList.add("selected");
  if (extra.empty) {
    li.classList.add("empty");
    li.textContent = extra.empty;
    return li;
  }

  const icon = document.createElement("span");
  icon.className = "icon-slot";
  icon.innerHTML = Icons.box;
  icon.style.color = "var(--hc-fg-faint)";

  const label = document.createElement("span");
  label.className = "tree-label-text";
  label.textContent = id;

  li.appendChild(icon);
  li.appendChild(label);

  if (extra.tag) {
    const tag = document.createElement("span");
    tag.className = "tag";
    tag.textContent = extra.tag;
    li.appendChild(tag);
  }

  if (extra.title) li.title = extra.title;

  return li;
}

function selectVersion(id) {
  state.config.version_id = id;
  $("version-select").value = id;
  $("selected-version").textContent = id || "—";
  renderInstalled();
  renderVersionList();
  saveConfigDebounced();
}

function renderVersionSelect() {
  const select = $("version-select");
  const filtered = filteredVersions();
  select.innerHTML = "";

  filtered.forEach((v) => {
    const opt = document.createElement("option");
    opt.value = v.id;
    opt.textContent = v.id;
    if (v.id === state.config.version_id) opt.selected = true;
    select.appendChild(opt);
  });

  if (filtered.length === 0) {
    select.appendChild(document.createElement("option"));
  } else if (!filtered.some((v) => v.id === state.config.version_id)) {
    selectVersion(filtered[0].id);
  } else {
    $("selected-version").textContent = state.config.version_id || "—";
  }

  const disabled = state.busy || filtered.length === 0;
  $("btn-play").disabled = disabled;
  $("btn-install").disabled = disabled;
}

function renderVersionList() {
  const list = $("version-list");
  list.innerHTML = "";
  const filtered = filteredVersions();

  if (filtered.length === 0) {
    let emptyMsg = "Loading…";
    if (state.versions.length) {
      emptyMsg = anyFilterActive() ? "No matches" : "Enable a version type filter";
    }
    list.appendChild(makeTreeItem("", { empty: emptyMsg }));
    return;
  }

  filtered.forEach((v) => {
    const li = makeTreeItem(v.id, {
      selected: v.id === state.config.version_id,
      tag: isInstalled(v.id) ? "local" : undefined,
    });
    li.addEventListener("click", () => {
      if (state.busy) return;
      selectVersion(v.id);
    });
    list.appendChild(li);
  });
}

function renderInstalled() {
  const list = $("installed-list");
  list.innerHTML = "";
  $("installed-count").textContent = String(state.installed.length);

  if (state.installed.length === 0) {
    list.appendChild(makeTreeItem("", { empty: "None yet" }));
    return;
  }

  state.installed.forEach((v) => {
    const li = makeTreeItem(v.id, {
      selected: v.id === state.config.version_id,
      title: v.jar_path,
      tag: "local",
    });
    li.addEventListener("click", () => {
      if (state.busy) return;
      selectVersion(v.id);
    });
    list.appendChild(li);
  });
}

function toggleSection(btnId, listId) {
  const btn = $(btnId);
  const list = $(listId);
  const chevron = btn.querySelector(".chevron");
  const hidden = list.classList.toggle("hidden");
  chevron.classList.toggle("open", !hidden);
  chevron.classList.toggle("closed", hidden);
}

function roundRamMb(value) {
  const rounded = Math.round(value / RAM_STEP) * RAM_STEP;
  return Math.min(Math.max(rounded, RAM_MIN), state.systemRamMb);
}

function getRamMinMb() {
  return roundRamMb(parseInt($("ram-min-slider").value, 10));
}

function getRamMaxMb() {
  return roundRamMb(parseInt($("ram-max-slider").value, 10));
}

function updateRamLabels() {
  $("ram-min-value").textContent = `${getRamMinMb()} MB`;
  $("ram-max-value").textContent = `${getRamMaxMb()} MB`;
}

function applyRamToSliders(ramMinMb, ramMaxMb) {
  const minSlider = $("ram-min-slider");
  const maxSlider = $("ram-max-slider");
  const maxCap = state.systemRamMb;

  let minVal = roundRamMb(ramMinMb || 1024);
  let maxVal = roundRamMb(ramMaxMb || 2048);

  if (minVal > maxVal) minVal = Math.max(RAM_MIN, maxVal - RAM_STEP);
  if (maxVal < minVal) maxVal = Math.min(maxCap, minVal + RAM_STEP);

  minSlider.max = String(maxCap);
  maxSlider.max = String(maxCap);
  minSlider.value = String(minVal);
  maxSlider.value = String(maxVal);
  updateRamLabels();
}

function setupRamSliders() {
  const minSlider = $("ram-min-slider");
  const maxSlider = $("ram-max-slider");

  const onMinChange = () => {
    let minVal = getRamMinMb();
    let maxVal = getRamMaxMb();
    if (minVal > maxVal) {
      maxVal = Math.min(state.systemRamMb, minVal);
      maxSlider.value = String(maxVal);
    }
    minSlider.value = String(minVal);
    updateRamLabels();
    saveConfigDebounced();
  };

  const onMaxChange = () => {
    let minVal = getRamMinMb();
    let maxVal = getRamMaxMb();
    if (maxVal < minVal) {
      minVal = Math.max(RAM_MIN, maxVal);
      minSlider.value = String(minVal);
    }
    maxSlider.value = String(maxVal);
    updateRamLabels();
    saveConfigDebounced();
  };

  minSlider.addEventListener("input", onMinChange);
  maxSlider.addEventListener("input", onMaxChange);
}

async function loadSystemRam() {
  try {
    const { total_mb: totalMb } = await invoke("get_system_ram_mb");
    state.systemRamMb = totalMb || 16384;
  } catch (e) {
    console.error("get_system_ram_mb:", e);
    state.systemRamMb = 16384;
  }
}

function readFormIntoConfig() {
  state.config.username = $("username").value;
  state.config.version_id = $("version-select").value;
  state.config.ram_min_mb = getRamMinMb();
  state.config.ram_max_mb = getRamMaxMb();
  state.config.jvm_args = $("jvm-args").value;
  ensureFilters();
  state.config.filters.show_releases = $("filter-releases").checked;
  state.config.filters.show_snapshots = $("filter-snapshots").checked;
  state.config.filters.show_old = $("filter-old").checked;
}

let saveTimer = null;
function saveConfigDebounced() {
  clearTimeout(saveTimer);
  saveTimer = setTimeout(async () => {
    readFormIntoConfig();
    try {
      await invoke("save_config", { cfg: state.config });
    } catch (e) {
      console.error("save_config:", e);
    }
  }, 300);
}

async function loadConfig() {
  state.config = await invoke("get_config");
  ensureFilters();
  $("username").value = state.config.username || "";
  applyRamToSliders(state.config.ram_min_mb, state.config.ram_max_mb);
  $("jvm-args").value = state.config.jvm_args || "";
  $("filter-releases").checked = state.config.filters.show_releases;
  $("filter-snapshots").checked = state.config.filters.show_snapshots;
  $("filter-old").checked = state.config.filters.show_old;
  $("selected-version").textContent = state.config.version_id || "—";
}

async function loadVersions() {
  setBusy(true);
  log("Fetching version manifest…", "info");
  try {
    state.versions = await invoke("get_versions");
    renderVersionSelect();
    renderVersionList();
    log(`Loaded ${state.versions.length} versions`, "success");
  } catch (e) {
    log(String(e), "error");
  } finally {
    setBusy(false);
  }
}

async function loadInstalled() {
  state.installed = await invoke("get_installed");
  renderInstalled();
  renderVersionList();
  renderVersionSelect();
}

async function refreshAll() {
  log("Refreshing…", "info");
  await loadInstalled();
  await loadVersions();
}

function getSelectedVersionEntry() {
  const versionId = state.config.version_id || $("version-select").value;
  if (!versionId) return null;
  return state.versions.find((v) => v.id === versionId) || null;
}

async function install() {
  const versionId = state.config.version_id || $("version-select").value;
  if (!versionId) {
    log("Select a version", "error");
    return;
  }

  const entry = getSelectedVersionEntry();
  if (!entry) {
    log("Version not found", "error");
    return;
  }

  state.config.version_id = versionId;
  await invoke("save_config", { cfg: state.config });

  setBusy(true);
  hideProgress();
  log(`Installing ${versionId}…`, "info");
  showProgress(0, 1, "Preparing…");

  try {
    await invoke("install_version", {
      versionId,
      url: entry.url,
    });
  } catch (e) {
    setBusy(false);
    hideProgress();
    log(String(e), "error");
  }
}

async function play() {
  const username = $("username").value.trim();
  const versionId = state.config.version_id || $("version-select").value;
  const ramMinMb = getRamMinMb();
  const ramMaxMb = getRamMaxMb();
  const jvmArgs = $("jvm-args").value;

  if (!username) {
    log("Enter a username", "error");
    return;
  }
  if (!versionId) {
    log("Select a version", "error");
    return;
  }

  const entry = getSelectedVersionEntry();
  if (!entry) {
    log("Version not found", "error");
    return;
  }

  readFormIntoConfig();
  state.config.username = username;
  state.config.version_id = versionId;
  await invoke("save_config", { cfg: state.config });

  setBusy(true);
  hideProgress();
  log(`Launching ${versionId} as ${username}…`, "info");
  showProgress(0, 1, "Preparing…");

  try {
    await invoke("launch_game", {
      username,
      versionId,
      url: entry.url,
      ramMinMb,
      ramMaxMb,
      jvmArgs,
    });
  } catch (e) {
    setBusy(false);
    hideProgress();
    log(String(e), "error");
  }
}

function isTextInput(el) {
  if (!el) return false;
  const tag = el.tagName;
  return tag === "INPUT" || tag === "TEXTAREA" || el.isContentEditable;
}

function bindShortcuts() {
  document.addEventListener("keydown", (e) => {
    if (e.key === "?" && !e.ctrlKey && !e.metaKey && !e.altKey) {
      if (!isTextInput(document.activeElement)) {
        setStatusbar(
          "Enter: Launch · Ctrl+Enter: Launch · Ctrl+Shift+I: Install · Ctrl+F: Search · Ctrl+R: Refresh · Ctrl+L: Logs · Ctrl+Shift+C: Crashes"
        );
      }
      return;
    }

    const ctrl = e.ctrlKey || e.metaKey;

    if (ctrl && e.key === "f") {
      e.preventDefault();
      $("version-filter").focus();
      $("version-filter").select();
      return;
    }

    if (ctrl && e.shiftKey && e.key.toLowerCase() === "i") {
      e.preventDefault();
      if (!state.busy) install();
      return;
    }

    if (ctrl && e.shiftKey && e.key.toLowerCase() === "c") {
      e.preventDefault();
      invoke("open_folder", { which: "crashes" });
      return;
    }

    if (ctrl && !e.shiftKey && e.key.toLowerCase() === "r") {
      e.preventDefault();
      if (!state.busy) refreshAll();
      return;
    }

    if (ctrl && !e.shiftKey && e.key.toLowerCase() === "l") {
      e.preventDefault();
      invoke("open_folder", { which: "logs" });
      return;
    }

    if (ctrl && e.key === "Enter") {
      e.preventDefault();
      if (!state.busy) play();
      return;
    }

    if (e.key === "Enter" && !ctrl && !isTextInput(document.activeElement)) {
      e.preventDefault();
      if (!state.busy) play();
    }
  });
}

function bindEvents() {
  $("username").addEventListener("input", saveConfigDebounced);
  $("version-filter").addEventListener("input", () => {
    renderVersionList();
    renderVersionSelect();
  });

  ["filter-releases", "filter-snapshots", "filter-old"].forEach((id) => {
    $(id).addEventListener("change", () => {
      renderVersionList();
      renderVersionSelect();
      saveConfigDebounced();
    });
  });

  $("jvm-args").addEventListener("input", saveConfigDebounced);

  $("btn-install").addEventListener("click", install);
  $("btn-play").addEventListener("click", play);
  $("btn-refresh").addEventListener("click", refreshAll);
  $("btn-open-root").addEventListener("click", () => invoke("open_folder", { which: "root" }));
  $("btn-open-versions").addEventListener("click", () => invoke("open_folder", { which: "versions" }));
  $("btn-open-logs").addEventListener("click", () => invoke("open_folder", { which: "logs" }));
  $("btn-open-crashes").addEventListener("click", () => invoke("open_folder", { which: "crashes" }));

  $("toggle-installed").addEventListener("click", () => toggleSection("toggle-installed", "installed-list"));
  $("toggle-available").addEventListener("click", () => toggleSection("toggle-available", "version-list"));

  listen("download-progress", (event) => {
    const { completed, total, current } = event.payload;
    if (total > 0 || current) showProgress(completed, total, current);
  });

  listen("install-done", async (event) => {
    setBusy(false);
    hideProgress();
    const { ok, error } = event.payload;
    if (ok) {
      log("Version installed successfully", "success");
      await loadInstalled();
    } else {
      log(error || "Install failed", "error");
    }
  });

  listen("launch-done", async (event) => {
    setBusy(false);
    hideProgress();
    const { ok, error } = event.payload;
    if (ok) {
      log("Minecraft launched successfully", "success");
      await loadInstalled();
    } else {
      log(error || "Launch failed", "error");
    }
  });

  bindShortcuts();
}

async function init() {
  injectIcons();
  setupRamSliders();
  bindEvents();
  $("data-path").textContent = await invoke("get_minecraft_dir");
  await loadSystemRam();
  await loadConfig();
  await refreshAll();
}

init();
