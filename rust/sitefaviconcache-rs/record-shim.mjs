// Record shim for the sitefaviconcache corpus.
//
// `resolveFavicon`'s recorded output embeds `Date.now() + TTL` in `expiresAtMs`
// and calls `outboundHttp.request` for every cache miss. Both are frozen here so
// the corpus is deterministic: `Date.now` returns a fixed instant and every
// outbound fetch rejects, so each recorded case takes the all-sources-failed
// path and stores a negative-cache entry (`bytes`/`contentType` null, TTL 1 h).
// The Rust port replays the frozen instant via SYNARA_FAVICON_NOW_MS.
import { outboundHttp } from '@synara/shared/outboundHttp'

Date.now = () => 946684800000 // 2000-01-01T00:00:00.000Z

outboundHttp.request = async () => {
  throw new Error('rust-anything record shim: outbound fetch disabled')
}

export { tryParseHost, resolveFavicon, clearSiteFaviconCache } from '../../apps/server/src/siteFaviconCache.ts'
