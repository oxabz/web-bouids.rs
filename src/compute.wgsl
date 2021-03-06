struct Boid{ //align(16) size(32)
    position:vec2<f32>; // offset(0)  align(8) size(8)
    speed:vec2<f32>;    // offset(8)  align(8) size(8)
    color:vec3<f32>;    // offset(16) align(16) size(12)
    // padding(4)
};

struct Params {
    deltaT:f32;
    separationReach: f32;
    separationScale: f32;
    alignementReach: f32;
    alignementScale: f32;
    cohesionReach: f32;
    cohesionScale: f32;
    colorMult: f32;
    centerAttraction: f32;
};

struct Boids{
    boids:[[stride(32)]]array<Boid>;
};

[[group(0), binding(0)]]
var<uniform> params: Params;
[[group(0), binding(1)]]
var<storage> in: Boids;
[[group(0), binding(2)]]
var<storage, read_write> out: Boids;

[[stage(compute), workgroup_size(64)]]
fn step([[builtin(global_invocation_id)]] global_invocation_id: vec3<u32>){
    let total = arrayLength(&in.boids);
    let index = global_invocation_id.x;
    if (index >= total) {
        return;
    }



    var vPos: vec2<f32> = in.boids[index].position;
    var vVel: vec2<f32> = in.boids[index].speed;
    var vColor: vec3<f32> =  in.boids[index].color;

    var sepSum: vec2<f32> = vec2<f32>(0.0, 0.0);
    var sepCount: u32 = 0u;
    var aliSum: vec2<f32> = vec2<f32>(0.0, 0.0);
    var aliCount: u32 = 0u;
    var cohSum: vec2<f32> = vec2<f32>(0.0, 0.0);
    var cohCount: u32 = 0u;

    var i:u32 = 0u;
    loop {
        if (i >= total) {
            break;
        }
        if (index == i) {
            continue;
        }

        let oPos = in.boids[i].position;
        let oVel = in.boids[i].speed;
        let dist = distance(oPos,vPos);

        if(dist < params.separationReach){
            sepSum = sepSum + normalize(vPos - oPos) / ( dist * dist);
            sepCount = sepCount + 1u;
        }
        if(dist < params.alignementReach){
            aliSum = aliSum + oVel;
            aliCount = aliCount + 1u;
        }
        if(dist < params.cohesionReach){
            cohSum = cohSum + oPos;
            cohCount = cohCount + 1u;
        }

        continuing {
          i = i + 1u;
        }
    }

    let inertia = 20.;

    vVel = vVel * inertia;

    if(sepCount>0u){
        vVel = vVel + sepSum * params.separationScale * params.deltaT;
    }
    if(aliCount>0u){
        vVel = vVel + aliSum * params.alignementScale * params.deltaT;
    }
    if(cohCount>0u){
        let centerOfGrav = cohSum / f32(cohCount);
        vVel = vVel + (- vPos + centerOfGrav)  * params.cohesionScale * params.deltaT;
    }
    let distance_center = length(vPos);
    vVel = vVel - vPos * distance_center * params.centerAttraction * params.deltaT;

    vVel = normalize(vVel) * clamp(length(vVel), 0.0, 1.0);

    vPos = vPos + vVel * params.deltaT;

    out.boids[index].position = vPos;
    out.boids[index].speed = vVel;
}