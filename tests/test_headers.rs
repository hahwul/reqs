#[tokio::test]
async fn test_headers() {
    let mut default_headers = reqwest::header::HeaderMap::new();
    default_headers.insert("X-Default", "default".parse().unwrap());
    default_headers.insert("X-Override", "default".parse().unwrap());

    let client = reqwest::Client::builder()
        .default_headers(default_headers)
        .build()
        .unwrap();

    let mut builder = client.get("http://example.com");
    builder = builder.header("X-Override", "custom");
    builder = builder.header("X-Custom", "custom");

    let req = builder.build().unwrap();

    println!("Headers: {:?}", req.headers());
    // In reqwest, builder.header overrides an existing header or adds a new one.
    // However, it doesn't show default headers when calling `req.headers()`.
    // Default headers are kept in the Client and merged right before the request is sent over the wire.
}
