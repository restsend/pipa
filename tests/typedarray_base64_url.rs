
use pipa::{JSRuntime, eval};

#[test]
fn test_btoa_basic() {
    let mut rt = JSRuntime::new();
    let mut ctx = rt.new_context();

    let result = eval(&mut ctx, "btoa('hello')").unwrap();
    assert!(result.is_string());
    let atom = result.get_atom();
    assert_eq!(ctx.get_atom_str(atom), "aGVsbG8=");
}

#[test]
fn test_atob_basic() {
    let mut rt = JSRuntime::new();
    let mut ctx = rt.new_context();

    let result = eval(&mut ctx, "atob('aGVsbG8=')").unwrap();
    assert!(result.is_string());
    let atom = result.get_atom();
    assert_eq!(ctx.get_atom_str(atom), "hello");
}

#[test]
fn test_btoa_atob_roundtrip() {
    let mut rt = JSRuntime::new();
    let mut ctx = rt.new_context();

    let result = eval(&mut ctx, "atob(btoa('test string 123'))").unwrap();
    assert!(result.is_string());
    let atom = result.get_atom();
    assert_eq!(ctx.get_atom_str(atom), "test string 123");
}

#[test]
fn test_btoa_empty() {
    let mut rt = JSRuntime::new();
    let mut ctx = rt.new_context();

    let result = eval(&mut ctx, "btoa('')").unwrap();
    assert!(result.is_string());
    let atom = result.get_atom();
    assert_eq!(ctx.get_atom_str(atom), "");
}

#[test]
fn test_encodeuri_basic() {
    let mut rt = JSRuntime::new();
    let mut ctx = rt.new_context();

    let result = eval(&mut ctx, "encodeURI('hello world')").unwrap();
    assert!(result.is_string());
    let atom = result.get_atom();
    assert_eq!(ctx.get_atom_str(atom), "hello%20world");
}

#[test]
fn test_decodeuri_basic() {
    let mut rt = JSRuntime::new();
    let mut ctx = rt.new_context();

    let result = eval(&mut ctx, "decodeURI('hello%20world')").unwrap();
    assert!(result.is_string());
    let atom = result.get_atom();
    assert_eq!(ctx.get_atom_str(atom), "hello world");
}

#[test]
fn test_encodeuri_preserves_unreserved() {
    let mut rt = JSRuntime::new();
    let mut ctx = rt.new_context();

    let result = eval(&mut ctx, "encodeURI('/path?key=value#hash')").unwrap();
    assert!(result.is_string());
    let atom = result.get_atom();
    assert_eq!(ctx.get_atom_str(atom), "/path?key=value#hash");
}

#[test]
fn test_encodeuricomponent_basic() {
    let mut rt = JSRuntime::new();
    let mut ctx = rt.new_context();

    let result = eval(&mut ctx, "encodeURIComponent('hello world')").unwrap();
    assert!(result.is_string());
    let atom = result.get_atom();
    assert_eq!(ctx.get_atom_str(atom), "hello%20world");
}

#[test]
fn test_encodeuricomponent_encodes_more() {
    let mut rt = JSRuntime::new();
    let mut ctx = rt.new_context();

    let result = eval(&mut ctx, "encodeURIComponent('/path?key=value')").unwrap();
    assert!(result.is_string());
    let atom = result.get_atom();
    assert_eq!(ctx.get_atom_str(atom), "%2Fpath%3Fkey%3Dvalue");
}

#[test]
fn test_decodeuricomponent_basic() {
    let mut rt = JSRuntime::new();
    let mut ctx = rt.new_context();

    let result = eval(&mut ctx, "decodeURIComponent('hello%20world')").unwrap();
    assert!(result.is_string());
    let atom = result.get_atom();
    assert_eq!(ctx.get_atom_str(atom), "hello world");
}

#[test]
fn test_encode_decode_roundtrip() {
    let mut rt = JSRuntime::new();
    let mut ctx = rt.new_context();

    let result = eval(
        &mut ctx,
        "decodeURIComponent(encodeURIComponent('test 123 !@#$%'))",
    )
    .unwrap();
    assert!(result.is_string());
    let atom = result.get_atom();
    assert_eq!(ctx.get_atom_str(atom), "test 123 !@#$%");
}

#[test]
fn test_encodeuri_undefined() {
    let mut rt = JSRuntime::new();
    let mut ctx = rt.new_context();

    let result = eval(&mut ctx, "encodeURI(undefined)").unwrap();
    assert!(result.is_string());
    let atom = result.get_atom();
    assert_eq!(ctx.get_atom_str(atom), "undefined");
}

#[test]
fn test_encodeuri_null() {
    let mut rt = JSRuntime::new();
    let mut ctx = rt.new_context();

    let result = eval(&mut ctx, "encodeURI(null)").unwrap();
    assert!(result.is_string());
    let atom = result.get_atom();
    assert_eq!(ctx.get_atom_str(atom), "null");
}

#[test]
fn test_arraybuffer_constructor() {
    let mut rt = JSRuntime::new();
    let mut ctx = rt.new_context();

    let result = eval(&mut ctx, "typeof ArrayBuffer").unwrap();
    assert!(result.is_string());
    let atom = result.get_atom();
    assert_eq!(ctx.get_atom_str(atom), "function");
}

#[test]
fn test_arraybuffer_instance() {
    let mut rt = JSRuntime::new();
    let mut ctx = rt.new_context();

    let result = eval(&mut ctx, "var buf = new ArrayBuffer(8); buf.byteLength").unwrap();
    assert!(result.is_int());
    assert_eq!(result.get_int(), 8);
}

#[test]
fn test_uint8array_constructor() {
    let mut rt = JSRuntime::new();
    let mut ctx = rt.new_context();

    let result = eval(&mut ctx, "typeof Uint8Array").unwrap();
    assert!(result.is_string());
    let atom = result.get_atom();
    assert_eq!(ctx.get_atom_str(atom), "function");
}

#[test]
fn test_uint8array_from_length() {
    let mut rt = JSRuntime::new();
    let mut ctx = rt.new_context();

    let result = eval(&mut ctx, "var arr = new Uint8Array(4); arr.length").unwrap();
    assert!(result.is_int());
    assert_eq!(result.get_int(), 4);
}

#[test]
fn test_uint8array_from_arraybuffer() {
    let mut rt = JSRuntime::new();
    let mut ctx = rt.new_context();

    let code = r#"
        var buf = new ArrayBuffer(8);
        var arr = new Uint8Array(buf);
        arr.length
    "#;
    let result = eval(&mut ctx, code).unwrap();
    assert!(result.is_int());
    assert_eq!(result.get_int(), 8);
}

#[test]
fn test_typedarray_kinds_exist() {
    let mut rt = JSRuntime::new();
    let mut ctx = rt.new_context();

    let kinds = [
        "Int8Array",
        "Uint8Array",
        "Uint8ClampedArray",
        "Int16Array",
        "Uint16Array",
        "Int32Array",
        "Uint32Array",
        "Float32Array",
        "Float64Array",
        "BigInt64Array",
        "BigUint64Array",
    ];

    for kind in &kinds {
        let code = format!("typeof {}", kind);
        let result = eval(&mut ctx, &code).unwrap();
        assert!(result.is_string());
        let atom = result.get_atom();
        assert_eq!(
            ctx.get_atom_str(atom),
            "function",
            "{} should be a function",
            kind
        );
    }
}

#[test]
fn test_dataview_constructor() {
    let mut rt = JSRuntime::new();
    let mut ctx = rt.new_context();

    let result = eval(&mut ctx, "typeof DataView").unwrap();
    assert!(result.is_string());
    let atom = result.get_atom();
    assert_eq!(ctx.get_atom_str(atom), "function");
}

#[test]
fn test_dataview_instance() {
    let mut rt = JSRuntime::new();
    let mut ctx = rt.new_context();

    let code = r#"
        var buf = new ArrayBuffer(8);
        var view = new DataView(buf);
        view.byteLength
    "#;
    let result = eval(&mut ctx, code).unwrap();
    assert!(result.is_int());
    assert_eq!(result.get_int(), 8);
}

#[test]
fn test_dataview_with_offset() {
    let mut rt = JSRuntime::new();
    let mut ctx = rt.new_context();

    let code = r#"
        var buf = new ArrayBuffer(16);
        var view = new DataView(buf, 4);
        view.byteOffset
    "#;
    let result = eval(&mut ctx, code).unwrap();
    assert!(result.is_int());
    assert_eq!(result.get_int(), 4);
}

#[test]
fn test_typedarray_byte_length() {
    let mut rt = JSRuntime::new();
    let mut ctx = rt.new_context();

    let code = r#"
        var arr = new Uint32Array(4);
        arr.byteLength
    "#;
    let result = eval(&mut ctx, code).unwrap();
    assert!(result.is_int());
    
    assert_eq!(result.get_int(), 16);
}

#[test]
fn test_typedarray_byte_offset() {
    let mut rt = JSRuntime::new();
    let mut ctx = rt.new_context();

    let code = r#"
        var buf = new ArrayBuffer(16);
        var arr = new Uint8Array(buf, 4);
        arr.byteOffset
    "#;
    let result = eval(&mut ctx, code).unwrap();
    assert!(result.is_int());
    assert_eq!(result.get_int(), 4);
}
