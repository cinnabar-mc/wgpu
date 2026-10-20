#version 460
layout(local_size_x = 1, local_size_y = 1, local_size_z = 1) in;

vec4 _26()
{
    return ((((vec4(ivec4(int(true)) + ivec4(0)) + vec4(bvec4(true))) + vec4(0.5)) + mix(vec4(0.0), vec4(1.0), vec4(0.100000001490116119384765625))) + vec4(intBitsToFloat(1))) + intBitsToFloat(ivec4(1));
}

ivec4 _49(ivec4 _51, ivec4 _52)
{
    bvec4 _55 = equal(_52, ivec4(0));
    bvec4 _60 = equal(_51, ivec4(int(0x80000000)));
    bvec4 _61 = equal(_52, ivec4(-1));
    bvec4 _62 = bvec4(_60.x && _61.x, _60.y && _61.y, _60.z && _61.z, _60.w && _61.w);
    ivec4 _65 = mix(_52, ivec4(1), bvec4(_55.x || _62.x, _55.y || _62.y, _55.z || _62.z, _55.w || _62.w));
    return _51 - ((_51 / _65) * _65);
}

vec4 _72(float _70, int _71)
{
    return (((vec2(2.0) + vec2(_70)) - vec2(4.0)) / vec2(8.0)).xyxy + vec4(_49(ivec4(_71), ivec4(2)));
}

vec2 _93()
{
    vec2 _98 = vec2(2.0);
    _98 += vec2(1.0);
    _98 -= vec2(3.0);
    _98 /= vec2(4.0);
    return _98;
}

vec3 _110(vec3 _109)
{
    return vec3(notEqual(_109, vec3(0.0)));
}

bool _119()
{
    return true;
}

bool _123()
{
    return false;
}

bool _126()
{
    return true;
}

bool _129()
{
    return false;
}

void _132()
{
    bool _149 = false;
    bool _143 = false;
    bool _153 = false;
    bool _147 = false;
    bool _140 = false;
    bool _151 = false;
    bool _145 = false;
    if (!true)
    {
        _140 = false;
    }
    else
    {
        _140 = true;
    }
    if (true)
    {
        _143 = false;
    }
    else
    {
        _143 = false;
    }
    if (!false)
    {
        _145 = false;
    }
    else
    {
        _145 = true;
    }
    if (!_119())
    {
        _147 = _123();
    }
    else
    {
        _147 = true;
    }
    if (_147)
    {
        if (!_126())
        {
            _151 = _129();
        }
        else
        {
            _151 = true;
        }
        _149 = _151;
    }
    else
    {
        _149 = false;
    }
    if (false)
    {
        _153 = _123();
    }
    else
    {
        _153 = true;
    }
}

int _200(int _202, int _203)
{
    return _202 / (((_203 == 0) || ((_202 == int(0x80000000)) && (_203 == (-1)))) ? 1 : _203);
}

uint _212(uint _214, uint _215)
{
    return _214 / ((_215 == 0u) ? 1u : _215);
}

ivec2 _223(ivec2 _225, ivec2 _226)
{
    bvec2 _229 = equal(_226, ivec2(0));
    bvec2 _232 = equal(_225, ivec2(int(0x80000000)));
    bvec2 _233 = equal(_226, ivec2(-1));
    bvec2 _234 = bvec2(_232.x && _233.x, _232.y && _233.y);
    return _225 / mix(_226, ivec2(1), bvec2(_229.x || _234.x, _229.y || _234.y));
}

uvec3 _239(uvec3 _241, uvec3 _242)
{
    return _241 / mix(_242, uvec3(1u), equal(_242, uvec3(0u)));
}

int _249(int _250, int _251)
{
    int _258 = ((_251 == 0) || ((_250 == int(0x80000000)) && (_251 == (-1)))) ? 1 : _251;
    return _250 - ((_250 / _258) * _258);
}

uint _262(uint _263, uint _264)
{
    return _263 % ((_264 == 0u) ? 1u : _264);
}

ivec2 _269(ivec2 _270, ivec2 _271)
{
    bvec2 _273 = equal(_271, ivec2(0));
    bvec2 _274 = equal(_270, ivec2(int(0x80000000)));
    bvec2 _275 = equal(_271, ivec2(-1));
    bvec2 _276 = bvec2(_274.x && _275.x, _274.y && _275.y);
    ivec2 _278 = mix(_271, ivec2(1), bvec2(_273.x || _276.x, _273.y || _276.y));
    return _270 - ((_270 / _278) * _278);
}

uvec3 _282(uvec3 _283, uvec3 _284)
{
    return _283 % mix(_284, uvec3(1u), equal(_284, uvec3(0u)));
}

uvec2 _290(uvec2 _292, uvec2 _293)
{
    return _292 / mix(_293, uvec2(1u), equal(_293, uvec2(0u)));
}

uvec2 _300(uvec2 _301, uvec2 _302)
{
    return _301 % mix(_302, uvec2(1u), equal(_302, uvec2(0u)));
}

void _308()
{
    int _320 = 0;
    int _323 = 0;
    float _326 = -1.0;
    ivec2 _327 = -ivec2(1);
    vec2 _328 = -vec2(1.0);
    int _329 = 2 + 1;
    uint _330 = 2u + 1u;
    float _331 = 2.0 + 1.0;
    ivec2 _332 = ivec2(2) + ivec2(1);
    uvec3 _333 = uvec3(2u) + uvec3(1u);
    vec4 _334 = vec4(2.0) + vec4(1.0);
    int _335 = 2 - 1;
    uint _336 = 2u - 1u;
    float _337 = 2.0 - 1.0;
    ivec2 _338 = ivec2(2) - ivec2(1);
    uvec3 _339 = uvec3(2u) - uvec3(1u);
    vec4 _340 = vec4(2.0) - vec4(1.0);
    int _341 = 2 * 1;
    uint _342 = 2u * 1u;
    float _343 = 2.0 * 1.0;
    ivec2 _344 = ivec2(2) * ivec2(1);
    uvec3 _345 = uvec3(2u) * uvec3(1u);
    vec4 _346 = vec4(2.0) * vec4(1.0);
    float _349 = 2.0 / 1.0;
    vec4 _352 = vec4(2.0) / vec4(1.0);
    float _355 = 2.0 - 1.0 * trunc(2.0 / 1.0);
    vec4 _358 = vec4(2.0) - vec4(1.0) * trunc(vec4(2.0) / vec4(1.0));
    ivec2 _361 = ivec2(2) + ivec2(1);
    ivec2 _362 = ivec2(2) + ivec2(1);
    uvec2 _363 = uvec2(2u) + uvec2(1u);
    uvec2 _364 = uvec2(2u) + uvec2(1u);
    vec2 _365 = vec2(2.0) + vec2(1.0);
    vec2 _366 = vec2(2.0) + vec2(1.0);
    ivec2 _367 = ivec2(2) - ivec2(1);
    ivec2 _368 = ivec2(2) - ivec2(1);
    uvec2 _369 = uvec2(2u) - uvec2(1u);
    uvec2 _370 = uvec2(2u) - uvec2(1u);
    vec2 _371 = vec2(2.0) - vec2(1.0);
    vec2 _372 = vec2(2.0) - vec2(1.0);
    vec2 _381 = vec2(2.0) * 1.0;
    vec2 _382 = vec2(1.0) * 2.0;
    vec2 _387 = vec2(2.0) / vec2(1.0);
    vec2 _388 = vec2(2.0) / vec2(1.0);
    vec2 _393 = vec2(2.0) - vec2(1.0) * trunc(vec2(2.0) / vec2(1.0));
    vec2 _394 = vec2(2.0) - vec2(1.0) * trunc(vec2(2.0) / vec2(1.0));
    mat3 _395 = mat3(vec3(0.0), vec3(0.0), vec3(0.0)) * 1.0;
    mat3 _396 = mat3(vec3(0.0), vec3(0.0), vec3(0.0)) * 2.0;
    vec3 _397 = mat4x3(vec3(0.0), vec3(0.0), vec3(0.0), vec3(0.0)) * vec4(1.0);
    vec4 _398 = vec3(2.0) * mat4x3(vec3(0.0), vec3(0.0), vec3(0.0), vec3(0.0));
    _323 = _320 + int(0x80000000);
}

void _402()
{
    int _404 = ~1;
    uint _405 = ~1u;
    ivec2 _406 = ~ivec2(1);
    uvec3 _407 = ~uvec3(1u);
    int _408 = 2 | 1;
    uint _409 = 2u | 1u;
    ivec2 _410 = ivec2(2) | ivec2(1);
    uvec3 _411 = uvec3(2u) | uvec3(1u);
    int _412 = 2 & 1;
    uint _413 = 2u & 1u;
    ivec2 _414 = ivec2(2) & ivec2(1);
    uvec3 _415 = uvec3(2u) & uvec3(1u);
    int _416 = 2 ^ 1;
    uint _417 = 2u ^ 1u;
    ivec2 _418 = ivec2(2) ^ ivec2(1);
    uvec3 _419 = uvec3(2u) ^ uvec3(1u);
    int _420 = 2 << int(1u);
    uint _421 = 2u << 1u;
    ivec2 _422 = ivec2(2) << ivec2(uvec2(1u));
    uvec3 _423 = uvec3(2u) << uvec3(1u);
    int _424 = 2 >> int(1u);
    uint _425 = 2u >> 1u;
    ivec2 _426 = ivec2(2) >> ivec2(uvec2(1u));
    uvec3 _427 = uvec3(2u) >> uvec3(1u);
}

void _429()
{
}

void _468()
{
    int _470 = 0;
    ivec3 _472 = ivec3(0);
    _470 = 1;
    _470++;
    _470--;
    _470 *= _470;
    _470 = _200(_470, _470);
    _470 = _249(_470, 1);
    _470 &= 0;
    _470 |= 0;
    _470 ^= 0;
    _470 = _470 << int(2u);
    _470 = _470 >> int(1u);
    _470++;
    _470--;
    _472.y++;
    _472.y--;
}

void _510()
{
    int _512 = -1;
    int _513 = -1;
    int _515 = -1;
    int _517 = -1;
    int _519 = -1;
    int _522 = -1;
    int _526 = -1;
    int _531 = -1;
    float _536 = -1.0;
    float _537 = -1.0;
    float _539 = -1.0;
    float _541 = -1.0;
    float _543 = -1.0;
    float _546 = -1.0;
    float _550 = -1.0;
    float _555 = -1.0;
}

void main()
{
    _132();
    _308();
    _402();
    _429();
    _468();
    _510();
}

