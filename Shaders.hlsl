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

RWBuffer<float> minmaxValues: register(u0);
RWTexture2D<uint4> conversionOutputTexture: register(u2);

[numthreads(1,1,1)]
void CS_convert_main(uint3 dispatchThreadID : SV_DispatchThreadID ) {

    uint width;
    uint height;

    conversionTexture.GetDimensions(width, height);

    uint2 texSize = uint2(width, height);

    float minValue = minmaxValues[0];
    float maxValue = minmaxValues[1];


    for (uint y = 0; y < height; y+= 1) {
        for (uint x = 0; x < width; x+= 1) {


            // sample
            {
                uint2 pos = uint2(x,y);
                float4 px = conversionTexture.Load(pos);
                
                float4 normalisedValue = (px.rgba - minValue) / (maxValue - minValue);
                uint4 uintValue = uint4(normalisedValue * 0xFFFF);


                conversionOutputTexture[pos] = uintValue;
            }
        }
    }
}

// max threads is 1024
groupshared float4 sumValues[1024];

[numthreads(1,1,1)]
void CS_preprocess_main(uint3 dispatchThreadID: SV_DispatchThreadID, uint flatThreadID: SV_GROUPINDEX) {

    sumValues[flatThreadID] = 0;

    uint width;
    uint height;
    conversionTexture.GetDimensions(width, height);
    
    uint3 threads = uint3((width + 16 -1) / 16, (height + 16 -1) / 16, 1);
    uint num_threads = threads.x * threads.y * threads.z;

    float4 localSum = float4(0.0f, 0.0f, 0.0f, 0.0f);

    uint3 thread_max = min((dispatchThreadID + 1) * 16 -1, float3(width, height, 1) - 1);

    for (uint y = dispatchThreadID.y; y < thread_max.y; y++) {
        for (uint x = dispatchThreadID.x; x < thread_max.x; x++) {
            localSum += conversionTexture.Load(x, y);
        }
    }

    // wait until all sums completed
    sumValues[flatThreadID] = localSum;
    GroupMemoryBarrierWithGroupSync();

    uint i = 0;
    for (uint active_threads = num_threads >> 1; active_threads > 0; active_threads >>= 1) {
        if (flatThreadID < active_threads) {
            uint cells_per_thread = num_threads / active_threads;

            uint index = flatThreadID * cells_per_thread;
            sumValues[index] = sumValues[index] + sumValues[index+(1 << i)];

            i++;

        }
        GroupMemoryBarrierWithGroupSync();
    }



    // Store the result in the output buffer

    if (dispatchThreadID.x == 0 && dispatchThreadID.y == 0) {
        minmaxValues[0] = sumValues[0];
    }
    


}