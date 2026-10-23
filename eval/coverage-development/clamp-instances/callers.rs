pub fn clamp_i32(x: i32, lo: i32, hi: i32) -> i32 { num_traits::clamp(x, lo, hi) }
pub fn clamp_i32_peer(x: i32, lo: i32, hi: i32) -> i32 { num_traits::clamp(x, lo, hi) }
pub fn clamp_min_i32(x: i32, lo: i32) -> i32 { num_traits::clamp_min(x, lo) }
pub fn clamp_min_i32_peer(x: i32, lo: i32) -> i32 { num_traits::clamp_min(x, lo) }
pub fn clamp_max_i32(x: i32, hi: i32) -> i32 { num_traits::clamp_max(x, hi) }
pub fn clamp_max_i32_peer(x: i32, hi: i32) -> i32 { num_traits::clamp_max(x, hi) }
pub fn clamp_u32(x: u32, lo: u32, hi: u32) -> u32 { num_traits::clamp(x, lo, hi) }
pub fn clamp_u32_peer(x: u32, lo: u32, hi: u32) -> u32 { num_traits::clamp(x, lo, hi) }
pub fn clamp_min_u32(x: u32, lo: u32) -> u32 { num_traits::clamp_min(x, lo) }
pub fn clamp_min_u32_peer(x: u32, lo: u32) -> u32 { num_traits::clamp_min(x, lo) }
pub fn clamp_max_u32(x: u32, hi: u32) -> u32 { num_traits::clamp_max(x, hi) }
pub fn clamp_max_u32_peer(x: u32, hi: u32) -> u32 { num_traits::clamp_max(x, hi) }
pub fn outside(x: u8, lo: u8, hi: u8) -> u8 { num_traits::clamp(x, lo, hi) }
