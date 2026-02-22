# swisys

A simple tool to switch system on reboot. Currently only supports [systemd-boot](https://wiki.archlinux.org/title/Systemd-boot) as the UEFI boot manager.

If you dual-boot (e.g. Windows and Linux), rebooting into another OS usually means waiting for the boot menu to appear and selecting your target manually, which is tedious and easy to miss.

This tool simply does the following:

1. Set systemd-boot EFI variable `LoaderEntryOneShot` to the next eligible item in `LoaderEntries`.
2. Set systemd-boot EFI variable `LoaderConfigTimeoutOneShot` to `1`, to quickly pass the loader menu as we've decided which system to enter. (It cannot be set to `0` due to systemd/systemd#38254.)
3. Reboot.

On Linux it sets EFI variable through [efivarfs](https://docs.kernel.org/filesystems/efivarfs.html) (`/sys/firmware/efi/efivars`) and its functionality is similar to `systemctl reboot --boot-loader-entry=auto-windows`. On Windows it sets EFI variable through Win32 API [SetFirmwareEnvironmentVariableExW](https://learn.microsoft.com/en-us/windows/win32/api/winbase/nf-winbase-setfirmwareenvironmentvariableexw).

## Installation

### Cargo

```bash
cargo install swisys
```

### AUR

```bash
yay -S swisys
# or: paru -S swisys
```

## Usage

Just run `swisys`.
