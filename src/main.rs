use nih_plug::util::window;

fn main() {
    let hann = window::hann(64);
    for h in hann {
        println!("{}", h);
    }
}
