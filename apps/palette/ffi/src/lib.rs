// We have a lot of c-types in here, stop warning about their names!
#![allow(non_camel_case_types)]

use palette_lib::PaletteData;

#[no_mangle]
pub extern "C" fn rs_PaletteData_new() -> *mut PaletteData {
    let gcs = PaletteData::new();
    Box::into_raw(Box::new(gcs))
}

#[no_mangle]
pub extern "C" fn rs_PaletteData_free(p_pcs: *mut PaletteData) {
    if !p_pcs.is_null() {
        unsafe {
            let _ = Box::from_raw(p_pcs);
        };
    }
}

#[no_mangle]
pub extern "C" fn rs_PaletteData_shuffle(p_pcs: *mut PaletteData) -> i8 {
    if p_pcs.is_null() {
        return -1;
    }
    let mut ps = unsafe { Box::from_raw(p_pcs) };
    ps.shuffle();
    std::mem::forget(ps);
    return 0;
}

#[no_mangle]
pub extern "C" fn rs_PaletteData_next(p_pcs: *mut PaletteData, p_out: *mut u8) -> i8 {
    if p_pcs.is_null() || p_out.is_null() {
        return -1;
    }

    let mut ps = unsafe { Box::from_raw(p_pcs) };
    let outs = unsafe { std::slice::from_raw_parts_mut(p_out, 1usize) };
    outs[0] = ps.next();
    std::mem::forget(ps);
    return 0;
}
