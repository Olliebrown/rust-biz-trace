// File and standard I/O
use std::fs::File;
use std::io::prelude::*;

// For overloading operators
use std::ops;

// Random number generation
use rand::prelude::*;

// Cartesian product iteration
use itertools::iproduct;

// Thread pool execution with communication channel
use threadpool::ThreadPool;
use std::sync::mpsc::channel;

// Ray-tracing properties
const WIDTH: i32 = 512;
const SAMPLES: i32 = 256;
const GAIN: f32 = 224.0 / SAMPLES as f32;

//Define a vector struct with overloaded operators
#[derive(Debug, Copy, Clone)]
struct V { x: f32, y: f32, z: f32 }

// Return as clamped byte-array representing RGB color
impl V {
    fn c(self) -> [u8; 3] {
        [self.x.clamp(0.0, 255.0) as u8,
         self.y.clamp(0.0, 255.0) as u8,
         self.z.clamp(0.0, 255.0) as u8]
    }
}

// Vector addition
impl ops::Add<V> for V {
    type Output = V;
    fn add(self, r: V) -> V { V{ x: self.x + r.x, y: self.y + r.y, z: self.z + r.z } }
}

// Scaler Multiplication
impl ops::Mul<f32> for V {
    type Output = V;
    fn mul(self, r: f32) -> V { V{ x: self.x*r, y: self.y * r, z: self.z * r } }
}

// Dot product
impl ops::Rem<V> for V {
    type Output = f32;
    fn rem(self, r: V) -> f32 { self.x * r.x + self.y * r.y + self.z * r.z }
}

// Cross product
impl ops::BitXor for V {
    type Output = V;
    fn bitxor(self, r: V) -> V { V{ x: self.y*r.z - self.z*r.y, y: self.z*r.x-self.x*r.z, z: self.x*r.y-self.y*r.x } }
}

// In-place normalization
impl ops::Not for V {
    type Output = V;
    fn not(self) -> V {self * (1.0 / (self % self).sqrt())}
}

fn r() -> f32 {
    let mut rng = rand::thread_rng();
    rng.gen()
}

// The intersection test for ray (o = origin, d = direction).
// - Return 2 if a sphere hit was found (and also return distance t and normal n).
// - Return 0 if no sphere hit was found but ray goes upward (t and n are meaningless)
// - Return 1 if no sphere hit was found but ray goes downward (t and n are for ground plane intersection)
fn trace(o: &V, d: &V) -> (i32, f32, V) {
    // The world is encoded in g, with rows (numbers) each with 9-bits of info (1 = sphere, 0 = nothing)
    /* Original says 'aek'
      let g[]={247570,280596,280600,249748,18578,18577,231184,16,16};

        16                    1
        16                    1
        231184   111    111   1
        18577       1  1   1  1   1
        18578       1  1   1  1  1
        249748   1111  11111  1 1
        280600  1   1  1      11
        280596  1   1  1      1 1
        247570   1111   111   1  1
    */

    let g = [202766, 202779, 6150, 6152, 7579, 5902];
    /* a '.ru' version (traces in about 6 seconds in release mode)
     * 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 // 0
     * 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 // 0
     * 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 // 0
     * 0 0 0 0 0 0 1 0 1 1 1 0 0 0 0 1 1 1 0 // 5902
     * 0 0 0 0 0 0 1 1 1 0 1 1 0 0 1 1 0 1 1 // 7579
     * 0 0 0 0 0 0 1 1 0 0 0 0 0 0 0 1 0 0 0 // 6152
     * 0 0 0 0 0 0 1 1 0 0 0 0 0 0 0 0 1 1 0 // 6150
     * 0 1 1 0 0 0 1 1 0 0 0 0 0 0 1 1 0 1 1 // 202779
     * 0 1 1 0 0 0 1 1 0 0 0 0 0 0 0 1 1 1 0 // 202766
     */

    // Initialize to max time and pointing upward
    let mut t = f32::MAX;
    let mut m = 0;
    let mut n = V{x:0.0, y:0.0, z:1.0};

    // Check if intersects with floor plane
    let p = -o.z/d.z;
    if p > 0.01 {
        t = p;
        m = 1;
    }

    // Loop over all spheres
    for (k, j) in iproduct!(0..19, 0..g.len()) {
        if g[j] & 1<<k > 0 { //For this line j, is there a sphere at column k ?
            // There is a sphere but does the ray hit it ?
            let p = *o + V{x: -k as f32, y: 0.0, z: -(j as f32) - 4.0};
            let b = p % *d;
            let c = p % p - 1.0;
            let q = b * b - c;

            // Does the ray hit the sphere (solution to quadratic is non-imaginary)
            if q > 0.0 {
                // It does but is it closer than previous hit and in front of camera?
                let s= -b - q.sqrt();
                if s < t && s > 0.01 {
                    t = s;
                    n = !(p + *d * t);
                    m = 2;
                }
            }
        }
    }

    // Return type of intersection, time, and normal
    (m, t, n)
}

// Sample the world and return the pixel color for a ray passing by point o (Origin) and d (Direction)
fn sample(o: &V, d: &V) -> V {
    // Trace this ray through the world
    let (m, t, n) = trace(o, d);

    // Match based on type of hit
    match m {
        // Sky color approaches black exponentially the steeper the ray angle
        0 => V{x: 0.7, y:0.6, z: 1.0} * (1.0 - d.z).powi(4),

        // Hit sphere or plane
        _ => {
            // Intersection point, light direction (jittered for soft shadows), and reflection direction
            // Note: Light is a point light located at (9, 9, 16)
            let mut h = *o + *d * t;
            let l = !(V{x: 9.0 + r(), y: 9.0 + r(), z: 16.0} + h * -1.0);
            let rv = *d + n * (n % *d * -2.0);

            // Calculated the lambertian diffuse component
            let mut b = l % n;

            // Trace shadow ray (can skip if lambertian is non-positive)
            if b < 0.0 || trace(&h, &l).0 > 0 {
                b = 0.0;
            }

            match m {
                // Hit ground plane
                1 => {
                    h = h * 0.2;
                    let check = if (h.x.ceil() + h.y.ceil()) as i32 & 1 == 1 {
                        V{x: 3.0, y: 1.0, z: 1.0}
                    } else {
                        V{x: 3.0, y: 3.0, z: 3.0}
                    };

                    // Ground plane checkerboard
                    check * (b * 0.2 + 0.1)
                },

                // Hit sphere (do recursive bounce for reflectivity)
                _ => {
                    // Combine diffuse with Phong specular component
                    let p = ((l % rv) * (if b > 0.0 { 1.0 } else { 0.0 })).powi(99);

                    // Trace reflection ray and attenuate by 50% for lost light
                    V{x: p, y: p, z: p} + sample(&h, &rv) * 0.5
                }
            }
        }
    }
}

fn trace_pixel(x: i32, y: i32, av: V, bv: V, cv: V, i: usize) -> (usize, V) {
    // Reuse the vector class to store not XYZ but a RGB pixel color
    let mut p = V{x: 13.0, y: 13.0, z: 13.0};

    // Cast SAMPLES rays per pixel (sub-pixel super sampling)
    for _ in 0..SAMPLES {
        // Ray origin random jitter
        let t = av*(r() - 0.5) * 99.0 + bv*(r() - 0.5) * 99.0;

        // Ray originates from (16, 16, 8) jittered by t
        // Direction is also jittered (by same t) which gets you the distance-attenuated blur
        p = sample(
            &(V{x:16.0, y: 16.0, z: 8.0} + t),
            &(!(t * -1.0 + (av*(r() + x as f32) + bv * (y as f32 + r()) + cv) * 16.0))
        ) * GAIN + p; // +p for color accumulation, GAIN is just a brightness gain
    }

    // Return the color and index
    (i, p)
}

fn main() {
    // Vectors for orienting camera
    let gv = !V{x: -6.0, y: -16.0, z: 0.0};             // Camera direction
    let av = !(V{x:0.0, y:0.0, z:1.0} ^ gv) * 0.002;    // Camera up vector, Z is pointing up
    let bv = !(gv ^ av) * 0.002;                        // The right vector, obtained via traditional cross-product
    let cv = (av + bv) * -(WIDTH as f32/2.0) + gv;      // Directional offset to create perspective (1/2 width of scene)

    // Tracing progress is written to standard error (one . per row)
    eprint!("Tracing ...");

    // Create a thread pool and a channel for thread communication
    let pool = ThreadPool::new(12);
    let (tx, rx) = channel();

    // Create tasks for all the pixels
    let pixel_tasks = iproduct!(
        (1..=WIDTH).rev(), (1..=WIDTH).rev()
    );

    // Add each task to the thread pool and queue them up to send out their results
    let mut task_count = 0;
    for (i, pixel) in pixel_tasks.enumerate() {
        let tx = tx.clone();
        pool.execute(move || tx.send(trace_pixel(pixel.1, pixel.0, av, bv, cv, i)).unwrap());
        task_count += 1;
    }

    // Receive all the pixel color results collected in a vector, then sort the vector
    let mut colors = rx.iter().take(task_count).collect::<Vec<(usize, V)>>();
    colors.sort_by(|a, b| a.0.cmp(&b.0));

    // Write the results to 'result.ppm'
    let mut out = File::create("result.ppm").unwrap();
    out.write(format!("P6 {} {} 255 ", WIDTH, WIDTH).as_bytes()).unwrap();
    for color in colors.iter() {
        out.write(&color.1.c()).unwrap();
    }

    // Indicate completion
    eprintln!(" done.");
}
