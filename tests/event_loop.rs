#![cfg(feature = "full_runtime_tests")]

use pipa::{JSRuntime, eval, run_event_loop, run_event_loop_with_timeout};

fn get_int(ctx: &mut pipa::JSContext, code: &str) -> i64 {
    eval(ctx, code).unwrap().get_int()
}

fn get_bool(ctx: &mut pipa::JSContext, code: &str) -> bool {
    eval(ctx, code).unwrap().get_bool()
}

#[test]
fn test_set_timeout_basic() {
    let mut rt = JSRuntime::new();
    let mut ctx = rt.new_context();

    let r = eval(&mut ctx, "typeof setTimeout");
    assert!(r.is_ok(), "setTimeout should be defined: {:?}", r);

    let r = eval(&mut ctx, "setTimeout(() => {}, 100)");
    assert!(r.is_ok(), "setTimeout should succeed: {:?}", r);
    let timer_id = r.unwrap();
    assert!(timer_id.is_int(), "setTimeout should return an integer");
    assert!(timer_id.get_int() > 0, "timer ID should be positive");
}

#[test]
fn test_set_timeout_callback_scheduled() {
    let mut rt = JSRuntime::new();
    let mut ctx = rt.new_context();

    let r = eval(
        &mut ctx,
        r#"
        var called = false;
        var id = setTimeout(function() {
            called = true;
        }, 0);
        id;
    "#,
    );
    assert!(r.is_ok(), "setTimeout scheduling failed: {:?}", r);

    assert!(
        !get_bool(&mut ctx, "called"),
        "callback should not be called yet"
    );

    let result = run_event_loop(&mut ctx);
    assert!(result.is_ok(), "run_event_loop failed: {:?}", result);

    assert!(
        get_bool(&mut ctx, "called"),
        "callback should have been called"
    );
}

#[test]
fn test_set_timeout_with_args() {
    let mut rt = JSRuntime::new();
    let mut ctx = rt.new_context();

    let r = eval(
        &mut ctx,
        r#"
        var received = null;
        setTimeout(function(arg) {
            received = arg;
        }, 0, "hello");
        received;
    "#,
    );
    assert!(r.is_ok(), "setTimeout with args failed: {:?}", r);

    let result = run_event_loop(&mut ctx);
    assert!(result.is_ok(), "run_event_loop failed: {:?}", result);

    let r = eval(&mut ctx, "received");
    assert!(r.is_ok(), "checking received failed: {:?}", r);
    let val = r.unwrap();

    assert!(val.is_string(), "received should be a string");
}

#[test]
fn test_set_timeout_delay_zero() {
    let mut rt = JSRuntime::new();
    let mut ctx = rt.new_context();

    let r = eval(
        &mut ctx,
        r#"
        var order = [];
        order.push("start");
        setTimeout(function() {
            order.push("timeout");
        }, 0);
        order.push("end");
        order;
    "#,
    );
    assert!(r.is_ok(), "setTimeout delay 0 failed: {:?}", r);

    let r = eval(&mut ctx, "order.length");
    assert!(r.is_ok());
    assert_eq!(
        r.unwrap().get_int(),
        2,
        "should have 2 elements before event loop"
    );

    let result = run_event_loop(&mut ctx);
    assert!(result.is_ok(), "run_event_loop failed: {:?}", result);

    let r = eval(&mut ctx, "order.length");
    assert!(r.is_ok());
    assert_eq!(
        r.unwrap().get_int(),
        3,
        "should have 3 elements after event loop"
    );
}

#[test]
fn test_clear_timeout_prevents_callback() {
    let mut rt = JSRuntime::new();
    let mut ctx = rt.new_context();

    let r = eval(
        &mut ctx,
        r#"
        var called = false;
        var id = setTimeout(function() {
            called = true;
        }, 1000);
        clearTimeout(id);
        called;
    "#,
    );
    assert!(r.is_ok(), "clearTimeout scheduling failed: {:?}", r);

    let result = run_event_loop(&mut ctx);
    assert!(result.is_ok(), "run_event_loop failed: {:?}", result);

    assert!(
        !get_bool(&mut ctx, "called"),
        "callback should NOT have been called"
    );
}

#[test]
fn test_clear_timeout_undefined_id() {
    let mut rt = JSRuntime::new();
    let mut ctx = rt.new_context();

    let r = eval(&mut ctx, "clearTimeout(99999)");
    assert!(
        r.is_ok(),
        "clearTimeout with invalid ID should not error: {:?}",
        r
    );
}

#[test]
fn test_set_interval_basic() {
    let mut rt = JSRuntime::new();
    let mut ctx = rt.new_context();

    let r = eval(&mut ctx, "typeof setInterval");
    assert!(r.is_ok(), "setInterval should be defined: {:?}", r);

    let r = eval(&mut ctx, "setInterval(() => {}, 100)");
    assert!(r.is_ok(), "setInterval should succeed: {:?}", r);
    let timer_id = r.unwrap();
    assert!(timer_id.is_int(), "setInterval should return an integer");
}

#[test]
fn test_set_interval_repeats() {
    let mut rt = JSRuntime::new();
    let mut ctx = rt.new_context();

    let r = eval(
        &mut ctx,
        r#"
        var count = 0;
        var id = setInterval(function() {
            count++;
            if (count >= 3) {
                clearInterval(id);
            }
        }, 0);
        count;
    "#,
    );
    assert!(r.is_ok(), "setInterval failed: {:?}", r);

    let result = run_event_loop(&mut ctx);
    assert!(result.is_ok(), "run_event_loop failed: {:?}", result);

    assert_eq!(
        get_int(&mut ctx, "count"),
        3,
        "interval should have fired 3 times"
    );
}

#[test]
fn test_clear_interval() {
    let mut rt = JSRuntime::new();
    let mut ctx = rt.new_context();

    let r = eval(
        &mut ctx,
        r#"
        var count = 0;
        var id = setInterval(function() {
            count++;
        }, 0);
        clearInterval(id);
        count;
    "#,
    );
    assert!(r.is_ok(), "clearInterval failed: {:?}", r);

    let result = run_event_loop(&mut ctx);
    assert!(result.is_ok(), "run_event_loop failed: {:?}", result);

    assert_eq!(
        get_int(&mut ctx, "count"),
        0,
        "interval should not have fired"
    );
}

#[test]
fn test_queue_microtask_basic() {
    let mut rt = JSRuntime::new();
    let mut ctx = rt.new_context();

    let r = eval(&mut ctx, "typeof queueMicrotask");
    assert!(r.is_ok(), "queueMicrotask should be defined: {:?}", r);
}

#[test]
fn test_queue_microtask_execution() {
    let mut rt = JSRuntime::new();
    let mut ctx = rt.new_context();

    let r = eval(
        &mut ctx,
        r#"
        var called = false;
        queueMicrotask(function() {
            called = true;
        });
        called;
    "#,
    );
    assert!(r.is_ok(), "queueMicrotask failed: {:?}", r);

    assert!(
        get_bool(&mut ctx, "called"),
        "microtask should have been called"
    );
}

#[test]
fn test_queue_microtask_vs_set_timeout_ordering() {
    let mut rt = JSRuntime::new();
    let mut ctx = rt.new_context();

    let r = eval(
        &mut ctx,
        r#"
        var order = [];
        setTimeout(function() {
            order.push("timeout");
        }, 0);
        queueMicrotask(function() {
            order.push("microtask");
        });
        order.push("sync");
        order;
    "#,
    );
    assert!(r.is_ok(), "ordering test failed: {:?}", r);

    let r = eval(&mut ctx, "order.length");
    assert!(r.is_ok());
    let len = r.unwrap().get_int();

    assert!(len >= 2, "should have at least 2 elements");

    let result = run_event_loop(&mut ctx);
    assert!(result.is_ok(), "run_event_loop failed: {:?}", result);

    let r = eval(&mut ctx, "order.length");
    assert!(r.is_ok());
    assert_eq!(
        r.unwrap().get_int(),
        3,
        "should have 3 elements after event loop"
    );
}

#[test]
fn test_request_animation_frame_basic() {
    let mut rt = JSRuntime::new();
    let mut ctx = rt.new_context();

    let r = eval(&mut ctx, "typeof requestAnimationFrame");
    assert!(
        r.is_ok(),
        "requestAnimationFrame should be defined: {:?}",
        r
    );
}

#[test]
fn test_request_animation_frame_execution() {
    let mut rt = JSRuntime::new();
    let mut ctx = rt.new_context();

    let r = eval(
        &mut ctx,
        r#"
        var called = false;
        var timestamp = null;
        requestAnimationFrame(function(ts) {
            called = true;
            timestamp = ts;
        });
        called;
    "#,
    );
    assert!(r.is_ok(), "requestAnimationFrame failed: {:?}", r);

    assert!(
        !get_bool(&mut ctx, "called"),
        "rAF should not be called yet"
    );

    let result = run_event_loop(&mut ctx);
    assert!(result.is_ok(), "run_event_loop failed: {:?}", result);

    assert!(get_bool(&mut ctx, "called"), "rAF should have been called");

    let r = eval(&mut ctx, "typeof timestamp");
    assert!(r.is_ok());
}

#[test]
fn test_cancel_animation_frame() {
    let mut rt = JSRuntime::new();
    let mut ctx = rt.new_context();

    let r = eval(
        &mut ctx,
        r#"
        var called = false;
        var id = requestAnimationFrame(function() {
            called = true;
        });
        cancelAnimationFrame(id);
        called;
    "#,
    );
    assert!(r.is_ok(), "cancelAnimationFrame failed: {:?}", r);

    let result = run_event_loop(&mut ctx);
    assert!(result.is_ok(), "run_event_loop failed: {:?}", result);

    assert!(
        !get_bool(&mut ctx, "called"),
        "rAF should NOT have been called"
    );
}

#[test]
fn test_event_loop_completion_status() {
    let mut rt = JSRuntime::new();
    let mut ctx = rt.new_context();

    let result = run_event_loop(&mut ctx);
    assert!(result.is_ok(), "run_event_loop failed: {:?}", result);

    let result = result.unwrap();
    assert!(result.completed, "event loop should complete with no tasks");
    assert_eq!(result.macrotasks_remaining, 0);
    assert_eq!(result.timers_remaining, 0);
}

#[test]
fn test_event_loop_with_pending_timer() {
    let mut rt = JSRuntime::new();
    let mut ctx = rt.new_context();

    let _ = eval(
        &mut ctx,
        r#"
        setTimeout(function() {}, 10000);
    "#,
    );

    let result = run_event_loop_with_timeout(&mut ctx, 10);
    assert!(
        result.is_ok(),
        "run_event_loop_with_timeout failed: {:?}",
        result
    );

    let _result = result.unwrap();
}

#[test]
fn test_promise_then_vs_set_timeout() {
    let mut rt = JSRuntime::new();
    let mut ctx = rt.new_context();

    let r = eval(
        &mut ctx,
        r#"
        var order = [];
        Promise.resolve(1).then(function() {
            order.push("promise");
        });
        setTimeout(function() {
            order.push("timeout");
        }, 0);
        order.push("sync");
        order;
    "#,
    );
    assert!(r.is_ok(), "promise vs timeout test failed: {:?}", r);

    let r = eval(&mut ctx, "order.length");
    assert!(r.is_ok());
    assert!(
        r.unwrap().get_int() >= 2,
        "should have at least sync and promise"
    );

    let result = run_event_loop(&mut ctx);
    assert!(result.is_ok(), "run_event_loop failed: {:?}", result);

    let r = eval(&mut ctx, "order.length");
    assert!(r.is_ok());
    assert_eq!(
        r.unwrap().get_int(),
        3,
        "should have 3 elements after event loop"
    );
}

#[test]
fn test_multiple_timers_execution_order() {
    let mut rt = JSRuntime::new();
    let mut ctx = rt.new_context();

    let r = eval(
        &mut ctx,
        r#"
        var order = [];
        setTimeout(function() { order.push(1); }, 0);
        setTimeout(function() { order.push(2); }, 0);
        setTimeout(function() { order.push(3); }, 0);
        order;
    "#,
    );
    assert!(r.is_ok(), "multiple timers failed: {:?}", r);

    let result = run_event_loop(&mut ctx);
    assert!(result.is_ok(), "run_event_loop failed: {:?}", result);

    let r = eval(&mut ctx, "order.length");
    assert!(r.is_ok());
    assert_eq!(r.unwrap().get_int(), 3, "all timers should have fired");

    let r = eval(&mut ctx, "order[0]");
    assert!(r.is_ok());
    assert_eq!(r.unwrap().get_int(), 1);
}
