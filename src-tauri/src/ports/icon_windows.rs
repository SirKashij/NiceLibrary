//! Reads an executable's icon from its resources, as Explorer shows it.

use std::ffi::c_void;
use std::os::windows::ffi::OsStrExt;
use std::path::Path;
use std::ptr;

use nicelibrary_core::{Icon, IconExtractor};
use windows_sys::Win32::Foundation::{FreeLibrary, HMODULE};
use windows_sys::Win32::Graphics::Gdi::{
    BI_RGB, BITMAPINFO, BITMAPINFOHEADER, CreateCompatibleDC, CreateDIBSection, DIB_RGB_COLORS, DeleteDC,
    DeleteObject, GdiFlush, HBITMAP, HDC, HGDIOBJ, SelectObject,
};
use windows_sys::Win32::System::LibraryLoader::{
    EnumResourceNamesW, FindResourceW, LOAD_LIBRARY_AS_DATAFILE, LOAD_LIBRARY_AS_IMAGE_RESOURCE, LoadLibraryExW,
    LoadResource, LockResource, SizeofResource,
};
use windows_sys::Win32::UI::WindowsAndMessaging::{
    CreateIconFromResourceEx, DI_NORMAL, DestroyIcon, DrawIconEx, HICON, LR_DEFAULTCOLOR, RT_GROUP_ICON, RT_ICON,
};
use windows_sys::core::PCWSTR;

/// Reads the first icon in an executable's resources, the one Explorer shows
/// for it, and picks its largest image. The file is only read as data; no
/// code of it runs.
pub struct WindowsIconExtractor;

impl IconExtractor for WindowsIconExtractor {
    fn largest_icon(&self, executable: &Path) -> Result<Option<Icon>, String> {
        let module = Module::load(executable)?;
        let Some(group) = module.first_icon_group() else {
            return Ok(None);
        };
        let Some(entry) = largest_entry(module.resource(group.as_pcwstr(), RT_GROUP_ICON).unwrap_or_default()) else {
            return Ok(None);
        };
        let Some(image) = module.resource(entry.id as usize as PCWSTR, RT_ICON) else {
            return Ok(None);
        };
        let Some((width, height)) = image_size(image) else {
            return Err(format!("the icon of {} is in a format that cannot be read", executable.display()));
        };
        draw(image, width, height).map(Some)
    }
}

/// An executable loaded as a file of resources.
struct Module(HMODULE);

impl Module {
    fn load(executable: &Path) -> Result<Self, String> {
        let path: Vec<u16> = executable.as_os_str().encode_wide().chain([0]).collect();
        // SAFETY: `path` is a null-terminated wide string that outlives the call.
        let module =
            unsafe { LoadLibraryExW(path.as_ptr(), ptr::null_mut(), LOAD_LIBRARY_AS_DATAFILE | LOAD_LIBRARY_AS_IMAGE_RESOURCE) };
        if module.is_null() {
            return Err(format!("{} cannot be read as an executable: {}", executable.display(), std::io::Error::last_os_error()));
        }
        Ok(Self(module))
    }

    /// The name of the first icon in the resources, if there is one.
    fn first_icon_group(&self) -> Option<ResourceName> {
        let mut found: Option<ResourceName> = None;
        // SAFETY: the callback only writes through the pointer it is given, which outlives the call.
        // Without icons the call fails; `found` then stays `None`.
        unsafe { EnumResourceNamesW(self.0, RT_GROUP_ICON, Some(remember_first), &mut found as *mut _ as isize) };
        found
    }

    /// The bytes of the resource `name` of `kind`, valid while the module is loaded.
    fn resource(&self, name: PCWSTR, kind: PCWSTR) -> Option<&[u8]> {
        // SAFETY: plain Win32 calls on a module this value owns; a resource
        // stays mapped, and so the slice valid, until the module is freed.
        unsafe {
            let found = FindResourceW(self.0, name, kind);
            if found.is_null() {
                return None;
            }
            let size = SizeofResource(self.0, found) as usize;
            let loaded = LoadResource(self.0, found);
            let bytes = LockResource(loaded) as *const u8;
            (!bytes.is_null() && size > 0).then(|| std::slice::from_raw_parts(bytes, size))
        }
    }
}

impl Drop for Module {
    fn drop(&mut self) {
        // SAFETY: the module was loaded by `Module::load` and is freed once.
        unsafe { FreeLibrary(self.0) };
    }
}

/// A resource's name: a number, or text.
enum ResourceName {
    Number(u16),
    Text(Vec<u16>),
}

impl ResourceName {
    fn as_pcwstr(&self) -> PCWSTR {
        match self {
            Self::Number(number) => *number as usize as PCWSTR,
            Self::Text(text) => text.as_ptr(),
        }
    }
}

unsafe extern "system" fn remember_first(_module: HMODULE, _kind: PCWSTR, name: PCWSTR, found: isize) -> i32 {
    let name = if (name as usize) >> 16 == 0 {
        ResourceName::Number(name as usize as u16)
    } else {
        // SAFETY: a name that is not a number is a null-terminated wide string, valid during the callback.
        let length = (0..).take_while(|&at| unsafe { *name.add(at) } != 0).count();
        ResourceName::Text(unsafe { std::slice::from_raw_parts(name, length + 1) }.to_vec())
    };
    // SAFETY: `found` is the `&mut Option<ResourceName>` handed to EnumResourceNamesW.
    unsafe { *(found as *mut Option<ResourceName>) = Some(name) };
    // The first is the one: stop.
    0
}

/// One image of an icon, as its group lists it.
struct Entry {
    size: u32,
    bit_count: u16,
    bytes: u32,
    id: u16,
}

/// The largest image an icon group lists: the widest, then the one with the
/// most colours, then the one with the most bytes.
fn largest_entry(group: &[u8]) -> Option<Entry> {
    let count = usize::from(u16::from_le_bytes(group.get(4..6)?.try_into().ok()?));
    (0..count)
        .filter_map(|index| {
            let entry = group.get(6 + index * 14..6 + (index + 1) * 14)?;
            let width = u32::from(entry[0]);
            Some(Entry {
                // 0 stands for 256 or more.
                size: if width == 0 { 256 } else { width },
                bit_count: u16::from_le_bytes([entry[6], entry[7]]),
                bytes: u32::from_le_bytes([entry[8], entry[9], entry[10], entry[11]]),
                id: u16::from_le_bytes([entry[12], entry[13]]),
            })
        })
        .max_by_key(|entry| (entry.size, entry.bit_count, entry.bytes))
}

/// The width and height of one icon image: a PNG image or a bitmap whose
/// height counts its mask too.
fn image_size(image: &[u8]) -> Option<(i32, i32)> {
    let number = |at: usize| image.get(at..at + 4).and_then(|bytes| bytes.try_into().ok());
    if image.starts_with(b"\x89PNG\r\n\x1a\n") {
        let (width, height) = (u32::from_be_bytes(number(16)?), u32::from_be_bytes(number(20)?));
        return Some((i32::try_from(width).ok()?, i32::try_from(height).ok()?));
    }
    let (width, height) = (i32::from_le_bytes(number(4)?), i32::from_le_bytes(number(8)?) / 2);
    Some((width, height))
}

/// The icon image drawn into pixels. Drawn once on black and once on white,
/// so every kind of icon, with an alpha channel or a mask, gives its colours
/// and its transparency alike.
fn draw(image: &[u8], width: i32, height: i32) -> Result<Icon, String> {
    if width <= 0 || height <= 0 || width > 4096 || height > 4096 {
        return Err(format!("an icon of {width} × {height} pixels cannot be drawn"));
    }
    // SAFETY: `image` is a whole icon image resource; the handle is destroyed below.
    let icon =
        unsafe { CreateIconFromResourceEx(image.as_ptr(), image.len() as u32, 1, 0x0003_0000, width, height, LR_DEFAULTCOLOR) };
    if icon.is_null() {
        return Err(format!("the icon could not be read: {}", std::io::Error::last_os_error()));
    }
    let icon = OwnedIcon(icon);
    let canvas = Canvas::new(width, height)?;
    let on_black = canvas.draw(&icon, 0x00)?;
    let on_white = canvas.draw(&icon, 0xFF)?;
    let rgba = on_black
        .chunks_exact(4)
        .zip(on_white.chunks_exact(4))
        .flat_map(|(black, white)| {
            // Bitmaps keep blue, green, red. What white adds over black is
            // what shows through, so alpha is what it does not add.
            let shows_through = (0..3).map(|c| u32::from(white[c].saturating_sub(black[c]))).sum::<u32>() / 3;
            let alpha = 255 - shows_through.min(255);
            let straight = |value: u8| if alpha == 0 { 0 } else { (u32::from(value) * 255 / alpha).min(255) as u8 };
            [straight(black[2]), straight(black[1]), straight(black[0]), alpha as u8]
        })
        .collect();
    Ok(Icon { width: width as u32, height: height as u32, rgba })
}

struct OwnedIcon(HICON);

impl Drop for OwnedIcon {
    fn drop(&mut self) {
        // SAFETY: the icon was created by CreateIconFromResourceEx and is destroyed once.
        unsafe { DestroyIcon(self.0) };
    }
}

/// A 32-bit bitmap, top row first, selected into a drawing context of its own.
struct Canvas {
    context: HDC,
    bitmap: HBITMAP,
    previous: HGDIOBJ,
    pixels: *mut u8,
    width: i32,
    height: i32,
}

impl Canvas {
    fn new(width: i32, height: i32) -> Result<Self, String> {
        // SAFETY: all-zero is a valid BITMAPINFO; the fields that matter are set below.
        let mut info: BITMAPINFO = unsafe { std::mem::zeroed() };
        info.bmiHeader = BITMAPINFOHEADER {
            biSize: size_of::<BITMAPINFOHEADER>() as u32,
            biWidth: width,
            // Negative: the top row comes first.
            biHeight: -height,
            biPlanes: 1,
            biBitCount: 32,
            biCompression: BI_RGB,
            ..info.bmiHeader
        };
        let mut pixels: *mut c_void = ptr::null_mut();
        // SAFETY: plain GDI calls; what they create is released in `Drop`.
        unsafe {
            let context = CreateCompatibleDC(ptr::null_mut());
            if context.is_null() {
                return Err("no drawing context could be created".into());
            }
            let bitmap = CreateDIBSection(context, &info, DIB_RGB_COLORS, &mut pixels, ptr::null_mut(), 0);
            if bitmap.is_null() || pixels.is_null() {
                DeleteDC(context);
                return Err("no bitmap could be created".into());
            }
            let previous = SelectObject(context, bitmap);
            Ok(Self { context, bitmap, previous, pixels: pixels as *mut u8, width, height })
        }
    }

    /// The icon drawn on a background of `shade` in every channel, as the bitmap's bytes.
    fn draw(&self, icon: &OwnedIcon, shade: u8) -> Result<Vec<u8>, String> {
        let length = self.width as usize * self.height as usize * 4;
        // SAFETY: the bitmap holds `length` bytes for as long as `self` lives.
        let pixels = unsafe { std::slice::from_raw_parts_mut(self.pixels, length) };
        pixels.fill(shade);
        // SAFETY: plain GDI call on a context and an icon this code owns.
        let drawn = unsafe {
            DrawIconEx(self.context, 0, 0, icon.0, self.width, self.height, 0, ptr::null_mut(), DI_NORMAL)
        };
        // SAFETY: plain GDI call; it makes the drawing reach the bitmap before it is read.
        unsafe { GdiFlush() };
        if drawn == 0 {
            return Err(format!("the icon could not be drawn: {}", std::io::Error::last_os_error()));
        }
        Ok(pixels.to_vec())
    }
}

impl Drop for Canvas {
    fn drop(&mut self) {
        // SAFETY: each object was created by `Canvas::new` and is released once.
        unsafe {
            SelectObject(self.context, self.previous);
            DeleteObject(self.bitmap);
            DeleteDC(self.context);
        }
    }
}
