//! 64-bit specific Apple (ios/darwin) definitions

pub type c_long = i64;
pub type c_ulong = u64;

s! {
    pub struct pthread_attr_t {
        __sig: c_long,
        __opaque: [::c_char; 56]
    }
}

pub const __PTHREAD_MUTEX_SIZE__: usize = 56;
pub const __PTHREAD_COND_SIZE__: usize = 40;
pub const __PTHREAD_RWLOCK_SIZE__: usize = 192;