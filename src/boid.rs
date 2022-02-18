use lazy_static::lazy_static;
use rand::{
    distributions::{Distribution, Uniform},
    SeedableRng,
};
use rand_pcg::{Lcg128Xsl64};
use std::sync::Mutex;
use bytemuck::{Pod, Zeroable};
#[repr(C, align(16))]
#[derive(Copy, Clone, Debug, Pod, Zeroable)]
pub struct Boid{
    position:[f32;2],
    speed:[f32;2],
    color:[f32;3],
    _pad:[f32; 1],
}

lazy_static!{
    static ref RNG: Mutex<Lcg128Xsl64> = Mutex::new(rand_pcg::Pcg64::seed_from_u64(42));
    static ref POS_DIST: Uniform<f32> = Uniform::from(-10.0..10.0);
    static ref SPEED_DIST: Uniform<f32> = Uniform::from(-1.0..1.0);
    static ref COLOR_DIST: Uniform<f32> = Uniform::from(0.0..1.0);
}

impl Boid {
    pub fn new(position: [f32;2], speed: [f32;2], color: [f32;3])->Self{
        Boid{ position, speed,  color, _pad:[0.0] }
    }

    pub fn rand_new()->Self{
        let rng = &mut *RNG.lock().unwrap();
        Boid{
            position: [POS_DIST.sample(rng), POS_DIST.sample(rng)],
            speed: [SPEED_DIST.sample(rng), SPEED_DIST.sample(rng)],
            color: [COLOR_DIST.sample(rng), COLOR_DIST.sample(rng), COLOR_DIST.sample(rng)],
            _pad: [0.0]
        }
    }
}