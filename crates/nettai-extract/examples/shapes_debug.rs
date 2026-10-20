//! Fit a pack's frames as shapes again and write each beside its pixels,
//! to look at (`NETTAI_SHAPES_DEBUG`'s folder: `nettai_extract::shapes`).
//!
//!     cargo run -p nettai-extract --example shapes_debug -- data/exe6 OUT

fn main() {
    let mut args = std::env::args().skip(1);
    let (Some(pack), Some(out)) = (args.next(), args.next()) else { panic!("usage: shapes_debug <pack> <out>") };
    // SAFETY: set before any thread starts.
    unsafe { std::env::set_var("NETTAI_SHAPES_DEBUG", &out) };
    let (mut b, _) = nettai_content::pack::load_graphics(std::path::Path::new(&pack)).unwrap_or_else(|r| panic!("{r}"));
    for p in nettai_extract::shapes::read(&mut b) {
        println!("{p}");
    }
}
