# LocalFlow Android APK 构建与签名指南

## 环境要求

- Flutter SDK 3.22+（stable channel）
- Android Studio Hedgehog+ 或 Android SDK 34
- JDK 17
- Gradle 8.5+
- Rust 工具链（用于 FFI 编译，可选）

## 快速开始

### 1. 生成 Flutter 项目

首次构建前，需要在 `apps/mobile` 目录生成 platform-specific 代码：

```bash
cd apps/mobile
flutter create --project-name localflow --platforms android .
```

这会生成 `android/` 目录下的完整 Gradle 项目和 `ios/` 目录。

### 2. 安装依赖

```bash
cd apps/mobile
flutter pub get
```

### 3. 在模拟器上运行

```bash
cd apps/mobile
flutter run
```

要求：Android 模拟器已启动（API 26+）。

## 构建 Debug APK

```bash
cd apps/mobile
flutter build apk --debug
```

APK 位置：`build/app/outputs/flutter-apk/app-debug.apk`

### 安装到设备

```bash
adb install build/app/outputs/flutter-apk/app-debug.apk
```

## 构建 Release APK

### 1. 生成签名密钥

```bash
keytool -genkey -v \
  -keystore localflow-release.keystore \
  -alias localflow \
  -keyalg RSA \
  -keysize 2048 \
  -validity 10000 \
  -storepass your_store_password \
  -keypass your_key_password
```

### 2. 配置签名

在 `android/key.properties` 中配置签名信息：

```properties
storePassword=your_store_password
keyPassword=your_key_password
keyAlias=localflow
storeFile=../localflow-release.keystore
```

### 3. 启用 Release 签名

编辑 `android/app/build.gradle`，取消 `signingConfig signingConfigs.release` 的注释。

### 4. 构建 Release APK

```bash
cd apps/mobile
flutter build apk --release
```

APK 位置：`build/app/outputs/flutter-apk/app-release.apk`

## 构建 App Bundle（推荐发布用）

```bash
cd apps/mobile
flutter build appbundle --release
```

AAB 位置：`build/app/outputs/bundle/release/app-release.aab`

## Rust FFI 集成（可选增强）

LocalFlow Mobile 的完整安全模型依赖 Rust Core 进行密钥管理和工作流执行。
为集成 Rust FFI：

### 1. 添加 Rust 交叉编译目标

```bash
rustup target add aarch64-linux-android armv7-linux-androideabi x86_64-linux-android i686-linux-android
```

### 2. 安装 cargo-ndk

```bash
cargo install cargo-ndk
```

### 3. 编译 Rust 共享库

```bash
cd D:\Steam\LocalFlow
cargo ndk \
  -t arm64-v8a \
  -t armeabi-v7a \
  -t x86_64 \
  -o apps/mobile/android/app/src/main/jniLibs \
  build --release -p localflow-core
```

### 4. 配置 Gradle 使用 FFI

在 `android/app/build.gradle` 中取消 NDK abiFilters 的注释：

```groovy
defaultConfig {
    ndk {
        abiFilters "arm64-v8a", "armeabi-v7a", "x86_64"
    }
}
```

## 权限最小化说明

LocalFlow Android 应用只请求以下权限：

| 权限 | 用途 | 说明 |
|------|------|------|
| `INTERNET` | 用户配置的 API 调用 | 仅在用户触发时使用 |
| `ACCESS_NETWORK_STATE` | 网络状态监测 | 用于断网提示 |

**未请求的权限**（零攻击面）：
- 存储读写（`READ/WRITE_EXTERNAL_STORAGE`）
- 相机、麦克风
- 位置
- 通讯录
- 后台服务
- 无障碍服务

## 安全实践

### API Key 保护
- API Key 存入 Android Keystore（AES-256 GCM 加密）
- Flutter UI 只能写入/检查 Key，不能读取已存 Key 的值
- Key 不出现在日志、崩溃报告、备份或导出包中

### 网络请求
- 默认 HTTPS 传输
- 自动阻止访问 localhost、内网地址、云元数据地址
- 请求和响应大小限制（默认 10MB）

### 数据存储
- 数据库文件在应用私有目录（`data/data/com.localflow.app/`）
- 备份默认禁用（`android:allowBackup="false"`）
- 导出文件不包含任何 API Key

## 低内存与后台处理

- Flutter 自动处理低内存场景（WidgetsBindingObserver）
- 工作流运行使用 `compute()` 在后台 isolate 执行
- 网络断开时显示用户友好的错误提示，不崩溃
- 使用 `connectivity_plus` 监测网络状态变化

## 常见问题

### Flutter pub get 失败
确保 Android SDK 路径正确：
```bash
flutter config --android-sdk /path/to/android-sdk
```

### Gradle 构建报错
检查 JDK 版本和 Gradle 缓存：
```bash
flutter clean
cd android
./gradlew clean
cd ..
flutter pub get
flutter build apk --debug
```

### 模拟器无法连接网络
在 AndroidManifest.xml 中检查 INTERNET 权限，
或在网络安全配置中添加模拟器的 IP 范围。

## 验证清单

- [ ] `flutter analyze` 无错误
- [ ] `flutter test` 全部通过
- [ ] Debug APK 可以安装并启动
- [ ] 可以创建 Agent
- [ ] 可以配置 API Key（存入 Keystore）
- [ ] API Key 不在日志或导出中出现
- [ ] 断网时显示错误而非崩溃
- [ ] Release APK 签名正确