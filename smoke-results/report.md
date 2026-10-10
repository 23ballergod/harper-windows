
# Installer smoke test

Machine: Microsoft Windows Server 2025 Datacenter 10.0.26100; GPU: Microsoft Hyper-V Video

## Installer file
- File: `Shah Re-Writer_0.1.0_x64-setup.exe`, 20.3 MB
- Properties: product 'Shah Re-Writer', description 'Shah Re-Writer', company '', copyright '', version '0.1.0'
- Code signature: NotSigned
- Screenshot `0-desktop-before.png`: 1024x768, average brightness 69/255, near-black 0% of the screen

## Installer window
- Visible windows (installer): 1
  - pid 9904: 'Shah Re-Writer Setup' (#32770) at 260,165 size 503x390; topmost=False layered=False click-through=False exstyle=0x10100
- Screenshot `1-installer-window.png`: 1024x768, average brightness 102/255, near-black 4% of the screen

## Install
- Silent install exit code 0 after 2 s
- Installed apps entry: 'Shah Re-Writer' version 0.1.0, publisher 'Shah Re-Writer', size 53 MB
- Install folder: `C:\Users\runneradmin\AppData\Local\Shah Re-Writer`
  - Shah Re-Writer.exe (53.2 MB)
  - uninstall.exe (0.1 MB)
- Shortcut: `C:\Users\runneradmin\AppData\Roaming\Microsoft\Windows\Start Menu\Programs\Shah Re-Writer.lnk`
- Shortcut: `C:\Users\runneradmin\Desktop\Shah Re-Writer.lnk`
- App file properties: product 'Shah Re-Writer', description 'Shah Re-Writer', company 'Shah Re-Writer', copyright '', version '0.1.0'

## Launch
- Running: pid 4776, "C:\Users\runneradmin\AppData\Local\Shah Re-Writer\Shah Re-Writer.exe", 161 MB RAM
- Running: pid 5908, "C:\Users\runneradmin\AppData\Local\Shah Re-Writer\Shah Re-Writer.exe" highlighter, 128 MB RAM
- Main app still running after 30 s: True
- Visible windows (after launch): 3
  - pid 4776: 'Shah Re-Writer Settings' (Tauri Window) at 44,4 size 936x719; topmost=False layered=False click-through=False exstyle=0x40110
  - pid 5908: '' (Winit Thread Event Target) at 0,0 size 16x16; topmost=False layered=True click-through=True exstyle=0x80800A0
  - pid 4776: '' (Tao Thread Event Target) at 0,0 size 16x16; topmost=False layered=True click-through=True exstyle=0x80800A0
- Screenshot `2-after-launch.png`: 1024x768, average brightness 227/255, near-black 0% of the screen

## Typing into Notepad
- Screenshot `3-notepad-typed.png`: 1024x768, average brightness 232/255, near-black 0% of the screen
- Visible windows (with Notepad focused): 4
  - pid 5908: 'Shah Re-Writer' (Window Class) at 0,0 size 1024x768; topmost=True layered=True click-through=True exstyle=0xC0138
  - pid 4776: 'Shah Re-Writer Settings' (Tauri Window) at 44,4 size 936x719; topmost=False layered=False click-through=False exstyle=0x40110
  - pid 5908: '' (Winit Thread Event Target) at 0,0 size 16x16; topmost=False layered=True click-through=True exstyle=0x80800A0
  - pid 4776: '' (Tao Thread Event Target) at 0,0 size 16x16; topmost=False layered=True click-through=True exstyle=0x80800A0

## Typing into Chrome
- Screenshot `5-chrome-typed.png`: 1024x768, average brightness 230/255, near-black 0% of the screen

## Suggestion card in Chrome
- First underline: #228B22 at 490,161
- Screenshot `6-chrome-card.png`: 1024x768, average brightness 228/255, near-black 0% of the screen
- After resting the mouse on it for 2 s: 2098 solid #228B22 pixels below it (the card's main button is about 3000)
- Window in front: 'Shah test - Google Chrome'
- Clicked the main suggestion at 537,297; window in front: 'Shah test - Google Chrome'
- Text after the click: 'This is an test. Their going to the store. She go to school every day.goes'

## Clicking out of the Chrome text box
- Underline-colored pixels 3 s after clicking the page: 5

## Web page text outside a text box
- Screenshot `7-chrome-page.png`: 1024x768, average brightness 231/255, near-black 0% of the screen
- Underline-colored pixels on the page: 3 (front window: 'Shah page - Google Chrome')
- Overlay windows showing with nothing to check: 0

## State while installed
- Starts with Windows (Run key): no
- Data folders:
  - `C:\Users\runneradmin\AppData\Roaming\harper-windows`: absent
  - `C:\Users\runneradmin\AppData\Local\harper-windows`: absent
  - `C:\Users\runneradmin\AppData\Roaming\com.shahrewriter.app`: 0.0 MB
  - `C:\Users\runneradmin\AppData\Local\com.shahrewriter.app`: 4.8 MB

## Underlines
- Screenshot `3-notepad-typed-without-app.png`: 1024x768, average brightness 207/255, near-black 0% of the screen
- Notepad: 417 colored pixels drawn by the app over the window
- Screenshot `5-chrome-typed-without-app.png`: 1024x768, average brightness 205/255, near-black 0% of the screen
- Chrome: 198 colored pixels drawn by the app over the window

### app.log (last 40 lines)
```
2026-10-10T02:43:25.138661Z  INFO harper_desktop_lib::logging: Shah Re-Writer 0.1.0 started (app)
```

### highlighter.log (last 40 lines)
```
2026-10-10T02:43:25.622366Z  INFO harper_desktop_lib::logging: Shah Re-Writer 0.1.0 started (highlighter)
2026-10-10T02:43:26.405199Z  INFO harper_desktop_lib::highlighter::window: Overlay GPU: Microsoft Basic Render Driver (Dx12, Cpu); alpha modes [Auto, Inherit, Opaque, PostMultiplied, PreMultiplied]
2026-10-10T02:43:26.807233Z  WARN egui_wgpu: Software rasterizer detected - loss of performance expected. backend: Dx12, device_type: Cpu, name: "Microsoft Basic Render Driver", driver: "10.0.26100.33438", vendor: Unknown (0x1414), device: 0x8C, subgroup_size: 4..=4, transient_saves_memory: false
2026-10-10T02:43:26.888077Z  INFO harper_desktop_lib::logging: focus: C:\Program Files\WindowsApps\Microsoft.WindowsTerminal_1.23.20211.0_x64__8wekyb3d8bbwe\WindowsTerminal.exe (checking on)
2026-10-10T02:43:27.187604Z  INFO harper_desktop_lib::logging: text element: focused element is a Some(Text) (class "TermControl", framework "XAML"); text pattern: yes; editable: no
2026-10-10T02:43:27.187657Z  INFO harper_desktop_lib::logging: text: no readable text box has keyboard focus
2026-10-10T02:43:27.235846Z  INFO harper_desktop_lib::logging: overlay: 0 underlines to draw across 1 screen(s)
2026-10-10T02:43:29.449848Z  INFO harper_desktop_lib::logging: focus: C:\Users\runneradmin\AppData\Local\Shah Re-Writer\Shah Re-Writer.exe (checking off)
2026-10-10T02:43:55.301499Z  INFO harper_desktop_lib::logging: focus: C:\Windows\System32\notepad.exe (checking on)
2026-10-10T02:43:55.330642Z  INFO harper_desktop_lib::logging: text element: classic Win32 edit box
2026-10-10T02:43:55.353249Z  INFO harper_desktop_lib::logging: text: 0 problems found, 0 with a position on screen
2026-10-10T02:44:01.315622Z  INFO harper_desktop_lib::logging: text: 0 problems found, but their positions on screen are unknown
2026-10-10T02:44:01.363414Z  INFO harper_desktop_lib::logging: text: 3 problems found, 3 with a position on screen
2026-10-10T02:44:01.363526Z  INFO harper_desktop_lib::logging: overlay: 3 underlines to draw across 1 screen(s)
2026-10-10T02:44:21.956843Z  INFO harper_desktop_lib::logging: text element: focused element is a Some(Pane) (class "Chrome_WidgetWin_1", framework "Chrome"); text pattern: no (The operation completed successfully.); editable: no
2026-10-10T02:44:21.956967Z  INFO harper_desktop_lib::logging: text: no readable text box has keyboard focus
2026-10-10T02:44:21.973000Z  INFO harper_desktop_lib::logging: focus: C:\Program Files\Google\Chrome\Application\chrome.exe (checking on)
2026-10-10T02:44:22.031439Z  INFO harper_desktop_lib::logging: overlay: 0 underlines to draw across 1 screen(s)
2026-10-10T02:44:23.032445Z  INFO harper_desktop_lib::logging: text element: focused element is a Some(Edit) (class "", framework "Chrome"); text pattern: yes; editable: yes
2026-10-10T02:44:23.070993Z  INFO harper_desktop_lib::logging: text: 0 problems found, 0 with a position on screen
2026-10-10T02:44:28.484039Z  INFO harper_desktop_lib::logging: text: 0 problems found, but their positions on screen are unknown
2026-10-10T02:44:28.525125Z  INFO harper_desktop_lib::logging: text: 3 problems found, 3 with a position on screen
2026-10-10T02:44:28.525218Z  INFO harper_desktop_lib::logging: overlay: 3 underlines to draw across 1 screen(s)
2026-10-10T02:44:48.317651Z  INFO harper_desktop_lib::logging: text: 4 problems found, 4 with a position on screen
2026-10-10T02:44:48.317757Z  INFO harper_desktop_lib::logging: overlay: 4 underlines to draw across 1 screen(s)
2026-10-10T02:44:56.884217Z  INFO harper_desktop_lib::logging: text: 4 problems found, but their positions on screen are unknown
2026-10-10T02:44:56.914913Z  INFO harper_desktop_lib::logging: text element: focused element is a Some(Pane) (class "Chrome_WidgetWin_1", framework "Chrome"); text pattern: no (The operation completed successfully.); editable: no
2026-10-10T02:44:56.914967Z  INFO harper_desktop_lib::logging: text: no readable text box has keyboard focus
2026-10-10T02:44:56.960661Z  INFO harper_desktop_lib::logging: text element: focused element is a Some(Document) (class "", framework "Chrome"); text pattern: yes; editable: no
2026-10-10T02:44:56.988382Z  INFO harper_desktop_lib::logging: overlay: 0 underlines to draw across 1 screen(s)
```

## Uninstall
- Visible windows (uninstaller): 0
- Screenshot `4-uninstaller-window.png`: 1024x768, average brightness 116/255, near-black 0% of the screen
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
Commit bd9aad0e9226cd8382de42478764ef46bb36a571, run 38016331242
