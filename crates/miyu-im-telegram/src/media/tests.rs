//! 发图、发文件（施工 I-185 中）：读宿主交的、照大小挑方法、拼请求体。

use base64::Engine;
use base64::engine::general_purpose::STANDARD;
use serde_json::{Map, json};

use super::{DOCUMENT_MOST, PHOTO_MOST, Upload, form};

fn encoded(bytes: &[u8]) -> String {
    format!("base64://{}", STANDARD.encode(bytes))
}

#[test]
fn images_go_as_photos_and_big_ones_as_documents() {
    let small = Upload::image(&encoded(b"png"), None).expect("读得出");
    assert_eq!((small.method, small.field), ("sendPhoto", "photo"));
    assert_eq!(small.bytes, b"png");
    let big = Upload::image(&encoded(&vec![0; PHOTO_MOST + 1]), None).expect("读得出");
    assert_eq!((big.method, big.field), ("sendDocument", "document"));
    let huge = Upload::image(&encoded(&vec![0; DOCUMENT_MOST + 1]), None);
    assert_eq!(huge.err().as_deref(), Some("file is larger than 50 MB"));
    assert!(Upload::image("base64://不是", None).is_err());
}

#[test]
fn documents_are_read_from_base64_or_a_local_path() {
    let named = Upload::document(&encoded(b"%PDF"), " 报告.pdf ").expect("读得出");
    assert_eq!(
        (named.method, named.name.as_str()),
        ("sendDocument", "报告.pdf")
    );
    let dir = std::env::temp_dir().join(format!("im-tg-media-{}", std::process::id()));
    std::fs::create_dir_all(&dir).expect("建得了");
    let path = dir.join("a.txt");
    std::fs::write(&path, b"hi").expect("写得进");
    let read = Upload::document(&path.to_string_lossy(), "").expect("照路径读");
    assert_eq!(
        (read.bytes.as_slice(), read.name.as_str()),
        (&b"hi"[..], "file")
    );
    assert!(Upload::document(&dir.join("missing").to_string_lossy(), "x").is_err());
    std::fs::remove_dir_all(&dir).expect("清得掉");
}

#[test]
fn the_form_carries_the_fields_and_the_file() {
    let upload = Upload {
        method: "sendPhoto",
        field: "photo",
        name: "a\"b\r\n.png".to_string(),
        bytes: b"\x89PNG".to_vec(),
        caption: None,
    };
    let mut fields = Map::new();
    fields.insert("chat_id".to_string(), json!("-1001"));
    fields.insert("reply_parameters".to_string(), json!({"message_id": 5}));
    let (content_type, body) = form(&fields, &upload);
    let boundary = content_type
        .strip_prefix("multipart/form-data; boundary=")
        .expect("带分隔符");
    let text = String::from_utf8_lossy(&body);
    assert!(text.contains("name=\"chat_id\"\r\n\r\n-1001\r\n"), "{text}");
    assert!(
        text.contains("name=\"reply_parameters\"\r\n\r\n{\"message_id\":5}\r\n"),
        "对象写成 JSON：{text}"
    );
    assert!(
        text.contains("name=\"photo\"; filename=\"a_b__.png\""),
        "文件名里的引号、换行换掉：{text}"
    );
    assert!(body.windows(4).any(|window| window == b"\x89PNG"));
    assert!(text.ends_with(&format!("\r\n--{boundary}--\r\n")));
}
