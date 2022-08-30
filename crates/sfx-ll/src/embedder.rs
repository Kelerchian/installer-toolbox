use super::common::{get_index_key, BLOCKSIZE, RES_NAME_COUNT, RES_TYPE};
use std::ffi::{c_void, CString};
use std::fs;
use std::io::{BufReader, Read};
use windows::core::PCSTR;
use windows::Win32::Foundation::HANDLE;
use windows::Win32::System::LibraryLoader;

pub fn with_resource_update_handle<'a, T>(
    file_path: &'a std::path::Path,
    update_fn: Box<dyn for<'b> FnOnce(&'b HANDLE) -> T + 'a>,
) -> T {
    let flag_remove_existing_resource = false;
    let cstr_file_path = CString::new(file_path.to_str().unwrap()).unwrap();
    let cstr_file_path_ptr = cstr_file_path.as_bytes_with_nul().as_ptr();

    let handle = unsafe {
        LibraryLoader::BeginUpdateResourceA(
            PCSTR::from_raw(cstr_file_path_ptr),
            &flag_remove_existing_resource,
        )
        .unwrap()
    };
    let result = update_fn(&handle);
    unsafe { LibraryLoader::EndUpdateResourceA(handle, false).as_bool() };
    result
}

pub fn embed_binary_as_archive(handle: &HANDLE, file_path: &std::path::Path) -> Result<u32, ()> {
    let res_type_cstr: CString = CString::new(RES_TYPE).unwrap();
    let file = fs::File::open(file_path).unwrap();
    let mut buf_reader = BufReader::new(file);
    let mut index = 0_u32;

    loop {
        let mut take_handle = (&mut buf_reader).take(BLOCKSIZE as u64);
        let mut buffer = [0; BLOCKSIZE];
        let read_res = take_handle.read(&mut buffer);

        let actual_read_size = read_res.unwrap();
        if actual_read_size == 0 {
            break;
        }

        let final_buffer = &buffer[..actual_read_size];
        let final_buffer_ptr = final_buffer as *const _ as *const c_void;

        let success = unsafe {
            let res_name = CString::new(get_index_key(&index)).unwrap();

            LibraryLoader::UpdateResourceA(
                *handle,
                PCSTR::from_raw(res_type_cstr.as_bytes_with_nul().as_ptr()),
                PCSTR::from_raw(res_name.as_bytes_with_nul().as_ptr()),
                0x0409,
                final_buffer_ptr,
                (std::mem::size_of::<u8>() * final_buffer.len())
                    .try_into()
                    .unwrap(),
            )
            .as_bool()
        };

        if !success {
            return Err(());
        }

        index += 1;
    }
    let block_count = index;
    Ok(block_count)
}

pub fn embed_block_count(handle: &HANDLE, block_count: &u32) -> Result<(), ()> {
    let res_type_cstr: CString = CString::new(RES_TYPE).unwrap();
    let res_name_count_cstr: CString = CString::new(RES_NAME_COUNT).unwrap();

    let block_count_in_string = format!("{}", block_count);
    let block_count_in_string_buffer = block_count_in_string.as_bytes();
    let block_count_in_string_buffer_ptr =
        block_count_in_string_buffer as *const _ as *const c_void;
    let success = unsafe {
        let success = LibraryLoader::UpdateResourceA(
            *handle,
            PCSTR::from_raw(res_type_cstr.as_bytes_with_nul().as_ptr()),
            PCSTR::from_raw(res_name_count_cstr.as_bytes_with_nul().as_ptr()),
            0x0409,
            block_count_in_string_buffer_ptr,
            (std::mem::size_of::<u8>() * block_count_in_string_buffer.len())
                .try_into()
                .unwrap(),
        );
        success.as_bool()
    };

    if success {
        Ok(())
    } else {
        Err(())
    }
}