use crate::common::get_custom_data_key;

use super::common::{get_index_key, RES_NAME_COUNT, RES_TYPE};
use std::ffi::CString;
use std::fs;
use std::io::BufWriter;
use std::io::Write;
use windows::core::PCSTR;
use windows::Win32::Foundation::HINSTANCE;
use windows::Win32::System::LibraryLoader;

pub fn read_block_count() -> Result<u32, ()> {
    let vec = read_resource_as_vec_u8(RES_TYPE, RES_NAME_COUNT).unwrap();
    let data = String::from_utf8(vec).unwrap().parse::<u32>().unwrap();
    Ok(data)
}

pub fn extract_binary(file_path: &std::path::Path, block_count: &u32) -> Result<(), ()> {
    let file = fs::File::options()
        .create_new(true)
        .write(true)
        .open(file_path)
        .unwrap();
    let mut buf_writer = BufWriter::new(file);
    for block_index in 0..*block_count {
        let chunk = read_resource_as_vec_u8(RES_TYPE, get_index_key(&block_index)).unwrap();
        buf_writer.write_all(chunk.as_slice()).unwrap();
    }
    Ok(())
}

pub fn read_resource_as_vec_u8(
    lptype: impl Into<String>,
    lpname: impl Into<String>,
) -> Option<Vec<u8>> {
    let cstr_lpname: CString = CString::new(lpname.into()).unwrap();
    let cstr_lptype: CString = CString::new(lptype.into()).unwrap();
    let resource_info_res = unsafe {
        LibraryLoader::FindResourceA(
            HINSTANCE::default(),
            PCSTR::from_raw(cstr_lpname.as_bytes_with_nul().as_ptr()),
            PCSTR::from_raw(cstr_lptype.as_bytes_with_nul().as_ptr()),
        )
    };

    if let Err(error) = &resource_info_res {
        if error.code().0 as u32 == 0x80070716 {
            return None;
        }
    }

    let resource_info = resource_info_res.unwrap();

    let load_resource_hglobal =
        unsafe { LibraryLoader::LoadResource(HINSTANCE::default(), resource_info) };
    let size_of_resource =
        unsafe { LibraryLoader::SizeofResource(HINSTANCE::default(), resource_info) };
    let pointer_to_first_byte = unsafe { LibraryLoader::LockResource(load_resource_hglobal) };

    let ptr_slice = std::ptr::slice_from_raw_parts::<u8>(
        pointer_to_first_byte as *const u8,
        size_of_resource.try_into().unwrap(),
    );

    let vec = unsafe {
        (*ptr_slice)
            .iter()
            .map(|ptr_to_u8| std::ptr::read(ptr_to_u8))
            .collect::<Vec<u8>>()
    };

    Some(vec)
}

pub fn read_custom_string(key: impl Into<String>) -> Option<String> {
    let vec_opt = read_resource_as_vec_u8(RES_TYPE, get_custom_data_key(&key.into()));
    match vec_opt {
        Some(vec) => {
            let data = String::from_utf8(vec).unwrap();
            Some(data)
        }
        None => None,
    }
}
