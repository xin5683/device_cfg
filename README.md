# Device Config

基于 Rust + axum 的嵌入式设备控制 Web 前后端，当前内置 WiFi 配置能力，适用于 Luckfox Lyra Ultra W、RK 系列等 Linux 设备。

## 定位

`device_cfg` 不是单纯的 WiFi 配网页，而是面向嵌入式设备控制面的轻量控制台：

- 单一二进制交付，适合设备侧部署
- 前端静态资源内嵌，无需额外文件系统依赖
- 后端通过 HTTP API 暴露设备控制能力
- 当前提供 WiFi 控制，后续可扩展设备概览、诊断工具、日志能力

## 当前能力

- WiFi 扫描
- WiFi 连接与断开
- 当前网络状态查询
- 跨域调用支持
- `armv7` 与 `aarch64` 交叉编译

## 快速开始

### 本地运行

```bash
PORT=3000 cargo run
```

访问 `http://localhost:3000`。

### 交叉编译

```bash
cargo build --release --target aarch64-unknown-linux-gnu
ls -lh target/aarch64-unknown-linux-gnu/release/device_cfg
```

如需 `armv7`：

```bash
cargo build --release --target armv7-unknown-linux-gnueabihf
ls -lh target/armv7-unknown-linux-gnueabihf/release/device_cfg
```

## 部署到设备

```bash
scp target/aarch64-unknown-linux-gnu/release/device_cfg root@<board-ip>:/usr/bin/
ssh root@<board-ip> "sudo env WPA_CTRL_DIR=/run/wpa_supplicant WIFI_IFACE=wlan1 PORT=3000 /usr/bin/device_cfg"
```

访问 `http://<board-ip>:3000`。

## 环境变量

- `PORT`：服务端口，默认 `80`
- `WPA_CTRL_DIR`：`wpa_supplicant` 控制目录，默认 `/var/run/wpa_supplicant`
- `WIFI_IFACE`：无线网卡接口名，默认 `wlan0`

## API

### `GET /api/scan`

扫描周边 WiFi。

### `GET /api/status`

读取当前连接状态。

### `POST /api/connect`

请求体示例：

```json
{
  "ssid": "MyWiFi",
  "password": "mypassword"
}
```

开放网络可省略 `password`。

### `POST /api/disconnect`

断开当前 WiFi。

## 架构

当前代码按“设备能力 / Web 接口 / 启动装配”分层：

```text
device_cfg/
├── Cargo.toml
├── .cargo/
│   └── config.toml
├── src/
│   ├── main.rs
│   ├── app.rs                # 应用启动、配置、路由装配
│   ├── device/
│   │   ├── mod.rs
│   │   └── wifi.rs           # 设备 WiFi 控制服务
│   ├── web/
│   │   ├── mod.rs
│   │   ├── api.rs            # HTTP API
│   │   └── assets.rs         # 静态资源输出
│   └── static/
│       ├── index.html
│       ├── style.css
│       └── app.js
```

这套结构适合继续扩展：

- `device/`：增加设备信息、LED、GPIO、系统状态等能力
- `web/api.rs`：增加对应控制接口
- `static/`：扩展前端页签和控制页面

## 运行依赖

目标设备需具备：

- `wpa_supplicant`
- `wpa_cli`
- `ip`
- `udhcpc` 或等价 DHCP 客户端

## 故障排查

### 扫描为空

- 检查天线与接口名是否正确
- 确认 `wpa_supplicant` 已运行
- 手工验证 `wpa_cli scan` 与 `wpa_cli scan_results`

### 连接失败

- 检查密码是否正确
- 查看 `wpa_cli status`
- 查看 `wpa_cli list_networks`

## 许可证

MIT
