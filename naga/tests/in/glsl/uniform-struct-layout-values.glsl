#version 450
struct Samples { float values[2]; };
struct Wrapped { Samples entries[2]; };
layout(std140, binding = 0) uniform Uniforms {
    Wrapped uniform_value;
    Samples uniform_samples;
    int selector;
};
layout(std140, binding = 1) buffer Storage {
    Wrapped destination;
    Samples destination_samples;
    float result;
};
Samples identity(Samples value) { return value; }
Samples from_uniform() { return uniform_samples; }
void modify(inout Samples value) { value.values[0] += 1.0; }
void main() {
    Samples samples = uniform_samples;
    Wrapped wrapped = uniform_value;
    Samples entries[2] = uniform_value.entries;
    samples = identity(uniform_samples);
    samples = from_uniform();
    samples = selector == 0 ? uniform_samples : Samples(float[2](1.0, 2.0));
    destination = wrapped;
    destination.entries = entries;
    destination_samples = samples;
    modify(destination_samples);
    result = samples.values[1] + destination_samples.values[0];
}
