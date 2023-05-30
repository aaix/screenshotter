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


[numthreads(1,1,1)]
void CS_minmax_main(uint3 dispatchThreadID: SV_DispatchThreadID) {

    uint width;
    uint height;
    conversionTexture.GetDimensions(width, height);
    uint2 texDimensions = uint2(width, height);

    // 16bit floating point max
    float minValue = 0.0;
    float maxValue = 1.0f;

    for (uint y = 0; y < texDimensions.y; y++) {
        for (uint x = 0; x < texDimensions.x; x++) {

            float4 px = conversionTexture.Load(float2(x,y));

            // Update minimum value
            minValue = min(minValue, min(min(px.r,px.g),min(px.b, px.a)));

            // Update maximum value
            maxValue = max(maxValue, max(max(px.r,px.g),max(px.b, px.a)));
        }
    }



    // Store the result in the output buffer

    minmaxValues[0] = minValue;
    minmaxValues[1] = maxValue;

}