# Rust Business Card Raytracer
A verbose and messy port of the C business card raytracer to Rust. I followed the analysis from Fabian Sanglard
(found here)[https://fabiensanglard.net/rayTracing_back_of_business_card/] and focused on his commented and spaced
code and not on the heavily minimized version that fits on a card but is not really for human consumption.

## Could This Ever Fit On a Business Card?
It would likely require a more 'rust-like' approach. The original C version takes advantage of many esoteric C
features (like typedefs for primitives, operator overloading, very lose type-casting, bools as integers) to shorten
the code a lot and those don't seem to gain you much in Rust. Most of them ARE there (other than bools as ints) but
they just don't shorten the code much.

- Typedefing primities is not a big deal when a lot of your types can be inferred.
- You CAN'T liberally coerce numbers between float and int in rust
- Operator overloading is done with traits which are more verbose than the C approach

Still, I'd be curious if just starting over and doing things in a more Rusty idiom if you could do it ... not for
me though! I'm brand new here (just started learning Rust a month ago), and I'm just jazzed I got this to work.
