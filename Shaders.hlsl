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
Texture2D<float4> renderTextureInput : register(t0);
SamplerState samplerLinear : register(s0);

cbuffer Constants : register(b0)
{
    struct Rect
    {
        float2 topLeft;
        float2 bottomRight;
    };

    Rect region;
};

// Pixel renderer shader
float4 PS_main(VertexOutput input) : SV_TARGET
{
    
    float4 px = renderTextureInput.Sample(samplerLinear, input.texcoord);

    if (
        input.texcoord.x > region.topLeft.x &&
        input.texcoord.y > region.topLeft.y &&
        input.texcoord.x < region.bottomRight.x &&
        input.texcoord.y < region.bottomRight.y
    ) {
        return px;
    } else {
       return float4(px.rgb * 0.35f, px.a);
    }
    
}

Texture2D<float4> convertTextureInput : register(t0);

uint4 PS_convert_main(VertexOutput input): SV_TARGET
{
    
    float4 px = convertTextureInput.Sample(samplerLinear, input.texcoord);

    return uint4(px.rgba * 0xFFFF);
    
}