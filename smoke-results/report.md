
# Installer smoke test

Machine: Microsoft Windows Server 2025 Datacenter 10.0.26100; GPU: Microsoft Hyper-V Video

## Installer file
- File: `Shah Re-Writer_1.0.0_x64-setup.exe`, 24.3 MB
- Properties: product 'Shah Re-Writer', description 'Shah Re-Writer', company '', copyright '', version '1.0.0'
- Code signature: NotSigned
- Screenshot `0-desktop-before.png`: 1024x768, average brightness 65/255, near-black 0% of the screen

## Installer window
- Visible windows (installer): 1
  - pid 8776: 'Shah Re-Writer Setup' (#32770) at 260,165 size 503x390; topmost=False layered=False click-through=False exstyle=0x10100
- Screenshot `1-installer-window.png`: 1024x768, average brightness 99/255, near-black 4% of the screen

## Install
- Silent install exit code 0 after 4 s
- Installed apps entry: 'Shah Re-Writer' version 1.0.0, publisher 'Shah Re-Writer', size 57 MB
- Install folder: `C:\Users\runneradmin\AppData\Local\Shah Re-Writer`
  - Shah Re-Writer.exe (57.2 MB)
  - uninstall.exe (0.1 MB)
- Shortcut: `C:\Users\runneradmin\AppData\Roaming\Microsoft\Windows\Start Menu\Programs\Shah Re-Writer.lnk`
- Shortcut: `C:\Users\runneradmin\Desktop\Shah Re-Writer.lnk`
- App file properties: product 'Shah Re-Writer', description 'Shah Re-Writer', company 'Shah Re-Writer', copyright '', version '1.0.0'

## Launch
- Running: pid 5700, "C:\Users\runneradmin\AppData\Local\Shah Re-Writer\Shah Re-Writer.exe", 161 MB RAM
- Running: pid 10068, "C:\Users\runneradmin\AppData\Local\Shah Re-Writer\Shah Re-Writer.exe" highlighter, 128 MB RAM
- Main app still running after 30 s: True
- Visible windows (after launch): 3
  - pid 5700: 'Shah Re-Writer Settings' (Tauri Window) at 44,4 size 936x719; topmost=False layered=False click-through=False exstyle=0x40110
  - pid 10068: '' (Winit Thread Event Target) at 0,0 size 16x16; topmost=False layered=True click-through=True exstyle=0x80800A0
  - pid 5700: '' (Tao Thread Event Target) at 0,0 size 16x16; topmost=False layered=True click-through=True exstyle=0x80800A0
- Screenshot `2-after-launch.png`: 1024x768, average brightness 225/255, near-black 0% of the screen

## Typing into Notepad
- Screenshot `3-notepad-typed.png`: 1024x768, average brightness 228/255, near-black 0% of the screen
- Visible windows (with Notepad focused): 4
  - pid 10068: 'Shah Re-Writer' (Window Class) at 0,0 size 1024x767; topmost=True layered=True click-through=True exstyle=0xC0138
  - pid 5700: 'Shah Re-Writer Settings' (Tauri Window) at 44,4 size 936x719; topmost=False layered=False click-through=False exstyle=0x40110
  - pid 10068: '' (Winit Thread Event Target) at 0,0 size 16x16; topmost=False layered=True click-through=True exstyle=0x80800A0
  - pid 5700: '' (Tao Thread Event Target) at 0,0 size 16x16; topmost=False layered=True click-through=True exstyle=0x80800A0

## Typing into Chrome
- Screenshot `5-chrome-typed.png`: 1024x768, average brightness 228/255, near-black 0% of the screen

## Suggestion card in Chrome
- First underline: #228B22 at 490,161
- Screenshot `6-chrome-card.png`: 1024x768, average brightness 226/255, near-black 0% of the screen
- After resting the mouse on it for 2 s: 2098 solid #228B22 pixels below it (the card's main button is about 3000)
- Window in front: 'Shah test - Google Chrome'
- Clicked the main suggestion at 537,297; window in front: 'Shah test - Google Chrome'
- Text after the click: 'cac'

## Clicking out of the Chrome text box
- Underline-colored pixels 3 s after clicking the page: 0

## Web page text outside a text box
- Screenshot `7-chrome-page.png`: 1024x768, average brightness 229/255, near-black 0% of the screen
- Underline-colored pixels on the page: 1 (front window: 'Shah page - Google Chrome')
- Overlay windows showing with nothing to check: 0

## State while installed
- Starts with Windows (Run key): no
- Data folders:
  - `C:\Users\runneradmin\AppData\Roaming\harper-windows`: absent
  - `C:\Users\runneradmin\AppData\Local\harper-windows`: absent
  - `C:\Users\runneradmin\AppData\Roaming\com.shahrewriter.app`: 0.0 MB
  - `C:\Users\runneradmin\AppData\Local\com.shahrewriter.app`: 4.8 MB

## Underlines
- Screenshot `3-notepad-typed-without-app.png`: 1024x768, average brightness 209/255, near-black 0% of the screen
- Notepad: 144 colored pixels drawn by the app over the window
- Screenshot `5-chrome-typed-without-app.png`: 1024x768, average brightness 210/255, near-black 0% of the screen
- Chrome: 2079 colored pixels drawn by the app over the window

### app.log (last 40 lines)
```
2026-10-18T23:12:45.725078Z  INFO harper_desktop_lib::logging: Shah Re-Writer 1.0.0 started (app)
```

### highlighter.log (last 40 lines)
```
2026-10-18T23:12:46.663454Z  INFO harper_desktop_lib::logging: Shah Re-Writer 1.0.0 started (highlighter)
2026-10-18T23:12:47.860026Z  INFO harper_desktop_lib::highlighter::window: Overlay GPU: Microsoft Basic Render Driver (Dx12, Cpu); alpha modes [Auto, Inherit, Opaque, PostMultiplied, PreMultiplied]
2026-10-18T23:12:48.354824Z  WARN egui_wgpu: Software rasterizer detected - loss of performance expected. backend: Dx12, device_type: Cpu, name: "Microsoft Basic Render Driver", driver: "10.0.26100.33438", vendor: Unknown (0x1414), device: 0x8C, subgroup_size: 4..=4, transient_saves_memory: false
2026-10-18T23:12:48.391738Z  INFO harper_desktop_lib::logging: focus: C:\Program Files\WindowsApps\Microsoft.WindowsTerminal_1.23.20211.0_x64__8wekyb3d8bbwe\WindowsTerminal.exe (checking on)
2026-10-18T23:12:48.984641Z  INFO harper_desktop_lib::logging: text element: focused element is a Some(Text) (class "TermControl", framework "XAML"); text pattern: yes; editable: no
2026-10-18T23:12:48.984725Z  INFO harper_desktop_lib::logging: text: no readable text box has keyboard focus
2026-10-18T23:12:49.122894Z  INFO harper_desktop_lib::logging: overlay: 0 underlines to draw across 1 screen(s)
2026-10-18T23:12:50.553682Z  INFO harper_desktop_lib::logging: focus: C:\Users\runneradmin\AppData\Local\Shah Re-Writer\Shah Re-Writer.exe (checking off)
2026-10-18T23:13:15.945119Z  INFO harper_desktop_lib::logging: focus: C:\Windows\System32\notepad.exe (checking on)
2026-10-18T23:13:15.976501Z  INFO harper_desktop_lib::logging: text element: classic Win32 edit box
2026-10-18T23:13:16.020196Z  INFO harper_desktop_lib::logging: text: 0 problems found, 0 with a position on screen
2026-10-18T23:13:22.003827Z  INFO harper_desktop_lib::logging: text: 3 problems found, 3 with a position on screen
2026-10-18T23:13:22.003929Z  INFO harper_desktop_lib::logging: overlay: 3 underlines to draw across 1 screen(s)
2026-10-18T23:13:43.428838Z  INFO harper_desktop_lib::logging: text: 3 problems found, but their positions on screen are unknown
2026-10-18T23:13:43.432170Z  INFO harper_desktop_lib::logging: focus: C:\Program Files\Google\Chrome\Application\chrome.exe (checking on)
2026-10-18T23:13:43.464672Z  INFO harper_desktop_lib::logging: text element: focused element is a Some(Pane) (class "Chrome_WidgetWin_1", framework "Chrome"); text pattern: no (The operation completed successfully.); editable: no
2026-10-18T23:13:43.464721Z  INFO harper_desktop_lib::logging: text: no readable text box has keyboard focus
2026-10-18T23:13:43.670080Z  INFO harper_desktop_lib::logging: overlay: 0 underlines to draw across 1 screen(s)
2026-10-18T23:13:44.430029Z  INFO harper_desktop_lib::logging: text: 0 problems found, 0 with a position on screen
2026-10-18T23:13:49.207195Z  INFO harper_desktop_lib::logging: text: 3 problems found, 3 with a position on screen
2026-10-18T23:13:49.207297Z  INFO harper_desktop_lib::logging: overlay: 3 underlines to draw across 1 screen(s)
2026-10-18T23:14:09.124386Z  INFO harper_desktop_lib::logging: text: 2 problems found, 2 with a position on screen
2026-10-18T23:14:09.124486Z  INFO harper_desktop_lib::logging: overlay: 2 underlines to draw across 1 screen(s)
2026-10-18T23:14:12.065151Z  INFO harper_desktop_lib::logging: text: 4 problems found, 4 with a position on screen
2026-10-18T23:14:12.065255Z  INFO harper_desktop_lib::logging: overlay: 4 underlines to draw across 1 screen(s)
2026-10-18T23:14:14.630827Z  INFO harper_desktop_lib::logging: text: 0 problems found, 0 with a position on screen
2026-10-18T23:14:14.630922Z  INFO harper_desktop_lib::logging: overlay: 0 underlines to draw across 1 screen(s)
2026-10-18T23:14:17.018037Z  INFO harper_desktop_lib::logging: text: 1 problems found, 1 with a position on screen
2026-10-18T23:14:17.018140Z  INFO harper_desktop_lib::logging: overlay: 1 underlines to draw across 1 screen(s)
2026-10-18T23:14:24.680165Z  INFO harper_desktop_lib::logging: text: no readable text box has keyboard focus
2026-10-18T23:14:24.727181Z  INFO harper_desktop_lib::logging: text element: focused element is a Some(Document) (class "", framework "Chrome"); text pattern: yes; editable: no
2026-10-18T23:14:24.825244Z  INFO harper_desktop_lib::logging: overlay: 0 underlines to draw across 1 screen(s)
```

## Uninstall
- Visible windows (uninstaller): 0
- Screenshot `4-uninstaller-window.png`: 1024x768, average brightness 112/255, near-black 0% of the screen
- Silent uninstall exit code 0
- Installed apps entry removed: True
- Install folder: removed
- Shortcuts left: none
- Run key left: none
- Data folders left (the stand-in model is 100 MB):
  - `C:\Users\runneradmin\AppData\Roaming\harper-windows`: absent
  - `C:\Users\runneradmin\AppData\Local\harper-windows`: absent
  - `C:\Users\runneradmin\AppData\Roaming\com.shahrewriter.app`: 0.0 MB
  - `C:\Users\runneradmin\AppData\Local\com.shahrewriter.app`: 104.9 MB

## Verdict
PASS: installs, launches without blacking out the screen, keeps running, and uninstalls.
Commit d7447eaf0940eb51e5f0eab81baa1bb4d39fddba, run 39173717851
