//! zlib compression byte-identical to pwiz 4.x (pwiz-sharp on .NET 10).
//!
//! .NET 10's `System.IO.Compression` is backed by zlib-ng 2.2.5
//! (dotnet/runtime release/10.0 `src/native/external/zlib-ng`, "Update to zlib-ng
//! 2.2.5 (#118457)"; its `cgmanifest.json` still names 2.2.1), vendored here unmodified. Classic
//! zlib and other zlib-ng releases produce different (equally valid) streams; we
//! vendor the exact sources rather than track whichever release a `-sys` crate
//! bundles, since a zlib-ng bump is a contract change.
//! See `shim/dotnet_zlib.c` for the exact call sequence.

use std::ffi::CStr;
use std::os::raw::c_char;

extern "C" {
    fn mzx_zng_bound(n: usize) -> usize;
    fn mzx_zng_deflate_dotnet(input: *const u8, n: usize, out: *mut u8, cap: usize) -> i64;
    fn mzx_zng_version() -> *const c_char;
}

/// The zlib-ng version linked in (expected: "2.2.5").
pub fn version() -> &'static str {
    // SAFETY: zlibng_version returns a pointer to a static NUL-terminated string.
    unsafe { CStr::from_ptr(mzx_zng_version()) }
        .to_str()
        .unwrap_or("?")
}

/// `new ZLibStream(ms, CompressionLevel.Optimal).Write(input); Dispose()` on .NET 10.
pub fn deflate_dotnet(input: &[u8]) -> Vec<u8> {
    // SAFETY: pure function of its argument.
    let cap = unsafe { mzx_zng_bound(input.len()) };
    let mut out = vec![0u8; cap];
    // SAFETY: `input` and `out` are valid for the given lengths; the shim never
    // writes more than `cap` bytes.
    let n = unsafe { mzx_zng_deflate_dotnet(input.as_ptr(), input.len(), out.as_mut_ptr(), cap) };
    assert!(
        n >= 0,
        "zlib-ng deflate failed (input {} bytes)",
        input.len()
    );
    out.truncate(n as usize);
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn is_2_2_5() {
        assert_eq!(version(), "2.2.5");
    }

    #[test]
    fn roundtrips_and_has_zlib_header() {
        let data: Vec<u8> = (0..100_000u32)
            .flat_map(|i| (i as f64).sqrt().to_le_bytes())
            .collect();
        let c = deflate_dotnet(&data);
        assert_eq!(&c[..2], &[0x78, 0x9c]);
        assert!(c.len() > 8192, "exercise the multi-chunk path");
    }
}
