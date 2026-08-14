I have a web page with a somewhat complicated state. I'd like to be able to give a url to someone where they can access the web page with the same state, but without that url being incredibly long. Can you think of a few ways of doing this.

Four approaches, roughly in order of decreasing coupling to a server:

1. Server-side state store (short IDs)
POST the state blob to your backend, get back an opaque key, share /s/aB3x9Q. Shortest possible URLs, arbitrary state size, and you can version/migrate stored states later. Costs: you need a database, an expiry policy, and access control. This is what most "share" buttons actually do.

2. Content-addressed store
Same as (1), but the key is a hash of the canonical serialized state. Deduplicates identical states for free, makes writes idempotent, and lets you cache aggressively (immutable content). Truncate the hash to ~8-10 base32 chars and handle collisions by checking on write.

3. Compress + encode in the fragment
Serialize compactly (avoid JSON key repetition — use a positional array or a binary format like MessagePack/CBOR), then deflate-raw via CompressionStream, then base64url. Put it in # rather than ? so it never hits the server and doesn't inflate your logs. Typical outcome: JSON that starts at 2 KB often lands under 300 chars. Fully stateless, works offline, no expiry — but the URL still scales with state size, and you own the deserialization compatibility problem forever (old links must keep working).

4. Attack the encoding, not the transport
Often the real problem is that the state is verbosely represented. Diff against a default state and encode only deltas. Replace enum strings with small integers. Round floats to the precision you actually render. Drop derived values you can recompute on load. This alone frequently makes (3) viable without any compression cleverness, and it makes (1) cheaper too.

Hybrid worth considering: encode in the fragment by default, and only fall back to the server store when the encoded length exceeds some threshold (say 1500 chars, comfortably under the ~2000-char limit older browsers and link previewers choke on). You get zero-infrastructure sharing for the common case and correctness for the tail.

One caveat on (3): fragments aren't sent to servers, which is a feature for privacy but means link-preview generators, server-side rendering, and analytics see nothing. If any of that matters, you need (1).

There won't be any server so I'm guessing I'll need a way to translate the json state to a smaller format and encode it. I'm building this in dioxus/rust.

Weighed serialization formats and compression strategies for URL-safe state encoding

Right — with no server, the whole problem reduces to: make the byte string small, then encode it URL-safely. Three stages, and most people over-invest in the wrong one.

The pipeline
State → serialize (compact binary) → compress (maybe) → base64url → fragment
1. Serialization — this is where the real win is

serde_json wastes enormous space on field names and ASCII numbers. Switch to a non-self-describing binary format:

postcard — my default recommendation. Varint-encoded, no field names, no type tags, no_std, tiny WASM footprint. Drops in behind your existing serde::Serialize derives. Typically 5-10x smaller than JSON for struct-heavy state.
bitcode — often marginally smaller than postcard (bit-level packing rather than byte-aligned), but heavier and the format is less stable across versions.
rmp-serde (MessagePack) — self-describing, so ~2-3x better than JSON rather than 5-10x. Only worth it if you specifically want forward-compatibility from the format itself.

The self-describing/not distinction is the real trade-off, and I'll come back to it because it will bite you.

2. Compression — conditional, not automatic

Deflate has fixed overhead and needs redundancy to exploit. On a 60-byte postcard blob it will usually make things bigger. On a 2 KB blob it'll help a lot. So: compress, compare, keep the smaller, and record which you did in a header byte.

miniz_oxide is pure Rust, no C toolchain, works cleanly in WASM. Use raw deflate (compress_to_vec), not zlib-wrapped — the 2-byte header buys you nothing here.

3. Encoding

base64::URL_SAFE_NO_PAD. 6 bits/char. You could theoretically squeeze ~6.04 bits/char using the full fragment-legal charset — not worth the code.

Concrete implementation
toml
[dependencies]
serde = { version = "1", features = ["derive"] }
postcard = { version = "1", features = ["alloc"] }
miniz_oxide = "0.8"
base64 = "0.22"
rust
use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};

const SCHEMA_V: u8 = 1;
const FLAG_DEFLATE: u8 = 0x80;
const MAX_DECOMPRESSED: usize = 256 * 1024;

pub fn encode(state: &State) -> String {
    let raw = postcard::to_allocvec(state).expect("infallible for our types");
    let deflated = miniz_oxide::deflate::compress_to_vec(&raw, 10);

    let (payload, flag) = if deflated.len() < raw.len() {
        (deflated, FLAG_DEFLATE)
    } else {
        (raw, 0)
    };

    let mut out = Vec::with_capacity(payload.len() + 1);
    out.push(SCHEMA_V | flag);
    out.extend_from_slice(&payload);
    URL_SAFE_NO_PAD.encode(out)
}

pub fn decode(s: &str) -> Result<State, DecodeError> {
    let bytes = URL_SAFE_NO_PAD.decode(s)?;
    let (header, body) = bytes.split_first().ok_or(DecodeError::Empty)?;

    let raw = if header & FLAG_DEFLATE != 0 {
        miniz_oxide::inflate::decompress_to_vec_with_limit(body, MAX_DECOMPRESSED)
            .map_err(|_| DecodeError::Inflate)?
    } else {
        body.to_vec()
    };

    match header & 0x7F {
        1 => Ok(postcard::from_bytes::<State>(&raw)?),
        v => Err(DecodeError::UnknownVersion(v)),
    }
}

The decompression limit matters: a hostile link is a trivial zip bomb against someone else's tab.

Dioxus wiring

Don't use the router for this — the state blob isn't a route. Touch web_sys directly, and use replace_state so dragging a slider doesn't produce 400 history entries:

rust
fn write_url(state: &State) {
    let win = web_sys::window().unwrap();
    let encoded = encode(state);
    let url = format!("{}#{}", win.location().pathname().unwrap(), encoded);
    let _ = win.history().unwrap().replace_state_with_url(
        &wasm_bindgen::JsValue::NULL, "", Some(&url),
    );
}

fn read_url() -> Option<State> {
    let hash = web_sys::window()?.location().hash().ok()?;
    decode(hash.strip_prefix('#')?).ok()
}

In the component: read once at mount, then use_effect to push changes back. Debounce the write if the state changes on every keystroke or drag frame — replace_state is cheap but not free.

The trap: postcard is not self-describing

This is the part worth thinking hard about now rather than in six months. Postcard encodes fields positionally. Reorder two u32 fields, add a field in the middle, change an enum variant order — every URL you've ever shared silently decodes into garbage. Not an error. Garbage.

Mitigations, pick one:

Freeze and version. Keep StateV1, StateV2 as separate frozen types with From impls forming a migration chain. The version byte dispatches. Correct, tedious, and the only approach that actually works long-term.
Append-only discipline. New fields go at the end wrapped in Option<T>. Never reorder, never remove. Fragile because it depends on humans remembering.
Accept self-describing. Use rmp-serde and eat the size cost in exchange for tolerating field addition. Reasonable if your state is small enough that JSON-ish overhead doesn't matter.

Add a round-trip test with a hardcoded base64 string per version, asserting it still decodes to the expected state. That test is what catches (2) failing.

Before reaching for compression, fix the representation

Cheaper than any of the above, and compounds with all of it:

Encode a diff against State::default(), not the full state. Fields at default cost you a single zero byte or nothing at all.
Enums as discriminants, never strings.
Round floats to display precision. An f64 costs 8 bytes; a slider position quantized to u8 or u16 costs 1-2 and is visually identical.
Drop derived fields. Anything recomputable on load is pure waste in the URL.
Arrays of similar numbers: delta-encode, then varint. Turns 8-byte floats into 1-byte deltas.

Realistically, a state that's 3 KB of JSON usually lands somewhere around 100-250 URL characters after this pipeline. If yours doesn't, the representation is the problem, not the encoding.
