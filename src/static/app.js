(function () {
    "use strict";

    var selectedNetwork = null;
    var statusPollTimer = null;

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

        initTabs();
        loadStatus();
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

    function escapeHtml(str) {
        if (!str) return "";
        var div = document.createElement("div");
        div.appendChild(document.createTextNode(str));
        return div.innerHTML;
    }

    init();
})();
