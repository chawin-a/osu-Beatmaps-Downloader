// build.rs

#[cfg(windows)]
extern crate winres;

#[cfg(windows)]
fn main() {
    let mut res = winres::WindowsResource::new();
    res.set_icon("src/img/icon.ico");
    res.compile().unwrap();
}

#[cfg(unix)]
fn main() {}
