// We have a lot of c-types in here, stop warning about their names!
#![allow(non_camel_case_types)]

use template_lib::{TemplateData};

#[no_mangle]
pub extern "C" fn rs_TemplateData_new(v: u8) -> *mut TemplateData {
    let gcs = TemplateData::new();
    Box::into_raw(Box::new(gcs))
}

#[no_mangle]
pub extern "C" fn rs_TemplateData_free(p_pcs: *mut TemplateData) {
    if !p_pcs.is_null() {
        unsafe {
            let _ = Box::from_raw(p_pcs);
        };
    }
}

#[no_mangle]
pub extern "C" fn rs_TemplateData_add_one(
    p_pcs: *mut TemplateData,
    p_out: *mut u8,
) -> i8 {
    if p_pcs.is_null() {
        return -1;
    }
    let ret: i8;
    // 取结构
    let mut ps = unsafe { Box::from_raw(p_pcs) };
    let outs = unsafe { std::slice::from_raw_parts_mut(p_out, 1usize) };
    ps.add_one(); 
    outs[0] = ps.number;
    std::mem::forget(ps);
    return 0;
}

#[no_mangle]
pub extern "C" fn rs_TemplateData_assign(
    p_pcs: *mut TemplateData,
    p_data: *const u16,
    data_len: usize,
    freeze: u8,
    p_out: *mut u8,
) -> i8 {
    if p_pcs.is_null() || p_data.is_null() || data_len == 0 {
        return -1;
    }
    let ret: i8;
    // 取结构
    let mut ps = unsafe { Box::from_raw(p_pcs) };
    // 取数据
    let slice = unsafe { std::slice::from_raw_parts(p_data, data_len as usize) };
    // 要求传入足够的32字节的数据缓冲区
    let outs = unsafe { std::slice::from_raw_parts_mut(p_out, 32usize) };

    match ps.assign(slice, freeze != 0) {
        Ok(n) => {
            let mut idx = 0usize;
            // 有效的out数据格式：
            // deadwood分数
            // deadwood长度 deadwood1 deadwood2 ...
            // meld1长度 meld1_1 meld1_2 ...
            // meld2长度 meld2_1 meld2_2...
            // ...
            // 长度32足够了
            // best deadwood value...
            outs[idx] = n;
            idx += 1;
            // best deadwood list...
            outs[idx] = ps.best_deadwood.len() as u8;
            idx += 1;
            for p in &ps.best_deadwood {
                outs[idx] = p.to_u8();
                idx += 1;
            }
            // melds list...
            for v in &ps.best_melds {
                outs[idx] = v.len() as u8;
                idx += 1;
                for p in v {
                    outs[idx] = p.to_u8();
                    idx += 1;
                }
            }
            // 返回out数据有效长度
            ret = idx as i8;
        }
        Err(_) => {
            // println!("{:?}", e);
            ret = -1;
        }
    }
    std::mem::forget(ps);
    return ret;
}

