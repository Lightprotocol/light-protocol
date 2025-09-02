use light_profiler::profile;

struct TestStruct;

impl TestStruct {
    #[profile]
    pub fn new() -> Self {
        TestStruct
    }

    #[profile]
    pub fn test_method(&self, x: i32) -> i32 {
        x * 2
    }
}

#[profile]
fn standalone_function(x: i32, y: i32) -> i32 {
    x + y
}

#[test]
fn test_struct_new() {
    let _instance = TestStruct::new();
}

#[test]
fn test_method() {
    let instance = TestStruct::new();
    assert_eq!(instance.test_method(5), 10);
}

#[test]
fn test_standalone() {
    assert_eq!(standalone_function(3, 4), 7);
}
