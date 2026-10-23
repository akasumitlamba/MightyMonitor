//! Resolve an exact driver-returned OS output into an independent active screen.
use crate::owned::OutputId;
#[cfg(target_os = "windows")]
use crate::Display;
use std::io;
#[derive(Clone, Copy)]
struct PathIdentity {
    output: OutputId,
    source_adapter: i64,
    source_id: u32,
}
fn unique_source(paths: &[PathIdentity], output: OutputId) -> io::Result<PathIdentity> {
    let matching: Vec<_> = paths.iter().filter(|path| path.output == output).collect();
    if matching.len() != 1 {
        return Err(io::Error::other(
            "Owned display is inactive or its identity is ambiguous",
        ));
    }
    let selected = *matching[0];
    if paths
        .iter()
        .filter(|path| {
            path.source_adapter == selected.source_adapter && path.source_id == selected.source_id
        })
        .count()
        != 1
    {
        return Err(io::Error::other(
            "Owned display is cloned; arrange it as a separate display before sharing",
        ));
    }
    Ok(selected)
}
/// Uses only the output identity returned by the creating driver, never connector
/// order, EDID guessing or the first newly enumerated monitor. Makes no changes.
#[cfg(target_os = "windows")]
pub fn resolve_output(output: OutputId) -> io::Result<Display> {
    use windows::Win32::{
        Devices::Display::{
            DisplayConfigGetDeviceInfo, GetDisplayConfigBufferSizes, QueryDisplayConfig,
            DISPLAYCONFIG_DEVICE_INFO_GET_SOURCE_NAME, DISPLAYCONFIG_MODE_INFO,
            DISPLAYCONFIG_PATH_INFO, DISPLAYCONFIG_SOURCE_DEVICE_NAME, QDC_ONLY_ACTIVE_PATHS,
        },
        Foundation::{ERROR_INSUFFICIENT_BUFFER, ERROR_SUCCESS, LUID},
    };
    fn luid(value: LUID) -> i64 {
        ((i64::from(value.HighPart)) << 32) | i64::from(value.LowPart)
    }
    // Hotplug can grow buffers between calls. Retry a bounded number of snapshots.
    for _ in 0..3 {
        let (mut path_count, mut mode_count) = (0, 0);
        // SAFETY: valid exclusively borrowed count outputs and a fixed query flag.
        let status = unsafe {
            GetDisplayConfigBufferSizes(QDC_ONLY_ACTIVE_PATHS, &mut path_count, &mut mode_count)
        };
        if status != ERROR_SUCCESS {
            return Err(io::Error::other(
                "Could not read active Windows display paths",
            ));
        }
        if path_count == 0 || path_count > 256 || mode_count > 1024 {
            return Err(io::Error::other(
                "Active Windows display topology is empty or too large",
            ));
        }
        let mut paths: Vec<DISPLAYCONFIG_PATH_INFO> =
            (0..path_count).map(|_| Default::default()).collect();
        let mut modes: Vec<DISPLAYCONFIG_MODE_INFO> =
            (0..mode_count.max(1)).map(|_| Default::default()).collect();
        mode_count = modes.len() as u32;
        // SAFETY: initialized arrays have the capacities passed by the counts;
        // native writes are synchronous and topology ID must be null for active paths.
        let status = unsafe {
            QueryDisplayConfig(
                QDC_ONLY_ACTIVE_PATHS,
                &mut path_count,
                paths.as_mut_ptr(),
                &mut mode_count,
                modes.as_mut_ptr(),
                None,
            )
        };
        if status == ERROR_INSUFFICIENT_BUFFER {
            continue;
        }
        if status != ERROR_SUCCESS
            || path_count as usize > paths.len()
            || mode_count as usize > modes.len()
        {
            return Err(io::Error::other(
                "Could not snapshot active Windows display paths",
            ));
        }
        paths.truncate(path_count as usize);
        let identities: Vec<_> = paths
            .iter()
            .map(|path| PathIdentity {
                output: OutputId {
                    adapter_luid: luid(path.targetInfo.adapterId),
                    target_id: path.targetInfo.id,
                },
                source_adapter: luid(path.sourceInfo.adapterId),
                source_id: path.sourceInfo.id,
            })
            .collect();
        let selected = unique_source(&identities, output)?;
        let source = paths
            .iter()
            .find(|path| {
                luid(path.sourceInfo.adapterId) == selected.source_adapter
                    && path.sourceInfo.id == selected.source_id
            })
            .ok_or_else(|| io::Error::other("Owned display source disappeared"))?;
        let mut name = DISPLAYCONFIG_SOURCE_DEVICE_NAME::default();
        name.header.r#type = DISPLAYCONFIG_DEVICE_INFO_GET_SOURCE_NAME;
        name.header.size = std::mem::size_of::<DISPLAYCONFIG_SOURCE_DEVICE_NAME>() as u32;
        name.header.adapterId = source.sourceInfo.adapterId;
        name.header.id = source.sourceInfo.id;
        // SAFETY: repr(C) output starts with the header; full allocation size is
        // advertised and the adapter/source IDs come from this active snapshot.
        if unsafe { DisplayConfigGetDeviceInfo(std::ptr::addr_of_mut!(name).cast()) } != 0 {
            return Err(io::Error::other(
                "Could not identify the owned Windows screen",
            ));
        }
        let end = name
            .viewGdiDeviceName
            .iter()
            .position(|&c| c == 0)
            .unwrap_or(name.viewGdiDeviceName.len());
        let device = String::from_utf16(&name.viewGdiDeviceName[..end])
            .map_err(|_| io::Error::other("Invalid Windows screen name"))?;
        let display = crate::select(&device)?;
        display.validate()?;
        return Ok(display);
    }
    Err(io::Error::other(
        "Windows display topology is changing; try again",
    ))
}
#[cfg(test)]
mod tests {
    use super::*;
    fn path(adapter: i64, target: u32, source: u32) -> PathIdentity {
        PathIdentity {
            output: OutputId {
                adapter_luid: adapter,
                target_id: target,
            },
            source_adapter: adapter,
            source_id: source,
        }
    }
    #[test]
    fn identifies_exact_adapter_target_without_enumeration_order() {
        let first = path(1, 12, 3);
        let owned = path(-2, 12, 7);
        assert_eq!(
            unique_source(&[first, owned], owned.output)
                .unwrap()
                .source_id,
            7
        );
        assert_eq!(
            unique_source(&[owned, first], owned.output)
                .unwrap()
                .source_id,
            7
        );
        assert!(unique_source(&[first], owned.output).is_err());
    }
    #[test]
    fn rejects_clones_and_ambiguous_target_identity() {
        let owned = path(1, 12, 3);
        assert!(unique_source(&[owned, path(1, 13, 3)], owned.output).is_err());
        assert!(unique_source(&[owned, owned], owned.output).is_err());
        assert!(unique_source(&[], owned.output).is_err());
    }
}
