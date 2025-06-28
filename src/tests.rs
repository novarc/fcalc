use super::*;
use std::sync::Mutex;

// Use a test mutex to ensure tests run serially to avoid global state conflicts
static TEST_MUTEX: Mutex<()> = Mutex::new(());

// Helper function to clear variables before each test
fn clear_variables() {
	match VARIABLES.lock() {
		Ok(mut variables) => variables.clear(),
		Err(poisoned) => {
			let mut variables = poisoned.into_inner();
			variables.clear();
		}
	}
}

// Helper function to get a variable value
fn get_variable(name: &str) -> Option<f64> {
	match VARIABLES.lock() {
		Ok(variables) => variables.get(name).copied(),
		Err(poisoned) => {
			let variables = poisoned.into_inner();
			variables.get(name).copied()
		}
	}
}

// Helper function to set a variable value
fn set_variable(name: &str, value: f64) {
	match VARIABLES.lock() {
		Ok(mut variables) => {
			variables.insert(name.to_string(), value);
		}
		Err(poisoned) => {
			let mut variables = poisoned.into_inner();
			variables.insert(name.to_string(), value);
		}
	}
}

// Helper function to clear functions before each test
fn clear_functions() {
	match FUNCTIONS.lock() {
		Ok(mut functions) => functions.clear(),
		Err(poisoned) => {
			let mut functions = poisoned.into_inner();
			functions.clear();
		}
	}
}

// Helper function to check if a function exists
fn function_exists(name: &str) -> bool {
	match FUNCTIONS.lock() {
		Ok(functions) => functions.contains_key(name),
		Err(poisoned) => {
			let functions = poisoned.into_inner();
			functions.contains_key(name)
		}
	}
}

// Helper function to get function parameter count
fn get_function_param_count(name: &str) -> Option<usize> {
	match FUNCTIONS.lock() {
		Ok(functions) => functions.get(name).map(|f| f.parameters.len()),
		Err(poisoned) => {
			let functions = poisoned.into_inner();
			functions.get(name).map(|f| f.parameters.len())
		}
	}
}

#[test]
fn test_simple_arithmetic() {
	let _guard = TEST_MUTEX
		.lock()
		.unwrap_or_else(|poisoned| poisoned.into_inner());
	clear_variables();

	assert_eq!(run("2 + 3"), Some(5.0));
	assert_eq!(run("10 - 4"), Some(6.0));
	assert_eq!(run("3 * 4"), Some(12.0));
	assert_eq!(run("15 / 3"), Some(5.0));
}

#[test]
fn test_operator_precedence() {
	let _guard = TEST_MUTEX
		.lock()
		.unwrap_or_else(|poisoned| poisoned.into_inner());
	clear_variables();

	assert_eq!(run("2 + 3 * 4"), Some(14.0)); // Should be 2 + (3 * 4) = 14
	assert_eq!(run("10 - 6 / 2"), Some(7.0)); // Should be 10 - (6 / 2) = 7
	assert_eq!(run("2 * 3 + 4"), Some(10.0)); // Should be (2 * 3) + 4 = 10
	assert_eq!(run("20 / 4 - 2"), Some(3.0)); // Should be (20 / 4) - 2 = 3
}

#[test]
fn test_parentheses() {
	let _guard = TEST_MUTEX
		.lock()
		.unwrap_or_else(|poisoned| poisoned.into_inner());
	clear_variables();

	assert_eq!(run("(2 + 3) * 4"), Some(20.0));
	assert_eq!(run("2 * (3 + 4)"), Some(14.0));
	assert_eq!(run("(10 - 6) / 2"), Some(2.0));
	assert_eq!(run("20 / (4 - 2)"), Some(10.0));
}

#[test]
fn test_variable_assignment() {
	let _guard = TEST_MUTEX
		.lock()
		.unwrap_or_else(|poisoned| poisoned.into_inner());
	clear_variables();

	assert_eq!(run("x = 5"), Some(5.0));
	assert_eq!(get_variable("x"), Some(5.0));

	assert_eq!(run("y = 10"), Some(10.0));
	assert_eq!(get_variable("y"), Some(10.0));

	assert_eq!(run("z = x + y"), Some(15.0));
	assert_eq!(get_variable("z"), Some(15.0));
}

#[test]
fn test_variable_usage() {
	let _guard = TEST_MUTEX
		.lock()
		.unwrap_or_else(|poisoned| poisoned.into_inner());
	clear_variables();
	set_variable("a", 5.0);
	set_variable("b", 3.0);

	assert_eq!(run("a + b"), Some(8.0));
	assert_eq!(run("a * b"), Some(15.0));
	assert_eq!(run("a - b"), Some(2.0));
	assert_eq!(run("a / b"), Some(5.0 / 3.0));
}

#[test]
fn test_complex_expressions() {
	let _guard = TEST_MUTEX
		.lock()
		.unwrap_or_else(|poisoned| poisoned.into_inner());
	clear_variables();

	assert_eq!(run("x = 2"), Some(2.0));
	assert_eq!(run("y = 3"), Some(3.0));
	assert_eq!(run("z = x * y + 1"), Some(7.0));
	assert_eq!(get_variable("z"), Some(7.0));

	run("result = (x + y) * z");
	assert_eq!(get_variable("result"), Some(35.0));
}

#[test]
fn test_floating_point_numbers() {
	let _guard = TEST_MUTEX
		.lock()
		.unwrap_or_else(|poisoned| poisoned.into_inner());
	clear_variables();

	assert_eq!(run("3.14 + 2.86"), Some(6.0));
	assert_eq!(run("5.5 * 2"), Some(11.0));
	assert_eq!(run("x = 3.14159"), Some(3.14159));
	assert_eq!(get_variable("x"), Some(3.14159));
}

#[test]
fn test_division_by_zero() {
	let _guard = TEST_MUTEX
		.lock()
		.unwrap_or_else(|poisoned| poisoned.into_inner());
	clear_variables();

	// Division by zero should return None (error)
	assert_eq!(run("5 / 0"), None);
	assert_eq!(run("x = 10 / 0"), None);
}

#[test]
fn test_undefined_variables() {
	let _guard = TEST_MUTEX
		.lock()
		.unwrap_or_else(|poisoned| poisoned.into_inner());
	clear_variables();

	// Using undefined variables should work (they default to 0)
	assert_eq!(run("undefined_var + 5"), Some(5.0));
	assert_eq!(run("x = undefined_var * 2"), Some(0.0));
}

#[test]
fn test_multiple_statements() {
	let _guard = TEST_MUTEX
		.lock()
		.unwrap_or_else(|poisoned| poisoned.into_inner());
	clear_variables();

	// Test semicolon-separated statements
	let result = run("x = 5; y = 10; z = x + y");
	assert_eq!(result, Some(15.0));
	assert_eq!(get_variable("x"), Some(5.0));
	assert_eq!(get_variable("y"), Some(10.0));
	assert_eq!(get_variable("z"), Some(15.0));
}

#[test]
fn test_newline_separated_statements() {
	let _guard = TEST_MUTEX
		.lock()
		.unwrap_or_else(|poisoned| poisoned.into_inner());
	clear_variables();

	// Test newline-separated statements
	let result = run("x = 3\ny = 4\nresult = x * y");
	assert_eq!(result, Some(12.0));
	assert_eq!(get_variable("x"), Some(3.0));
	assert_eq!(get_variable("y"), Some(4.0));
	assert_eq!(get_variable("result"), Some(12.0));
}

#[test]
fn test_chained_assignments() {
	let _guard = TEST_MUTEX
		.lock()
		.unwrap_or_else(|poisoned| poisoned.into_inner());
	clear_variables();

	// Test that assignment returns the assigned value for chaining
	assert_eq!(run("x = y = 5"), Some(5.0));
	assert_eq!(get_variable("x"), Some(5.0));
	assert_eq!(get_variable("y"), Some(5.0));
}

#[test]
fn test_assignment_with_expression() {
	let _guard = TEST_MUTEX
		.lock()
		.unwrap_or_else(|poisoned| poisoned.into_inner());
	clear_variables();

	set_variable("a", 10.0);
	set_variable("b", 5.0);

	assert_eq!(run("result = a * 2 + b"), Some(25.0));
	assert_eq!(get_variable("result"), Some(25.0));
}

#[test]
fn test_empty_input() {
	let _guard = TEST_MUTEX
		.lock()
		.unwrap_or_else(|poisoned| poisoned.into_inner());
	clear_variables();

	assert_eq!(run(""), None);
	assert_eq!(run("   "), None);
}

#[test]
fn test_whitespace_handling() {
	let _guard = TEST_MUTEX
		.lock()
		.unwrap_or_else(|poisoned| poisoned.into_inner());
	clear_variables();

	assert_eq!(run("  2   +   3  "), Some(5.0));
	assert_eq!(run(" x = 5 "), Some(5.0));
	assert_eq!(get_variable("x"), Some(5.0));
}

#[test]
fn test_variable_names() {
	let _guard = TEST_MUTEX
		.lock()
		.unwrap_or_else(|poisoned| poisoned.into_inner());
	clear_variables();

	// Test various valid variable names
	assert_eq!(run("var1 = 5"), Some(5.0));
	assert_eq!(run("_underscore = 10"), Some(10.0));
	assert_eq!(run("camelCase = 15"), Some(15.0));
	assert_eq!(run("snake_case = 20"), Some(20.0));

	assert_eq!(get_variable("var1"), Some(5.0));
	assert_eq!(get_variable("_underscore"), Some(10.0));
	assert_eq!(get_variable("camelCase"), Some(15.0));
	assert_eq!(get_variable("snake_case"), Some(20.0));
}

#[test]
fn test_large_numbers() {
	let _guard = TEST_MUTEX
		.lock()
		.unwrap_or_else(|poisoned| poisoned.into_inner());
	clear_variables();

	assert_eq!(run("1000000 + 1"), Some(1000001.0));
	assert_eq!(run("x = 999999999"), Some(999999999.0));
	assert_eq!(get_variable("x"), Some(999999999.0));
}

#[test]
fn test_negative_results() {
	let _guard = TEST_MUTEX
		.lock()
		.unwrap_or_else(|poisoned| poisoned.into_inner());
	clear_variables();

	assert_eq!(run("3 - 8"), Some(-5.0));
	assert_eq!(run("x = 2 - 10"), Some(-8.0));
	assert_eq!(get_variable("x"), Some(-8.0));
}

#[test]
fn test_fractional_results() {
	let _guard = TEST_MUTEX
		.lock()
		.unwrap_or_else(|poisoned| poisoned.into_inner());
	clear_variables();

	assert_eq!(run("7 / 2"), Some(3.5));
	assert_eq!(run("1 / 3"), Some(1.0 / 3.0));
}

#[test]
fn test_function_parsing() {
	let _guard = TEST_MUTEX
		.lock()
		.unwrap_or_else(|poisoned| poisoned.into_inner());
	clear_variables();
	clear_functions();

	// Test that function definitions are parsed correctly and return None
	assert_eq!(run("(x, y) => { x + y }"), None);
	assert_eq!(run("(a) => { a * 2 }"), None);
	assert_eq!(run("() => { 42 }"), None);

	// Test that functions can be defined alongside other expressions
	assert_eq!(run("x = 5; (a) => { a + x }"), None);
}

#[test]
fn test_named_function_definition() {
	let _guard = TEST_MUTEX
		.lock()
		.unwrap_or_else(|poisoned| poisoned.into_inner());
	clear_variables();
	clear_functions();

	// Test named function definition with two parameters
	assert_eq!(run("sum = (a, b) => { a + b }"), None);
	assert!(function_exists("sum"));
	assert_eq!(get_function_param_count("sum"), Some(2));

	// Test named function definition with one parameter
	assert_eq!(run("double = (x) => { x * 2 }"), None);
	assert!(function_exists("double"));
	assert_eq!(get_function_param_count("double"), Some(1));

	// Test named function definition with no parameters
	assert_eq!(run("answer = () => { 42 }"), None);
	assert!(function_exists("answer"));
	assert_eq!(get_function_param_count("answer"), Some(0));
}

#[test]
fn test_function_calls() {
	let _guard = TEST_MUTEX
		.lock()
		.unwrap_or_else(|poisoned| poisoned.into_inner());
	clear_variables();
	clear_functions();

	// Define functions first
	run("sum = (a, b) => { a + b }");
	run("square = (x) => { x * x }");
	run("constant = () => { 100 }");

	// Test function calls (currently returns parameter count as demo)
	assert_eq!(run("sum(3, 4)"), Some(2.0)); // 2 parameters
	assert_eq!(run("square(5)"), Some(1.0)); // 1 parameter
	assert_eq!(run("constant()"), Some(0.0)); // 0 parameters
}

#[test]
fn test_multiple_function_definitions() {
	let _guard = TEST_MUTEX
		.lock()
		.unwrap_or_else(|poisoned| poisoned.into_inner());
	clear_variables();
	clear_functions();

	// Define multiple functions
	assert_eq!(run("add = (x, y) => { x + y }"), None);
	assert_eq!(run("multiply = (a, b) => { a * b }"), None);
	assert_eq!(run("negate = (n) => { 0 - n }"), None);

	// Verify all functions exist
	assert!(function_exists("add"));
	assert!(function_exists("multiply"));
	assert!(function_exists("negate"));

	// Verify parameter counts
	assert_eq!(get_function_param_count("add"), Some(2));
	assert_eq!(get_function_param_count("multiply"), Some(2));
	assert_eq!(get_function_param_count("negate"), Some(1));
}

#[test]
fn test_function_with_complex_body() {
	let _guard = TEST_MUTEX
		.lock()
		.unwrap_or_else(|poisoned| poisoned.into_inner());
	clear_variables();
	clear_functions();

	// Test function with complex arithmetic in body
	assert_eq!(run("complex = (x, y) => { x * 2 + y / 2 - 1 }"), None);
	assert!(function_exists("complex"));
	assert_eq!(get_function_param_count("complex"), Some(2));

	// Test calling the complex function
	assert_eq!(run("complex(5, 10)"), Some(2.0)); // Returns parameter count
}

#[test]
fn test_function_name_variations() {
	let _guard = TEST_MUTEX
		.lock()
		.unwrap_or_else(|poisoned| poisoned.into_inner());
	clear_variables();
	clear_functions();

	// Test various valid function names
	assert_eq!(run("func1 = (x) => { x }"), None);
	assert_eq!(run("_private = (a, b) => { a + b }"), None);
	assert_eq!(run("camelCase = (n) => { n * 2 }"), None);
	assert_eq!(run("snake_case = (x, y, z) => { x + y + z }"), None);

	// Verify all functions exist
	assert!(function_exists("func1"));
	assert!(function_exists("_private"));
	assert!(function_exists("camelCase"));
	assert!(function_exists("snake_case"));

	// Verify parameter counts
	assert_eq!(get_function_param_count("func1"), Some(1));
	assert_eq!(get_function_param_count("_private"), Some(2));
	assert_eq!(get_function_param_count("camelCase"), Some(1));
	assert_eq!(get_function_param_count("snake_case"), Some(3));
}

#[test]
fn test_function_call_nonexistent() {
	let _guard = TEST_MUTEX
		.lock()
		.unwrap_or_else(|poisoned| poisoned.into_inner());
	clear_variables();
	clear_functions();

	// Test calling a function that doesn't exist should return None (error)
	assert_eq!(run("nonexistent(1, 2, 3)"), None);
	assert_eq!(run("undefined()"), None);
}

#[test]
fn test_anonymous_vs_named_functions() {
	let _guard = TEST_MUTEX
		.lock()
		.unwrap_or_else(|poisoned| poisoned.into_inner());
	clear_variables();
	clear_functions();

	// Define anonymous function (auto-named)
	assert_eq!(run("(x) => { x + 1 }"), None);

	// Define named function
	assert_eq!(run("increment = (x) => { x + 1 }"), None);

	// Check that named function exists with correct name
	assert!(function_exists("increment"));
	assert_eq!(get_function_param_count("increment"), Some(1));

	// Check that auto-generated function also exists
	// (The exact auto-generated name depends on implementation)
	let functions = FUNCTIONS.lock().unwrap();
	assert!(functions.len() >= 2); // At least 2 functions should exist
}

#[test]
fn test_function_redefinition() {
	let _guard = TEST_MUTEX
		.lock()
		.unwrap_or_else(|poisoned| poisoned.into_inner());
	clear_variables();
	clear_functions();

	// Define a function
	assert_eq!(run("test = (x) => { x * 2 }"), None);
	assert!(function_exists("test"));
	assert_eq!(get_function_param_count("test"), Some(1));

	// Redefine the same function with different parameters
	assert_eq!(run("test = (a, b) => { a + b }"), None);
	assert!(function_exists("test"));
	assert_eq!(get_function_param_count("test"), Some(2)); // Should be updated
}

#[test]
fn test_function_calls_with_expressions() {
	let _guard = TEST_MUTEX
		.lock()
		.unwrap_or_else(|poisoned| poisoned.into_inner());
	clear_variables();
	clear_functions();

	// Define a function
	run("calc = (x, y) => { x + y }");

	// Test function calls with variable arguments
	run("a = 5");
	run("b = 3");

	// Call function with variables (currently returns parameter count)
	assert_eq!(run("calc(a, b)"), Some(2.0));

	// Call function with expressions as arguments
	assert_eq!(run("calc(2 + 3, 4 * 2)"), Some(2.0));
}

#[test]
fn test_mixing_variables_and_functions() {
	let _guard = TEST_MUTEX
		.lock()
		.unwrap_or_else(|poisoned| poisoned.into_inner());
	clear_variables();
	clear_functions();

	// Mix variable assignments and function definitions
	assert_eq!(run("x = 10"), Some(10.0));
	assert_eq!(run("double = (n) => { n * 2 }"), None);
	assert_eq!(run("y = 20"), Some(20.0));
	assert_eq!(run("add = (a, b) => { a + b }"), None);

	// Verify variables exist
	assert_eq!(get_variable("x"), Some(10.0));
	assert_eq!(get_variable("y"), Some(20.0));

	// Verify functions exist
	assert!(function_exists("double"));
	assert!(function_exists("add"));

	// Test function calls
	assert_eq!(run("double(5)"), Some(1.0)); // 1 parameter
	assert_eq!(run("add(x, y)"), Some(2.0)); // 2 parameters
}
