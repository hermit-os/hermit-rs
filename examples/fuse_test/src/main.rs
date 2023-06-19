#[cfg(target_os = "hermit")]
use hermit_sys as _;

use std::fs;

fn main() {
    let paths = fs::read_dir("/root").unwrap();

    let _ = fs::remove_dir("/root/new_dir/");

    for path in paths {
        let path = path.unwrap();
        println!("Path: {}", path.path().display());
        println!("Name: {}", path.file_name().into_string().unwrap());
        let file_type =  path.file_type().unwrap();
        if file_type.is_dir() {
            println!("Is dir!\n");
        }
        else if file_type.is_file() {
            println!("Is file!\n");
        }
        else if file_type.is_symlink() {
            println!("Is symlink!\n");
        }
        else{
            println!("Unknown type!\n");
        }
    }
    println!("Done.");
}