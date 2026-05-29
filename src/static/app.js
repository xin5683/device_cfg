(function () {
    "use strict";

    var selectedNetwork = null;
    var statusPollTimer = null;

    var statusContent = document.getElementById("status-content");
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
            if (e.target === modal) closeModal();
        });
        passwordInput.addEventListener("keydown", function (e) {
            if (e.key === "Enter") doConnect();
        });

        loadStatus();
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
                statusContent.innerHTML = '<p class="hint">无法获取状态: ' + escapeHtml(result.message) + "</p>";
            }
        } catch (e) {
            statusContent.innerHTML = '<p class="hint">服务未响应</p>';
        }
    }

    function renderStatus(s) {
        var connected = s.state === "COMPLETED";
        var html = "";

        html += '<div class="status-row"><span class="status-label">状态</span>';
        html += '<span class="status-value ' + (connected ? "status-connected" : "status-disconnected") + '">';
        html += connected ? "已连接" : "未连接";
        html += "</span></div>";

        if (connected && s.ssid) {
            html += '<div class="status-row"><span class="status-label">SSID</span>';
            html += '<span class="status-value">' + escapeHtml(s.ssid) + "</span></div>";
        }

        if (connected && s.ip) {
            html += '<div class="status-row"><span class="status-label">IP 地址</span>';
            html += '<span class="status-value">' + escapeHtml(s.ip) + "</span></div>";
        }

        if (connected && s.bssid) {
            html += '<div class="status-row"><span class="status-label">BSSID</span>';
            html += '<span class="status-value">' + escapeHtml(s.bssid) + "</span></div>";
        }

        statusContent.innerHTML = html;
        disconnectBtn.style.display = connected ? "block" : "none";
    }

    async function doScan() {
        scanBtn.disabled = true;
        scanBtn.innerHTML = '<span class="spinner"></span>扫描中';
        networkList.innerHTML = '<p class="status-loading">正在扫描 WiFi 网络，请稍候...</p>';

        try {
            var result = await api("/api/scan");
            if (result.success) {
                renderNetworks(result.data);
            } else {
                networkList.innerHTML = '<p class="hint">扫描失败: ' + escapeHtml(result.message) + "</p>";
            }
        } catch (e) {
            networkList.innerHTML = '<p class="hint">扫描请求失败</p>';
        }

        scanBtn.disabled = false;
        scanBtn.textContent = "扫描";
    }

    function renderNetworks(networks) {
        if (!networks || networks.length === 0) {
            networkList.innerHTML = '<p class="hint">未发现 WiFi 网络，请确认天线已连接</p>';
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
            html += '<div class="network-meta">' + n.signal + " dBm &middot; " + n.frequency + " MHz</div>";
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
            document.getElementById("modal-title").textContent = "连接开放网络";
        } else {
            passwordGroup.style.display = "block";
            passwordInput.value = "";
            document.getElementById("modal-title").textContent = "连接 WiFi";
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
        connectBtn.innerHTML = '<span class="spinner"></span>连接中';

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
        disconnectBtn.innerHTML = '<span class="spinner"></span>断开中';

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

    function escapeHtml(str) {
        if (!str) return "";
        var div = document.createElement("div");
        div.appendChild(document.createTextNode(str));
        return div.innerHTML;
    }

    init();
})();
