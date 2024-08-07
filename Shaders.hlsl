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

RWTexture2D<float4> conversionTexture : register(u1);
RWStructuredBuffer<float> preprocessorOutput : register(u0);
RWTexture2D<uint4> conversionOutputTexture : register(u2);

#define THREAD_COUNT_X 32
#define THREAD_COUNT_Y 32
#define BLOCK_SIZE 128

[numthreads(THREAD_COUNT_X, THREAD_COUNT_Y, 1)]
void CS_convert_main(uint3 groupID : SV_GroupID, uint3 groupThreadID : SV_GroupThreadID, uint3 dispatchThreadID : SV_DispatchThreadID) {
    uint width, height;
    conversionTexture.GetDimensions(width, height);

    // Calculate the top-left corner of the block this thread processes
    uint baseX = dispatchThreadID.x * BLOCK_SIZE;
    uint baseY = dispatchThreadID.y * BLOCK_SIZE;

    float maxLuminosity = preprocessorOutput[0];

    for (uint i = 0; i < BLOCK_SIZE; ++i) {
        for (uint j = 0; j < BLOCK_SIZE; ++j) {
            uint x = baseX + i;
            uint y = baseY + j;

            if (x < width && y < height) {
                float4 hdrPixel = conversionTexture.Load(int3(x, y, 0));

                // Normalize the pixel values to convert from HDR to SDR
                float3 sdrColor = hdrPixel.rgb / maxLuminosity;

                // Ensure the pixel values are within the [0, 1] range
                sdrColor = clamp(sdrColor, 0.0f, 1.0f);

                // Convert to 8-bit color
                uint4 sdrPixel = uint4(sdrColor * 255.0f, 255.0f);

                // Write the SDR pixel to the output texture
                conversionOutputTexture[uint2(x, y)] = sdrPixel;
            }
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

    float luminance = px.r * 0.2 + px.g * 0.7 + px.b * 0.1;

    preprocessorOutput[0] = px.r;
}