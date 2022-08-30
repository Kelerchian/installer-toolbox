pub const BLOCKSIZE: usize = 1000; // in bytes;
pub const RES_TYPE: &str = "sfxr/type";
pub const RES_NAME_COUNT: &str = "sfxr/data/count";
pub fn get_index_key(index: &u32) -> String {
    format!("sfxr/data/index/{}", index)
}
