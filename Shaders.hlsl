// Vertex shader input structure
struct VertexInput
{
    float2 position : POSITION;
    float2 texcoord : TEXCOORD;
};

// Vertex shader output structure
struct VertexOutput
{
    float4 position : SV_POSITION;
    float2 texcoord : TEXCOORD;
};

// Vertex shader
VertexOutput VS_main(VertexInput input)
{
    VertexOutput output;
    output.position = float4(input.position, 0.0, 1.0);
    output.texcoord = input.texcoord;
    return output;
}

// Texture and sampler
Texture2D<float4> textureInput : register(t0);
SamplerState samplerLinear : register(s0);

// Pixel shader
float4 PS_main(VertexOutput input) : SV_TARGET
{
    
    float4 px = textureInput.Sample(samplerLinear, input.texcoord);
    return float4(px.rgb * 0.5f, px.a);
}