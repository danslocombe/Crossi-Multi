// Based on https://www.shadertoy.com/view/4lB3Dc

#version 330

// Input vertex attributes (from vertex shader)
in vec2 fragTexCoord;
in vec4 fragColor;

// Input uniform values
uniform sampler2D texture0;
uniform vec4 colDiffuse;
uniform int iTime;
uniform float amp;
uniform int vignette;
uniform int crt;

// Output fragment color
out vec4 finalColor;

float rand(vec2 co) {
    return fract(sin(dot(co.xy ,vec2(12.9898,78.233))) * 43758.5453);
}

// From https://www.shadertoy.com/view/MdyyWt

const float PI = 3.14159265;
const float SCREEN_CURVE_RADIUS = 1.8 * PI;

vec2 curveScreen( vec2 uv ) {
    uv -= vec2(0.5);
    uv *= 2.0;

    float r = (PI * 0.5)  / SCREEN_CURVE_RADIUS;
    float dd = cos(uv.x * r) * cos(uv.y * r);
    float s = cos(r);

    vec2 ret = (s/dd) * uv;

    ret *= 0.5;
    ret += vec2(0.5);
    return ret;
}

const float SCREEN_CORNER_RADIUS = 0.1;

// From https://www.shadertoy.com/view/sltBWM
vec4 Televisionfy(in vec4 pixel, const in vec2 uv)
{
    float vignette = pow(uv.x * (1.0 - uv.x) * uv.y * (1.0 - uv.y), 0.25) * 2.2;
    vignette = (1.0 + vignette) * 0.5;
    //vignette = (1.0 + vignette * 0.5) * 0.75;
    return pixel * vignette;
}

void main()
{
    float jitter_amplitude = 0.21 * amp;
    float color_amplitude = 0.2 + 0.125 * amp;
    float bar_amplitude = 0.2 * amp;

    vec4 texColor = vec4(0);

    vec2 pos = fragTexCoord;
    pos = curveScreen(pos);

    if (pos.x < 0.05 || pos.x > 0.95 || pos.y < 0.05 || pos.y > 0.95) {
        finalColor = vec4(vec3(0.0), 1.0);
        return;
    }

    vec2 sampleCoord = pos;

    if (crt != 0) {
        // Jitter each line left and right
        sampleCoord.x += rand(vec2(iTime, pos.y) - 0.5) * (jitter_amplitude / 64.0);

        // Jitter the whole picture up and down
        sampleCoord.y += (rand(vec2(iTime))-0.5) * (jitter_amplitude/32.0);

        // Slightly add color noise to each line
        texColor += color_amplitude * (vec4(-0.5)+vec4(rand(vec2(pos.y,iTime)),rand(vec2(pos.y,iTime+1.0)),rand(vec2(pos.y,iTime+2.0)),0))*0.1;
        //texColor += color_amplitude * (vec4(-0.5)+vec4(rand(vec2(pos.x,iTime)),rand(vec2(pos.x,iTime+1.0)),rand(vec2(pos.x,iTime+2.0)),0))*0.1;

        float bar_center = 0.14;
        float bar_width_base = 0.04 * amp * 0.01;
        float bar_dropoff_top  = 0.03;
        float bar_dropoff_bot  = 0.01;

        // Either sample the texture, or just make the pixel white (to get the staticy-bit at the bottom)
        float whiteNoise = rand(vec2(floor(sampleCoord.y*80.0),floor(sampleCoord.x*50.0))+vec2(iTime,0));
        if (
                ((sampleCoord.y < bar_center +  bar_width_base - bar_dropoff_top * whiteNoise) 
                && 
                (sampleCoord.y > bar_center  - bar_width_base -bar_dropoff_bot * whiteNoise))
            ||
                ((sampleCoord.y < (1.0-bar_center) +  bar_width_base - bar_dropoff_top * whiteNoise) 
                && 
                (sampleCoord.y > (1.0-bar_center)  - bar_width_base -bar_dropoff_bot * whiteNoise))
        )
        {
            // Use white. (I'm adding here so the color noise still applies)
            texColor += vec4(1);
        } else {
            // Sample the texture.
            texColor += texture(texture0, sampleCoord);
        }
    }
    else
    {
        texColor = texture(texture0, sampleCoord);
    }

    finalColor = texColor;
    if (vignette != 0) {
        finalColor = Televisionfy(texColor, pos);
    }
}