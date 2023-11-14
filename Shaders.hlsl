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

RWTexture2D<float4> conversionTexture: register(u1);
RWStructuredBuffer<float> preprocessorOutput: register(u0);
RWTexture2D<uint4> conversionOutputTexture: register(u2);

#define THREAD_COUNT 1024
#define THREAD_COUNT_X 32
#define THREAD_COUNT_Y 32


[numthreads(1, 1, 1)]
void CS_convert_main(uint3 threadID: SV_GROUPTHREADID, uint flatThreadID: SV_GROUPINDEX) {  

    uint width;
    uint height;
    conversionTexture.GetDimensions(width, height);

    for (uint y = thread_start.y; y < thread_max.y; y++) {
        for (uint x = thread_start.x; x < thread_max.x; x++) {
            float4 px = conversionTexture.Load(x,y);
            conversionOutputTexture[uint2(x,y)] = uint4(255,0,0,255);
        }
    }
}

[numthreads(1, 1, 1)]
void CS_preprocess_main(uint3 threadID: SV_GROUPTHREADID, uint flatThreadID: SV_GROUPINDEX) {

    uint width;
    uint height;
    conversionTexture.GetDimensions(width, height);

    uint mip = (32 - firstbithigh(max(width, height)))-1;

    float4 px = conversionTexture.Load(uint2(1,1), mip);

    float luminance = px.r * 0.2126 + px.g * 0.7152 + px.b * 0.0722;

    preprocessorOutput[0] = luminance;
}