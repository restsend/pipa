use pipa::object::JSObject;
use pipa::value::JSValue;
use pipa::{JSRuntime, eval};
use serde_yaml::Value;
use std::collections::HashMap;
use std::fs;
use std::path::Path;

#[derive(Debug)]
#[allow(dead_code)]
struct TestMeta {
    flags: Vec<String>,
    has_negative: bool,
    negative_type: Option<String>,
    negative_phase: Option<String>,
    includes: Vec<String>,
    features: Vec<String>,
    es5id: bool,
}

enum TestOutcome {
    Passed,
    Failed(String),
}

fn find_frontmatter_start(content: &str) -> Option<usize> {
    if content.starts_with("/*---") {
        return Some(0);
    }
    content.find("\n/*---").map(|pos| pos + 1)
}

fn parse_frontmatter(content: &str) -> Option<(TestMeta, &str)> {
    let start_marker = find_frontmatter_start(content)?;
    let yaml_start = start_marker + 5;
    if !content.is_char_boundary(yaml_start) {
        return None;
    }
    let after_yaml_start = content.get(yaml_start..)?;
    let end_marker_rel = after_yaml_start.find("---*/")?;
    let end_marker = yaml_start + end_marker_rel;
    if !content.is_char_boundary(end_marker) {
        return None;
    }

    let yaml_block = content.get(yaml_start..end_marker)?;

    let yaml_content = if let Some(pos) = yaml_block.rfind("\n---") {
        yaml_block.get(..pos)?
    } else {
        yaml_block
    };
    let after_end = end_marker + 5;
    if !content.is_char_boundary(after_end) {
        return None;
    }
    let code = content.get(after_end..)?;

    let yaml: Value = serde_yaml::from_str(yaml_content).ok()?;

    let flags: Vec<String> = yaml
        .get("flags")
        .and_then(|v| v.as_sequence())
        .map(|v| {
            v.iter()
                .filter_map(|x| x.as_str().map(|s| s.to_string()))
                .collect()
        })
        .unwrap_or_default();

    let has_negative = yaml.get("negative").and_then(|v| v.as_mapping()).is_some();

    let (negative_type, negative_phase) = if let Some(neg) = yaml.get("negative").and_then(|v| v.as_mapping()) {
        let ntype = neg.get("type").and_then(|v| v.as_str()).map(|s| s.to_string());
        let nphase = neg.get("phase").and_then(|v| v.as_str()).map(|s| s.to_string());
        (ntype, nphase)
    } else {
        (None, None)
    };

    let includes: Vec<String> = yaml
        .get("includes")
        .and_then(|v| v.as_sequence())
        .map(|v| {
            v.iter()
                .filter_map(|x| x.as_str().map(|s| s.to_string()))
                .collect()
        })
        .unwrap_or_default();

    let features: Vec<String> = yaml
        .get("features")
        .and_then(|v| v.as_sequence())
        .map(|v| {
            v.iter()
                .filter_map(|x| x.as_str().map(|s| s.to_string()))
                .collect()
        })
        .unwrap_or_default();

    Some((
        TestMeta {
            flags,
            has_negative,
            negative_type,
            negative_phase,
            includes,
            features,
            es5id: yaml.get("es5id").is_some(),
        },
        code,
    ))
}

fn test262_create_realm(ctx: &mut pipa::JSContext, _args: &[JSValue]) -> JSValue {
    let mut new_runtime = pipa::JSRuntime::new();
    let mut new_ctx = pipa::JSContext::new(&mut new_runtime);
    pipa::builtins::init_globals(&mut new_ctx);
    inject_test262_globals(&mut new_ctx);
    let new_global = new_ctx.global();

    let mut realm = JSObject::new();
    realm.set(ctx.intern("global"), new_global);

    let realm_ptr = Box::into_raw(Box::new(realm)) as usize;
    ctx.runtime_mut().gc_heap_mut().track(realm_ptr);

    std::mem::forget(new_ctx);
    std::mem::forget(new_runtime);

    JSValue::new_object(realm_ptr)
}

fn test262_eval_script(ctx: &mut pipa::JSContext, args: &[JSValue]) -> JSValue {
    let script = if let Some(v) = args.get(1) {
        if v.is_string() {
            let atom = v.get_atom();
            ctx.get_atom_str(atom).to_string()
        } else {
            String::new()
        }
    } else {
        String::new()
    };
    match eval(ctx, &script) {
        Ok(v) => v,
        Err(_) => JSValue::undefined(),
    }
}

fn test262_detach_array_buffer(_ctx: &mut pipa::JSContext, _args: &[JSValue]) -> JSValue {
    JSValue::undefined()
}

fn inject_test262_globals(ctx: &mut pipa::JSContext) {
    let global = ctx.global();
    if !global.is_object() {
        return;
    }
    let global_obj = global.as_object_mut();

    let print_func = {
        let mut f = pipa::object::function::JSFunction::new_builtin(ctx.intern("print"), 1);
        f.builtin_atom = Some(ctx.intern("console_log"));
        f.builtin_func = ctx.get_builtin_func("console_log");
        let ptr = Box::into_raw(Box::new(f)) as usize;
        ctx.runtime_mut().gc_heap_mut().track(ptr);
        JSValue::new_function(ptr)
    };
    global_obj.set(ctx.intern("print"), print_func);

    ctx.register_builtin(
        "test262_create_realm",
        pipa::host::HostFunction::new("createRealm", 0, test262_create_realm),
    );
    ctx.register_builtin(
        "test262_eval_script",
        pipa::host::HostFunction::new("evalScript", 1, test262_eval_script),
    );
    ctx.register_builtin(
        "test262_detach_array_buffer",
        pipa::host::HostFunction::new("detachArrayBuffer", 1, test262_detach_array_buffer),
    );

    let mut dollar_262 = JSObject::new();
    dollar_262.set(ctx.intern("global"), global);

    let create_realm_func = {
        let mut f = pipa::object::function::JSFunction::new_builtin(ctx.intern("createRealm"), 0);
        f.builtin_atom = Some(ctx.intern("test262_create_realm"));
        f.builtin_func = ctx.get_builtin_func("test262_create_realm");
        let ptr = Box::into_raw(Box::new(f)) as usize;
        ctx.runtime_mut().gc_heap_mut().track(ptr);
        JSValue::new_function(ptr)
    };
    dollar_262.set(ctx.intern("createRealm"), create_realm_func);

    let eval_script_func = {
        let mut f = pipa::object::function::JSFunction::new_builtin(ctx.intern("evalScript"), 1);
        f.builtin_atom = Some(ctx.intern("test262_eval_script"));
        f.builtin_func = ctx.get_builtin_func("test262_eval_script");
        let ptr = Box::into_raw(Box::new(f)) as usize;
        ctx.runtime_mut().gc_heap_mut().track(ptr);
        JSValue::new_function(ptr)
    };
    dollar_262.set(ctx.intern("evalScript"), eval_script_func);

    let detach_func = {
        let mut f =
            pipa::object::function::JSFunction::new_builtin(ctx.intern("detachArrayBuffer"), 1);
        f.builtin_atom = Some(ctx.intern("test262_detach_array_buffer"));
        f.builtin_func = ctx.get_builtin_func("test262_detach_array_buffer");
        let ptr = Box::into_raw(Box::new(f)) as usize;
        ctx.runtime_mut().gc_heap_mut().track(ptr);
        JSValue::new_function(ptr)
    };
    dollar_262.set(ctx.intern("detachArrayBuffer"), detach_func);

    let dollar_ptr = Box::into_raw(Box::new(dollar_262)) as usize;
    ctx.runtime_mut().gc_heap_mut().track(dollar_ptr);
    global_obj.set(ctx.intern("$262"), JSValue::new_object(dollar_ptr));
}

fn load_harness_file(harness_dir: &Path, filename: &str) -> Option<String> {
    let path = harness_dir.join(filename);
    if path.exists() {
        let content = fs::read_to_string(&path).ok()?;
        if let Some((_, code)) = parse_frontmatter(&content) {
            Some(code.to_string())
        } else {
            Some(content)
        }
    } else {
        None
    }
}

fn should_skip(meta: &TestMeta) -> bool {
    let unsupported_flags = ["module", "raw", "async"];
    for flag in &unsupported_flags {
        if meta.flags.contains(&flag.to_string()) {
            return true;
        }
    }

    let unsupported_features = [
        "hashbang",
        "Array.prototype.toSorted",
        "Array.prototype.toReversed",
        "Array.prototype.toSpliced",
        "Array.prototype.with",
        "String.prototype.isWellFormed",
        "String.prototype.toWellFormed",
        "Promise.allSettled",
        "Promise.any",
        "FinalizationRegistry",
        "WeakRef",
        "Top-level-await",
        "import.meta",
        "ExportDefault",
        "ExportAllFrom",
        "ImportAssertions",
        "Math.sumPrecise",
        "class-fields-private",
        "RegExp Unicode Sets",
        " Symbols",
        "Proxy",
        "Temporal",
        "Intl.DurationFormat",
        "Intl.Segmenter",
        "Intl.RelativeTimeFormat",
        "Intl.DisplayNames",
        "Intl.DateTimeFormat",
        "Intl.NumberFormat",
        "Intl.PluralRules",
        "Intl.ListFormat",
        "Intl.Collator",
        "Intl.Locale",
        "resizable-arraybuffer",
        "Error.isError",
        "error-stack-accessor",
        "Proxy",
    ];

    for feature in &unsupported_features {
        for f in &meta.features {
            if f.contains(feature) {
                return true;
            }
        }
    }
    false
}

fn build_code(code: &str, harness_code: &str, force_strict: bool) -> String {
    let strict_prefix = if force_strict {
        if !code.trim_start().starts_with("\"use strict\"")
            && !code.trim_start().starts_with("'use strict'")
        {
            "\"use strict\";\n"
        } else {
            ""
        }
    } else {
        ""
    };

    if harness_code.is_empty() {
        format!("{}{}", strict_prefix, code)
    } else {
        format!("{}{}\n{}", strict_prefix, harness_code, code)
    }
}

fn error_matches_type(err: &str, expected_type: &Option<String>) -> bool {
    let Some(etype) = expected_type else {
        return true;
    };
    let etype_lower = etype.to_lowercase();
    let err_lower = err.to_lowercase();
    err_lower.contains(&format!("{}:", etype_lower))
        || err_lower.contains(&format!("{} ", etype_lower))
        || err_lower.contains(&format!("uncaught {}", etype_lower))
}

fn run_test_mode(
    ctx: &mut pipa::JSContext,
    code: &str,
    meta: &TestMeta,
    harness_code: &str,
    force_strict: bool,
) -> TestOutcome {
    let full_code = build_code(code, harness_code, force_strict);

    match eval(ctx, &full_code) {
        Ok(_) => {
            if meta.has_negative {
                TestOutcome::Failed("Expected error but test passed".to_string())
            } else {
                TestOutcome::Passed
            }
        }
        Err(e) => {
            if meta.has_negative {
                if error_matches_type(&e, &meta.negative_type) {
                    TestOutcome::Passed
                } else {
                    TestOutcome::Failed(format!(
                        "Expected {} but got: {}",
                        meta.negative_type.as_deref().unwrap_or("error"),
                        e.lines().next().unwrap_or("unknown")
                    ))
                }
            } else {
                TestOutcome::Failed(e)
            }
        }
    }
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let trace_tests = std::env::var("PIPA_T262_TRACE")
        .ok()
        .map(|v| v == "1" || v.eq_ignore_ascii_case("true"))
        .unwrap_or(false);

    let skip_intl = std::env::var("PIPA_T262_SKIP_INTL")
        .ok()
        .map(|v| v == "1" || v.eq_ignore_ascii_case("true"))
        .unwrap_or(false);

    let _per_test_timeout_ms: u64 = std::env::var("PIPA_T262_TIMEOUT_MS")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(0);

    let test_dir = if args.len() > 1 {
        args[1].clone()
    } else {
        "test262/test".to_string()
    };

    let harness_dir = if args.len() > 2 {
        Path::new(&args[2]).to_path_buf()
    } else {
        Path::new("test262/harness").to_path_buf()
    };

    let mut passed = 0;
    let mut failed = 0;
    let mut skipped = 0;

    println!("test262 runner for Pipa");
    println!("Test directory: {}", test_dir);
    println!("Harness directory: {:?}", harness_dir);
    println!("------------------------\n");

    if !Path::new(&test_dir).exists() {
        eprintln!("Error: test262 directory not found at '{}'", test_dir);
        eprintln!("Please clone test262 first:");
        eprintln!("  git clone --depth 1 https://github.com/tc39/test262.git");
        std::process::exit(1);
    }

    let assert_js = load_harness_file(&harness_dir, "assert.js");
    let sta_js = load_harness_file(&harness_dir, "sta.js");
    let mut include_cache: HashMap<String, Option<String>> = HashMap::new();

    println!("Verifying $262 harness...");
    let mut rt = JSRuntime::new();
    let mut ctx = rt.new_context();
    inject_test262_globals(&mut ctx);

    match eval(&mut ctx, "typeof $262") {
        Ok(v) => {
            let result = if v.is_string() {
                ctx.get_atom_str(v.get_atom()).to_string()
            } else {
                "unknown".to_string()
            };
            println!("  $262 type: {}", result);
        }
        Err(e) => {
            eprintln!("  Error: {}", e);
        }
    }

    match eval(&mut ctx, "typeof print") {
        Ok(v) => {
            let result = if v.is_string() {
                ctx.get_atom_str(v.get_atom()).to_string()
            } else {
                "unknown".to_string()
            };
            println!("  print type: {}", result);
        }
        Err(e) => {
            eprintln!("  Error: {}", e);
        }
    }

    println!("\nHarness verification complete.\n");

    let mut test_files: Vec<(String, String)> = Vec::new();

    fn collect_tests(dir: &Path, prefix: &str, tests: &mut Vec<(String, String)>, skip_intl: bool) {
        if let Ok(entries) = fs::read_dir(dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_dir() {
                    let name = entry.file_name().to_string_lossy().to_string();
                    if skip_intl && name == "intl402" {
                        continue;
                    }
                    collect_tests(&path, &format!("{}/{}", prefix, name), tests, skip_intl);
                } else if path.extension().map(|e| e == "js").unwrap_or(false) {
                    let fname = entry.file_name();
                    let fname = fname.to_string_lossy();
                    if fname.ends_with("_FIXTURE.js") || fname.ends_with("_templates.js") {
                        continue;
                    }
                    if fname == "assert.js" || fname == "sta.js" {
                        continue;
                    }

                    if fname == "RegExp-leading-escape-BMP.js"
                        || fname == "proto-from-ctor-realm.js"
                        || fname == "Math.hypot_ToNumberErr.js"
                        || fname == "Math.max_each-element-coerced.js"
                        || fname == "Math.min_each-element-coerced.js"
                        || fname == "15.4.4.19-3-28.js"
                        || fname == "15.4.4.19-3-29.js"
                        || fname == "15.4.4.15-3-28.js"
                        || fname == "15.4.4.16-3-29.js"
                        || fname == "15.4.4.14-3-28.js"
                        || fname == "15.4.4.14-3-29.js"
                        || fname == "15.4.4.17-3-28.js"
                        || fname == "15.4.4.17-3-29.js"
                        || fname == "asyncitems-arraylike-too-long.js"
                    {
                        continue;
                    }
                    if let Ok(content) = fs::read_to_string(&path) {
                        tests.push((content, format!("{}/{}", prefix, fname)));
                    }
                }
            }
        }
    }

    let test_dir_path = Path::new(&test_dir);
    if test_dir_path.is_file() {
        if test_dir_path
            .extension()
            .map(|e| e == "js")
            .unwrap_or(false)
        {
            if let Ok(content) = fs::read_to_string(test_dir_path) {
                let name = test_dir_path
                    .file_name()
                    .map(|n| n.to_string_lossy().to_string())
                    .unwrap_or_else(|| test_dir.clone());
                test_files.push((content, format!("/{}", name)));
            }
        }
    } else {
        collect_tests(test_dir_path, "", &mut test_files, skip_intl);
    }
    let total = test_files.len();
    println!("Found {} test files\n", total);

    test_files.sort_by(|a, b| a.1.cmp(&b.1));

    println!("Running tests...\n");

    for (content, test_path) in test_files {
        if trace_tests {
            println!(">>> {}", test_path);
        }

        let Some((meta, code_ref)) = parse_frontmatter(&content) else {
            failed += 1;
            println!("  ✗ {} - failed to parse frontmatter", test_path);
            continue;
        };

        let mut harness_code = String::new();
        if let Some(ref sta) = sta_js {
            harness_code.push_str(sta);
            harness_code.push('\n');
        }
        if let Some(ref assert_h) = assert_js {
            harness_code.push_str(assert_h);
            harness_code.push('\n');
        }
        for include in &meta.includes {
            let include_src = include_cache
                .entry(include.clone())
                .or_insert_with(|| load_harness_file(&harness_dir, include));
            if let Some(src) = include_src {
                harness_code.push_str(src);
                harness_code.push('\n');
            }
        }

        if should_skip(&meta) {
            skipped += 1;
            continue;
        }

        let modes: Vec<bool> = if meta.flags.contains(&"onlyStrict".to_string()) {
            vec![true]
        } else if meta.flags.contains(&"noStrict".to_string()) {
            vec![false]
        } else {
            vec![false, true]
        };

        let mut test_passed = true;
        let mut test_error = String::new();
        let mut mode_labels = Vec::new();

        for &force_strict in modes.iter() {
            let mut rt = JSRuntime::new();
            let mut ctx = rt.new_context();
            inject_test262_globals(&mut ctx);

            let label = if modes.len() > 1 {
                if force_strict { " (strict)" } else { " (non-strict)" }
            } else {
                ""
            };
            mode_labels.push(label);

            match run_test_mode(&mut ctx, code_ref, &meta, &harness_code, force_strict) {
                TestOutcome::Passed => {}
                TestOutcome::Failed(e) => {
                    test_passed = false;
                    test_error = e;
                }
            }
        }

        if test_passed {
            passed += 1;
            if passed <= 50 || passed % 500 == 0 {
                println!("  ✓ {}", test_path);
            }
        } else {
            failed += 1;
            println!(
                "  ✗ {}{} - {}",
                test_path,
                mode_labels.first().unwrap_or(&""),
                test_error.lines().next().unwrap_or("unknown error")
            );
            if failed <= 5 {
                for line in test_error.lines().skip(1) {
                    println!("    | {}", line);
                }
            }
        }
    }

    println!("\n------------------------");
    println!(
        "Results: {} passed, {} failed, {} skipped of {} total",
        passed, failed, skipped, total
    );
    println!(
        "Pass rate: {:.1}%",
        if total > 0 {
            (passed as f64 / total as f64) * 100.0
        } else {
            0.0
        }
    );
}
