#version 330

// Shadow Menu motion blur: this frame mixed with the last blurred one.
uniform sampler2D InSampler;
uniform sampler2D PrevSampler;

in vec2 texCoord;

layout(std140) uniform ShadowBlurConfig {
    float BlendFactor;
};

out vec4 fragColor;

void main() {
    vec4 current = texture(InSampler, texCoord);
    vec4 previous = texture(PrevSampler, texCoord);
    fragColor = vec4(mix(current.rgb, previous.rgb, BlendFactor), 1.0);
}
