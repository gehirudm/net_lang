use netlang::{
    ast::HttpMethod,
    interpreter::{ExecutionError, Limits, execute, execute_with_limits},
    lexer::Lexer,
    parser::Parser,
    runtime::{Runtime, StandardRuntime, Value},
    semantic::{SemanticErrorKind, analyze},
};
use std::{
    sync::{Arc, Condvar, Mutex},
    time::{Duration, Instant},
};

fn parse(source: &str) -> netlang::ast::Program {
    Parser::new(Lexer::new(source).unwrap().tokenize().unwrap())
        .unwrap()
        .parse_program()
        .unwrap()
}
fn run(source: &str) -> (Result<Value, ExecutionError>, String) {
    let mut output = Vec::new();
    let result = execute(&parse(source), &mut StandardRuntime::new(&mut output));
    (result, String::from_utf8(output).unwrap())
}

#[test]
fn workers_can_mutate_locals_but_parent_values_remain_unchanged() {
    let (result, output) = run(r#"
        let input = {values: [1, 2]};
        parallel n in [1, 2, 3] {
            let copy = input; copy.values[0] = n;
            n = n + 10;
            fn local(x) { x = x + 1; return x; }
            print(copy.values[0], local(n));
        }
        print(input.values[0]);
    "#);
    result.unwrap();
    assert_eq!(output, "1 12\n2 13\n3 14\n1\n");
}

#[test]
fn direct_captured_writes_are_semantic_errors_before_any_effect() {
    for source in [
        "print(1); let x = 0; parallel n in [1] { x = n; }",
        "let x = {a: 0}; parallel n in [1] { x.a = n; }",
        "let x = [0]; parallel n in [1] { x[0] = n; }",
        "parallel n in [1] { parallel m in [1] { n = m; } }",
        "let x = 0; parallel n in [1] { fn f() { x = 1; } f(); }",
    ] {
        let (result, output) = run(source);
        let ExecutionError::Semantic(errors) = result.unwrap_err() else {
            panic!("{source}")
        };
        assert!(
            errors
                .iter()
                .any(|e| e.kind == SemanticErrorKind::CapturedAssignment)
        );
        assert!(output.is_empty());
    }
}

#[test]
fn indirect_capture_mutation_is_rejected_at_runtime() {
    for source in [
        "let x = 0; fn change() { x = 1; } parallel n in [1] { change(); }",
        "let x = {a: 0}; fn change() { x.a = 1; } let f = change; parallel n in [1] { f(); }",
        "fn counter() { let x = 0; fn next() { x = x + 1; } return next; } let f = counter(); parallel n in [1] { f(); }",
    ] {
        let program = parse(source);
        analyze(&program).unwrap();
        let ExecutionError::Runtime(error) =
            execute(&program, &mut StandardRuntime::new(Vec::new())).unwrap_err()
        else {
            panic!()
        };
        assert!(error.message.contains("read-only parallel capture"));
        assert!(error.message.contains("iteration 1"));
    }
    // A function created inside the worker can mutate that worker's locals.
    let (result, output) = run(
        "parallel n in [1, 2] { let x = n; fn next() { x = x + 1; return x; } print(next()); }",
    );
    result.unwrap();
    assert_eq!(output, "2\n3\n");
}

#[test]
fn returns_cannot_cross_a_worker_boundary_and_nested_loops_are_isolated() {
    let errors = analyze(&parse("fn main() { parallel n in [1] { return n; } }")).unwrap_err();
    assert_eq!(errors[0].kind, SemanticErrorKind::ReturnAcrossParallel);
    let (result, output) = run(r#"
        fn value(n) { return n + 1; }
        fn main() {
            parallel n in [1, 2] {
                parallel m in [3, 4] { print(n, value(m)); }
                n = n + 10;
                print(n);
            }
            return 42;
        }
    "#);
    assert_eq!(result.unwrap(), Value::Integer(42));
    assert_eq!(output, "1 4\n1 5\n11\n2 4\n2 5\n12\n");
}

#[derive(Default)]
struct Counts {
    active: usize,
    peak: usize,
    started: usize,
    completed: usize,
    threads: std::collections::HashSet<std::thread::ThreadId>,
}
#[derive(Default)]
struct Shared {
    counts: Mutex<Counts>,
    arrived: Condvar,
}
struct Host {
    shared: Arc<Shared>,
    output: Vec<String>,
    gate: bool,
}
impl Runtime for Host {
    fn print(&mut self, text: &str) -> Result<(), String> {
        self.output.push(text.into());
        Ok(())
    }
    fn fork(&mut self) -> Result<Box<dyn Runtime + Send>, String> {
        Ok(Box::new(Host {
            shared: self.shared.clone(),
            output: Vec::new(),
            gate: self.gate,
        }))
    }
    fn request(&mut self, _: HttpMethod, _: &str, _: &Value) -> Result<Value, String> {
        let mut counts = self.shared.counts.lock().unwrap();
        counts.started += 1;
        counts.threads.insert(std::thread::current().id());
        counts.active += 1;
        counts.peak = counts.peak.max(counts.active);
        self.shared.arrived.notify_all();
        if self.gate {
            let deadline = Instant::now() + Duration::from_secs(2);
            while counts.started < 2 {
                let Some(left) = deadline.checked_duration_since(Instant::now()) else {
                    return Err("workers did not overlap".into());
                };
                counts = self.shared.arrived.wait_timeout(counts, left).unwrap().0;
            }
        }
        counts.active -= 1;
        counts.completed += 1;
        Ok(Value::Null)
    }
}

#[test]
fn worker_count_is_bounded_and_the_parent_joins_every_iteration() {
    let shared = Arc::new(Shared::default());
    let mut host = Host {
        shared: shared.clone(),
        output: Vec::new(),
        gate: true,
    };
    execute_with_limits(
        &parse(r#"parallel n in [1,2,3,4,5] { GET "test"; print(n); } print("done");"#),
        &mut host,
        Limits {
            parallel_workers: 2,
            ..Limits::default()
        },
    )
    .unwrap();
    let counts = shared.counts.lock().unwrap();
    assert_eq!(counts.peak, 2);
    assert_eq!(counts.completed, 5);
    assert_eq!(counts.active, 0);
    assert_eq!(host.output, ["1", "2", "3", "4", "5", "done"]);
}

#[test]
fn failures_join_the_current_batch_and_do_not_start_later_batches() {
    let shared = Arc::new(Shared::default());
    let mut host = Host {
        shared: shared.clone(),
        output: Vec::new(),
        gate: true,
    };
    let error = execute_with_limits(
        &parse(
            r#"parallel n in [0,1,2,3] {
        GET "test"; print(n); if n == 0 { 1 / 0; }
    } print("not reached");"#,
        ),
        &mut host,
        Limits {
            parallel_workers: 2,
            ..Limits::default()
        },
    )
    .unwrap_err();
    assert!(
        error
            .to_string()
            .contains("parallel iteration 1: division by zero")
    );
    let counts = shared.counts.lock().unwrap();
    assert_eq!(counts.started, 2);
    assert_eq!(counts.completed, 2);
    assert_eq!(host.output, ["0", "1"]);
}

#[test]
fn step_and_output_limits_cover_workers() {
    let mut output = Vec::new();
    let error = execute_with_limits(
        &parse("parallel n in [1,2] { while true {} }"),
        &mut StandardRuntime::new(&mut output),
        Limits {
            steps: 100,
            ..Limits::default()
        },
    )
    .unwrap_err();
    assert!(error.to_string().contains("step limit"));
    let error = execute_with_limits(
        &parse(r#"parallel n in [1] { print("ok"); print("too long"); }"#),
        &mut StandardRuntime::new(&mut output),
        Limits {
            parallel_output_bytes: 4,
            ..Limits::default()
        },
    )
    .unwrap_err();
    assert!(error.to_string().contains("output buffer limit"));
    assert_eq!(output, b"ok\n");
    for workers in [0, 65] {
        assert!(
            execute_with_limits(
                &parse("print(1);"),
                &mut StandardRuntime::new(Vec::new()),
                Limits {
                    parallel_workers: workers,
                    ..Limits::default()
                }
            )
            .unwrap_err()
            .to_string()
            .contains("worker limit")
        );
    }
}

#[test]
fn empty_iterables_and_unsupported_hosts_are_explicit() {
    struct NoFork;
    impl Runtime for NoFork {
        fn print(&mut self, _: &str) -> Result<(), String> {
            Ok(())
        }
    }
    execute(&parse("parallel n in [] {}"), &mut NoFork).unwrap();
    assert!(
        execute(&parse("parallel n in [1] {}"), &mut NoFork)
            .unwrap_err()
            .to_string()
            .contains("worker forks")
    );
    assert!(
        run("parallel n in 1 {}")
            .0
            .unwrap_err()
            .to_string()
            .contains("requires an array")
    );
}

#[test]
fn nested_parallelism_does_not_multiply_threads() {
    let shared = Arc::new(Shared::default());
    let mut host = Host {
        shared: shared.clone(),
        output: Vec::new(),
        gate: true,
    };
    execute_with_limits(
        &parse(
            r#"parallel n in [1,2] {
        parallel m in [3,4] { GET "test"; print(n, m); }
    }"#,
        ),
        &mut host,
        Limits {
            parallel_workers: 2,
            ..Limits::default()
        },
    )
    .unwrap();
    let counts = shared.counts.lock().unwrap();
    assert_eq!(counts.threads.len(), 2);
    assert_eq!(counts.completed, 4);
    assert_eq!(host.output, ["1 3", "1 4", "2 3", "2 4"]);
}

#[test]
fn worker_panics_are_reported_after_joining() {
    struct Panicking;
    impl Runtime for Panicking {
        fn print(&mut self, _: &str) -> Result<(), String> {
            Ok(())
        }
        fn fork(&mut self) -> Result<Box<dyn Runtime + Send>, String> {
            Ok(Box::new(Panicking))
        }
        fn request(&mut self, _: HttpMethod, _: &str, _: &Value) -> Result<Value, String> {
            panic!("host failed");
        }
    }
    let error = execute(
        &parse(r#"parallel n in [1] { GET "test"; }"#),
        &mut Panicking,
    )
    .unwrap_err();
    assert!(error.to_string().contains("parallel iteration 1 panicked"));
}
