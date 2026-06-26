fn main() {
    let urls = vec![
        "",
        "http://127.0.0.1:2624/v1/",
        "127.0.0.1:2624/v1/",
        "localhost:2624/v1/",
        "https://api.tex2doc.cn/v1/",
        "api.tex2doc.cn/v1/"
    ];
    for u in urls {
        match url::Url::parse(u) {
            Ok(parsed) => {
                match parsed.join("auth/login") {
                    Ok(joined) => println!("{:?} => OK: {}", u, joined),
                    Err(e) => println!("{:?} => Join Error: {:?}", u, e),
                }
            }
            Err(e) => println!("{:?} => Parse Error: {:?}", u, e),
        }
    }
}
