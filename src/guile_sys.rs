/// Raw bindings to the Guile 3.x C API.
///
/// Only compiled for x86_64 macOS because the Homebrew guile bottle is
/// x86_64-only on Apple Silicon Macs.  The build.rs links libguile-3.0 for
/// that target.
///
/// SCM is `uintptr_t` in Guile's C headers (typedef via scm_t_bits).
/// Immediate constants are computed from libguile/scm.h:
///   scm_tc8_flag = scm_tc3_imm24 + 0x00 = 4
///   SCM_MAKIFLAG_BITS(n) = (n << 8) + 4
#[cfg(all(target_arch = "x86_64", target_os = "macos"))]
pub mod guile {
    use std::os::raw::{c_char, c_int};

    pub type Scm = usize;

    pub const SCM_BOOL_F: Scm = 4;     // MAKIFLAG_BITS(0): false
    pub const SCM_EOL: Scm = 772;      // MAKIFLAG_BITS(3): empty list '()
    pub const SCM_BOOL_T: Scm = 1028;  // MAKIFLAG_BITS(4): true

    unsafe extern "C" {
        /// Construct a pair (cons cell).
        pub fn scm_cons(car: Scm, cdr: Scm) -> Scm;
        /// Scheme string from a null-terminated UTF-8 C string.
        pub fn scm_from_utf8_string(s: *const c_char) -> Scm;
        /// Scheme symbol from a null-terminated UTF-8 C string.
        pub fn scm_from_utf8_symbol(name: *const c_char) -> Scm;
        /// Scheme real number from a C double.
        pub fn scm_from_double(x: f64) -> Scm;
        /// Scheme exact integer from u32.
        pub fn scm_from_uint32(n: u32) -> Scm;
        /// Scheme boolean from C int (0 = #f, non-zero = #t).
        pub fn scm_from_bool(n: c_int) -> Scm;
    }

    // -- Safe wrappers ------------------------------------------------------

    use std::ffi::CString;

    pub fn scm_str(s: &str) -> Scm {
        let cs = CString::new(s).unwrap_or_default();
        unsafe { scm_from_utf8_string(cs.as_ptr()) }
    }

    pub fn scm_sym(name: &str) -> Scm {
        let cs = CString::new(name).unwrap_or_default();
        unsafe { scm_from_utf8_symbol(cs.as_ptr()) }
    }

    pub fn scm_f64(x: f64) -> Scm {
        unsafe { scm_from_double(x) }
    }

    pub fn scm_u32(n: u32) -> Scm {
        unsafe { scm_from_uint32(n) }
    }

    pub fn scm_bool(b: bool) -> Scm {
        unsafe { scm_from_bool(b as c_int) }
    }

    /// Build a proper Scheme list from an iterator of SCM values.
    pub fn scm_list_from_iter<I: DoubleEndedIterator<Item = Scm>>(iter: I) -> Scm {
        let items: Vec<Scm> = iter.collect();
        let mut acc = SCM_EOL;
        for item in items.into_iter().rev() {
            acc = unsafe { scm_cons(item, acc) };
        }
        acc
    }

    /// Build an alist from (symbol-name, scm-value) pairs.
    pub fn scm_alist(pairs: &[(&str, Scm)]) -> Scm {
        let mut acc = SCM_EOL;
        for (key, val) in pairs.iter().rev() {
            let pair = unsafe { scm_cons(scm_sym(key), *val) };
            acc = unsafe { scm_cons(pair, acc) };
        }
        acc
    }

    /// Option<&str> → SCM string or #f.
    pub fn scm_opt_str(opt: Option<&str>) -> Scm {
        match opt {
            Some(s) => scm_str(s),
            None => SCM_BOOL_F,
        }
    }
}
