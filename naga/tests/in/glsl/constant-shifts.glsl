#version 450
const uint unsigned_wrap = 0x80000001u << 1;
const int signed_wrap = 0x40000001 << 2;
const int negative_wrap = -2147483647 << 1;
const uvec2 vector_wrap = uvec2(0xffffffffu, 0x80000001u) << uvec2(1u, 2u);
void main() {
    uint folded_at_runtime = 0x80000001u << 1;
}
