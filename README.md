# WiFi 配置 Web 管理工具

基于 Rust + axum 的 WiFi 配置 Web 服务，适用于 Luckfox Lyra Ultra W (Buildroot) 等嵌入式设备。

## 特性

- **前后端分离**: JSON API + 嵌入式前端，API 可独立调用
- **CORS 支持**: 允许跨域请求，方便开发和测试
- **单一二进制**: 前端资源通过 `include_str!` 打包，无需额外文件
- **交叉编译**: 支持 ARM 目标平台 (armv7-unknown-linux-gnueabihf)

## 快速开始

### 本地开发

```bash
# 运行（默认端口 80，可通过环境变量修改）
PORT=3000 cargo run

# 访问
open http://localhost:3000
```

### 交叉编译

```bash
# 编译 ARM 版本
cargo build --release --target armv7-unknown-linux-gnueabihf

# 产物位置
ls -lh target/armv7-unknown-linux-gnueabihf/release/wifi_cfg
```

### 部署到目标板

```bash
# 复制二进制到板子
scp target/armv7-unknown-linux-gnueabihf/release/wifi_cfg root@<board-ip>:/usr/bin/

# 确保 wpa_supplicant 已运行
ssh root@<board-ip> "wpa_supplicant -B -i wlan0 -c /etc/wpa_supplicant.conf"

# 启动服务
ssh root@<board-ip> "/usr/bin/wifi_cfg"

# 浏览器访问
open http://<board-ip>
```

## API 端点

所有 API 返回 JSON 格式，支持 CORS 跨域请求。

### 扫描 WiFi 网络

```bash
GET /api/scan
```

**响应示例:**
```json
{
  "success": true,
  "data": [
    {
      "ssid": "MyWiFi",
      "bssid": "AA:BB:CC:DD:EE:FF",
      "signal": -45,
      "security": "WPA2",
      "frequency": 2437
    },
    {
      "ssid": "OpenNetwork",
      "bssid": "11:22:33:44:55:66",
      "signal": -68,
      "security": "开放",
      "frequency": 2412
    }
  ]
}
```

### 获取连接状态

```bash
GET /api/status
```

**响应示例:**
```json
{
  "success": true,
  "data": {
    "state": "COMPLETED",
    "ssid": "MyWiFi",
    "bssid": "AA:BB:CC:DD:EE:FF",
    "ip": "192.168.1.100"
  }
}
```

**state 可能的值:**
- `COMPLETED` - 已连接
- `DISCONNECTED` - 未连接
- `ASSOCIATING` - 正在连接
- `AUTHENTICATING` - 认证中

### 连接 WiFi

```bash
POST /api/connect
Content-Type: application/json
```

**请求体 (加密网络):**
```json
{
  "ssid": "MyWiFi",
  "password": "mypassword"
}
```

**请求体 (开放网络):**
```json
{
  "ssid": "OpenNetwork"
}
```

**响应:**
```json
{
  "success": true,
  "data": "连接成功"
}
```

### 断开连接

```bash
POST /api/disconnect
```

**响应:**
```json
{
  "success": true,
  "data": "已断开连接"
}
```

## API 测试示例

使用 `curl` 测试 API：

```bash
# 扫描网络
curl http://localhost:3000/api/scan | jq

# 查看状态
curl http://localhost:3000/api/status | jq

# 连接 WiFi
curl -X POST http://localhost:3000/api/connect \
  -H "Content-Type: application/json" \
  -d '{"ssid":"MyWiFi","password":"mypassword"}'

# 断开连接
curl -X POST http://localhost:3000/api/disconnect
```

## 前端独立部署

前端文件位于 `src/static/`，可以用任何静态服务器托管：

```bash
# 使用 Python 启动静态服务器
cd src/static
python3 -m http.server 8080

# 访问 http://localhost:8080
# 前端会自动调用 API（需要后端服务运行）
```

## 系统要求

### 目标板

- `wpa_supplicant` 已安装并运行
- `wpa_cli` 命令可用
- 控制接口: `/var/run/wpa_supplicant`
- 网络接口: `wlan0`

### 开发环境

- Rust 1.70+
- 交叉编译工具链: `arm-linux-gnueabihf-gcc`

## 项目结构

```
wifi_cfg/
├── Cargo.toml              # 依赖配置
├── .cargo/
│   └── config.toml         # 交叉编译 linker 配置
├── src/
│   ├── main.rs             # 入口：路由、静态资源、CORS
│   ├── wifi.rs             # WiFi 操作层（wpa_cli 封装）
│   ├── handlers.rs         # HTTP 请求处理器
│   └── static/             # 前端资源（编译时嵌入）
│       ├── index.html
│       ├── style.css
│       └── app.js
└── target/                 # 编译产物
```

## 故障排查

### wpa_cli 命令失败

确保 `wpa_supplicant` 已运行：
```bash
wpa_supplicant -B -i wlan0 -c /etc/wpa_supplicant.conf
```

检查控制接口是否存在：
```bash
ls -l /var/run/wpa_supplicant/wlan0
```

### 扫描无结果

- 确认天线已连接
- 等待几秒后重试（扫描需要时间）
- 检查 `wpa_cli scan_results` 是否有输出

### 连接超时

- 检查密码是否正确
- 确认信号强度足够（>-75 dBm）
- 查看 `wpa_cli status` 的详细状态

## 许可证

MIT
