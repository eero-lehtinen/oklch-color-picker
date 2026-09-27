use std::{ffi::OsString, os::windows::ffi::OsStringExt, ptr};

use raw_window_handle::{HasWindowHandle, RawWindowHandle};
use windows_sys::Win32::{
    Devices::Display::{
        DISPLAYCONFIG_DEVICE_INFO_GET_ADVANCED_COLOR_INFO,
        DISPLAYCONFIG_DEVICE_INFO_GET_SOURCE_NAME, DISPLAYCONFIG_DEVICE_INFO_HEADER,
        DISPLAYCONFIG_GET_ADVANCED_COLOR_INFO, DISPLAYCONFIG_MODE_INFO, DISPLAYCONFIG_PATH_INFO,
        DISPLAYCONFIG_SOURCE_DEVICE_NAME, DisplayConfigGetDeviceInfo, GetDisplayConfigBufferSizes,
        QDC_ONLY_ACTIVE_PATHS, QueryDisplayConfig,
    },
    Foundation::{ERROR_SUCCESS, HWND, LUID},
    Graphics::Gdi::{
        CreateDCW, DeleteDC, GetMonitorInfoW, HMONITOR, MONITOR_DEFAULTTONEAREST, MONITORINFO,
        MONITORINFOEXW, MonitorFromWindow,
    },
    UI::ColorSystem::GetICMProfileW,
};

pub fn init(_: &impl HasWindowHandle) {}

pub fn monitor(window: &impl HasWindowHandle) -> Option<isize> {
    hmonitor(window).map(|m| m as isize)
}

/// Windows doesn't color-manage OpenGL windows unless HDR or Auto Color
/// Management is on, in which case it treats the output as sRGB.
pub fn icc_profile(window: &impl HasWindowHandle) -> Option<Vec<u8>> {
    let device = device_name(hmonitor(window)?)?;
    if advanced_color_enabled(&device) {
        return None;
    }
    std::fs::read(icm_profile_path(&device)?).ok()
}

fn hmonitor(window: &impl HasWindowHandle) -> Option<HMONITOR> {
    let RawWindowHandle::Win32(handle) = window.window_handle().ok()?.as_raw() else {
        return None;
    };
    Some(unsafe { MonitorFromWindow(handle.hwnd.get() as HWND, MONITOR_DEFAULTTONEAREST) })
}

fn device_name(monitor: HMONITOR) -> Option<[u16; 32]> {
    let mut info = MONITORINFOEXW {
        monitorInfo: MONITORINFO {
            cbSize: size_of::<MONITORINFOEXW>() as u32,
            ..Default::default()
        },
        ..Default::default()
    };
    let ok = unsafe { GetMonitorInfoW(monitor, (&raw mut info).cast()) };
    (ok != 0).then_some(info.szDevice)
}

fn icm_profile_path(device: &[u16; 32]) -> Option<OsString> {
    unsafe {
        let hdc = CreateDCW(device.as_ptr(), device.as_ptr(), ptr::null(), ptr::null());
        if hdc.is_null() {
            return None;
        }
        let mut len = 0;
        GetICMProfileW(hdc, &mut len, ptr::null_mut());
        let mut buf = vec![0u16; len as usize];
        let ok = len > 0 && GetICMProfileW(hdc, &mut len, buf.as_mut_ptr()) != 0;
        DeleteDC(hdc);
        ok.then(|| OsString::from_wide(until_nul(&buf)))
    }
}

fn advanced_color_enabled(device: &[u16; 32]) -> bool {
    unsafe {
        let (mut num_paths, mut num_modes) = (0, 0);
        if GetDisplayConfigBufferSizes(QDC_ONLY_ACTIVE_PATHS, &mut num_paths, &mut num_modes)
            != ERROR_SUCCESS
        {
            return false;
        }
        let mut paths: Vec<DISPLAYCONFIG_PATH_INFO> =
            (0..num_paths).map(|_| Default::default()).collect();
        let mut modes: Vec<DISPLAYCONFIG_MODE_INFO> =
            (0..num_modes).map(|_| Default::default()).collect();
        if QueryDisplayConfig(
            QDC_ONLY_ACTIVE_PATHS,
            &mut num_paths,
            paths.as_mut_ptr(),
            &mut num_modes,
            modes.as_mut_ptr(),
            ptr::null_mut(),
        ) != ERROR_SUCCESS
        {
            return false;
        }

        for path in &paths[..num_paths as usize] {
            let mut source = DISPLAYCONFIG_SOURCE_DEVICE_NAME {
                header: info_header::<DISPLAYCONFIG_SOURCE_DEVICE_NAME>(
                    DISPLAYCONFIG_DEVICE_INFO_GET_SOURCE_NAME,
                    path.sourceInfo.adapterId,
                    path.sourceInfo.id,
                ),
                ..Default::default()
            };
            if DisplayConfigGetDeviceInfo(&mut source.header) != 0
                || until_nul(&source.viewGdiDeviceName) != until_nul(device)
            {
                continue;
            }

            let mut color = DISPLAYCONFIG_GET_ADVANCED_COLOR_INFO {
                header: info_header::<DISPLAYCONFIG_GET_ADVANCED_COLOR_INFO>(
                    DISPLAYCONFIG_DEVICE_INFO_GET_ADVANCED_COLOR_INFO,
                    path.targetInfo.adapterId,
                    path.targetInfo.id,
                ),
                ..Default::default()
            };
            if DisplayConfigGetDeviceInfo(&mut color.header) != 0 {
                return false;
            }
            const ADVANCED_COLOR_ENABLED: u32 = 1 << 1;
            const WIDE_COLOR_ENFORCED: u32 = 1 << 2;
            return color.Anonymous.value & (ADVANCED_COLOR_ENABLED | WIDE_COLOR_ENFORCED) != 0;
        }
        false
    }
}

fn info_header<T>(r#type: i32, adapter_id: LUID, id: u32) -> DISPLAYCONFIG_DEVICE_INFO_HEADER {
    DISPLAYCONFIG_DEVICE_INFO_HEADER {
        r#type,
        size: size_of::<T>() as u32,
        adapterId: adapter_id,
        id,
    }
}

fn until_nul(s: &[u16]) -> &[u16] {
    &s[..s.iter().position(|&c| c == 0).unwrap_or(s.len())]
}
