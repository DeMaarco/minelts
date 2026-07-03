const invoke = window.__TAURI__.core.invoke;
const listen = window.__TAURI__.event.listen;

const state = {
  busy: false,
  versions: [],
  installed: [],
  config: {
    username: "",
    version_id: "",
    show_snapshots: false,
    ram_mb: 2048,
  },
};

const $ = (id) => document.getElementById(id);

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
  $("btn-play").disabled = busy || state.versions.length === 0;
  $("btn-refresh").disabled = busy;
  $("btn-open-root").disabled = busy;
  $("btn-open-versions").disabled = busy;
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

function filteredVersions() {
  const filter = $("version-filter").value.trim().toLowerCase();
  return state.versions.filter((v) => {
    if (!state.config.show_snapshots && v.type !== "release") return false;
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

  $("btn-play").disabled = state.busy || filtered.length === 0;
}

function renderVersionList() {
  const list = $("version-list");
  list.innerHTML = "";
  const filtered = filteredVersions();

  if (filtered.length === 0) {
    list.appendChild(
      makeTreeItem("", { empty: state.versions.length ? "No matches" : "Loading…" })
    );
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

let saveTimer = null;
function saveConfigDebounced() {
  clearTimeout(saveTimer);
  saveTimer = setTimeout(async () => {
    state.config.username = $("username").value;
    state.config.version_id = $("version-select").value;
    state.config.show_snapshots = $("show-snapshots").checked;
    state.config.ram_mb = parseInt($("ram-mb").value, 10) || 2048;
    try {
      await invoke("save_config", { cfg: state.config });
    } catch (e) {
      console.error("save_config:", e);
    }
  }, 300);
}

async function loadConfig() {
  state.config = await invoke("get_config");
  $("username").value = state.config.username || "";
  $("show-snapshots").checked = state.config.show_snapshots;
  $("ram-mb").value = state.config.ram_mb || 2048;
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

async function play() {
  const username = $("username").value.trim();
  const versionId = state.config.version_id || $("version-select").value;
  const ramMb = parseInt($("ram-mb").value, 10) || 2048;

  if (!username) {
    log("Enter a username", "error");
    return;
  }
  if (!versionId) {
    log("Select a version", "error");
    return;
  }

  const entry = state.versions.find((v) => v.id === versionId);
  if (!entry) {
    log("Version not found", "error");
    return;
  }

  state.config.username = username;
  state.config.version_id = versionId;
  state.config.ram_mb = ramMb;
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
      ramMb,
    });
  } catch (e) {
    setBusy(false);
    hideProgress();
    log(String(e), "error");
  }
}

function bindEvents() {
  $("username").addEventListener("input", saveConfigDebounced);
  $("version-filter").addEventListener("input", () => {
    renderVersionList();
    renderVersionSelect();
  });
  $("show-snapshots").addEventListener("change", () => {
    renderVersionList();
    renderVersionSelect();
    saveConfigDebounced();
  });
  $("ram-mb").addEventListener("change", saveConfigDebounced);

  $("btn-play").addEventListener("click", play);
  $("btn-refresh").addEventListener("click", refreshAll);
  $("btn-open-root").addEventListener("click", () => invoke("open_folder", { which: "root" }));
  $("btn-open-versions").addEventListener("click", () => invoke("open_folder", { which: "versions" }));

  $("toggle-installed").addEventListener("click", () => toggleSection("toggle-installed", "installed-list"));
  $("toggle-available").addEventListener("click", () => toggleSection("toggle-available", "version-list"));

  listen("download-progress", (event) => {
    const { completed, total, current } = event.payload;
    if (total > 0 || current) showProgress(completed, total, current);
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
}

async function init() {
  injectIcons();
  bindEvents();
  $("data-path").textContent = await invoke("get_minecraft_dir");
  await loadConfig();
  await refreshAll();
}

init();
