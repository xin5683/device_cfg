(function () {
    "use strict";

    var selectedNetwork = null;
    var statusPollTimer = null;
    var daemonLogPollTimer = null;
    var ansiUp = window.AnsiUp ? new window.AnsiUp() : null;

    var heroStatusContent = document.getElementById("hero-status-content");
    var networkList = document.getElementById("network-list");
    var scanBtn = document.getElementById("scan-btn");
    var disconnectBtn = document.getElementById("disconnect-btn");
    var modal = document.getElementById("modal");
    var modalSsid = document.getElementById("modal-ssid");
    var passwordGroup = document.getElementById("password-group");
    var passwordInput = document.getElementById("password-input");
    var connectBtn = document.getElementById("connect-btn");
    var cancelBtn = document.getElementById("cancel-btn");
    var toast = document.getElementById("toast");
    var updateCheckBtn = document.getElementById("update-check-btn");
    var updateApplyBtn = document.getElementById("update-apply-btn");
    var updateTitle = document.getElementById("update-title");
    var updateSummary = document.getElementById("update-summary");
    var updateDetails = document.getElementById("update-details");
    var systemVersion = document.getElementById("system-version");
    var daemonStatusContent = document.getElementById("daemon-status-content");
    var daemonStartHeroBtn = document.getElementById("daemon-start-hero-btn");
    var daemonCheckBtn = document.getElementById("daemon-check-btn");
    var daemonPullBtn = document.getElementById("daemon-pull-btn");
    var daemonUpgradeBtn = document.getElementById("daemon-upgrade-btn");
    var daemonStartBtn = document.getElementById("daemon-start-btn");
    var daemonRestartBtn = document.getElementById("daemon-restart-btn");
    var daemonLogsBtn = document.getElementById("daemon-logs-btn");
    var daemonTitle = document.getElementById("daemon-title");
    var daemonSummary = document.getElementById("daemon-summary");
    var daemonDetails = document.getElementById("daemon-details");
    var daemonLogView = document.getElementById("daemon-log-view");

    function init() {
        scanBtn.addEventListener("click", doScan);
        disconnectBtn.addEventListener("click", doDisconnect);
        connectBtn.addEventListener("click", doConnect);
        cancelBtn.addEventListener("click", closeModal);
        modal.addEventListener("click", function (e) {
            if (e.target === modal || e.target.classList.contains("modal-backdrop")) {
                closeModal();
            }
        });
        passwordInput.addEventListener("keydown", function (e) {
            if (e.key === "Enter") doConnect();
        });
        updateCheckBtn.addEventListener("click", checkUpdate);
        updateApplyBtn.addEventListener("click", applyUpdate);
        daemonCheckBtn.addEventListener("click", checkDaemon);
        daemonPullBtn.addEventListener("click", pullDaemon);
        daemonUpgradeBtn.addEventListener("click", upgradeDaemon);
        daemonStartBtn.addEventListener("click", startDaemon);
        daemonRestartBtn.addEventListener("click", restartDaemon);
        daemonStartHeroBtn.addEventListener("click", startDaemon);
        daemonLogsBtn.addEventListener("click", function () { loadDaemonLogs(false); });

        initTabs();
        loadSystemInfo();
        loadStatus();
        loadDaemonStatus();
    }

    function initTabs() {
        var tabItems = document.querySelectorAll(".tab-item");
        var tabPanels = document.querySelectorAll(".tab-panel");

        tabItems.forEach(function (item) {
            item.addEventListener("click", function () {
                if (item.disabled) return;

                var targetTab = item.getAttribute("data-tab");

                tabItems.forEach(function (t) { t.classList.remove("active"); });
                tabPanels.forEach(function (p) { p.classList.remove("active"); });

                item.classList.add("active");
                var targetPanel = document.querySelector('[data-panel="' + targetTab + '"]');
                if (targetPanel) {
                    targetPanel.classList.add("active");
                }

                if (targetTab === "daemon") {
                    startDaemonLogPoll();
                    loadDaemonStatus();
                    loadDaemonLogs();
                } else {
                    stopDaemonLogPoll();
                }
            });
        });
    }

    async function api(path, options) {
        var resp = await fetch(path, options);
        return resp.json();
    }

    async function loadStatus() {
        try {
            var result = await api("/api/status");
            if (result.success) {
                renderStatus(result.data);
            } else {
                heroStatusContent.innerHTML = '<p class="status-loading">无法获取状态: ' + escapeHtml(result.message) + "</p>";
            }
        } catch (e) {
            heroStatusContent.innerHTML = '<p class="status-loading">服务未响应</p>';
        }
    }

    async function loadSystemInfo() {
        try {
            var result = await api("/api/system/info");
            if (result.success && result.data && result.data.version) {
                systemVersion.textContent = "v" + result.data.version;
            } else {
                systemVersion.textContent = "-";
            }
        } catch (e) {
            systemVersion.textContent = "-";
        }
    }

    function renderStatus(s) {
        var connected = s.state === "COMPLETED";
        var stateText = connected ? "已连接" : stateLabel(s.state);
        var html = "";

        html += '<div class="status-summary">';
        html += '<div class="status-summary-copy">';
        html += '<div class="status-summary-title">当前连接状态</div>';
        html += '<div class="status-summary-subtitle">' + escapeHtml(statusDescription(s.state, connected)) + "</div>";
        html += "</div>";
        html += '<div class="status-state ' + (connected ? "connected" : "disconnected") + '">';
        html += escapeHtml(stateText);
        html += "</div>";
        html += "</div>";

        html += '<div class="status-details">';
        html += renderStatusItem("网络名称", connected && s.ssid ? s.ssid : "未连接");
        html += renderStatusItem("IP 地址", connected && s.ip ? s.ip : "-");
        html += renderStatusItem("BSSID", connected && s.bssid ? s.bssid : "-");
        if (!connected && s.state && s.state !== "DISCONNECTED") {
            html += renderStatusItem("连接阶段", s.state);
        }
        html += "</div>";

        heroStatusContent.innerHTML = html;
        disconnectBtn.style.display = connected ? "block" : "none";
    }

    function renderStatusItem(label, value) {
        var html = "";
        html += '<div class="status-item">';
        html += '<span class="status-item-label">' + escapeHtml(label) + "</span>";
        html += '<span class="status-item-value">' + escapeHtml(value) + "</span>";
        html += "</div>";
        return html;
    }

    function statusDescription(state, connected) {
        if (connected) return "设备已接入无线网络";
        if (state === "SCANNING") return "正在搜索附近的无线网络";
        if (state === "ASSOCIATING") return "正在建立接入点连接";
        if (state === "AUTHENTICATING") return "正在进行身份认证";
        if (state === "INACTIVE") return "无线接口当前空闲";
        return "设备当前未连接到无线网络";
    }

    async function doScan() {
        setScanBusy(true);
        networkList.innerHTML = '<div class="empty-state"><div class="empty-icon"><svg width="48" height="48" viewBox="0 0 48 48" fill="none"><circle cx="24" cy="24" r="20" stroke="currentColor" stroke-width="2" opacity="0.2"/></svg></div><p>正在扫描 WiFi 网络...</p></div>';

        try {
            var result = await api("/api/scan");
            if (result.success) {
                renderNetworks(result.data);
            } else {
                networkList.innerHTML = '<div class="empty-state"><div class="empty-icon"><svg width="48" height="48" viewBox="0 0 48 48" fill="none"><circle cx="24" cy="24" r="20" stroke="currentColor" stroke-width="2" opacity="0.2"/></svg></div><p>扫描失败: ' + escapeHtml(result.message) + "</p></div>";
            }
        } catch (e) {
            networkList.innerHTML = '<div class="empty-state"><div class="empty-icon"><svg width="48" height="48" viewBox="0 0 48 48" fill="none"><circle cx="24" cy="24" r="20" stroke="currentColor" stroke-width="2" opacity="0.2"/></svg></div><p>扫描请求失败</p></div>';
        }

        setScanBusy(false);
    }

    function setScanBusy(busy) {
        scanBtn.disabled = busy;
        var btnContent = scanBtn.querySelector("span");
        if (busy) {
            scanBtn.innerHTML = '<span class="spinner"></span><span>扫描中</span>';
        } else {
            scanBtn.innerHTML = '<svg width="16" height="16" viewBox="0 0 16 16" fill="none"><path d="M14 8a6 6 0 11-12 0 6 6 0 0112 0z" stroke="currentColor" stroke-width="1.5"/><path d="M8 2v2M8 12v2M2 8h2M12 8h2" stroke="currentColor" stroke-width="1.5" stroke-linecap="round"/></svg><span>扫描网络</span>';
        }
    }

    function renderNetworks(networks) {
        if (!networks || networks.length === 0) {
            networkList.innerHTML = '<div class="empty-state"><div class="empty-icon"><svg width="48" height="48" viewBox="0 0 48 48" fill="none"><circle cx="24" cy="24" r="20" stroke="currentColor" stroke-width="2" opacity="0.2"/><path d="M12 20a14 14 0 0124 0M16 26a8 8 0 0116 0" stroke="currentColor" stroke-width="2" stroke-linecap="round" opacity="0.3"/></svg></div><p>未发现 WiFi 网络，请确认天线已连接</p></div>';
            return;
        }

        var html = "";
        for (var i = 0; i < networks.length; i++) {
            var n = networks[i];
            var bars = signalBars(n.signal);
            var isOpen = n.security === "开放";

            html += '<div class="network-item" data-idx="' + i + '">';
            html += '<div class="signal-icon">' + bars + "</div>";
            html += '<div class="network-info">';
            html += '<div class="network-ssid">' + escapeHtml(n.ssid) + "</div>";
            html += '<div class="network-meta">' + n.signal + " dBm · " + n.frequency + " MHz</div>";
            html += "</div>";
            html += '<span class="security-badge' + (isOpen ? " open" : "") + '">' + escapeHtml(n.security) + "</span>";
            html += "</div>";
        }

        networkList.innerHTML = html;

        var items = networkList.querySelectorAll(".network-item");
        for (var j = 0; j < items.length; j++) {
            items[j].addEventListener("click", (function (idx) {
                return function () {
                    openModal(networks[idx]);
                };
            })(j));
        }
    }

    function signalBars(signal) {
        var level;
        if (signal >= -50) level = 4;
        else if (signal >= -65) level = 3;
        else if (signal >= -75) level = 2;
        else level = 1;

        var html = "";
        for (var i = 1; i <= 4; i++) {
            html += '<div class="signal-bar' + (i <= level ? " active" : "") + '"></div>';
        }
        return html;
    }

    function openModal(network) {
        selectedNetwork = network;
        modalSsid.textContent = network.ssid;

        if (network.security === "开放") {
            passwordGroup.style.display = "none";
            document.getElementById("modal-title").textContent = "连接到开放网络";
        } else {
            passwordGroup.style.display = "block";
            passwordInput.value = "";
            document.getElementById("modal-title").textContent = "连接到网络";
            setTimeout(function () { passwordInput.focus(); }, 100);
        }

        modal.style.display = "flex";
    }

    function closeModal() {
        modal.style.display = "none";
        selectedNetwork = null;
    }

    async function doConnect() {
        if (!selectedNetwork) return;

        var password = passwordInput.value;
        var isOpen = selectedNetwork.security === "开放";

        if (!isOpen && !password) {
            showToast("请输入 WiFi 密码", "error");
            return;
        }

        connectBtn.disabled = true;
        connectBtn.innerHTML = '<span class="spinner"></span><span>连接中</span>';

        var body = { ssid: selectedNetwork.ssid };
        if (!isOpen) {
            body.password = password;
        }

        try {
            var result = await api("/api/connect", {
                method: "POST",
                headers: { "Content-Type": "application/json" },
                body: JSON.stringify(body),
            });

            closeModal();

            if (result.success) {
                showToast("连接成功！IP: " + (result.data || ""), "success");
                startStatusPoll();
            } else {
                showToast("连接失败: " + result.message, "error");
            }
        } catch (e) {
            showToast("连接请求失败", "error");
            closeModal();
        }

        connectBtn.disabled = false;
        connectBtn.textContent = "连接";
    }

    async function doDisconnect() {
        disconnectBtn.disabled = true;
        disconnectBtn.innerHTML = '<span class="spinner"></span><span>断开中</span>';

        try {
            var result = await api("/api/disconnect", { method: "POST" });
            if (result.success) {
                showToast("已断开连接", "info");
                loadStatus();
            } else {
                showToast("断开失败: " + result.message, "error");
            }
        } catch (e) {
            showToast("断开请求失败", "error");
        }

        disconnectBtn.disabled = false;
        disconnectBtn.textContent = "断开连接";
    }

    async function checkUpdate() {
        setUpdateCheckBusy(true);
        updateApplyBtn.disabled = true;
        updateTitle.textContent = "正在检查更新";
        updateSummary.textContent = "正在通过镜像站请求 GitHub Release 信息。";
        updateDetails.innerHTML = "";

        try {
            var result = await api("/api/update/check");
            if (result.success) {
                renderUpdateInfo(result.data);
            } else {
                updateTitle.textContent = "检查更新失败";
                updateSummary.textContent = result.message || "无法获取最新版本信息";
                showToast("检查更新失败", "error");
            }
        } catch (e) {
            updateTitle.textContent = "检查更新失败";
            updateSummary.textContent = "更新服务未响应";
            showToast("更新服务未响应", "error");
        }

        setUpdateCheckBusy(false);
    }

    async function applyUpdate() {
        updateApplyBtn.disabled = true;
        updateApplyBtn.innerHTML = '<span class="spinner"></span><span>更新中</span>';
        updateTitle.textContent = "正在更新";
        updateSummary.textContent = "正在下载并替换当前程序，请不要断电。";

        try {
            var result = await api("/api/update/apply", { method: "POST" });
            if (result.success) {
                updateTitle.textContent = "更新完成";
                updateSummary.textContent = "已从 v" + result.data.previous_version + " 更新到 v" + result.data.updated_version + "，请重启服务后生效。";
                updateDetails.innerHTML = renderUpdateRows([
                    ["目标平台", result.data.target],
                    ["更新包", result.data.asset_name],
                    ["SHA256", result.data.sha256],
                    ["下载地址", result.data.downloaded_from],
                    ["安装路径", result.data.installed_path],
                ]);
                showToast("更新完成，请重启服务", "success");
            } else {
                updateTitle.textContent = "更新失败";
                updateSummary.textContent = result.message || "下载或安装更新失败";
                showToast("更新失败", "error");
            }
        } catch (e) {
            updateTitle.textContent = "更新失败";
            updateSummary.textContent = "更新请求失败";
            showToast("更新请求失败", "error");
        }

        updateApplyBtn.innerHTML = '<svg width="16" height="16" viewBox="0 0 16 16" fill="none"><path d="M8 2v8M8 10l3-3M8 10L5 7" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round"/><path d="M3 12.5h10" stroke="currentColor" stroke-width="1.5" stroke-linecap="round"/></svg><span>立即更新</span>';
    }

    async function loadDaemonStatus() {
        try {
            var result = await api("/api/daemon/status");
            if (result.success) {
                renderDaemonStatus(result.data);
            } else {
                daemonStatusContent.innerHTML = '<p class="status-loading">无法获取状态: ' + escapeHtml(result.message) + "</p>";
                daemonTitle.textContent = "状态读取失败";
                daemonSummary.textContent = result.message || "无法获取守护进程状态";
            }
        } catch (e) {
            daemonStatusContent.innerHTML = '<p class="status-loading">服务未响应</p>';
            daemonTitle.textContent = "状态读取失败";
            daemonSummary.textContent = "服务未响应";
        }
    }

    function renderDaemonStatus(s) {
        var installedVersion = s.installed_version ? "v" + s.installed_version : "-";
        var stateText = s.running ? "运行中" : (s.installed ? "已安装" : "未安装");
        var stateClass = s.running ? "connected" : "disconnected";
        var subtitle = s.running ? "PID " + s.pid : (s.installed ? "可启动" : "请先拉取程序");
        var html = "";

        html += '<div class="status-summary">';
        html += '<div class="status-summary-copy">';
        html += '<div class="status-summary-title">进程状态</div>';
        html += '<div class="status-summary-subtitle">' + escapeHtml(subtitle) + "</div>";
        html += "</div>";
        html += '<div class="status-state ' + stateClass + '">' + escapeHtml(stateText) + "</div>";
        html += "</div>";
        html += '<div class="status-details">';
        html += renderStatusItem("安装版本", installedVersion);
        html += renderStatusItem("PID", s.pid || "-");
        html += renderStatusItem("目标架构", compactTarget(s.target));
        html += "</div>";

        daemonStatusContent.innerHTML = html;
        daemonStartHeroBtn.style.display = s.installed && !s.running ? "block" : "none";
        daemonStartBtn.disabled = !s.installed || s.running;
        daemonRestartBtn.disabled = !s.installed || !s.running;
        daemonTitle.textContent = "UDP 网关" + stateText;
        daemonSummary.textContent = s.running ? "守护进程正在运行。" : (s.installed ? "守护进程已安装，可以启动。" : "守护进程尚未安装，请先拉取 release 包。");
        daemonDetails.innerHTML = renderUpdateRows([
            ["运行状态", stateText],
            ["PID", s.pid || "-"],
            ["安装版本", installedVersion],
            ["目标平台", s.target || "-"],
            ["可执行文件", s.executable_path || "-"],
            ["日志文件", s.log_path || "-"],
            ["版本文件", s.version_path || "-"],
        ]);
    }

    async function checkDaemon() {
        setDaemonBusy(daemonCheckBtn, true, "检查中");
        daemonUpgradeBtn.disabled = true;
        daemonTitle.textContent = "正在检查";
        daemonSummary.textContent = "正在通过镜像站请求 GitHub Release 信息。";

        try {
            var result = await api("/api/daemon/check");
            if (result.success) {
                renderDaemonUpdateInfo(result.data);
            } else {
                daemonTitle.textContent = "检查失败";
                daemonSummary.textContent = result.message || "无法获取最新版本信息";
                showToast("UDP 网关检查失败", "error");
            }
        } catch (e) {
            daemonTitle.textContent = "检查失败";
            daemonSummary.textContent = "服务未响应";
            showToast("UDP 网关服务未响应", "error");
        }

        setDaemonBusy(daemonCheckBtn, false, "检查");
    }

    function renderDaemonUpdateInfo(info) {
        var installed = info.installed_version ? "v" + info.installed_version : "未安装";
        if (info.update_available && info.asset_name && info.sha256) {
            daemonTitle.textContent = info.installed_version ? "发现新版本" : "可拉取程序";
            daemonSummary.textContent = installed + "，最新 v" + info.latest_version + "。";
            daemonUpgradeBtn.disabled = !info.installed_version;
        } else if (info.update_available && !info.asset_name) {
            daemonTitle.textContent = "无匹配架构包";
            daemonSummary.textContent = "最新版本 v" + info.latest_version + " 没有匹配 " + info.target + " 的 release asset。";
            daemonUpgradeBtn.disabled = true;
        } else if (info.update_available && !info.sha256) {
            daemonTitle.textContent = "缺少校验摘要";
            daemonSummary.textContent = "最新版本 v" + info.latest_version + " 未提供 GitHub sha256 digest。";
            daemonUpgradeBtn.disabled = true;
        } else {
            daemonTitle.textContent = "已是最新版本";
            daemonSummary.textContent = "当前 " + installed + " 与最新 release 一致。";
            daemonUpgradeBtn.disabled = true;
        }

        daemonDetails.innerHTML = renderUpdateRows([
            ["安装版本", installed],
            ["最新版本", "v" + info.latest_version],
            ["目标平台", info.target],
            ["更新包", info.asset_name || "-"],
            ["SHA256", info.sha256 || "-"],
            ["Release", info.release_url],
            ["镜像顺序", info.mirror_urls && info.mirror_urls.length ? info.mirror_urls.join("\n") : "-"],
        ]);
    }

    async function pullDaemon() {
        await installDaemon("/api/daemon/pull", daemonPullBtn, "拉取中", "拉取");
    }

    async function upgradeDaemon() {
        await installDaemon("/api/daemon/upgrade", daemonUpgradeBtn, "升级中", "升级");
    }

    async function installDaemon(path, button, busyText, normalText) {
        setDaemonBusy(button, true, busyText);
        daemonTitle.textContent = busyText;
        daemonSummary.textContent = "正在下载、校验并安装守护进程。";

        try {
            var result = await api(path, { method: "POST" });
            if (result.success) {
                daemonTitle.textContent = "安装完成";
                daemonSummary.textContent = "已安装 v" + result.data.installed_version + "。";
                daemonDetails.innerHTML = renderUpdateRows([
                    ["上一版本", result.data.previous_version ? "v" + result.data.previous_version : "-"],
                    ["安装版本", "v" + result.data.installed_version],
                    ["目标平台", result.data.target],
                    ["更新包", result.data.asset_name],
                    ["SHA256", result.data.sha256],
                    ["下载地址", result.data.downloaded_from],
                    ["安装路径", result.data.installed_path],
                ]);
                showToast("UDP 网关安装完成", "success");
                loadDaemonStatus();
            } else {
                daemonTitle.textContent = "安装失败";
                daemonSummary.textContent = result.message || "下载或安装失败";
                showToast("UDP 网关安装失败", "error");
            }
        } catch (e) {
            daemonTitle.textContent = "安装失败";
            daemonSummary.textContent = "请求失败";
            showToast("UDP 网关请求失败", "error");
        }

        setDaemonBusy(button, false, normalText);
    }

    async function startDaemon() {
        setDaemonBusy(daemonStartBtn, true, "启动中");
        daemonStartHeroBtn.disabled = true;
        try {
            var result = await api("/api/daemon/start", { method: "POST" });
            if (result.success) {
                showToast("UDP 网关已启动", "success");
                await loadDaemonStatus();
                await loadDaemonLogs();
            } else {
                showToast("UDP 网关启动失败: " + result.message, "error");
            }
        } catch (e) {
            showToast("UDP 网关启动请求失败", "error");
        }
        setDaemonBusy(daemonStartBtn, false, "启动");
        daemonStartHeroBtn.disabled = false;
        loadDaemonStatus();
    }

    async function restartDaemon() {
        setDaemonBusy(daemonRestartBtn, true, "重启中");
        try {
            var result = await api("/api/daemon/restart", { method: "POST" });
            if (result.success) {
                showToast("UDP 网关已重启", "success");
                await loadDaemonStatus();
                await loadDaemonLogs();
            } else {
                showToast("UDP 网关重启失败: " + result.message, "error");
            }
        } catch (e) {
            showToast("UDP 网关重启请求失败", "error");
        }
        setDaemonBusy(daemonRestartBtn, false, "重启");
        loadDaemonStatus();
    }

    async function loadDaemonLogs(silent) {
        if (!silent) setDaemonBusy(daemonLogsBtn, true, "读取中");
        try {
            var result = await api("/api/daemon/logs?lines=160");
            if (result.success) {
                renderAnsiLog(result.data.lines && result.data.lines.length ? result.data.lines.join("\n") : "暂无日志");
            } else {
                daemonLogView.textContent = result.message || "日志读取失败";
            }
        } catch (e) {
            daemonLogView.textContent = "日志请求失败";
        }
        if (!silent) setDaemonBusy(daemonLogsBtn, false, "日志");
    }

    function startDaemonLogPoll() {
        if (daemonLogPollTimer) return;
        daemonLogPollTimer = setInterval(function () { loadDaemonLogs(true); }, 2000);
    }

    function stopDaemonLogPoll() {
        if (!daemonLogPollTimer) return;
        clearInterval(daemonLogPollTimer);
        daemonLogPollTimer = null;
    }

    function renderAnsiLog(text) {
        daemonLogView.innerHTML = ansiUp ? ansiUp.ansi_to_html(text) : escapeHtml(text);
        daemonLogView.scrollTop = daemonLogView.scrollHeight;
    }

    function setDaemonBusy(button, busy, text) {
        if (!button.getAttribute("data-normal-html")) {
            button.setAttribute("data-normal-html", button.innerHTML);
        }
        button.disabled = busy;
        if (busy) {
            button.innerHTML = '<span class="spinner"></span><span>' + escapeHtml(text) + "</span>";
        } else {
            button.innerHTML = button.getAttribute("data-normal-html");
        }
    }

    function renderUpdateInfo(info) {
        var hasAsset = !!info.asset_name;
        var hasChecksum = !!info.checksum_asset_name;
        if (info.update_available && hasAsset && hasChecksum) {
            updateTitle.textContent = "发现新版本";
            updateSummary.textContent = "当前 v" + info.current_version + "，可更新到 v" + info.latest_version + "。";
            updateApplyBtn.disabled = false;
        } else if (info.update_available && !hasAsset) {
            updateTitle.textContent = "发现新版本但无匹配包";
            updateSummary.textContent = "最新版本 v" + info.latest_version + " 没有匹配 " + info.target + " 的更新包。";
            updateApplyBtn.disabled = true;
        } else if (info.update_available && !hasChecksum) {
            updateTitle.textContent = "发现新版本但缺少校验文件";
            updateSummary.textContent = "最新版本 v" + info.latest_version + " 未提供 SHA256 校验文件。";
            updateApplyBtn.disabled = true;
        } else {
            updateTitle.textContent = "已是最新版本";
            updateSummary.textContent = "当前版本 v" + info.current_version + " 与最新 release 一致。";
            updateApplyBtn.disabled = true;
        }

        updateDetails.innerHTML = renderUpdateRows([
            ["当前版本", "v" + info.current_version],
            ["最新版本", "v" + info.latest_version],
            ["目标平台", info.target],
            ["更新包", info.asset_name || "-"],
            ["校验文件", info.checksum_asset_name || "-"],
            ["Release", info.release_url],
            ["镜像顺序", info.mirror_urls && info.mirror_urls.length ? info.mirror_urls.join("\n") : "-"],
        ]);
    }

    function renderUpdateRows(rows) {
        var html = "";
        for (var i = 0; i < rows.length; i++) {
            html += '<div class="update-row">';
            html += '<span class="update-row-label">' + escapeHtml(rows[i][0]) + "</span>";
            html += '<span class="update-row-value">' + escapeHtml(rows[i][1]) + "</span>";
            html += "</div>";
        }
        return html;
    }

    function setUpdateCheckBusy(busy) {
        updateCheckBtn.disabled = busy;
        if (busy) {
            updateCheckBtn.innerHTML = '<span class="spinner"></span><span>检查中</span>';
        } else {
            updateCheckBtn.innerHTML = '<svg width="16" height="16" viewBox="0 0 16 16" fill="none"><path d="M13.5 8a5.5 5.5 0 11-1.61-3.89" stroke="currentColor" stroke-width="1.5" stroke-linecap="round"/><path d="M13.5 3.5v4h-4" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round"/></svg><span>检查更新</span>';
        }
    }

    function startStatusPoll() {
        if (statusPollTimer) clearInterval(statusPollTimer);
        var count = 0;
        statusPollTimer = setInterval(async function () {
            count++;
            await loadStatus();
            if (count >= 10) {
                clearInterval(statusPollTimer);
                statusPollTimer = null;
            }
        }, 2000);
    }

    function showToast(msg, type) {
        toast.textContent = msg;
        toast.className = "toast " + type;
        toast.style.display = "block";
        setTimeout(function () {
            toast.style.display = "none";
        }, 3000);
    }

    function stateLabel(state) {
        if (state === "SCANNING") return "扫描中";
        if (state === "ASSOCIATING") return "关联中";
        if (state === "AUTHENTICATING") return "认证中";
        if (state === "INACTIVE") return "空闲";
        return "未连接";
    }

    function compactTarget(target) {
        if (!target) return "-";
        if (target.indexOf("aarch64") === 0) return "aarch64";
        if (target.indexOf("armv7") === 0) return "armv7";
        if (target.indexOf("x86_64") === 0) return "x86_64";
        return target;
    }

    function escapeHtml(str) {
        if (!str) return "";
        var div = document.createElement("div");
        div.appendChild(document.createTextNode(str));
        return div.innerHTML;
    }

    init();
})();
