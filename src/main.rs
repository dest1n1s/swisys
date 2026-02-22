pub use efivar::efi::Variable;
use efivar::efi::VariableFlags;
pub use efivar::system;
pub use efivar::utils::read_nt_utf16_string;
use log::{debug, error, info};
pub use pretty_env_logger;
use std::env;
pub use std::str::FromStr;
pub use uuid::Uuid;

fn read_nt_utf16_strings(cursor: &mut &[u8]) -> Result<Vec<String>, String> {
    std::iter::from_fn(|| {
        if cursor.is_empty() {
            None
        } else {
            Some(read_nt_utf16_string(cursor).map_err(|e| e.to_string()))
        }
    })
    .collect()
}

const SYSTEMD_BOOT_VENDOR_UUID: &str = "4a67b082-0a4c-41cf-b6c7-440b29bb8c4f";

fn read_systemd_boot_efi_variable(
    efivars: &dyn efivar::VarManager,
    variable_name: &str,
) -> Result<Vec<u8>, String> {
    let vendor_uuid = Uuid::from_str(SYSTEMD_BOOT_VENDOR_UUID).map_err(|e| e.to_string())?;

    let (data, _) = efivars
        .read(&Variable::new_with_vendor(variable_name, vendor_uuid))
        .map_err(|e| e.to_string())?;

    Ok(data)
}

fn write_systemd_boot_efi_variable(
    efivars: &mut dyn efivar::VarManager,
    variable_name: &str,
    attributes: VariableFlags,
    value: &str,
) -> Result<(), String> {
    let vendor_uuid = Uuid::from_str(SYSTEMD_BOOT_VENDOR_UUID).map_err(|e| e.to_string())?;
    let value_u8 = value
        .encode_utf16()
        .chain([0u16])
        .flat_map(|v| v.to_le_bytes())
        .collect::<Vec<u8>>();

    efivars
        .write(
            &Variable::new_with_vendor(variable_name, vendor_uuid),
            attributes,
            &value_u8,
        )
        .map_err(|e| e.to_string())?;

    Ok(())
}

fn run() -> Result<(), String> {
    #[cfg(unix)]
    sudo::with_env(&["RUST_LOG", "CARGO_"])
        .map_err(|e| format!("Error escalating to root privileges: {}", e))?;

    let mut efivars =
        system().map_err(|e| format!("Error initializing EFI variable system: {}", e))?;

    let loader_entries_data = read_systemd_boot_efi_variable(efivars.as_ref(), "LoaderEntries")
        .map_err(|e| format!("Error reading systemd-boot EFI variable LoaderEntries. Is systemd-boot installed? Error: {}", e))?;

    let loader_entries = read_nt_utf16_strings(&mut &loader_entries_data[..])
        .map_err(|e| format!("Error occurred while reading systemd-boot EFI variable LoaderEntries. The EFI variable maybe broken. Error: {}", e))?;

    // Filter out entries `auto-reboot` and `auto-reboot-to-firmware-setup`
    let filtered_loader_entries = loader_entries
        .into_iter()
        .filter(|entry| !entry.contains("reboot"))
        .collect::<Vec<String>>();

    debug!("Filtered loader entries: {:?}", filtered_loader_entries);

    let loader_entry_selected_data =
        read_systemd_boot_efi_variable(efivars.as_ref(), "LoaderEntrySelected").map_err(|e| {
            format!(
                "Error reading systemd-boot EFI variable LoaderEntrySelected. Error: {}",
                e
            )
        })?;

    let loader_entry_selected = read_nt_utf16_string(&mut &loader_entry_selected_data[..])
        .map_err(|e| format!("Error occurred while reading systemd-boot EFI variable LoaderEntrySelected. The EFI variable maybe broken. Error: {}", e))?;

    debug!("LoaderEntrySelected: {}", loader_entry_selected);

    if !filtered_loader_entries.contains(&loader_entry_selected) {
        return Err("LoaderEntrySelected is not in loader entries".to_string());
    }

    // Find the position of the LoaderEntrySelected in the filtered loader entries, and get the next entry
    let position = filtered_loader_entries
        .iter()
        .position(|entry| entry == &loader_entry_selected)
        .ok_or_else(|| {
            "Could not find LoaderEntrySelected in filtered entries (logic error)".to_string()
        })?;

    let new_loader_entry_selected =
        &filtered_loader_entries[(position + 1) % filtered_loader_entries.len()];

    debug!("New LoaderEntrySelected: {}", new_loader_entry_selected);

    write_systemd_boot_efi_variable(
        efivars.as_mut(),
        "LoaderEntryOneShot",
        VariableFlags::default(),
        new_loader_entry_selected,
    )
    .map_err(|e| {
        format!(
            "Error occurred while writing to systemd-boot EFI variable LoaderEntryOneShot: {}",
            e
        )
    })?;

    // Setting `LoaderConfigTimeoutOneShot` to `0` does not work as expected. Ref: https://github.com/systemd/systemd/issues/38254
    // Set it to `1` instead.

    write_systemd_boot_efi_variable(
        efivars.as_mut(),
        "LoaderConfigTimeoutOneShot",
        VariableFlags::default(),
        "1",
    )
    .map_err(|e| {
        format!(
            "Error occurred while writing to systemd-boot EFI variable LoaderConfigTimeoutOneShot: {}",
            e
        )
    })?;

    debug!("Wrote to systemd-boot EFI variable LoaderEntryOneShot.");

    info!(
        "Successfully set next boot to {}. Rebooting the system...",
        new_loader_entry_selected
    );

    system_shutdown::reboot().map_err(|e| format!("Error rebooting the system: {}", e))?;

    Ok(())
}

fn main() {
    if env::var("RUST_LOG").is_err() {
        unsafe {
            env::set_var("RUST_LOG", "info");
        }
    }
    pretty_env_logger::init();

    if let Err(e) = run() {
        error!("{}", e);
        std::process::exit(1);
    }
}
