mod functions;
pub mod patterns;
use functions::*;

fn main() {
    nannou::app(model).update(update).run();
}
