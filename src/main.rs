use std::ops;
use std::io::Write;
use rand::prelude::*;
use itertools::iproduct;

type I = i32;
type F = f32;

//Define a vector struct with overloaded operators
#[derive(Debug, Copy, Clone)]
struct V { x: F, y: F, z: F }

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
impl ops::Mul<F> for V {
    type Output = V;
    fn mul(self, r: F) -> V { V{ x: self.x*r, y: self.y * r, z: self.z * r } }
}

// Dot product
impl ops::Rem<V> for V {
    type Output = F;
    fn rem(self, r: V) -> F { self.x * r.x + self.y * r.y + self.z * r.z }
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
fn trace(o: &V, d: &V) -> (I, F, V) {
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
            let p = *o + V{x: -k as F, y: 0.0, z: -(j as F) - 4.0};
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
        // Sky color as gradient
        0 => V{x: 0.7, y:0.6, z: 1.0} * (1.0 - d.z).powi(4),

        // Hit sphere or plane
        _ => {
            // Intersection point, light direction, and half vector
            let mut h = *o + *d * t;
            let l = !(V{x: 9.0 + r(), y: 9.0 + r(), z: 16.0} + h * -1.0);
            let rv = *d + n * (n % *d * -2.0);

            // Calculated the lambertian factor
            let mut b = l % n;

            // Calculate illumination factor (lambertian coefficient > 0 or in shadow)?
            if b < 0.0 || trace(&h, &l).0 > 0 {
                b = 0.0;
            }

            // Calculate the color 'p' with diffuse and specular component
            let p = ((l % rv) * (if b > 0.0 { 1.0 } else { 0.0 })).powi(99);

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
                _ => V{x: p, y: p, z: p} + sample(&h, &rv) * 0.5 //Attenuate color by 50% since it is bouncing (* .5)
            }
        }
    }
}

fn main() {
    // Vectors for orienting camera
    let gv = !V{x: -6.0, y: -16.0, z: 0.0};         // Camera direction
    let av = !(V{x:0.0, y:0.0, z:1.0} ^ gv) * 0.002;// Camera up vector, Z is pointing up
    let bv = !(gv ^ av) * 0.002;                    // The right vector, obtained via traditional cross-product
    let cv = (av + bv) * -256.0 + gv;               // See https://news.ycombinator.com/item?id=6425965.

    // Image data is written to standard out as binary arrays
    let mut out = std::io::stdout();
    out.write("P6 512 512 255 ".as_bytes()).unwrap(); // The PPM Header is issued

    // Tracing progress is written to standard error (one . per row)
    let mut last = 512;
    eprint!("Tracing ");
    for (y, x) in iproduct!((1..=512i32).rev(), (1..=512i32).rev()) {
        // Reuse the vector class to store not XYZ but a RGB pixel color
        let mut p = V{x: 13.0, y: 13.0, z: 13.0};

        // Cast 64 rays per pixel (sub-pixel super sampling)
        for _ in 0..64 {
           // Ray origin random jitter
           let t = av*(r() - 0.5) * 99.0 + bv*(r() - 0.5) * 99.0;

           // Ray originates from (16, 16, 8) jittered by t
           // Direction is also jittered which gets you the depth-of-field like blur
           p = sample(
               &(V{x:16.0, y: 16.0, z: 8.0} + t),
               &(!(t * -1.0 + (av*(r() + x as F) + bv * (y as F + r()) + cv) * 16.0))
           ) * 3.5 + p; // +p for color accumulation
        }

        // Output binary colors to stdout
        out.write(&p.c()).unwrap();

        // Output progress to stderr
        if y < last { eprint!("."); last -= 1; }
    }
    eprintln!(" done.");
}
