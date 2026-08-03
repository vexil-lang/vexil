# Compression Layering

Vexil messages are canonical, uncompressed bytes. Compression belongs outside
the generated codec, so it does not change schema hashes, compatibility, or the
meaning of a Vexil payload.

For an application protocol, use an explicit envelope:

1. Encode one bounded Vexil message or a deliberately defined batch.
2. Compress that complete frame or batch.
3. Apply integrity protection or authenticated encryption to the compressed
   bytes when the protocol requires it.
4. Carry the compression algorithm, dictionary identity, compressed length,
   decompressed limit, and schema identity in the surrounding protocol.

Do not infer compression from the payload. Reject unknown algorithms and
dictionary identifiers, truncated streams, trailing compressed data when the
profile forbids it, and output that exceeds the declared decompressed limit.
Bound both compressed input and decompressed output before allocating. These
limits are application or transport policy and are separate from Vexil's
collection and recursion limits.

Compress before encryption; encrypted bytes are not usefully compressible, and
compressing attacker-controlled and secret material together can create side
channels. A real transport profile still needs its own threat model and rules
for negotiation, authentication, replay, and failure handling.

The compressed `.vxb` form in `vexil-store` is a file-container feature. It does
not make ordinary generated wire messages compressed and should not be treated
as a transport profile.
