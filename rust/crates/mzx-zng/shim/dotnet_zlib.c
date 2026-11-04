/*
 * Reproduces what pwiz 4.x BinaryDataEncoder.ZlibCompress does through .NET 10:
 *
 *   new ZLibStream(ms, CompressionLevel.Optimal)   -> deflateInit2(Z_DEFAULT_COMPRESSION, Z_DEFLATED, 15, 8, Z_DEFAULT_STRATEGY)
 *   z.Write(raw)                                   -> DeflateStream.WriteCore: SetInput(all), then
 *                                                     deflate(Z_NO_FLUSH) into an 8192-byte buffer until avail_in == 0
 *   Dispose()                                      -> PurgeBuffers: deflate(Z_FINISH) into 8192-byte buffers until Z_STREAM_END
 *
 * The output-buffer chunking is observable in the compressed bytes, so it is part of the contract.
 * Sources: dotnet/runtime release/10.0 DeflateStream.cs, Deflater.cs, ZLibNative.cs;
 * zlib-ng 2.2.5 (dotnet/runtime release/10.0 src/native/external/zlib-ng).
 */
#include <stdint.h>
#include <string.h>
#include "zlib-ng.h"

#define DOTNET_BUFFER 8192

size_t mzx_zng_bound(size_t n) {
    return (size_t)zng_deflateBound(NULL, n) + DOTNET_BUFFER;
}

/* Returns bytes written to out, or -1 on error / insufficient capacity. */
int64_t mzx_zng_deflate_dotnet(const uint8_t *in, size_t n, uint8_t *out, size_t cap) {
    if (n > UINT32_MAX) return -1;
    zng_stream s;
    memset(&s, 0, sizeof s);
    if (zng_deflateInit2(&s, Z_DEFAULT_COMPRESSION, Z_DEFLATED, 15, 8, Z_DEFAULT_STRATEGY) != Z_OK) return -1;
    uint8_t buf[DOTNET_BUFFER];
    size_t on = 0;
    int ret;
    s.next_in = in;
    s.avail_in = (uint32_t)n;
    while (s.avail_in != 0) {
        s.next_out = buf;
        s.avail_out = DOTNET_BUFFER;
        ret = zng_deflate(&s, Z_NO_FLUSH);
        if (ret != Z_OK && ret != Z_BUF_ERROR) goto fail;
        size_t k = DOTNET_BUFFER - s.avail_out;
        if (on + k > cap) goto fail;
        memcpy(out + on, buf, k);
        on += k;
    }
    do {
        s.next_out = buf;
        s.avail_out = DOTNET_BUFFER;
        ret = zng_deflate(&s, Z_FINISH);
        if (ret != Z_OK && ret != Z_STREAM_END && ret != Z_BUF_ERROR) goto fail;
        size_t k = DOTNET_BUFFER - s.avail_out;
        if (on + k > cap) goto fail;
        memcpy(out + on, buf, k);
        on += k;
    } while (ret != Z_STREAM_END);
    zng_deflateEnd(&s);
    return (int64_t)on;
fail:
    zng_deflateEnd(&s);
    return -1;
}

const char *mzx_zng_version(void) { return zlibng_version(); }
