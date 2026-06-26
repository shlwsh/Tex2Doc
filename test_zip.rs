use std::fs;
use std::io::Cursor;

fn main() {
    let bytes = fs::read("D:\\temp\\upload.zip").unwrap();
    let reader = Cursor::new(&bytes);
    match zip::ZipArchive::new(reader) {
        Ok(mut archive) => {
            println!("Archive opened successfully. Files: {}", archive.len());
            for i in 0..archive.len() {
                let file = archive.by_index(i).unwrap();
                println!("File {}: {}", i, file.name());
                if let Some(enclosed) = file.enclosed_name() {
                    println!("  enclosed: {}", enclosed.display());
                } else {
                    println!("  no enclosed name");
                }
            }
        }
        Err(e) => {
            println!("Error opening archive: {:?}", e);
        }
    }
}
