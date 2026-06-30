#[test]
fn test_cloud_convert_blocking_on_zip() {
    let bytes = std::fs::read("D:\\temp\\upload.zip").unwrap();
    let result = crate::cloud_convert::convert_local_blocking(
        "http://127.0.0.1:2624/v1/",
        None, // Need to mock or pass a valid token? Wait, check_local_conversion needs a token!
        &bytes,
        "upload.zip",
        "",
        std::path::Path::new("D:\\temp\\output"),
        "auto",
        "standard",
    );
    println!("Result: {:?}", result);
}
