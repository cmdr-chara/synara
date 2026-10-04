#![cfg(target_os = "macos")]

use std::{
    path::{Path, PathBuf},
    process::Command,
    time::Duration,
};
use synara_runtime::{
    DeviceAvailability, DeviceBackend, DeviceCancellation, DeviceTools, ToolDevice,
};

const BUNDLE_ID: &str = "dev.synara.acceptance";

async fn wait_for(
    tools: &DeviceTools,
    id: &str,
    availability: DeviceAvailability,
    cancel: &DeviceCancellation,
) -> ToolDevice {
    for _ in 0..120 {
        let devices = tools.discover(cancel).await.expect("simulator discovery");
        if let Some(device) = devices.into_iter().find(|device| {
            device.descriptor.id.as_str() == id && device.availability == availability
        }) {
            return device;
        }
        tokio::time::sleep(Duration::from_secs(1)).await;
    }
    panic!("simulator {id} did not reach {availability:?}");
}

fn build_fixture_app() -> (tempfile::TempDir, PathBuf) {
    let root = tempfile::tempdir().expect("fixture tempdir");
    let bundle = root.path().join("SynaraAcceptance.app");
    std::fs::create_dir(&bundle).expect("fixture bundle");

    let source = root.path().join("main.m");
    std::fs::write(
        &source,
        r#"#import <UIKit/UIKit.h>

@interface SynaraAcceptanceDelegate : UIResponder <UIApplicationDelegate>
@property(strong, nonatomic) UIWindow *window;
@end

@implementation SynaraAcceptanceDelegate
- (BOOL)application:(UIApplication *)application
    didFinishLaunchingWithOptions:(NSDictionary *)launchOptions {
    (void)application;
    (void)launchOptions;
    self.window = [[UIWindow alloc] initWithFrame:[UIScreen mainScreen].bounds];
    UIViewController *controller = [UIViewController new];
    controller.view.backgroundColor = [UIColor systemBackgroundColor];
    self.window.rootViewController = controller;
    [self.window makeKeyAndVisible];
    return YES;
}
@end

int main(int argc, char *argv[]) {
    @autoreleasepool {
        return UIApplicationMain(
            argc,
            argv,
            nil,
            NSStringFromClass([SynaraAcceptanceDelegate class])
        );
    }
}
"#,
    )
    .expect("fixture source");

    let sdk = Command::new("/usr/bin/xcrun")
        .args(["--sdk", "iphonesimulator", "--show-sdk-path"])
        .output()
        .expect("query simulator SDK");
    assert!(
        sdk.status.success(),
        "xcrun could not locate the iPhone Simulator SDK"
    );
    let sdk = String::from_utf8(sdk.stdout)
        .expect("SDK path UTF-8")
        .trim()
        .to_owned();
    let target = match std::env::consts::ARCH {
        "aarch64" => "arm64-apple-ios16.0-simulator",
        "x86_64" => "x86_64-apple-ios16.0-simulator",
        arch => panic!("unsupported hosted macOS architecture: {arch}"),
    };
    let executable = bundle.join("SynaraAcceptance");
    let compile = Command::new("/usr/bin/xcrun")
        .args([
            "--sdk",
            "iphonesimulator",
            "clang",
            "-fobjc-arc",
            "-target",
            target,
        ])
        .arg("-isysroot")
        .arg(&sdk)
        .args(["-framework", "UIKit", "-framework", "Foundation"])
        .arg(&source)
        .arg("-o")
        .arg(&executable)
        .output()
        .expect("compile simulator fixture");
    assert!(
        compile.status.success(),
        "simulator fixture compile failed: {}",
        String::from_utf8_lossy(&compile.stderr)
    );

    std::fs::write(
        bundle.join("Info.plist"),
        r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>CFBundleDisplayName</key><string>Synara Acceptance</string>
    <key>CFBundleExecutable</key><string>SynaraAcceptance</string>
    <key>CFBundleIdentifier</key><string>dev.synara.acceptance</string>
    <key>CFBundleName</key><string>SynaraAcceptance</string>
    <key>CFBundlePackageType</key><string>APPL</string>
    <key>CFBundleShortVersionString</key><string>1.0</string>
    <key>CFBundleVersion</key><string>1</string>
    <key>MinimumOSVersion</key><string>16.0</string>
    <key>UIDeviceFamily</key>
    <array><integer>1</integer><integer>2</integer></array>
    <key>UILaunchScreen</key><dict/>
</dict>
</plist>
"#,
    )
    .expect("fixture Info.plist");

    assert!(Path::new(&executable).is_file());
    (root, bundle)
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[ignore = "requires a real macOS CoreSimulator runtime"]
async fn real_core_simulator_accepts_synara_lifecycle_and_app_operations() {
    let cancel = DeviceCancellation::new();
    let tools = DeviceTools::new(DeviceBackend::AppleSimulator, None, None)
        .expect("Apple Simulator DeviceTools");

    let devices = tools.discover(&cancel).await.expect("simulator discovery");
    let target = devices
        .iter()
        .find(|device| device.availability == DeviceAvailability::Stopped)
        .or_else(|| {
            devices
                .iter()
                .find(|device| device.availability == DeviceAvailability::Ready)
        })
        .cloned()
        .expect("an available iOS Simulator runtime");
    assert_eq!(target.descriptor.platform, "iOS Simulator");
    let id = target.descriptor.id.as_str().to_owned();

    let mut checks = Vec::new();
    let ready = if target.availability == DeviceAvailability::Stopped {
        assert!(tools.can_boot(&target));
        tools
            .set_running(&target, true, &cancel)
            .await
            .expect("boot selected simulator");
        checks.push("boot");
        wait_for(&tools, &id, DeviceAvailability::Ready, &cancel).await
    } else {
        target
    };

    let png = tools
        .capture(&ready, &cancel)
        .await
        .expect("capture real simulator screenshot");
    assert!(png.starts_with(b"\x89PNG\r\n\x1a\n"));
    assert!(png.len() > 1024);
    checks.push("screenshot");

    let recordings = tempfile::tempdir().expect("owned recording directory");
    let destination = recordings.path().join("simulator.mov");
    let stop = DeviceCancellation::new();
    assert!(tools.can_record_video(&ready));
    let (recorded, ()) = tokio::join!(
        tools.record_video(&ready, destination.clone(), &stop, &cancel),
        async {
            tokio::time::sleep(Duration::from_secs(1)).await;
            stop.cancel();
        },
    );
    assert_eq!(
        recorded.expect("record and finalize real simulator video"),
        destination
    );
    assert!(
        std::fs::metadata(&destination)
            .expect("finalized MOV")
            .len()
            > 1024
    );
    checks.push("record-stop-finalize-mov");

    tools
        .open_url(&ready, "https://example.com/synara-a06", &cancel)
        .await
        .expect("open reviewed HTTP URL");
    checks.push("open-url");

    tools
        .launch_app(&ready, "com.apple.mobilesafari", &cancel)
        .await
        .expect("launch installed Safari");
    tools
        .terminate_app(&ready, "com.apple.mobilesafari", &cancel)
        .await
        .expect("terminate installed Safari");
    checks.push("launch-terminate-system-app");

    let (_fixture_root, bundle) = build_fixture_app();
    tools
        .install_app(&ready, &bundle, &cancel)
        .await
        .expect("install reviewed local simulator app");
    tools
        .launch_app(&ready, BUNDLE_ID, &cancel)
        .await
        .expect("launch installed fixture app");
    tools
        .terminate_app(&ready, BUNDLE_ID, &cancel)
        .await
        .expect("terminate installed fixture app");
    checks.push("install-launch-terminate-local-app");

    assert!(tools.can_shutdown(&ready));
    tools
        .set_running(&ready, false, &cancel)
        .await
        .expect("shutdown selected simulator");
    wait_for(&tools, &id, DeviceAvailability::Stopped, &cancel).await;
    checks.push("shutdown");

    if let Ok(path) = std::env::var("SYNARA_SIMULATOR_EVIDENCE") {
        let evidence = serde_json::json!({
            "device_id": id,
            "device_name": ready.descriptor.name,
            "runtime": ready.runtime,
            "checks": checks,
        });
        std::fs::write(
            path,
            serde_json::to_vec_pretty(&evidence).expect("serialize evidence"),
        )
        .expect("write simulator evidence");
    }
}
