# idevice desktop

基于 React、Tauri 2 和 [`jkcoxson/idevice`](https://github.com/jkcoxson/idevice) 的 iPhone / iPad 桌面管理工具。Rust 依赖固定在提交 `8eed181f39a16ea70380ec8c3cff6bed07a1ef69`，避免上游接口变化直接破坏构建。

## 已接入的真实功能

- usbmuxd 设备发现、热插拔监听、设备选择、配对、取消配对和断开
- Lockdown 设备概览、配对信息、电池与 AFC 存储容量
- Diagnostics Relay：电池、MobileGestalt、IORegistry、NAND、Wi-Fi
- AFC 文件浏览、上传、下载、新建目录和递归删除
- Installation Proxy 应用列表、IPA 安装、卸载及进度事件
- OS Trace 实时结构化日志、暂停、筛选和清空
- Developer Mode、Developer Disk Image 挂载与卸载
- iOS 17+ CoreDevice/RSD 软件隧道、应用启动、debug proxy 附加和 JIT 会话
- DVT/RSD 与旧版 Lockdown 两套定位模拟通道

直接在普通浏览器里运行时会自动使用设计演示数据；通过 Tauri 启动时会自动切换到真实设备命令。

## 开发运行

需要 Node.js、Rust、系统的 usbmuxd 服务，以及 Tauri 对应平台的系统开发工具。

```bash
npm install
npm run desktop:dev
```

只预览前端设计：

```bash
npm run dev
```

## 构建

```bash
npm run build
npm run desktop:build
```

## 开发者功能说明

- 初次配对必须使用 USB，并在设备上点按“信任”。
- iOS 16 及更早版本挂载 DDI 需要 `DeveloperDiskImage.dmg` 和对应 `.signature`。
- iOS 17 及更高版本个性化挂载需要镜像、`BuildManifest.plist` 和 trust cache。
- JIT 会启动所选应用并维持 debug proxy 附加；关闭开关、切换设备或退出页面会结束会话。
