fn foo() -> i32 {
    // TODO: implement error handling
    42
}

fn bar() {
    // FIXME: this is broken
    let _ = foo();
}

fn baz() {
    // HACK: workaround for issue #42
    println!("baz");
}
