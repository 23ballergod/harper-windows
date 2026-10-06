
# Installer smoke test

Machine: Microsoft Windows Server 2025 Datacenter 10.0.26100; GPU: Microsoft Hyper-V Video

## Installer file
- File: `Harper for Windows_2.12.0_x64-setup.exe`, 20.1 MB
- Properties: product 'Harper for Windows', description 'Harper for Windows', company '', copyright '', version '2.12.0'
- Code signature: NotSigned
- Screenshot `0-desktop-before.png`: 1024x768, average brightness 70/255, near-black 0% of the screen

## Installer window
- Visible windows (installer): 1
  - pid 10164: 'Harper for Windows Setup' (#32770) at 260,165 size 503x390; topmost=False layered=False click-through=False exstyle=0x10100
- Screenshot `1-installer-window.png`: 1024x768, average brightness 102/255, near-black 4% of the screen

## Install
- Silent install exit code 0 after 4 s
- Installed apps entry: 'Harper for Windows' version 2.12.0, publisher 'harper-windows', size 53 MB
- Install folder: `C:\Users\runneradmin\AppData\Local\Harper for Windows`
  - harper-desktop.exe (52.5 MB)
  - uninstall.exe (0.1 MB)
- Shortcut: `C:\Users\runneradmin\AppData\Roaming\Microsoft\Windows\Start Menu\Programs\Harper for Windows.lnk`
- Shortcut: `C:\Users\runneradmin\Desktop\Harper for Windows.lnk`
- App file properties: product 'Harper for Windows', description 'Harper for Windows', company 'harper-windows', copyright '', version '2.12.0'

## Launch
- Running: pid 8648, "C:\Users\runneradmin\AppData\Local\Harper for Windows\harper-desktop.exe", 162 MB RAM
- Running: pid 7884, "C:\Users\runneradmin\AppData\Local\Harper for Windows\harper-desktop.exe" highlighter, 190 MB RAM
- Main app still running after 30 s: True
- Visible windows (after launch): 4
  - pid 7884: 'Harper' (Window Class) at 0,0 size 1024x768; topmost=True layered=True click-through=True exstyle=0xC0138
  - pid 8648: 'Harper Settings' (Tauri Window) at 44,4 size 936x719; topmost=False layered=False click-through=False exstyle=0x40110
  - pid 7884: '' (Winit Thread Event Target) at 0,0 size 16x16; topmost=False layered=True click-through=True exstyle=0x80800A0
  - pid 8648: '' (Tao Thread Event Target) at 0,0 size 16x16; topmost=False layered=True click-through=True exstyle=0x80800A0
- Screenshot `2-after-launch.png`: 1024x768, average brightness 0/255, near-black 100% of the screen

## Typing into Notepad
- Main app still running: True
- Visible windows (with Notepad focused): 4
  - pid 7884: 'Harper' (Window Class) at 0,0 size 1024x768; topmost=True layered=True click-through=True exstyle=0xC0138
  - pid 8648: 'Harper Settings' (Tauri Window) at 44,4 size 936x719; topmost=False layered=False click-through=False exstyle=0x40110
  - pid 7884: '' (Winit Thread Event Target) at 0,0 size 16x16; topmost=False layered=True click-through=True exstyle=0x80800A0
  - pid 8648: '' (Tao Thread Event Target) at 0,0 size 16x16; topmost=False layered=True click-through=True exstyle=0x80800A0
- Screenshot `3-notepad-typed.png`: 1024x768, average brightness 0/255, near-black 100% of the screen

## State while installed
- Starts with Windows (Run key): no
- Data folders:
  - `C:\Users\runneradmin\AppData\Roaming\harper-windows`: 0.0 MB
  - `C:\Users\runneradmin\AppData\Local\harper-windows`: absent
  - `C:\Users\runneradmin\AppData\Roaming\com.harper-windows.app`: absent
  - `C:\Users\runneradmin\AppData\Local\com.harper-windows.app`: 3.6 MB

## Uninstall
- Visible windows (uninstaller): 0
- Screenshot `4-uninstaller-window.png`: 1024x768, average brightness 116/255, near-black 0% of the screen
- Silent uninstall exit code 0
- Installed apps entry removed: True
- Install folder: removed
- Shortcuts left: none
- Run key left: none
- Data folders left (the stand-in model is 100 MB):
  - `C:\Users\runneradmin\AppData\Roaming\harper-windows`: 0.0 MB
  - `C:\Users\runneradmin\AppData\Local\harper-windows`: 100.0 MB
  - `C:\Users\runneradmin\AppData\Roaming\com.harper-windows.app`: absent
  - `C:\Users\runneradmin\AppData\Local\com.harper-windows.app`: 3.6 MB

## Verdict
- FAIL: the screen went black after launching the app (100% near-black)
- FAIL: the screen was black while typing in Notepad
Installer from run 37257546353
