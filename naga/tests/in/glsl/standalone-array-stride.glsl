#version 450
struct Samples { float values[2]; };
layout(std140, binding = 0) uniform Uniforms { Samples uniform_samples; };
layout(std430, binding = 1) buffer Storage { Samples storage_samples; };
void main() {
    Samples samples = Samples(float[2](1.0, 2.0));
    float copy[2] = samples.values;
    storage_samples = samples;
    storage_samples.values[1] = copy[0] + uniform_samples.values[1];
}
