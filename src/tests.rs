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
	assert_eq!(run("fn add(x, y) { x + y }"), None);
	assert_eq!(run("fn double(a) { a * 2 }"), None);
	assert_eq!(run("fn answer() { 42 }"), None);

	// Test that functions can be defined alongside other expressions
	assert_eq!(run("x = 5; fn increment(a) { a + x }"), None);
}

#[test]
fn test_named_function_definition() {
	let _guard = TEST_MUTEX
		.lock()
		.unwrap_or_else(|poisoned| poisoned.into_inner());
	clear_variables();
	clear_functions();

	// Test named function definition with two parameters
	assert_eq!(run("fn sum(a, b) { a + b }"), None);
	assert!(function_exists("sum"));
	assert_eq!(get_function_param_count("sum"), Some(2));

	// Test named function definition with one parameter
	assert_eq!(run("fn double(x) { x * 2 }"), None);
	assert!(function_exists("double"));
	assert_eq!(get_function_param_count("double"), Some(1));

	// Test named function definition with no parameters
	assert_eq!(run("fn answer() { 42 }"), None);
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
	run("fn sum(a, b) { a + b }");
	run("fn square(x) { x * x }");
	run("fn constant() { 100 }");

	// Test function calls (should return actual computed results)
	assert_eq!(run("sum(3, 4)"), Some(7.0)); // 3 + 4 = 7
	assert_eq!(run("square(5)"), Some(25.0)); // 5 * 5 = 25
	assert_eq!(run("constant()"), Some(100.0)); // constant function returns 100
}

#[test]
fn test_multiple_function_definitions() {
	let _guard = TEST_MUTEX
		.lock()
		.unwrap_or_else(|poisoned| poisoned.into_inner());
	clear_variables();
	clear_functions();

	// Define multiple functions
	assert_eq!(run("fn add(x, y) { x + y }"), None);
	assert_eq!(run("fn multiply(a, b) { a * b }"), None);
	assert_eq!(run("fn negate(n) { 0 - n }"), None);

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
	assert_eq!(run("fn complex(x, y) { x * 2 + y / 2 - 1 }"), None);
	assert!(function_exists("complex"));
	assert_eq!(get_function_param_count("complex"), Some(2));

	// Test calling the complex function
	assert_eq!(run("complex(5, 10)"), Some(14.0)); // 5 * 2 + 10 / 2 - 1 = 10 + 5 - 1 = 14
}

#[test]
fn test_function_name_variations() {
	let _guard = TEST_MUTEX
		.lock()
		.unwrap_or_else(|poisoned| poisoned.into_inner());
	clear_variables();
	clear_functions();

	// Test various valid function names
	assert_eq!(run("fn func1(x) { x }"), None);
	assert_eq!(run("fn _private(a, b) { a + b }"), None);
	assert_eq!(run("fn camelCase(n) { n * 2 }"), None);
	assert_eq!(run("fn snake_case(x, y, z) { x + y + z }"), None);

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

	// Define named function with new syntax
	assert_eq!(run("fn increment(x) { x + 1 }"), None);

	// Check that named function exists with correct name
	assert!(function_exists("increment"));
	assert_eq!(get_function_param_count("increment"), Some(1));

	// Only named functions now (no more anonymous functions)
	let functions = FUNCTIONS.lock().unwrap();
	assert!(functions.len() >= 1); // At least 1 function should exist
}

#[test]
fn test_function_redefinition() {
	let _guard = TEST_MUTEX
		.lock()
		.unwrap_or_else(|poisoned| poisoned.into_inner());
	clear_variables();
	clear_functions();

	// Define a function
	assert_eq!(run("fn test(x) { x * 2 }"), None);
	assert!(function_exists("test"));
	assert_eq!(get_function_param_count("test"), Some(1));

	// Redefine the same function with different parameters
	assert_eq!(run("fn test(a, b) { a + b }"), None);
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
	run("fn calc(x, y) { x + y }");

	// Test function calls with variable arguments
	run("a = 5");
	run("b = 3");

	// Call function with variables (should return actual sum)
	assert_eq!(run("calc(a, b)"), Some(8.0)); // 5 + 3 = 8

	// Call function with expressions as arguments
	assert_eq!(run("calc(2 + 3, 4 * 2)"), Some(13.0)); // (2 + 3) + (4 * 2) = 5 + 8 = 13
}

#[test]
fn test_function_call_assignment() {
	let _guard = TEST_MUTEX
		.lock()
		.unwrap_or_else(|poisoned| poisoned.into_inner());
	clear_variables();
	clear_functions();

	// Define a simple function
	run("fn add(a, b) { a + b }");
	assert!(function_exists("add"));

	// Test assigning function call result to a variable
	assert_eq!(run("x = add(2, 3)"), Some(5.0));
	assert_eq!(get_variable("x"), Some(5.0));

	// Test using the variable in another expression
	assert_eq!(run("y = x * 2"), Some(10.0));
	assert_eq!(get_variable("y"), Some(10.0));

	// Test chaining function calls
	assert_eq!(run("z = add(x, y)"), Some(15.0));
	assert_eq!(get_variable("z"), Some(15.0));

	// Test function call in complex expression
	assert_eq!(run("result = add(1, 2) + add(3, 4)"), Some(10.0));
	assert_eq!(get_variable("result"), Some(10.0));
}

#[test]
fn test_multiple_function_call_assignments() {
	let _guard = TEST_MUTEX
		.lock()
		.unwrap_or_else(|poisoned| poisoned.into_inner());
	clear_variables();
	clear_functions();

	// Define multiple functions
	run("fn multiply(a, b) { a * b }");
	run("fn subtract(a, b) { a - b }");

	assert!(function_exists("multiply"));
	assert!(function_exists("subtract"));

	// Test multiple function call assignments
	assert_eq!(run("a = multiply(3, 4)"), Some(12.0));
	assert_eq!(run("b = subtract(10, 3)"), Some(7.0));
	assert_eq!(run("c = multiply(a, b)"), Some(84.0));

	assert_eq!(get_variable("a"), Some(12.0));
	assert_eq!(get_variable("b"), Some(7.0));
	assert_eq!(get_variable("c"), Some(84.0));
}

#[test]
fn test_function_call_with_variables() {
	let _guard = TEST_MUTEX
		.lock()
		.unwrap_or_else(|poisoned| poisoned.into_inner());
	clear_variables();
	clear_functions();

	// Define function
	run("fn power(base, exp) { base * base }"); // Simple square for testing
	assert!(function_exists("power"));

	// Set up variables
	set_variable("base", 5.0);
	set_variable("exp", 2.0);

	// Test function call with variables as arguments
	assert_eq!(run("result = power(base, exp)"), Some(25.0));
	assert_eq!(get_variable("result"), Some(25.0));

	// Test mixing literals and variables
	assert_eq!(run("result2 = power(3, exp)"), Some(9.0));
	assert_eq!(get_variable("result2"), Some(9.0));
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
	assert_eq!(run("fn double(n) { n * 2 }"), None);
	assert_eq!(run("y = 20"), Some(20.0));
	assert_eq!(run("fn add(a, b) { a + b }"), None);

	// Verify variables exist
	assert_eq!(get_variable("x"), Some(10.0));
	assert_eq!(get_variable("y"), Some(20.0));

	// Verify functions exist
	assert!(function_exists("double"));
	assert!(function_exists("add"));

	// Test function calls
	assert_eq!(run("double(5)"), Some(10.0)); // 5 * 2 = 10
	assert_eq!(run("add(x, y)"), Some(30.0)); // 10 + 20 = 30
}

#[test]
fn test_lambda_function_definition() {
	let _guard = TEST_MUTEX
		.lock()
		.unwrap_or_else(|poisoned| poisoned.into_inner());
	clear_variables();
	clear_functions();

	// Test lambda function definition with two parameters
	assert_eq!(run("add = (a, b) => {a + b}"), None);
	assert!(function_exists("add"));
	assert_eq!(get_function_param_count("add"), Some(2));

	// Test lambda function definition with one parameter
	assert_eq!(run("double = (x) => {x * 2}"), None);
	assert!(function_exists("double"));
	assert_eq!(get_function_param_count("double"), Some(1));

	// Test lambda function definition with no parameters
	assert_eq!(run("answer = () => {42}"), None);
	assert!(function_exists("answer"));
	assert_eq!(get_function_param_count("answer"), Some(0));
}

#[test]
fn test_lambda_function_calls() {
	let _guard = TEST_MUTEX
		.lock()
		.unwrap_or_else(|poisoned| poisoned.into_inner());
	clear_variables();
	clear_functions();

	// Define lambda functions first
	run("sum = (a, b) => {a + b}");
	run("square = (x) => {x * x}");
	run("constant = () => {100}");

	// Test lambda function calls
	assert_eq!(run("sum(3, 4)"), Some(7.0)); // 3 + 4 = 7
	assert_eq!(run("square(5)"), Some(25.0)); // 5 * 5 = 25
	assert_eq!(run("constant()"), Some(100.0)); // constant function returns 100
}

#[test]
fn test_lambda_function_with_complex_body() {
	let _guard = TEST_MUTEX
		.lock()
		.unwrap_or_else(|poisoned| poisoned.into_inner());
	clear_variables();
	clear_functions();

	// Test lambda function with complex arithmetic in body
	assert_eq!(run("complex = (x, y) => {x * 2 + y / 2 - 1}"), None);
	assert!(function_exists("complex"));
	assert_eq!(get_function_param_count("complex"), Some(2));

	// Test calling the complex lambda function
	assert_eq!(run("complex(5, 10)"), Some(14.0)); // 5 * 2 + 10 / 2 - 1 = 10 + 5 - 1 = 14
}

#[test]
fn test_lambda_function_name_variations() {
	let _guard = TEST_MUTEX
		.lock()
		.unwrap_or_else(|poisoned| poisoned.into_inner());
	clear_variables();
	clear_functions();

	// Test various valid lambda function names
	assert_eq!(run("func1 = (x) => {x}"), None);
	assert_eq!(run("_private = (a, b) => {a + b}"), None);
	assert_eq!(run("camelCase = (n) => {n * 2}"), None);
	assert_eq!(run("snake_case = (x, y, z) => {x + y + z}"), None);

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
fn test_lambda_function_redefinition() {
	let _guard = TEST_MUTEX
		.lock()
		.unwrap_or_else(|poisoned| poisoned.into_inner());
	clear_variables();
	clear_functions();

	// Define a lambda function
	assert_eq!(run("test = (x) => {x * 2}"), None);
	assert!(function_exists("test"));
	assert_eq!(get_function_param_count("test"), Some(1));

	// Redefine the same function with different parameters
	assert_eq!(run("test = (a, b) => {a + b}"), None);
	assert!(function_exists("test"));
	assert_eq!(get_function_param_count("test"), Some(2)); // Should be updated
}

#[test]
fn test_lambda_function_calls_with_expressions() {
	let _guard = TEST_MUTEX
		.lock()
		.unwrap_or_else(|poisoned| poisoned.into_inner());
	clear_variables();
	clear_functions();

	// Define a lambda function
	run("calc = (x, y) => {x + y}");

	// Test function calls with variable arguments
	run("a = 5");
	run("b = 3");

	// Call function with variables
	assert_eq!(run("calc(a, b)"), Some(8.0)); // 5 + 3 = 8

	// Call function with expressions as arguments
	assert_eq!(run("calc(2 + 3, 4 * 2)"), Some(13.0)); // (2 + 3) + (4 * 2) = 5 + 8 = 13
}

#[test]
fn test_lambda_function_call_assignment() {
	let _guard = TEST_MUTEX
		.lock()
		.unwrap_or_else(|poisoned| poisoned.into_inner());
	clear_variables();
	clear_functions();

	// Define a simple lambda function
	run("add = (a, b) => {a + b}");
	assert!(function_exists("add"));

	// Test assigning lambda function call result to a variable
	assert_eq!(run("x = add(2, 3)"), Some(5.0));
	assert_eq!(get_variable("x"), Some(5.0));

	// Test using the variable in another expression
	assert_eq!(run("y = x * 2"), Some(10.0));
	assert_eq!(get_variable("y"), Some(10.0));

	// Test chaining lambda function calls
	assert_eq!(run("z = add(x, y)"), Some(15.0));
	assert_eq!(get_variable("z"), Some(15.0));

	// Test lambda function call in complex expression
	assert_eq!(run("result = add(1, 2) + add(3, 4)"), Some(10.0));
	assert_eq!(get_variable("result"), Some(10.0));
}

#[test]
fn test_mixing_lambda_and_named_functions() {
	let _guard = TEST_MUTEX
		.lock()
		.unwrap_or_else(|poisoned| poisoned.into_inner());
	clear_variables();
	clear_functions();

	// Mix lambda functions and named functions
	assert_eq!(run("lambda_add = (a, b) => {a + b}"), None);
	assert_eq!(run("fn named_multiply(x, y) { x * y }"), None);
	assert_eq!(run("lambda_square = (n) => {n * n}"), None);

	// Verify all functions exist
	assert!(function_exists("lambda_add"));
	assert!(function_exists("named_multiply"));
	assert!(function_exists("lambda_square"));

	// Test calling both types of functions
	assert_eq!(run("lambda_add(3, 4)"), Some(7.0)); // 3 + 4 = 7
	assert_eq!(run("named_multiply(2, 5)"), Some(10.0)); // 2 * 5 = 10
	assert_eq!(run("lambda_square(3)"), Some(9.0)); // 3 * 3 = 9

	// Test mixing function calls step by step
	assert_eq!(run("temp1 = named_multiply(2, 3)"), Some(6.0)); // 2 * 3 = 6
	assert_eq!(run("temp2 = lambda_square(2)"), Some(4.0)); // 2 * 2 = 4
	assert_eq!(run("result = lambda_add(temp1, temp2)"), Some(10.0)); // 6 + 4 = 10
}

#[test]
fn test_lambda_function_with_single_parameter_no_parens() {
	let _guard = TEST_MUTEX
		.lock()
		.unwrap_or_else(|poisoned| poisoned.into_inner());
	clear_variables();
	clear_functions();

	// Test lambda function with single parameter (should still require parentheses for consistency)
	assert_eq!(run("increment = (x) => {x + 1}"), None);
	assert!(function_exists("increment"));
	assert_eq!(get_function_param_count("increment"), Some(1));

	// Test calling the function
	assert_eq!(run("increment(5)"), Some(6.0)); // 5 + 1 = 6
}

#[test]
fn test_lambda_function_whitespace_handling() {
	let _guard = TEST_MUTEX
		.lock()
		.unwrap_or_else(|poisoned| poisoned.into_inner());
	clear_variables();
	clear_functions();

	// Test lambda function with various whitespace
	assert_eq!(run("  add  =  ( a , b )  =>  { a + b }  "), None);
	assert!(function_exists("add"));
	assert_eq!(get_function_param_count("add"), Some(2));

	// Test calling the function
	assert_eq!(run("add(2, 3)"), Some(5.0)); // 2 + 3 = 5
}

#[test]
fn test_lambda_and_variables_interaction() {
	let _guard = TEST_MUTEX
		.lock()
		.unwrap_or_else(|poisoned| poisoned.into_inner());
	clear_variables();
	clear_functions();

	// Mix lambda functions with variable assignments
	assert_eq!(run("x = 10"), Some(10.0));
	assert_eq!(run("multiply = (a, b) => {a * b}"), None);
	assert_eq!(run("y = 5"), Some(5.0));

	// Verify variables exist
	assert_eq!(get_variable("x"), Some(10.0));
	assert_eq!(get_variable("y"), Some(5.0));

	// Verify function exists
	assert!(function_exists("multiply"));

	// Test using variables in lambda function calls
	assert_eq!(run("result = multiply(x, y)"), Some(50.0)); // 10 * 5 = 50
	assert_eq!(get_variable("result"), Some(50.0));
}

#[test]
fn test_fn_functions_with_variable_access() {
	let _guard = TEST_MUTEX
		.lock()
		.unwrap_or_else(|poisoned| poisoned.into_inner());
	clear_variables();
	clear_functions();

	// Set up global variables
	assert_eq!(run("global_x = 100"), Some(100.0));
	assert_eq!(run("global_y = 50"), Some(50.0));

	// Define function that uses global variables
	assert_eq!(
		run("fn use_globals(multiplier) { global_x * multiplier + global_y }"),
		None
	);
	assert!(function_exists("use_globals"));

	// Test function call
	assert_eq!(run("use_globals(2)"), Some(250.0)); // 100 * 2 + 50 = 250

	// Modify global variables and test again
	assert_eq!(run("global_x = 10"), Some(10.0));
	assert_eq!(run("use_globals(3)"), Some(80.0)); // 10 * 3 + 50 = 80
}

#[test]
fn test_nested_function_calls() {
	let _guard = TEST_MUTEX
		.lock()
		.unwrap_or_else(|poisoned| poisoned.into_inner());
	clear_variables();
	clear_functions();

	// Define helper functions
	assert_eq!(run("fn add(a, b) { a + b }"), None);
	assert_eq!(run("fn multiply(x, y) { x * y }"), None);
	assert_eq!(run("fn square(n) { n * n }"), None);

	// Test nested function calls
	assert_eq!(run("add(multiply(3, 4), square(2))"), Some(16.0)); // (3 * 4) + (2 * 2) = 12 + 4 = 16
	assert_eq!(run("multiply(add(2, 3), square(3))"), Some(45.0)); // (2 + 3) * (3 * 3) = 5 * 9 = 45
	assert_eq!(run("square(add(3, 2))"), Some(25.0)); // (3 + 2)^2 = 5^2 = 25
}

#[test]
fn test_functions_calling_other_functions() {
	let _guard = TEST_MUTEX
		.lock()
		.unwrap_or_else(|poisoned| poisoned.into_inner());
	clear_variables();
	clear_functions();

	// Define base functions
	assert_eq!(run("fn add(a, b) { a + b }"), None);
	assert_eq!(run("fn multiply(x, y) { x * y }"), None);

	// Define function that calls other functions
	assert_eq!(
		run("fn calculate(a, b, c) { multiply(add(a, b), c) }"),
		None
	);
	assert!(function_exists("calculate"));

	// Test the composite function
	assert_eq!(run("calculate(2, 3, 4)"), Some(20.0)); // (2 + 3) * 4 = 5 * 4 = 20
	assert_eq!(run("calculate(1, 1, 10)"), Some(20.0)); // (1 + 1) * 10 = 2 * 10 = 20
}

#[test]
fn test_functions_with_complex_expressions() {
	let _guard = TEST_MUTEX
		.lock()
		.unwrap_or_else(|poisoned| poisoned.into_inner());
	clear_variables();
	clear_functions();

	// Define function with complex mathematical expression
	assert_eq!(
		run("fn quadratic(a, b, c, x) { a * x * x + b * x + c }"),
		None
	);
	assert!(function_exists("quadratic"));

	// Test quadratic function
	assert_eq!(run("quadratic(1, 2, 3, 2)"), Some(11.0)); // 1*2*2 + 2*2 + 3 = 4 + 4 + 3 = 11
	assert_eq!(run("quadratic(2, -3, 1, 3)"), Some(10.0)); // 2*3*3 + (-3)*3 + 1 = 18 - 9 + 1 = 10

	// Define function with nested arithmetic
	assert_eq!(
		run("fn complex_calc(x, y) { (x + y) * (x - y) + x * y }"),
		None
	);
	assert!(function_exists("complex_calc"));

	// Test complex calculation
	assert_eq!(run("complex_calc(5, 3)"), Some(31.0)); // (5+3)*(5-3) + 5*3 = 8*2 + 15 = 16 + 15 = 31
}

#[test]
fn test_function_parameter_shadowing() {
	let _guard = TEST_MUTEX
		.lock()
		.unwrap_or_else(|poisoned| poisoned.into_inner());
	clear_variables();
	clear_functions();

	// Set up a global variable
	assert_eq!(run("x = 100"), Some(100.0));

	// Define function with parameter that shadows global variable
	assert_eq!(run("fn shadow_test(x) { x * 2 }"), None);
	assert!(function_exists("shadow_test"));

	// Test that function uses parameter, not global variable
	assert_eq!(run("shadow_test(5)"), Some(10.0)); // Should use parameter x=5, not global x=100

	// Verify global variable is unchanged
	assert_eq!(get_variable("x"), Some(100.0));
}

#[test]
fn test_multiple_parameter_functions() {
	let _guard = TEST_MUTEX
		.lock()
		.unwrap_or_else(|poisoned| poisoned.into_inner());
	clear_variables();
	clear_functions();

	// Define function with many parameters
	assert_eq!(
		run("fn sum_five(a, b, c, d, e) { a + b + c + d + e }"),
		None
	);
	assert!(function_exists("sum_five"));
	assert_eq!(get_function_param_count("sum_five"), Some(5));

	// Test function call
	assert_eq!(run("sum_five(1, 2, 3, 4, 5)"), Some(15.0));
	assert_eq!(run("sum_five(10, 20, 30, 40, 50)"), Some(150.0));

	// Define function with different parameter patterns
	assert_eq!(run("fn weighted_sum(a, b, c) { a * 3 + b * 2 + c }"), None);
	assert!(function_exists("weighted_sum"));

	// Test weighted sum
	assert_eq!(run("weighted_sum(1, 2, 3)"), Some(10.0)); // 1*3 + 2*2 + 3 = 3 + 4 + 3 = 10
}

#[test]
fn test_function_with_zero_parameters() {
	let _guard = TEST_MUTEX
		.lock()
		.unwrap_or_else(|poisoned| poisoned.into_inner());
	clear_variables();
	clear_functions();

	// Define functions with no parameters
	assert_eq!(run("fn pi() { 3.14159 }"), None);
	assert_eq!(run("fn get_answer() { 42 }"), None);
	assert_eq!(run("fn random_number() { 123.456 }"), None);

	assert!(function_exists("pi"));
	assert!(function_exists("get_answer"));
	assert!(function_exists("random_number"));

	// Test zero-parameter function calls
	assert_eq!(run("pi()"), Some(3.14159));
	assert_eq!(run("get_answer()"), Some(42.0));
	assert_eq!(run("random_number()"), Some(123.456));

	// Test using these functions in expressions
	assert_eq!(run("result = pi() * 2"), Some(6.28318));
	assert_eq!(run("answer_plus_one = get_answer() + 1"), Some(43.0));
}

#[test]
fn test_function_call_error_handling() {
	let _guard = TEST_MUTEX
		.lock()
		.unwrap_or_else(|poisoned| poisoned.into_inner());
	clear_variables();
	clear_functions();

	// Define a function that might cause division by zero
	assert_eq!(run("fn divide(a, b) { a / b }"), None);
	assert!(function_exists("divide"));

	// Test normal division
	assert_eq!(run("divide(10, 2)"), Some(5.0));

	// Test division by zero (should return None)
	assert_eq!(run("divide(10, 0)"), None);
	assert_eq!(run("result = divide(5, 0)"), None);
}

#[test]
fn test_function_definition_with_statements() {
	let _guard = TEST_MUTEX
		.lock()
		.unwrap_or_else(|poisoned| poisoned.into_inner());
	clear_variables();
	clear_functions();

	// Test function definition mixed with variable assignments
	assert_eq!(run("x = 5; fn double(n) { n * 2 }; y = 10"), None);

	// Verify variable assignments worked
	assert_eq!(get_variable("x"), Some(5.0));
	assert_eq!(get_variable("y"), Some(10.0));

	// Verify function was defined
	assert!(function_exists("double"));

	// Test function call
	assert_eq!(run("double(7)"), Some(14.0));
}

#[test]
fn test_complex_function_chains() {
	let _guard = TEST_MUTEX
		.lock()
		.unwrap_or_else(|poisoned| poisoned.into_inner());
	clear_variables();
	clear_functions();

	// Define a chain of functions
	assert_eq!(run("fn increment(x) { x + 1 }"), None);
	assert_eq!(run("fn double(x) { x * 2 }"), None);
	assert_eq!(run("fn square(x) { x * x }"), None);

	// Test chaining function calls
	assert_eq!(run("result = square(double(increment(3)))"), Some(64.0));
	// increment(3) = 4, double(4) = 8, square(8) = 64

	// Test with variables
	assert_eq!(run("base = 2"), Some(2.0));
	assert_eq!(run("final = square(double(increment(base)))"), Some(36.0));
	// increment(2) = 3, double(3) = 6, square(6) = 36
}

#[test]
fn test_function_with_conditional_logic() {
	let _guard = TEST_MUTEX
		.lock()
		.unwrap_or_else(|poisoned| poisoned.into_inner());
	clear_variables();
	clear_functions();

	// Define function that simulates absolute value using arithmetic
	assert_eq!(run("fn abs_like(x) { x * x / x }"), None); // x^2/x = |x| for x != 0
	assert!(function_exists("abs_like"));

	// Test positive number
	assert_eq!(run("abs_like(5)"), Some(5.0)); // 5*5/5 = 25/5 = 5
	assert_eq!(run("abs_like(-3)"), Some(-3.0)); // (-3)*(-3)/(-3) = 9/(-3) = -3

	// Test with zero (should return None due to division by zero)
	assert_eq!(run("abs_like(0)"), None);
}

#[test]
fn test_function_mathematical_operations() {
	let _guard = TEST_MUTEX
		.lock()
		.unwrap_or_else(|poisoned| poisoned.into_inner());
	clear_variables();
	clear_functions();

	// Define mathematical functions
	assert_eq!(run("fn cube(x) { x * x * x }"), None);
	assert_eq!(run("fn avg(a, b) { (a + b) / 2 }"), None);
	assert_eq!(
		run("fn distance(x1, y1, x2, y2) { ((x2 - x1) * (x2 - x1) + (y2 - y1) * (y2 - y1)) }"),
		None
	);

	// Test cube function
	assert_eq!(run("cube(3)"), Some(27.0)); // 3^3 = 27
	assert_eq!(run("cube(-2)"), Some(-8.0)); // (-2)^3 = -8

	// Test average function
	assert_eq!(run("avg(10, 20)"), Some(15.0)); // (10+20)/2 = 15
	assert_eq!(run("avg(-5, 5)"), Some(0.0)); // (-5+5)/2 = 0

	// Test distance squared function (avoiding square root)
	assert_eq!(run("distance(0, 0, 3, 4)"), Some(25.0)); // 3^2 + 4^2 = 9 + 16 = 25
	assert_eq!(run("distance(1, 1, 4, 5)"), Some(25.0)); // (4-1)^2 + (5-1)^2 = 9 + 16 = 25
}

#[test]
fn test_function_with_large_expressions() {
	let _guard = TEST_MUTEX
		.lock()
		.unwrap_or_else(|poisoned| poisoned.into_inner());
	clear_variables();
	clear_functions();

	// Define function with very large expression
	assert_eq!(
		run("fn polynomial(x) { x * x * x * x + 3 * x * x * x + 2 * x * x + x + 1 }"),
		None
	);
	assert!(function_exists("polynomial"));

	// Test polynomial function: x^4 + 3x^3 + 2x^2 + x + 1
	assert_eq!(run("polynomial(0)"), Some(1.0)); // 0 + 0 + 0 + 0 + 1 = 1
	assert_eq!(run("polynomial(1)"), Some(8.0)); // 1 + 3 + 2 + 1 + 1 = 8
	assert_eq!(run("polynomial(2)"), Some(49.0)); // 16 + 24 + 8 + 2 + 1 = 51... wait let me recalculate
	// 2^4 + 3*2^3 + 2*2^2 + 2 + 1 = 16 + 24 + 8 + 2 + 1 = 51
	assert_eq!(run("polynomial(2)"), Some(51.0));
}

#[test]
fn test_mixed_function_types_interaction() {
	let _guard = TEST_MUTEX
		.lock()
		.unwrap_or_else(|poisoned| poisoned.into_inner());
	clear_variables();
	clear_functions();

	// Define both named and lambda functions
	assert_eq!(run("fn named_add(a, b) { a + b }"), None);
	assert_eq!(run("lambda_multiply = (x, y) => {x * y}"), None);
	assert_eq!(run("fn named_subtract(a, b) { a - b }"), None);
	assert_eq!(run("lambda_divide = (x, y) => {x / y}"), None);

	// Test interactions between different function types
	assert_eq!(run("result1 = named_add(5, 3)"), Some(8.0));
	assert_eq!(run("result2 = lambda_multiply(4, 2)"), Some(8.0));
	assert_eq!(
		run("combined = named_subtract(result1, result2)"),
		Some(0.0)
	); // 8 - 8 = 0

	// Test nested calls mixing function types
	assert_eq!(
		run("nested = lambda_divide(named_add(10, 5), lambda_multiply(3, 1))"),
		Some(5.0)
	);
	// named_add(10, 5) = 15, lambda_multiply(3, 1) = 3, lambda_divide(15, 3) = 5
}

#[test]
fn test_function_stress_test() {
	let _guard = TEST_MUTEX
		.lock()
		.unwrap_or_else(|poisoned| poisoned.into_inner());
	clear_variables();
	clear_functions();

	// Define many functions
	for i in 1..=10 {
		let func_def = format!("fn func{}(x) {{ x + {} }}", i, i);
		assert_eq!(run(&func_def), None);
		assert!(function_exists(&format!("func{}", i)));
	}

	// Test all functions
	for i in 1..=10 {
		let func_call = format!("func{}(10)", i);
		let expected = 10.0 + i as f64;
		assert_eq!(run(&func_call), Some(expected));
	}

	// Test chaining many function calls
	assert_eq!(run("func1(func2(func3(0)))"), Some(6.0)); // func3(0)=3, func2(3)=5, func1(5)=6
}

#[test]
fn test_edge_case_function_names() {
	let _guard = TEST_MUTEX
		.lock()
		.unwrap_or_else(|poisoned| poisoned.into_inner());
	clear_variables();
	clear_functions();

	// Test edge case function names
	assert_eq!(run("fn a(x) { x }"), None);
	assert_eq!(run("fn _a(x) { x * 2 }"), None);
	assert_eq!(run("fn a1(x) { x * 3 }"), None);
	assert_eq!(run("fn _1a(x) { x * 4 }"), None);
	assert_eq!(
		run("fn very_long_function_name_that_should_work(x) { x * 5 }"),
		None
	);

	// Test all functions work
	assert_eq!(run("a(1)"), Some(1.0));
	assert_eq!(run("_a(1)"), Some(2.0));
	assert_eq!(run("a1(1)"), Some(3.0));
	assert_eq!(run("_1a(1)"), Some(4.0));
	assert_eq!(
		run("very_long_function_name_that_should_work(1)"),
		Some(5.0)
	);
}

#[test]
fn test_function_with_arithmetic_precedence() {
	let _guard = TEST_MUTEX
		.lock()
		.unwrap_or_else(|poisoned| poisoned.into_inner());
	clear_variables();
	clear_functions();

	// Define function that tests arithmetic precedence
	assert_eq!(run("fn precedence_test(a, b, c) { a + b * c }"), None);
	assert!(function_exists("precedence_test"));

	// Test that multiplication happens before addition
	assert_eq!(run("precedence_test(2, 3, 4)"), Some(14.0)); // 2 + (3 * 4) = 2 + 12 = 14
	assert_eq!(run("precedence_test(10, 2, 3)"), Some(16.0)); // 10 + (2 * 3) = 10 + 6 = 16

	// Define function with more complex precedence
	assert_eq!(
		run("fn complex_precedence(x, y, z) { x * y / z + x - y }"),
		None
	);
	assert!(function_exists("complex_precedence"));

	// Test complex precedence: (x * y) / z + x - y
	assert_eq!(run("complex_precedence(6, 4, 2)"), Some(10.0)); // (6 * 4) / 2 + 6 - 4 = 24/2 + 6 - 4 = 12 + 6 - 4 = 14
}

#[test]
fn test_function_return_values_in_expressions() {
	let _guard = TEST_MUTEX
		.lock()
		.unwrap_or_else(|poisoned| poisoned.into_inner());
	clear_variables();
	clear_functions();

	// Define utility functions
	assert_eq!(run("fn triple(x) { x * 3 }"), None);
	assert_eq!(run("fn halve(x) { x / 2 }"), None);

	// Test using function return values in complex expressions
	assert_eq!(run("result = triple(4) + halve(10) * 2"), Some(22.0));
	// triple(4) = 12, halve(10) = 5, 5 * 2 = 10, 12 + 10 = 22

	assert_eq!(
		run("complex = triple(halve(8)) - halve(triple(2))"),
		Some(9.0)
	);
	// halve(8) = 4, triple(4) = 12, triple(2) = 6, halve(6) = 3, 12 - 3 = 9

	// Test function calls in assignment expressions
	assert_eq!(run("x = triple(3)"), Some(9.0));
	assert_eq!(run("y = halve(x)"), Some(4.5)); // halve(9) = 4.5
	assert_eq!(get_variable("x"), Some(9.0));
	assert_eq!(get_variable("y"), Some(4.5));
}
